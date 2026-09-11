//! G8 Capability Registry population (implementation handoff §11). Builds
//! real `CapabilityRecord` entries from each of the six read providers'
//! genuinely observed state — never hardcoded/fixture values. This is the
//! first pass where the registry stops being populated by G3's fixture
//! data.
//!
//! Every record this module produces has `write_support: false` and
//! `authorization_ownership: Knowledge::Unknown` — G8 has no write-capable
//! capability and has not evidenced who would own authorization for one,
//! so fabricating a `Known` value here would be dishonest (handoff §11).
//! `privilege_requirement` is `NoDirectPrivilege` for every record: each
//! read this gate adds is an unprivileged D-Bus/kernel-file read, and that
//! has been evidenced (not assumed) by every adapter module in this tree
//! running unprivileged in its own Layer 1 tests.
//!
//! Capability Registry = what exists; Provider Arbitrator = who may write
//! and under what conditions (handoff §11). This module never constructs
//! or invokes an arbitrator — G8 is read-only, so "who may write" has no
//! answer to populate yet.

use std::time::{SystemTime, UNIX_EPOCH};

use guardian_provider_api::{
    Availability, BootAvailability, CapabilityId, CapabilityRecord, DiagnosticCost, Health,
    InterfaceKind, Knowledge, PrivilegeRequirement, ProviderId,
};
use zbus::Connection;

use super::accounts::{AccountsError, AccountsProvider};
use super::logind::{LogindError, LogindProvider};
use super::psi::{PsiAvailability, PsiFileSource, PsiResourceStatus};
use super::systemd::{SystemdError, SystemdProvider};
use super::udisks::{UdisksError, UdisksProvider};
use super::upower::{BatteryPresence, UpowerError, UpowerProvider};
use crate::psi::{PsiReading, PsiResourceKind};

/// Real UTC ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`), not epoch-seconds text —
/// `CapabilityRecord::last_observed_at` has no documented format, but a
/// field literally named `_iso` producing `"12345s-since-epoch"` was a real
/// honesty defect an earlier audit pass caught. Pure/testable: converts a
/// Unix-epoch-seconds count via the standard proleptic-Gregorian
/// civil-from-days algorithm (Hinnant, unsigned/no external crate needed).
#[must_use]
pub fn iso8601_utc(epoch_secs: u64) -> String {
    let days = epoch_secs / 86400;
    let secs_of_day = epoch_secs % 86400;
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(i64::MAX));
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Days-since-epoch (1970-01-01) to (year, month, day). Howard Hinnant's
/// `civil_from_days`, the standard closed-form algorithm for the proleptic
/// Gregorian calendar — see <http://howardhinnant.github.io/date_algorithms.html>.
/// Kept entirely in `i64` (the algorithm's own natural domain): `doe` is
/// always in `[0, 146_096]`, `mp` in `[0, 11]`, `d` in `[1, 31]` and `m` in
/// `[1, 12]` by construction, so every value here fits `u32`/`u64` far
/// below any real truncation/sign-loss boundary — the casts at the end are
/// therefore lossless by the algorithm's own invariants, not merely
/// "probably fine".
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    iso8601_utc(secs)
}

fn capability_id(id: &str) -> CapabilityId {
    CapabilityId::new(id).expect("fixed literal is a valid CapabilityId")
}

fn provider_id(id: &str) -> ProviderId {
    ProviderId::new(id).expect("fixed literal is a valid ProviderId")
}

fn record(
    cap_id: &str,
    prov_id: &str,
    availability: Availability,
    health: Health,
    interface_kind: InterfaceKind,
    boot_availability: BootAvailability,
) -> CapabilityRecord {
    CapabilityRecord {
        capability_id: capability_id(cap_id),
        provider_id: provider_id(prov_id),
        provider_version: None,
        availability,
        health,
        read_support: true,
        write_support: false,
        authorization_ownership: Knowledge::Unknown,
        privilege_requirement: PrivilegeRequirement::NoDirectPrivilege,
        boot_availability: [boot_availability].into_iter().collect(),
        interface_kind,
        interface_name: None,
        interface_hash: None,
        diagnostic_cost: DiagnosticCost::default(),
        last_observed_at: now_iso(),
    }
}

