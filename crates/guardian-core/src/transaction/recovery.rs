//! Restart-recovery classification (P0-TXN-011; G4 handoff §20). Six
//! distinguishable outcomes for a recovered *nonterminal* transaction --
//! not new transaction states, but the required output of the recovery
//! function that decides what must happen next.

use crate::transaction::apply::ApplyOutcome;
use crate::transaction::observation::ObservationOutcome;
use crate::transaction::state::TransactionState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryClassification {
    SafeToResume,
    MustObserve,
    MustRollback,
    AlreadyCommitted,
    StateAmbiguous,
    RequiresHumanRecovery,
}

/// Inputs available to the recovery classifier -- deliberately narrow: only
/// what is actually durable, never anything a recovering process would
/// have to guess.
#[derive(Clone, Copy, Debug)]
pub struct RecoverySnapshot {
    pub state: TransactionState,
    pub apply_outcome: Option<ApplyOutcome>,
    pub last_observation: Option<ObservationOutcome>,
}

/// Classifies a recovered nonterminal transaction (G4 handoff §20). A
/// corrupt/unparseable persisted record never reaches this function at all
/// -- see [`crate::transaction::persistence`] -- it is classified
/// `RequiresHumanRecovery` directly at load time.
#[must_use]
pub fn classify(snapshot: RecoverySnapshot) -> RecoveryClassification {
    use TransactionState::{Applying, Observing, RollingBack};

    match snapshot.state {
        state if state.is_pre_mutation() => RecoveryClassification::SafeToResume,
        Applying => classify_applying(snapshot.apply_outcome),
        Observing => classify_observing(snapshot.last_observation),
        RollingBack => RecoveryClassification::MustRollback,
        TransactionState::Committed => RecoveryClassification::AlreadyCommitted,
        // RollbackFailed ("an unresolved rollback with no further automated
        // path"), and defensively any other fully-resolved terminal state a
        // caller should not have passed in here at all: both fail closed to
        // the same answer rather than guessing differently for the
        // supposedly-unreachable branch.
        _ => RecoveryClassification::RequiresHumanRecovery,
    }
}

fn classify_applying(apply_outcome: Option<ApplyOutcome>) -> RecoveryClassification {
    match apply_outcome {
        // No Apply-intent durably recorded, or intent recorded but the
        // provider was never invoked, or the provider call cleanly failed
        // with no mutation: all three are provably safe to resume from
        // Authorized (G4 handoff §19.1: "crash before/at step 2/4").
        None | Some(ApplyOutcome::NotRecorded | ApplyOutcome::ConfirmedFailureNoMutation) => {
            RecoveryClassification::SafeToResume
        }
        // The provider call completed (or is believed to have completed --
        // ResponseLostOrUnknown, §19.1 "crash between step 4 and 5") and
        // Observe can still meaningfully answer what happened either way.
        Some(ApplyOutcome::ConfirmedSuccess | ApplyOutcome::ResponseLostOrUnknown) => {
            RecoveryClassification::MustObserve
        }
        // The provider call was in flight when the crash occurred -- whether
        // the external mutation happened is inherently uncertain from
        // Guardian's own persistence alone (§19.1, "crash during step 4").
        // Never guessed as either extreme.
        Some(ApplyOutcome::PartialOrUncertainMutation) => RecoveryClassification::StateAmbiguous,
    }
}

const fn classify_observing(
    last_observation: Option<ObservationOutcome>,
) -> RecoveryClassification {
    match last_observation {
        // No observation recorded yet, or the last one met the
        // postcondition: either way the remaining work is
        // Observe-then-Confirm, which this classification covers.
        None | Some(ObservationOutcome::PostconditionMet) => RecoveryClassification::MustObserve,
        Some(ObservationOutcome::PostconditionNotMet) => RecoveryClassification::MustRollback,
        Some(ObservationOutcome::Ambiguous) => RecoveryClassification::StateAmbiguous,
    }
}
