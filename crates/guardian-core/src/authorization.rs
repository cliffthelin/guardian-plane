//! Authorization request/decision abstractions for G1 — Identity & Authorization.
//!
//! Every authorization outcome maps to one of the 17 existing
//! [`crate::error::GuardianDbusError`] categories
//! (`docs/guardian/30_TDD/GUARDIAN_G1_IMPLEMENTATION_HANDOFF.md` §6). No new
//! public error category is introduced here.

pub mod polkit;

use std::future::Future;

use crate::error::{GuardianDbusError, GuardianErrorCategory};
use crate::identity::CallerIdentity;

/// The four G1 test-only polkit actions (TDD contract §9). Production actions
/// added in later gates get their own variants; this enum is deliberately
/// G1-scoped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolkitAction {
    Read,
    LowRiskWrite,
    ModerateWrite,
    HighRiskWrite,
}

impl PolkitAction {
    /// The exact polkit action identifier, as fixed by TDD contract §9.
    #[must_use]
    pub const fn action_id(self) -> &'static str {
        match self {
            Self::Read => "guardian.test.read",
            Self::LowRiskWrite => "guardian.test.low-risk-write",
            Self::ModerateWrite => "guardian.test.moderate-write",
            Self::HighRiskWrite => "guardian.test.high-risk-write",
        }
    }
}

/// A bounded authorization request.
///
/// Carries only the real, resolved caller identity and the action/interactive
/// flag Guardian itself determined — there is no field here for a
/// client-supplied UID, PID, username, role, or `is_admin` claim to occupy.
/// This is a structural (type-level) guarantee, not merely a runtime check:
/// nothing a client sends as method arguments can reach this struct except
/// through [`CallerIdentity`], which is itself only constructed by
/// [`crate::identity::resolve_caller_identity`] from the real bus sender.
#[derive(Clone, Debug)]
pub struct AuthorizationRequest {
    subject: CallerIdentity,
    action: PolkitAction,
    interactive: bool,
}

impl AuthorizationRequest {
    #[must_use]
    pub const fn new(subject: CallerIdentity, action: PolkitAction, interactive: bool) -> Self {
        Self {
            subject,
            action,
            interactive,
        }
    }

    #[must_use]
    pub const fn subject(&self) -> &CallerIdentity {
        &self.subject
    }

    #[must_use]
    pub const fn action(&self) -> PolkitAction {
        self.action
    }

    /// Whether this request came from an explicit, user-initiated action and
    /// may therefore enter an interactive authentication flow. A background
    /// or automated code path must always construct this as `false`
    /// (TDD contract §8.3; P0-AUTH-003).
    #[must_use]
    pub const fn interactive(&self) -> bool {
        self.interactive
    }
}

/// Why an [`AuthorizationOutcome::Unavailable`] result occurred.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationUnavailableReason {
    /// The request was non-interactive but the action requires interactive
    /// authentication to proceed. The request fails closed rather than
    /// prompting (P0-AUTH-003).
    InteractionRequiredButDisallowed,
    /// Interactive authentication was allowed for this request, but no usable
    /// authentication mechanism/agent is available to complete it.
    NoAuthenticationAgent,
}

/// The internal authorization result, before mapping to the public typed
/// error a caller actually receives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationOutcome {
    /// The action is authorized; the caller may proceed.
    Authorized,
    /// The action was explicitly denied.
    Denied,
    /// Authorization could not be completed for a reason unrelated to an
    /// explicit denial — see [`AuthorizationUnavailableReason`].
    Unavailable(AuthorizationUnavailableReason),
}

impl AuthorizationOutcome {
    /// Maps this outcome to the public typed error a denied/unavailable
    /// caller receives, per the G1 handoff §6 mapping. Returns `None` when
    /// authorized — callers proceed to the bounded action in that case.
    #[must_use]
    pub fn into_dbus_error(self, action: PolkitAction) -> Option<GuardianDbusError> {
        match self {
            Self::Authorized => None,
            Self::Denied => Some(
                GuardianErrorCategory::NotAuthorized
                    .with_message(format!("authorization denied for {}", action.action_id())),
            ),
            Self::Unavailable(AuthorizationUnavailableReason::InteractionRequiredButDisallowed) => {
                Some(GuardianErrorCategory::NotAuthorized.with_message(format!(
                    "interaction-required-but-disallowed for {}",
                    action.action_id()
                )))
            }
            Self::Unavailable(AuthorizationUnavailableReason::NoAuthenticationAgent) => Some(
                GuardianErrorCategory::AuthenticationUnavailable.with_message(format!(
                    "no authentication mechanism available for {}",
                    action.action_id()
                )),
            ),
        }
    }
}

/// A pluggable authorization decision source.
///
/// Production code uses [`polkit::PolkitAuthorizer`], backed by the real
/// system polkit authority. Tests use a deterministic test double to prove
/// the surrounding plumbing (ordering, error mapping, interactive-flag
/// routing) without requiring a real bus or root.
pub trait Authorizer {
    /// Decides the outcome for `request`. Must not have any observable side
    /// effect on Guardian's own state — only the caller, after inspecting the
    /// outcome, may cause a mutation (TDD contract GP-05/GP-06; G1 handoff §7).
    fn authorize(
        &self,
        request: &AuthorizationRequest,
    ) -> impl Future<Output = AuthorizationOutcome> + Send;
}