#[cfg(test)]
fn probe_record(
    cap_id: &str,
    prov_id: &str,
    is_up: bool,
    interface_kind: InterfaceKind,
) -> CapabilityRecord {
    let (availability, health) = if is_up {
        (Availability::Available, Health::Healthy)
    } else {
        (Availability::Unavailable, Health::Error)
    };
    record(
        cap_id,
        prov_id,
        availability,
        health,
        interface_kind,
        BootAvailability::SystemBus,
    )
}

fn read_state<T, E>(result: &Result<T, E>, absent: impl Fn(&E) -> bool) -> (Availability, Health) {
    match result {
        Ok(_) => (Availability::Available, Health::Healthy),
        Err(error) if absent(error) => (Availability::Unavailable, Health::Error),
        Err(_) => (Availability::Degraded, Health::Error),
    }
}

/// `P1-SYS-001..003` — one capability: real allowlisted-unit state reads,
/// availability tied to whether `systemd1` is genuinely reachable on the
/// bus right now.
pub async fn systemd_capabilities(connection: &Connection) -> Vec<CapabilityRecord> {
    let provider = SystemdProvider::new(connection);
    let result = provider.unit_state("systemd-logind.service").await;
    let (availability, health) = read_state(&result, |error| {
        matches!(error, SystemdError::ProviderUnavailable(_))
    });
    vec![record(
        "systemd.unit.state",
        "guardian.g8.systemd",
        availability,
        health,
        InterfaceKind::DBus,
        BootAvailability::SystemBus,
    )]
}

/// `P1-PSI-001..005` — one capability per real kernel pressure resource.
///
/// **Acceptance-repair Blocker 4.** `guardian-daemon` runs under the
/// accepted `ProcSubset=pid` sandbox, which denies it `/proc/pressure`
/// access **by pathname** unconditionally — so probing by pathname here
/// (the pre-repair behavior, still `psi_capabilities_by_pathname` below)
/// can *never* observe a live inherited PSI descriptor from inside the
/// real daemon: it always reads absent and always reports `Unsupported`,
/// even while a real, working, kernel-triggered PSI monitor is running on
/// the very same descriptor the production ingress path
/// (`guardian-daemon::psi_ingress`) holds. That produced a real,
/// evidenced observability inconsistency (this pass's evidence document,
/// §F.15 Finding 2): `Capabilities1` reporting PSI `unsupported` while
/// `Incidents1` simultaneously showed a real PSI-driven incident.
///
/// `psi_state`, when `Some`, is the **same live per-resource state**
/// `guardian-daemon`'s own PSI startup/runtime reporting maintains
/// (Blocker 3, `guardian_daemon::psi_ingress::availability_from_startup`
/// and its runtime updates) — this function never opens
/// `/proc/pressure` itself in that case; it only classifies state the
/// daemon already observed through its own descriptors. `None` preserves
/// the original pathname-probing behavior unchanged (used when no live
/// descriptor-state has been wired in — e.g. a caller other than the
/// real sandboxed daemon, such as this module's own pre-existing unit
/// test, or a future non-daemon consumer that genuinely does have
/// `/proc/pressure` visible by pathname).
///
/// Classification, given `Some(state)`: a resource `state` reports
/// [`PsiResourceStatus::Available`] is `Available`/`Healthy`; one it
/// reports [`PsiResourceStatus::Degraded`] is `Degraded`/`Error` (a real
/// fault on a resource a descriptor genuinely exists for); a resource
/// with no descriptor at all
/// ([`PsiResourceStatus::NoDescriptor`]/unobserved) is `Unsupported`
/// **unless** at least one sibling resource in the same snapshot is
/// `Available` — a sibling's success is positive proof PSI is supported
/// on this kernel, so that specific resource's own absence is then a
/// truthful `Unavailable`/`Error` gap, never a silently-swallowed
/// `Unsupported`. With zero resources ever observed available, this
/// daemon genuinely cannot distinguish "no live signal was ever wired"
/// from "the kernel has no PSI at all", so every resource in that case is
/// `Unsupported` — the same taxonomy the original pathname probe already
/// used for true kernel absence.
#[must_use]
pub fn psi_capabilities(psi_state: Option<&PsiAvailability>) -> Vec<CapabilityRecord> {
    let Some(state) = psi_state else {
        return psi_capabilities_by_pathname();
    };
    let any_available = state.available_count() > 0;
    [
        (PsiResourceKind::Cpu, "psi.pressure.cpu"),
        (PsiResourceKind::Memory, "psi.pressure.memory"),
        (PsiResourceKind::Io, "psi.pressure.io"),
    ]
    .into_iter()
    .map(|(kind, cap_id)| {
        let (availability, health) = match state.get(kind) {
            Some(PsiResourceStatus::Available) => (Availability::Available, Health::Healthy),
            Some(PsiResourceStatus::Degraded(_)) => (Availability::Degraded, Health::Error),
            Some(PsiResourceStatus::NoDescriptor) | None => {
                if any_available {
                    (Availability::Unavailable, Health::Error)
                } else {
                    (Availability::Unsupported, Health::Unknown)
                }
            }
        };
        record(
            cap_id,
            "guardian.g8.psi",
            availability,
            health,
            InterfaceKind::KernelInterface,
            BootAvailability::EarlyBoot,
        )
    })
    .collect()
}

