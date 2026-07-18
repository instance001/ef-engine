use ef_engine::{
    apply_attempt, metabolize_constraint, AttemptInput, AttemptOutcomeKind, EngineState,
};

fn main() {
    let mut state = EngineState::new();

    let attempts = vec![
        AttemptInput::new("attempt-a", "direct_patch")
            .with_summary("Tried the default direct patch route.")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
        AttemptInput::new("attempt-b", "narrow_direct_patch")
            .with_summary("Reduced scope but still targeted the missing patch surface.")
            .with_outcome(AttemptOutcomeKind::Stuck)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface")
            .with_retry_parent("attempt-a"),
        AttemptInput::new("attempt-c", "alt_tool_same_surface")
            .with_summary("Changed tool posture but kept the same missing surface assumption.")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface")
            .with_retry_parent("attempt-b"),
    ];

    let mut last_vault_id = None;
    for input in attempts {
        let outcome = apply_attempt(&mut state, input);
        last_vault_id = outcome.vault_entry_id;
    }

    let constraint = metabolize_constraint(
        &mut state,
        last_vault_id
            .as_deref()
            .expect("cascade should produce a vault entry"),
        "Build or verify the target patch surface before attempting patch-style routes.",
    )
    .expect("vault should metabolize");

    let blocked_routes = vec![
        apply_attempt(
            &mut state,
            AttemptInput::new("attempt-d", "renamed_direct_patch")
                .with_summary("Looked different by name only.")
                .with_outcome(AttemptOutcomeKind::Failed)
                .with_failure_class("surface_gap")
                .with_lock_signal("missing_patch_surface"),
        ),
        apply_attempt(
            &mut state,
            AttemptInput::new("attempt-e", "tool_switch_same_assumption")
                .with_summary("Changed tool but preserved the same missing surface assumption.")
                .with_outcome(AttemptOutcomeKind::Failed)
                .with_failure_class("surface_gap")
                .with_lock_signal("missing_patch_surface"),
        ),
    ];

    println!("== Eureka Cascade Demo ==");
    println!();
    println!("Triangulated failure cluster metabolized into:");
    println!("  id: {}", constraint.constraint_id);
    println!("  forbidden_pattern: {}", constraint.forbidden_pattern);
    println!("  replacement_guidance: {}", constraint.replacement_guidance);
    println!();
    println!("Blocked future attempts:");
    for blocked in blocked_routes {
        println!("  attempt: {}", blocked.attempt_id);
        if let Some(routing) = blocked.routing_decision {
            println!("    allowed: {}", routing.allowed);
            println!(
                "    blocked_by_constraint_ids: {}",
                routing.blocked_by_constraint_ids.join(", ")
            );
        }
    }
    println!();
    println!("{}", serde_json::to_string_pretty(&state).unwrap());
}
