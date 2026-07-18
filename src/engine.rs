use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EngineState {
    #[serde(default)]
    pub attempts: Vec<AttemptRecord>,
    #[serde(default)]
    pub vault: Vec<VaultEntry>,
    #[serde(default)]
    pub constraints: Vec<ConstraintRecord>,
    #[serde(default)]
    pub triangulation_sessions: Vec<TriangulationSession>,
    #[serde(default)]
    pub route_log: Vec<RoutingDecision>,
    #[serde(default)]
    pub log: Vec<String>,
}

impl EngineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attempt(&self, attempt_id: &str) -> Option<&AttemptRecord> {
        self.attempts.iter().find(|attempt| attempt.attempt_id == attempt_id)
    }

    pub fn vault_entry(&self, vault_id: &str) -> Option<&VaultEntry> {
        self.vault.iter().find(|entry| entry.vault_id == vault_id)
    }

    pub fn constraint(&self, constraint_id: &str) -> Option<&ConstraintRecord> {
        self.constraints
            .iter()
            .find(|constraint| constraint.constraint_id == constraint_id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptInput {
    pub attempt_id: String,
    pub method: String,
    #[serde(default)]
    pub summary: String,
    pub outcome: AttemptOutcomeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    #[serde(default)]
    pub lock_signals: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_parent_attempt_id: Option<String>,
}

impl AttemptInput {
    pub fn new(attempt_id: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            attempt_id: attempt_id.into(),
            method: method.into(),
            summary: String::new(),
            outcome: AttemptOutcomeKind::Succeeded,
            failure_class: None,
            lock_signals: Vec::new(),
            retry_parent_attempt_id: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn with_outcome(mut self, outcome: AttemptOutcomeKind) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_failure_class(mut self, failure_class: impl Into<String>) -> Self {
        self.failure_class = Some(failure_class.into());
        self
    }

    pub fn with_lock_signal(mut self, lock_signal: impl Into<String>) -> Self {
        self.lock_signals.push(lock_signal.into());
        self
    }

    pub fn with_retry_parent(mut self, attempt_id: impl Into<String>) -> Self {
        self.retry_parent_attempt_id = Some(attempt_id.into());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttemptOutcomeKind {
    Succeeded,
    Failed,
    Stuck,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub method: String,
    pub summary: String,
    pub outcome: AttemptOutcomeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    #[serde(default)]
    pub lock_signals: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_delta: Option<RetryDelta>,
}

impl AttemptRecord {
    pub fn is_failed_or_stuck(&self) -> bool {
        matches!(
            self.outcome,
            AttemptOutcomeKind::Failed | AttemptOutcomeKind::Stuck
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_attempt_id: Option<String>,
    pub changed_method: bool,
    pub changed_summary: bool,
    pub changed_lock_signals: bool,
}

impl RetryDelta {
    pub fn is_meaningfully_different(&self) -> bool {
        self.changed_method || self.changed_summary || self.changed_lock_signals
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEntry {
    pub vault_id: String,
    pub source_attempt_ids: Vec<String>,
    pub failure_class: String,
    #[serde(default)]
    pub lock_signals: Vec<String>,
    pub status: VaultStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VaultStatus {
    Open,
    Metabolized,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriangulationSession {
    pub triangulation_id: String,
    pub compared_attempt_ids: Vec<String>,
    pub shared_lock_point: String,
    pub failure_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstraintRecord {
    pub constraint_id: String,
    pub source_vault_id: String,
    pub failure_class: String,
    pub forbidden_pattern: String,
    pub replacement_guidance: String,
    #[serde(default)]
    pub lock_signals: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub attempt_id: String,
    pub allowed: bool,
    #[serde(default)]
    pub blocked_by_constraint_ids: Vec<String>,
    #[serde(default)]
    pub rationale: Vec<String>,
}

impl RoutingDecision {
    pub fn is_blocked(&self) -> bool {
        !self.allowed
    }
}

#[derive(Clone, Debug, Default)]
pub struct EngineOutcome {
    pub attempt_id: String,
    pub vault_entry_id: Option<String>,
    pub routing_decision: Option<RoutingDecision>,
    pub lines: Vec<String>,
}

pub fn apply_attempt(state: &mut EngineState, input: AttemptInput) -> EngineOutcome {
    let prior_attempt = input
        .retry_parent_attempt_id
        .as_deref()
        .and_then(|attempt_id| state.attempt(attempt_id))
        .cloned();

    let attempt = AttemptRecord {
        attempt_id: input.attempt_id.clone(),
        method: input.method.clone(),
        summary: input.summary.clone(),
        outcome: input.outcome.clone(),
        failure_class: input.failure_class.clone(),
        lock_signals: input.lock_signals.clone(),
        retry_delta: Some(build_retry_delta(&input, prior_attempt.as_ref())),
    };

    let routing_decision = evaluate_routing(state, &attempt);
    let mut lines = vec![format!(
        "Attempt '{}' used method '{}'.",
        attempt.attempt_id, attempt.method
    )];

    if !routing_decision.allowed {
        lines.push(format!(
            "Attempt '{}' is blocked by metabolized constraints.",
            attempt.attempt_id
        ));
        state.route_log.push(routing_decision.clone());
        state.attempts.push(attempt);
        state.log.extend(lines.clone());
        return EngineOutcome {
            attempt_id: input.attempt_id,
            vault_entry_id: None,
            routing_decision: Some(routing_decision),
            lines,
        };
    }

    let vault_entry_id = if matches!(input.outcome, AttemptOutcomeKind::Failed | AttemptOutcomeKind::Stuck) {
        Some(vault_failure(state, &attempt))
    } else {
        None
    };

    if let Some(vault_id) = vault_entry_id.as_ref() {
        lines.push(format!("Failure vaulted as '{}'.", vault_id));
    } else {
        lines.push("Attempt completed without vaulting.".to_string());
    }

    state.route_log.push(routing_decision.clone());
    state.attempts.push(attempt);
    state.log.extend(lines.clone());

    EngineOutcome {
        attempt_id: input.attempt_id,
        vault_entry_id,
        routing_decision: Some(routing_decision),
        lines,
    }
}

pub fn metabolize_constraint(
    state: &mut EngineState,
    vault_id: &str,
    replacement_guidance: &str,
) -> Result<ConstraintRecord, String> {
    let Some(vault_index) = state.vault.iter().position(|entry| entry.vault_id == vault_id) else {
        return Err(format!("Vault entry '{}' was not found.", vault_id));
    };

    let failure_class = state.vault[vault_index].failure_class.clone();
    let lock_signals = state.vault[vault_index].lock_signals.clone();
    let source_attempt_ids = state.vault[vault_index].source_attempt_ids.clone();

    let shared_lock_point = if lock_signals.is_empty() {
        "unknown_lock_point".to_string()
    } else {
        most_common_lock_signal(&lock_signals).unwrap_or_else(|| "unknown_lock_point".to_string())
    };

    let triangulation_id = format!("triangulation-{}", state.triangulation_sessions.len() + 1);
    state.triangulation_sessions.push(TriangulationSession {
        triangulation_id,
        compared_attempt_ids: source_attempt_ids,
        shared_lock_point: shared_lock_point.clone(),
        failure_class: failure_class.clone(),
    });

    let constraint = ConstraintRecord {
        constraint_id: format!("constraint-{}", state.constraints.len() + 1),
        source_vault_id: vault_id.to_string(),
        failure_class,
        forbidden_pattern: shared_lock_point,
        replacement_guidance: replacement_guidance.to_string(),
        lock_signals: lock_signals.clone(),
    };

    state.vault[vault_index].status = VaultStatus::Metabolized;
    state.constraints.push(constraint.clone());
    state.log.push(format!(
        "Vault '{}' metabolized into constraint '{}'.",
        vault_id, constraint.constraint_id
    ));

    Ok(constraint)
}

fn build_retry_delta(input: &AttemptInput, prior_attempt: Option<&AttemptRecord>) -> RetryDelta {
    match prior_attempt {
        Some(previous) => RetryDelta {
            from_attempt_id: Some(previous.attempt_id.clone()),
            changed_method: previous.method != input.method,
            changed_summary: previous.summary != input.summary,
            changed_lock_signals: previous.lock_signals != input.lock_signals,
        },
        None => RetryDelta {
            from_attempt_id: None,
            changed_method: true,
            changed_summary: true,
            changed_lock_signals: !input.lock_signals.is_empty(),
        },
    }
}

fn vault_failure(state: &mut EngineState, attempt: &AttemptRecord) -> String {
    let failure_class = attempt
        .failure_class
        .clone()
        .unwrap_or_else(|| "unknown_failure_class".to_string());

    if let Some(existing) = state.vault.iter_mut().find(|entry| {
        entry.failure_class == failure_class && entry.status == VaultStatus::Open
    }) {
        if !existing.source_attempt_ids.contains(&attempt.attempt_id) {
            existing.source_attempt_ids.push(attempt.attempt_id.clone());
        }
        for signal in &attempt.lock_signals {
            if !existing.lock_signals.contains(signal) {
                existing.lock_signals.push(signal.clone());
            }
        }
        return existing.vault_id.clone();
    }

    let vault_id = format!("vault-{}", state.vault.len() + 1);
    state.vault.push(VaultEntry {
        vault_id: vault_id.clone(),
        source_attempt_ids: vec![attempt.attempt_id.clone()],
        failure_class,
        lock_signals: attempt.lock_signals.clone(),
        status: VaultStatus::Open,
    });
    vault_id
}

fn evaluate_routing(state: &EngineState, attempt: &AttemptRecord) -> RoutingDecision {
    let mut blocked_by_constraint_ids = Vec::new();
    let mut rationale = Vec::new();

    for constraint in &state.constraints {
        let lock_match = attempt
            .lock_signals
            .iter()
            .any(|signal| constraint.lock_signals.contains(signal));
        let failure_class_match = attempt.failure_class.as_deref() == Some(constraint.failure_class.as_str());

        if lock_match || failure_class_match {
            blocked_by_constraint_ids.push(constraint.constraint_id.clone());
            rationale.push(format!(
                "Constraint '{}' blocks failure class '{}' with forbidden pattern '{}'.",
                constraint.constraint_id, constraint.failure_class, constraint.forbidden_pattern
            ));
        }
    }

    RoutingDecision {
        attempt_id: attempt.attempt_id.clone(),
        allowed: blocked_by_constraint_ids.is_empty(),
        blocked_by_constraint_ids,
        rationale,
    }
}

fn most_common_lock_signal(lock_signals: &[String]) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for signal in lock_signals {
        *counts.entry(signal.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(signal, _)| signal)
}