/// The original, pre-repair pathname probe. Preserved unmodified as the
/// `psi_state: None` fallback in [`psi_capabilities`] — see that
/// function's doc comment for when this path is (and, inside the real
/// sandboxed daemon, structurally always is not) reachable.
/// `PsiReading::Unavailable` (kernel PSI absent for this resource) is a
/// real, distinct, non-error `Unsupported` state, never conflated with a
/// read failure.
fn psi_capabilities_by_pathname() -> Vec<CapabilityRecord> {
    let source = PsiFileSource::real();
    [
        (PsiResourceKind::Cpu, "psi.pressure.cpu"),
        (PsiResourceKind::Memory, "psi.pressure.memory"),
        (PsiResourceKind::Io, "psi.pressure.io"),
    ]
    .into_iter()
    .map(|(kind, cap_id)| {
        let (availability, health) = match source.read(kind) {
            Ok(PsiReading::Present(_)) => (Availability::Available, Health::Healthy),
            Ok(PsiReading::Unavailable) => (Availability::Unsupported, Health::Unknown),
            Err(_) => (Availability::Degraded, Health::Error),
        };
        record(
            cap_id,
            "guardian.g8.psi",
            availability,
            health,
            InterfaceKind::KernelInterface,
            BootAvailability::EarlyBoot,
        )
    })
    .collect()
}

/// `P1-LGI-001..002` — one capability: real inhibitor enumeration.
pub async fn logind_capabilities(connection: &Connection) -> Vec<CapabilityRecord> {
    let provider = LogindProvider::new(connection);
    let result = provider.list_inhibitors().await;
    let (availability, health) = read_state(&result, |error| {
        matches!(error, LogindError::ProviderUnavailable(_))
    });
    vec![record(
        "logind.inhibitors",
        "guardian.g8.logind",
        availability,
        health,
        InterfaceKind::DBus,
        BootAvailability::SystemBus,
    )]
}

/// `P1-UPW-001..002` — two capabilities: display-device state and honest
/// battery-presence.
pub async fn upower_capabilities(connection: &Connection) -> Vec<CapabilityRecord> {
    let provider = UpowerProvider::new(connection);
    let display = provider.display_device().await;
    let battery = provider.battery_presence().await;
    let display_state = read_state(&display, |error| {
        matches!(error, UpowerError::ProviderUnavailable(_))
    });
    let battery_state = match &battery {
        Ok(BatteryPresence::Present | BatteryPresence::NotPresent) => {
            (Availability::Available, Health::Healthy)
        }
        Err(UpowerError::ProviderUnavailable(_)) => (Availability::Unavailable, Health::Error),
        Err(UpowerError::MalformedResponse(_)) => (Availability::Degraded, Health::Error),
    };
    vec![
        record(
            "upower.display-device",
            "guardian.g8.upower",
            display_state.0,
            display_state.1,
            InterfaceKind::DBus,
            BootAvailability::SystemBus,
        ),
        record(
            "upower.battery-presence",
            "guardian.g8.upower",
            battery_state.0,
            battery_state.1,
            InterfaceKind::DBus,
            BootAvailability::SystemBus,
        ),
    ]
}

