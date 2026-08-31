//! Real polkit-backed [`Authorizer`], calling `org.freedesktop.PolicyKit1.Authority`
//! over whatever [`zbus::Connection`] it is given.
//!
//! This is genuine production code, not a placeholder: it performs the real
//! `CheckAuthorization` D-Bus call. Whether it observes *real* polkit
//! behavior depends on which connection it is given — a private test bus has
//! no polkit authority service to answer it, so this authorizer is only
//! meaningfully exercised against the real system bus (Layer 2).

use std::collections::HashMap;

use zbus::Connection;
use zbus::zvariant::Value;

use super::{
    AuthorizationOutcome, AuthorizationRequest, AuthorizationUnavailableReason, Authorizer,
};

const ALLOW_USER_INTERACTION: u32 = 0x01;

#[zbus::proxy(
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority",
    interface = "org.freedesktop.PolicyKit1.Authority",
    gen_blocking = false
)]
trait PolicyKitAuthority {
    #[zbus(name = "CheckAuthorization")]
    #[allow(clippy::too_many_arguments)]
    fn check_authorization(
        &self,
        subject: (&str, HashMap<&str, Value<'_>>),
        action_id: &str,
        details: HashMap<&str, &str>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

/// Authorizes G1 test actions against the real system polkit authority on
/// `connection`.
pub struct PolkitAuthorizer<'c> {
    connection: &'c Connection,
}

impl<'c> PolkitAuthorizer<'c> {
    #[must_use]
    pub const fn new(connection: &'c Connection) -> Self {
        Self { connection }
    }
}

impl Authorizer for PolkitAuthorizer<'_> {
    async fn authorize(&self, request: &AuthorizationRequest) -> AuthorizationOutcome {
        let Ok(proxy) = PolicyKitAuthorityProxy::new(self.connection).await else {
            return AuthorizationOutcome::Unavailable(
                AuthorizationUnavailableReason::NoAuthenticationAgent,
            );
        };

        let mut subject_details = HashMap::new();
        subject_details.insert("name", Value::from(request.subject().unique_name()));
        let flags = if request.interactive() {
            ALLOW_USER_INTERACTION
        } else {
            0
        };

        let result = proxy
            .check_authorization(
                ("system-bus-name", subject_details),
                request.action().action_id(),
                HashMap::new(),
                flags,
                "",
            )
            .await;

        // `is_challenge` means polkit determined interactive authentication
        // is required to decide this action, and could not be resolved
        // within this call. Which typed outcome that maps to depends on
        // which flag *we* asked for: a non-interactive request that hits a
        // challenge is exactly P0-AUTH-003's "background action cannot
        // prompt" case; an interactive request that still comes back as a
        // challenge means no agent completed it (not registered, or the
        // call returned before the agent could answer).
        match result {
            Ok((true, _, _)) => AuthorizationOutcome::Authorized,
            Ok((false, true, _)) if !request.interactive() => AuthorizationOutcome::Unavailable(
                AuthorizationUnavailableReason::InteractionRequiredButDisallowed,
            ),
            Ok((false, true, _)) | Err(_) => AuthorizationOutcome::Unavailable(
                AuthorizationUnavailableReason::NoAuthenticationAgent,
            ),
            Ok((false, false, _)) => AuthorizationOutcome::Denied,
        }
    }
}
