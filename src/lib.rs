//! `EF Engine` is a tiny `Entropy Folding` core.
//!
//! The goal is legibility:
//! copy the pieces you want, rename the nouns, and adapt the mechanism to your
//! own host without inheriting a large framework.

pub mod engine;
pub mod save;

pub use engine::{
    apply_attempt, metabolize_constraint, AttemptInput, AttemptOutcomeKind, AttemptRecord,
    ConstraintRecord, EngineOutcome, EngineState, RetryDelta, RoutingDecision,
    TriangulationSession, VaultEntry,
    VaultStatus,
};
pub use save::{load_state_from_path, save_state_to_path, EngineSaveEnvelope};