/// `P1-ACC-001..003` — user/session discovery over `org.freedesktop.
/// Accounts`, plus real filesystem-backed installed-session enumeration
/// (`real_session_directories`), which has no D-Bus dependency and so is
/// never marked `Unavailable` merely because `Accounts` is down.
pub async fn accounts_capabilities(connection: &Connection) -> Vec<CapabilityRecord> {
    let provider = AccountsProvider::new(connection);
    let users = provider.list_cached_users().await;
    let user_state = read_state(&users, |error| {
        matches!(error, AccountsError::ProviderUnavailable(_))
    });
    let installed =
        super::accounts::scan_installed_sessions(&super::accounts::real_session_directories());
    let (session_availability, session_health) = if installed.is_empty() {
        (Availability::Degraded, Health::Warning)
    } else {
        (Availability::Available, Health::Healthy)
    };
    vec![
        record(
            "accounts.user.cache",
            "guardian.g8.accounts",
            user_state.0,
            user_state.1,
            InterfaceKind::DBus,
            BootAvailability::SystemBus,
        ),
        record(
            "accounts.session.enumeration",
            "guardian.g8.accounts",
            session_availability,
            session_health,
            InterfaceKind::KernelInterface,
            BootAvailability::PreLogin,
        ),
    ]
}

/// `P1-UDS-001..004` — real drive/block topology and the six precondition
/// checks, as pure validation logic over that topology (never a callable
/// `PowerOff()`).
pub async fn udisks_capabilities(connection: &Connection) -> Vec<CapabilityRecord> {
    let provider = UdisksProvider::new(connection);
    let topology = provider.topology().await;
    let state = read_state(&topology, |error| {
        matches!(error, UdisksError::ProviderUnavailable(_))
    });
    vec![
        record(
            "storage.drive.topology",
            "guardian.g8.udisks",
            state.0,
            state.1,
            InterfaceKind::DBus,
            BootAvailability::SystemBus,
        ),
        record(
            "storage.drive.poweroff-preconditions",
            "guardian.g8.udisks",
            state.0,
            state.1,
            InterfaceKind::DBus,
            BootAvailability::SystemBus,
        ),
    ]
}

