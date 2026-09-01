//! P0-TXN-011 restart recovery: all six classifications, each proven with a
//! purpose-built fixture (G4 handoff §20).

use guardian_core::transaction::observation::ObservationOutcome;
use guardian_core::transaction::recovery::{RecoveryClassification, RecoverySnapshot, classify};
use guardian_core::transaction::{ApplyOutcome, TransactionState};

fn snapshot(
    state: TransactionState,
    apply_outcome: Option<ApplyOutcome>,
    last_observation: Option<ObservationOutcome>,
) -> RecoverySnapshot {
    RecoverySnapshot {
        state,
        apply_outcome,
        last_observation,
    }
}

/// "safe to resume (nonterminal, no Apply attempted yet)"
#[test]
fn safe_to_resume_when_no_apply_attempted() {
    for state in [
        TransactionState::Created,
        TransactionState::Validating,
        TransactionState::Validated,
        TransactionState::Authorizing,
        TransactionState::Authorized,
    ] {
        assert_eq!(
            classify(snapshot(state, None, None)),
            RecoveryClassification::SafeToResume
        );
    }
}

/// Crash-before-invocation: Apply-intent exists (or doesn't) but the
/// provider was provably never invoked -- still safe to resume (G4 handoff
/// §18.6/§19.1).
#[test]
fn safe_to_resume_when_applying_but_provider_never_invoked() {
    let outcome = classify(snapshot(
        TransactionState::Applying,
        Some(ApplyOutcome::NotRecorded),
        None,
    ));
    assert_eq!(outcome, RecoveryClassification::SafeToResume);
    assert_ne!(outcome, RecoveryClassification::AlreadyCommitted);
}

/// "must observe (Apply attempted, no Observe result recorded)"
#[test]
fn must_observe_when_apply_confirmed_success_but_not_yet_observed() {
    assert_eq!(
        classify(snapshot(
            TransactionState::Applying,
            Some(ApplyOutcome::ConfirmedSuccess),
            None
        )),
        RecoveryClassification::MustObserve
    );
}

/// "must rollback (Apply attempted, Observe failed/ambiguous, rollback not
/// yet attempted)"
#[test]
fn must_rollback_when_observation_determined_postcondition_not_met() {
    assert_eq!(
        classify(snapshot(
            TransactionState::Observing,
            Some(ApplyOutcome::ConfirmedSuccess),
            Some(ObservationOutcome::PostconditionNotMet)
        )),
        RecoveryClassification::MustRollback
    );
}

/// "already committed (Apply + Observe + Confirm all recorded successful)"
#[test]
fn already_committed_when_state_is_committed() {
    assert_eq!(
        classify(snapshot(
            TransactionState::Committed,
            Some(ApplyOutcome::ConfirmedSuccess),
            None
        )),
        RecoveryClassification::AlreadyCommitted
    );
}

/// "state ambiguous (Apply attempted, response lost, no Observe possible)"
#[test]
fn state_ambiguous_when_apply_outcome_is_partial_or_uncertain() {
    assert_eq!(
        classify(snapshot(
            TransactionState::Applying,
            Some(ApplyOutcome::PartialOrUncertainMutation),
            None
        )),
        RecoveryClassification::StateAmbiguous
    );
}

#[test]
fn state_ambiguous_when_observation_itself_was_ambiguous() {
    assert_eq!(
        classify(snapshot(
            TransactionState::Observing,
            Some(ApplyOutcome::ConfirmedSuccess),
            Some(ObservationOutcome::Ambiguous)
        )),
        RecoveryClassification::StateAmbiguous
    );
}

/// "requires human/recovery handling (... `ROLLBACK_FAILED` with no further
/// automated path)"
#[test]
fn requires_human_recovery_when_rollback_failed() {
    assert_eq!(
        classify(snapshot(
            TransactionState::RollbackFailed,
            Some(ApplyOutcome::PartialOrUncertainMutation),
            None
        )),
        RecoveryClassification::RequiresHumanRecovery
    );
}

/// Rolling-back-still-in-progress must resume rollback, never be silently
/// treated as resolved.
#[test]
fn must_rollback_when_rolling_back_was_interrupted() {
    assert_eq!(
        classify(snapshot(
            TransactionState::RollingBack,
            Some(ApplyOutcome::PartialOrUncertainMutation),
            None
        )),
        RecoveryClassification::MustRollback
    );
}

/// Lost-response case specifically: the call is believed to have completed,
/// so Observe (not a guess) is the required next step -- never silently
/// `AlreadyCommitted`.
#[test]
fn response_lost_routes_to_must_observe_not_already_committed() {
    let outcome = classify(snapshot(
        TransactionState::Applying,
        Some(ApplyOutcome::ResponseLostOrUnknown),
        None,
    ));
    assert_eq!(outcome, RecoveryClassification::MustObserve);
    assert_ne!(outcome, RecoveryClassification::AlreadyCommitted);
}
