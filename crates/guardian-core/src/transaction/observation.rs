//! The transaction observation contract (TDD contract §15; G4 handoff §15).
//! "A provider returning 'method call succeeded' MUST NOT automatically
//! mean the transaction succeeded" -- `Observe`'s job is purely "did the
//! expected state occur," using the fixture's reported state. No G5
//! diagnostic-budget/PSI logic exists here.

/// Typed observation policy fields -- not a free-form opaque callback.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ObservationPolicy {
    pub expected_properties: Vec<String>,
    pub forbidden_properties: Vec<String>,
    pub minimum_observation_duration_ms: u64,
    pub maximum_observation_duration_ms: u64,
    pub health_checks: Vec<String>,
    pub commit_condition: String,
    pub rollback_condition: String,
}

/// A single observation result -- kept structurally distinct from "the
/// provider call itself succeeded" (that is `ApplyOutcome::ConfirmedSuccess`,
/// a different fact entirely).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationOutcome {
    PostconditionMet,
    PostconditionNotMet,
    Ambiguous,
}
