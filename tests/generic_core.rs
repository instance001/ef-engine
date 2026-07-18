use std::time::{SystemTime, UNIX_EPOCH};

use ef_engine::{
    apply_attempt, load_state_from_path, metabolize_constraint, save_state_to_path, AttemptInput,
    AttemptOutcomeKind, EngineSaveEnvelope, EngineState,
};

#[test]
fn failed_attempt_is_vaulted() {
    let mut state = EngineState::new();

    let outcome = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_path")
            .with_summary("Tried the direct route")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    assert_eq!(outcome.vault_entry_id.as_deref(), Some("vault-1"));
    assert_eq!(state.vault.len(), 1);
    assert_eq!(state.vault[0].failure_class, "surface_gap");
}

#[test]
fn retry_delta_records_meaningful_change() {
    let mut state = EngineState::new();

    let first = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_path")
            .with_summary("First attempt")
            .with_outcome(AttemptOutcomeKind::Stuck)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    let _second = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-b", "smaller_decomposition")
            .with_summary("Changed approach")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface")
            .with_retry_parent(first.attempt_id),
    );

    let retry_delta = state.attempt("attempt-b").unwrap().retry_delta.as_ref().unwrap();
    assert_eq!(retry_delta.from_attempt_id.as_deref(), Some("attempt-a"));
    assert!(retry_delta.changed_method);
    assert!(retry_delta.changed_summary);
}

#[test]
fn metabolization_creates_constraint_and_triangulation() {
    let mut state = EngineState::new();

    let first = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_path")
            .with_summary("First attempt")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    let second = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-b", "smaller_decomposition")
            .with_summary("Second attempt")
            .with_outcome(AttemptOutcomeKind::Stuck)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface")
            .with_retry_parent(first.attempt_id),
    );

    let constraint = metabolize_constraint(
        &mut state,
        second.vault_entry_id.as_deref().unwrap(),
        "Avoid patch routes when the surface is missing.",
    )
    .unwrap();

    assert_eq!(constraint.source_vault_id, "vault-1");
    assert_eq!(constraint.forbidden_pattern, "missing_patch_surface");
    assert_eq!(state.triangulation_sessions.len(), 1);
    assert_eq!(
        state.triangulation_sessions[0].shared_lock_point,
        "missing_patch_surface"
    );
    assert_eq!(state.vault[0].status, ef_engine::engine::VaultStatus::Metabolized);
}

#[test]
fn metabolized_constraint_blocks_related_future_attempts() {
    let mut state = EngineState::new();

    let initial = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_path")
            .with_summary("First attempt")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    metabolize_constraint(
        &mut state,
        initial.vault_entry_id.as_deref().unwrap(),
        "Avoid patch routes when the surface is missing.",
    )
    .unwrap();

    let blocked = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-b", "alternate_direct_path")
            .with_summary("Same underlying lock")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    let routing = blocked.routing_decision.unwrap();
    assert!(!routing.allowed);
    assert_eq!(routing.blocked_by_constraint_ids, vec!["constraint-1".to_string()]);
    assert_eq!(blocked.vault_entry_id, None);
}

#[test]
fn save_roundtrip_preserves_engine_state() {
    let mut state = EngineState::new();
    let first = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_path")
            .with_summary("First attempt")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );
    metabolize_constraint(
        &mut state,
        first.vault_entry_id.as_deref().unwrap(),
        "Avoid patch routes when the surface is missing.",
    )
    .unwrap();

    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    path.push(format!("ef_engine_state_{nonce}.json"));

    let payload = EngineSaveEnvelope {
        save_version: 1,
        host_id: "generic-test-host".to_string(),
        notes: vec!["roundtrip".to_string()],
        state: state.clone(),
    };

    save_state_to_path(&path, &payload).expect("save should succeed");
    let restored = load_state_from_path(&path).expect("load should succeed");
    std::fs::remove_file(&path).ok();

    assert_eq!(restored.host_id, payload.host_id);
    assert_eq!(restored.notes, payload.notes);
    assert_eq!(restored.state.constraints.len(), 1);
    assert_eq!(restored.state.vault.len(), 1);
    assert_eq!(restored.state.attempts.len(), 1);
}
