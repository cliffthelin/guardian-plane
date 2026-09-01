//! Apply-intent vs. Apply-outcome (G4 handoff §18.2). `state == Applying`
//! alone is never proof that a mutation occurred -- durable Apply-intent
//! (recorded before the provider is ever invoked) and durable Apply-outcome
//! (recorded after) are two separate facts.

use std::fmt;

/// What Guardian durably knows about a specific `Apply` attempt's outcome.
/// `NotRecorded` is the state immediately after intent is persisted but
/// before the provider has been invoked -- it is not itself evidence that
/// the provider *was* invoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyOutcome {
    NotRecorded,
    ConfirmedSuccess,
    ConfirmedFailureNoMutation,
    PartialOrUncertainMutation,
    ResponseLostOrUnknown,
}

impl ApplyOutcome {
    #[must_use]
    pub const fn wire_token(self) -> &'static str {
        match self {
            Self::NotRecorded => "not_recorded",
            Self::ConfirmedSuccess => "confirmed_success",
            Self::ConfirmedFailureNoMutation => "confirmed_failure_no_mutation",
            Self::PartialOrUncertainMutation => "partial_or_uncertain_mutation",
            Self::ResponseLostOrUnknown => "response_lost_or_unknown",
        }
    }
}

impl fmt::Display for ApplyOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_token())
    }
}

impl std::str::FromStr for ApplyOutcome {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "confirmed_success" => Self::ConfirmedSuccess,
            "confirmed_failure_no_mutation" => Self::ConfirmedFailureNoMutation,
            "partial_or_uncertain_mutation" => Self::PartialOrUncertainMutation,
            "response_lost_or_unknown" => Self::ResponseLostOrUnknown,
            _ => Self::NotRecorded,
        })
    }
}

/// The durable record of one `Apply` attempt for a given `idempotency_key`.
/// `attempt_started_at` is persisted as the **Apply-intent** fact, durably,
/// *before* the provider is ever invoked (G4 handoff §19.1 step 2) --
/// `outcome` is the separate **Apply-outcome** fact, persisted only after
/// the provider call and its result are known (step 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyRecord {
    pub idempotency_key: String,
    /// Logical attempt sequence number -- monotonically increasing per
    /// process, sufficient to order attempts without depending on wall-clock
    /// behavior across a simulated crash/restart.
    pub attempt_started_at: u64,
    pub outcome: ApplyOutcome,
}

impl ApplyRecord {
    #[must_use]
    pub const fn intent_only(idempotency_key: String, attempt_started_at: u64) -> Self {
        Self {
            idempotency_key,
            attempt_started_at,
            outcome: ApplyOutcome::NotRecorded,
        }
    }
}
