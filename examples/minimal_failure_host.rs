use ef_engine::{
    apply_attempt, metabolize_constraint, AttemptInput, AttemptOutcomeKind, EngineState,
};

fn main() {
    let mut state = EngineState::new();

    let first = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-a", "direct_patch")
            .with_summary("Tried the direct patch route against the target surface.")
            .with_outcome(AttemptOutcomeKind::Stuck)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    let second = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-b", "smaller_decomposition")
            .with_summary("Retried with a smaller decomposition and the same target.")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface")
            .with_retry_parent(&first.attempt_id),
    );

    let constraint = metabolize_constraint(
        &mut state,
        second
            .vault_entry_id
            .as_deref()
            .expect("second attempt should be vaulted"),
        "Avoid direct patch routes when the target surface does not exist yet.",
    )
    .expect("vault should metabolize");

    let blocked = apply_attempt(
        &mut state,
        AttemptInput::new("attempt-c", "alternate_direct_patch")
            .with_summary("Tried a renamed direct route against the same missing surface.")
            .with_outcome(AttemptOutcomeKind::Failed)
            .with_failure_class("surface_gap")
            .with_lock_signal("missing_patch_surface"),
    );

    println!("== Minimal Failure Host ==");
    println!();
    println!("First attempt:");
    for line in first.lines {
        println!("  {}", line);
    }
    println!();
    println!("Second attempt:");
    for line in second.lines {
        println!("  {}", line);
    }
    println!();
    println!("Metabolized constraint:");
    println!("  id: {}", constraint.constraint_id);
    println!("  failure_class: {}", constraint.failure_class);
    println!("  forbidden_pattern: {}", constraint.forbidden_pattern);
    println!("  replacement_guidance: {}", constraint.replacement_guidance);
    println!();
    println!("Blocked future route:");
    for line in blocked.lines {
        println!("  {}", line);
    }
    if let Some(routing) = blocked.routing_decision {
        println!("  allowed: {}", routing.allowed);
        println!(
            "  blocked_by_constraint_ids: {}",
            routing.blocked_by_constraint_ids.join(", ")
        );
    }
    println!();
    println!("{}", serde_json::to_string_pretty(&state).unwrap());
}