/// Populates the full, real G8 Capability Registry snapshot across all six
/// providers. Each sub-call is independent — one provider being down never
/// prevents the others' real records from being collected.
///
/// `psi_state` is threaded straight to [`psi_capabilities`] — see its doc
/// comment (acceptance-repair Blocker 4).
pub async fn populate_registry(
    connection: &Connection,
    psi_state: Option<&PsiAvailability>,
) -> Vec<CapabilityRecord> {
    let mut records = Vec::new();
    records.extend(systemd_capabilities(connection).await);
    records.extend(psi_capabilities(psi_state));
    records.extend(logind_capabilities(connection).await);
    records.extend(upower_capabilities(connection).await);
    records.extend(accounts_capabilities(connection).await);
    records.extend(udisks_capabilities(connection).await);
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-audit regression: `last_observed_at` previously emitted
    /// `"12345s-since-epoch"` from a function named `now_iso`, despite its
    /// name — a genuine honesty defect. Known epoch/ISO pairs, independent
    /// of the local machine's clock.
    #[test]
    fn iso8601_utc_matches_known_epoch_values() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_utc(1), "1970-01-01T00:00:01Z");
        assert_eq!(iso8601_utc(86_400), "1970-01-02T00:00:00Z");
        // 2000-01-01T00:00:00Z
        assert_eq!(iso8601_utc(946_684_800), "2000-01-01T00:00:00Z");
        // 2038-01-19T03:14:07Z (i32 rollover instant, exercised since this
        // is u64-based and must not wrap there)
        assert_eq!(iso8601_utc(2_147_483_647), "2038-01-19T03:14:07Z");
    }

    #[test]
    fn probe_record_up_is_available_and_healthy() {
        let rec = probe_record("test.cap", "test.provider", true, InterfaceKind::DBus);
        assert_eq!(rec.availability, Availability::Available);
        assert_eq!(rec.health, Health::Healthy);
        assert!(!rec.write_support);
        assert!(rec.read_support);
        assert_eq!(rec.authorization_ownership, Knowledge::Unknown);
        assert_eq!(
            rec.privilege_requirement,
            PrivilegeRequirement::NoDirectPrivilege
        );
    }

    #[test]
    fn probe_record_down_is_unavailable_never_healthy() {
        let rec = probe_record("test.cap", "test.provider", false, InterfaceKind::DBus);
        assert_eq!(rec.availability, Availability::Unavailable);
        assert_ne!(rec.health, Health::Healthy);
        assert!(!rec.availability.is_usable());
    }

    #[test]
    fn no_capability_this_module_produces_ever_claims_write_support() {
        let up = probe_record("a", "b", true, InterfaceKind::DBus);
        let down = probe_record("a", "b", false, InterfaceKind::DBus);
        assert!(!up.write_support);
        assert!(!down.write_support);
    }

    #[test]
    fn psi_capabilities_never_marks_a_genuinely_absent_resource_as_a_hard_error() {
        // Real /proc/pressure is read on whatever host runs this test.
        // This only checks the taxonomy invariant, not a specific host
        // outcome: Unsupported/Unknown is a legitimate non-error state.
        // `None` -- no live daemon descriptor-state wired in -- exercises
        // the original pathname-probe fallback, unmodified by Blocker 4.
        let records = psi_capabilities(None);
        assert_eq!(records.len(), 3);
        for rec in &records {
            assert_ne!(
                rec.availability,
                Availability::Unavailable,
                "PSI resource absence must be Unsupported, never Unavailable"
            );
        }
    }

    // ---- Acceptance-repair Blocker 4: live-state-driven PSI capability
    // truth, never a pathname re-probe. ----------------------------

    fn find(records: &[CapabilityRecord], cap_id: &str) -> CapabilityRecord {
        records
            .iter()
            .find(|record| record.capability_id.as_str() == cap_id)
            .unwrap_or_else(|| panic!("no record for {cap_id}"))
            .clone()
    }

    /// A live inherited CPU PSI descriptor must not coexist with a
    /// reported `Unsupported` state — the exact defect this pass's
    /// evidence document recorded as Finding 2.
    #[test]
    fn inherited_cpu_psi_available_is_never_reported_unsupported() {
        let mut state = PsiAvailability::default();
        state.set(PsiResourceKind::Cpu, PsiResourceStatus::Available);
        state.set(PsiResourceKind::Memory, PsiResourceStatus::Available);
        state.set(PsiResourceKind::Io, PsiResourceStatus::Available);
        let records = psi_capabilities(Some(&state));
        let cpu = find(&records, "psi.pressure.cpu");
        assert_eq!(cpu.availability, Availability::Available);
        assert_eq!(cpu.health, Health::Healthy);
        assert_ne!(cpu.availability, Availability::Unsupported);
    }

    /// A missing `:graceful` descriptor, alongside evidence PSI is
    /// genuinely supported here (a sibling resource's live descriptor),
    /// is a truthful `Unavailable`/`Degraded`-family state — never
    /// silently `Unsupported`, which would misrepresent a real
    /// provisioning gap as "this kernel has no PSI".
    #[test]
    fn a_missing_graceful_descriptor_alongside_a_working_sibling_is_truthfully_unavailable() {
        let mut state = PsiAvailability::default();
        state.set(PsiResourceKind::Cpu, PsiResourceStatus::Available);
        state.set(PsiResourceKind::Io, PsiResourceStatus::Available);
        state.set(PsiResourceKind::Memory, PsiResourceStatus::NoDescriptor);
        let records = psi_capabilities(Some(&state));
        let memory = find(&records, "psi.pressure.memory");
        assert_ne!(
            memory.availability,
            Availability::Unsupported,
            "a sibling resource proves PSI is supported here; memory's own absence is a real \
             gap, not an unsupported kernel"
        );
        assert_eq!(memory.availability, Availability::Unavailable);
        assert_eq!(memory.health, Health::Error);
    }

    /// Genuinely absent kernel PSI support — no resource ever resolved a
    /// descriptor, so the daemon has no positive signal to distinguish
    /// this from "no live state was ever wired" — is correctly
    /// `Unsupported`, matching the taxonomy the original pathname probe
    /// already used for true kernel absence.
    #[test]
    fn genuinely_absent_kernel_psi_support_is_reported_unsupported() {
        let mut state = PsiAvailability::default();
        state.set(PsiResourceKind::Cpu, PsiResourceStatus::NoDescriptor);
        state.set(PsiResourceKind::Memory, PsiResourceStatus::NoDescriptor);
        state.set(PsiResourceKind::Io, PsiResourceStatus::NoDescriptor);
        let records = psi_capabilities(Some(&state));
        for cap_id in ["psi.pressure.cpu", "psi.pressure.memory", "psi.pressure.io"] {
            let rec = find(&records, cap_id);
            assert_eq!(
                rec.availability,
                Availability::Unsupported,
                "{cap_id}: zero live descriptors anywhere must classify as Unsupported"
            );
            assert_eq!(rec.health, Health::Unknown);
        }
    }

    /// Partial CPU/memory/io availability is represented correctly and
    /// **individually** — a `Degraded` resource (descriptor present,
    /// registration failed) is distinguished from both an `Available` one
    /// and a fully-absent one, all in the same snapshot.
    #[test]
    fn partial_availability_across_resources_is_represented_individually() {
        let mut state = PsiAvailability::default();
        state.set(PsiResourceKind::Cpu, PsiResourceStatus::Available);
        state.set(
            PsiResourceKind::Memory,
            PsiResourceStatus::Degraded("EBUSY: a trigger already exists".to_owned()),
        );
        state.set(PsiResourceKind::Io, PsiResourceStatus::NoDescriptor);
        let records = psi_capabilities(Some(&state));

        let cpu = find(&records, "psi.pressure.cpu");
        assert_eq!(cpu.availability, Availability::Available);
        assert_eq!(cpu.health, Health::Healthy);

        let memory = find(&records, "psi.pressure.memory");
        assert_eq!(memory.availability, Availability::Degraded);
        assert_eq!(memory.health, Health::Error);

        let io = find(&records, "psi.pressure.io");
        assert_eq!(
            io.availability,
            Availability::Unavailable,
            "io has no descriptor but cpu proves PSI is supported here, so io's own absence \
             must be a truthful gap"
        );
        assert_eq!(io.health, Health::Error);

        // All three genuinely differ -- this is the "individually"
        // requirement, not merely "not all identical".
        assert_ne!(cpu.availability, memory.availability);
        assert_ne!(memory.availability, io.availability);
        assert_ne!(cpu.availability, io.availability);
    }

    #[test]
    fn live_malformed_read_is_degraded_and_never_healthy() {
        let result: Result<(), AccountsError> = Err(AccountsError::MalformedResponse(
            "wrong signature".to_owned(),
        ));
        let state = read_state(&result, |error| {
            matches!(error, AccountsError::ProviderUnavailable(_))
        });
        assert_eq!(state, (Availability::Degraded, Health::Error));
    }

    #[test]
    fn absent_read_is_unavailable_and_valid_read_is_healthy() {
        let absent: Result<(), AccountsError> =
            Err(AccountsError::ProviderUnavailable("absent".to_owned()));
        assert_eq!(
            read_state(&absent, |error| matches!(
                error,
                AccountsError::ProviderUnavailable(_)
            )),
            (Availability::Unavailable, Health::Error)
        );
        let valid: Result<(), AccountsError> = Ok(());
        assert_eq!(
            read_state(&valid, |_| false),
            (Availability::Available, Health::Healthy)
        );
    }
}
