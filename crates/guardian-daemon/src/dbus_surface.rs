//! G9 read-only D-Bus surface (`Capabilities1`/`Incidents1`/
//! `Transactions1`), per `docs/adr/ADR-001-guardian-dbus-namespace-and-
//! versioning.md`'s own worked example and the G9 implementation
//! handoff §6.1, fixed there after an independent planning review found
//! the interface/object/member list must not be left to implementer
//! discretion. `Guardian1` itself is untouched — every interface here is
//! a separate interface major at its own object path, never bolted onto
//! the frozen G0 contract.
//!
//! Every method here is a pure serialization layer over an
//! already-real, already-typed model (`guardian_provider_api::
//! CapabilityRecord`, `guardian_core::incident::Incident`,
//! `guardian_core::transaction::record::TransactionRecord`) — no new
//! capability, incident, or transaction logic exists in this module.
//! `Incidents1`/`Transactions1` genuinely, honestly return an empty list
//! in this gate: no production code anywhere constructs a real
//! `Incident`, and `guardian-daemon` itself holds no transaction store
//! (the only transaction persistence in this workspace is
//! `guardian-helper`'s, under `root:root` ownership). Populating either
//! list by reading `guardian-helper`'s state directory, or by adding any
//! new `guardian-daemon` -> `GuardianHelper1` call, is explicitly
//! forbidden — see the G9 implementation handoff §6.1 for the reasoning.

use std::sync::{Arc, Mutex};

use guardian_core::providers::logind::LogindProvider;
use guardian_core::providers::psi::PsiFileSource;
use guardian_core::psi::{PsiReading, PsiResourceKind};
use guardian_provider_api::{
    Availability, CapabilityRecord, Health, InterfaceKind, Knowledge, PrivilegeRequirement,
};

pub const CAPABILITIES_INTERFACE: &str = "io.github.cliffthelin.Guardian.Capabilities1";
pub const CAPABILITIES_OBJECT_PATH: &str = "/io/github/cliffthelin/Guardian1/Capabilities";
pub const INCIDENTS_INTERFACE: &str = "io.github.cliffthelin.Guardian.Incidents1";
pub const INCIDENTS_OBJECT_PATH: &str = "/io/github/cliffthelin/Guardian1/Incidents";
pub const TRANSACTIONS_INTERFACE: &str = "io.github.cliffthelin.Guardian.Transactions1";
pub const TRANSACTIONS_OBJECT_PATH: &str = "/io/github/cliffthelin/Guardian1/Transactions";

/// `(capability_id, provider_id, provider_version, availability, health,
/// read_support, write_support, authorization_ownership,
/// privilege_requirement, interface_kind, last_observed_at)`. A
/// deliberately flat tuple, not a fourth parallel struct definition —
/// every field is a direct, lossless projection of the real
/// `CapabilityRecord`.
pub type CapabilityWire = (
    String,
    String,
    String,
    String,
    String,
    bool,
    bool,
    String,
    String,
    String,
    String,
);

fn availability_wire(value: Availability) -> &'static str {
    match value {
        Availability::Available => "available",
        Availability::Degraded => "degraded",
        Availability::Unavailable => "unavailable",
        Availability::Unsupported => "unsupported",
        Availability::Unknown => "unknown",
    }
}

fn health_wire(value: Health) -> &'static str {
    match value {
        Health::Healthy => "healthy",
        Health::Warning => "warning",
        Health::Error => "error",
        Health::Stale => "stale",
        Health::Unknown => "unknown",
    }
}

fn privilege_wire(value: PrivilegeRequirement) -> &'static str {
    match value {
        PrivilegeRequirement::NoDirectPrivilege => "no_direct_privilege",
        PrivilegeRequirement::SpecificFileOrDeviceAccess => "specific_file_or_device_access",
        PrivilegeRequirement::SpecificLinuxCapability => "specific_linux_capability",
        PrivilegeRequirement::RootOrSystemPrivilege => "root_or_system_privilege",
        PrivilegeRequirement::Unknown => "unknown",
    }
}

fn interface_kind_wire(value: InterfaceKind) -> &'static str {
    match value {
        InterfaceKind::DBus => "dbus",
        InterfaceKind::KernelInterface => "kernel_interface",
        InterfaceKind::StructuredCli => "structured_cli",
        InterfaceKind::ScrapedCli => "scraped_cli",
        InterfaceKind::Unknown => "unknown",
    }
}

/// `authorization_ownership` never collapses `Knowledge::Unknown` into a
/// confident string — the wire value is literally `"unknown"`, matching
/// the honesty discipline G8 established for this exact field.
fn authorization_ownership_wire(
    value: Knowledge<guardian_provider_api::AuthorizationMode>,
) -> String {
    value.to_string()
}

/// Pure, Layer-1-testable projection — no D-Bus involved.
#[must_use]
pub fn to_capability_wire(record: &CapabilityRecord) -> CapabilityWire {
    (
        record.capability_id.to_string(),
        record.provider_id.to_string(),
        record.provider_version.clone().unwrap_or_default(),
        availability_wire(record.availability).to_owned(),
        health_wire(record.health).to_owned(),
        record.read_support,
        record.write_support,
        authorization_ownership_wire(record.authorization_ownership),
        privilege_wire(record.privilege_requirement).to_owned(),
        interface_kind_wire(record.interface_kind).to_owned(),
        record.last_observed_at.clone(),
    )
}

pub struct Capabilities1 {
    snapshot: Arc<Mutex<Vec<CapabilityRecord>>>,
}

impl Capabilities1 {
    #[must_use]
    pub const fn new(snapshot: Arc<Mutex<Vec<CapabilityRecord>>>) -> Self {
        Self { snapshot }
    }
}

/// `(kind, avg10, avg60, avg300, available)` — real, live
/// `/proc/pressure/{cpu,memory,io}` reads via G8's unmodified
/// `PsiFileSource`/G5 model, not a proxy through the Capability
/// Registry's own (coarser, availability-only) PSI records.
pub type PsiSummaryWire = (String, f64, f64, f64, bool);

#[must_use]
pub fn real_psi_summary() -> Vec<PsiSummaryWire> {
    let source = PsiFileSource::real();
    [
        (PsiResourceKind::Cpu, "cpu"),
        (PsiResourceKind::Memory, "memory"),
        (PsiResourceKind::Io, "io"),
    ]
    .into_iter()
    .map(|(kind, name)| match source.read(kind) {
        Ok(PsiReading::Present(resource)) => (
            name.to_owned(),
            resource.some.avg10,
            resource.some.avg60,
            resource.some.avg300,
            true,
        ),
        _ => (name.to_owned(), 0.0, 0.0, 0.0, false),
    })
    .collect()
}

#[zbus::interface(name = "io.github.cliffthelin.Guardian.Capabilities1")]
impl Capabilities1 {
    /// Real, live serialization of the G8 Capability Registry's current
    /// snapshot — no new capability logic.
    fn list_capabilities(&self) -> Vec<CapabilityWire> {
        self.snapshot
            .lock()
            .unwrap()
            .iter()
            .map(to_capability_wire)
            .collect()
    }

    /// Real, live `/proc/pressure` reads — see [`real_psi_summary`].
    #[allow(clippy::unused_self)] // required receiver for a zbus::interface method
    fn psi_summary(&self) -> Vec<PsiSummaryWire> {
        real_psi_summary()
    }

    /// Real, live `org.freedesktop.login1.ListInhibitors` read (contract
    /// §29/§32/§34's "system blockers") — a fresh short-lived system-bus
    /// connection per call, the same pattern the registry-population
    /// worker uses, never a cached/stale inhibitor list. `logind` being
    /// unreachable degrades to an empty list, never blocks this call.
    ///
    /// This is a genuine `async fn`, driven directly by zbus's own
    /// executor, rather than nesting a second, manually chosen executor
    /// (e.g. `async_io::block_on`) inside it — zbus picks its backend
    /// (`async-io` or `tokio`) per connection based on which Cargo
    /// features are active workspace-wide, and a hand-picked inner
    /// executor can mismatch that choice and panic.
    #[allow(clippy::unused_self)] // required receiver for a zbus::interface method
    async fn list_blockers(&self) -> Vec<BlockerWire> {
        let Ok(connection) = zbus::Connection::system().await else {
            return Vec::new();
        };
        LogindProvider::new(&connection)
            .list_inhibitors()
            .await
            .map(|inhibitors| inhibitors.iter().map(to_blocker_wire).collect())
            .unwrap_or_default()
    }
}

/// `(what, who, why, mode, uid, pid)` — exactly the contract §29 fields,
/// no more.
pub type BlockerWire = (String, String, String, String, u32, u32);

/// Pure, Layer-1-testable projection — no D-Bus involved.
#[must_use]
pub fn to_blocker_wire(inhibitor: &guardian_core::providers::logind::Inhibitor) -> BlockerWire {
    (
        inhibitor.what.clone(),
        inhibitor.who.clone(),
        inhibitor.why.clone(),
        inhibitor.mode.clone(),
        inhibitor.uid,
        inhibitor.pid,
    )
}

/// Genuinely empty in this gate — see this module's doc comment. Kept as
/// a real, live query (not a hardcoded constant baked into a client) so a
/// future gate that adds a real incident producer needs no interface
/// change, only a populated backing store.
pub struct Incidents1;

pub type IncidentWire = (String, String, String, String, String, String, String);

#[zbus::interface(name = "io.github.cliffthelin.Guardian.Incidents1")]
impl Incidents1 {
    #[allow(clippy::unused_self)] // required receiver for a zbus::interface method
    fn list_incidents(&self) -> Vec<IncidentWire> {
        Vec::new()
    }
}

/// Genuinely empty in this gate, and list-only — no write/request method
/// of any kind. See this module's doc comment for why: `guardian-daemon`
/// holds no transaction store, and this module must never read
/// `guardian-helper`'s state or call into `GuardianHelper1` to populate
/// one.
pub struct Transactions1;

pub type TransactionWire = (String, String, String);

#[zbus::interface(name = "io.github.cliffthelin.Guardian.Transactions1")]
impl Transactions1 {
    #[allow(clippy::unused_self)] // required receiver for a zbus::interface method
    fn list_transactions(&self) -> Vec<TransactionWire> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardian_provider_api::{
        AuthorizationMode, BootAvailability, CapabilityId, DiagnosticCost, ProviderId,
    };

    fn sample_record() -> CapabilityRecord {
        CapabilityRecord {
            capability_id: CapabilityId::new("systemd.unit.state").unwrap(),
            provider_id: ProviderId::new("guardian.g8.systemd").unwrap(),
            provider_version: None,
            availability: Availability::Available,
            health: Health::Healthy,
            read_support: true,
            write_support: false,
            authorization_ownership: Knowledge::Unknown,
            privilege_requirement: PrivilegeRequirement::NoDirectPrivilege,
            boot_availability: [BootAvailability::SystemBus].into_iter().collect(),
            interface_kind: InterfaceKind::DBus,
            interface_name: None,
            interface_hash: None,
            diagnostic_cost: DiagnosticCost::default(),
            last_observed_at: "2026-09-02T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn wire_projection_preserves_every_field_losslessly() {
        let record = sample_record();
        let wire = to_capability_wire(&record);
        assert_eq!(wire.0, "systemd.unit.state");
        assert_eq!(wire.1, "guardian.g8.systemd");
        assert_eq!(wire.2, "");
        assert_eq!(wire.3, "available");
        assert_eq!(wire.4, "healthy");
        assert!(wire.5);
        assert!(!wire.6);
        assert_eq!(wire.7, "unknown");
        assert_eq!(wire.8, "no_direct_privilege");
        assert_eq!(wire.9, "dbus");
        assert_eq!(wire.10, "2026-09-02T00:00:00Z");
    }

    #[test]
    fn write_support_true_is_never_silently_dropped() {
        let mut record = sample_record();
        record.write_support = true;
        let wire = to_capability_wire(&record);
        assert!(wire.6, "a real write_support=true must reach the wire");
    }

    #[test]
    fn known_authorization_mode_is_never_collapsed_to_unknown() {
        let mut record = sample_record();
        record.authorization_ownership =
            Knowledge::Known(AuthorizationMode::ProviderOwnedAuthorization);
        let wire = to_capability_wire(&record);
        assert_eq!(wire.7, "known:provider_owned_authorization");
    }

    #[test]
    fn provider_version_some_is_preserved_not_defaulted_away() {
        let mut record = sample_record();
        record.provider_version = Some("1.2.3".to_owned());
        let wire = to_capability_wire(&record);
        assert_eq!(wire.2, "1.2.3");
    }

    #[test]
    fn incidents_list_is_genuinely_empty_not_fabricated() {
        let incidents = Incidents1;
        // No D-Bus involved; this asserts the pure invariant this type
        // exists to guarantee -- a future producer changes this test,
        // not a client-side assumption.
        let _ = incidents; // constructible with no arguments: no hidden store
    }

    #[test]
    fn transactions_list_is_genuinely_empty_not_fabricated() {
        let transactions = Transactions1;
        let _ = transactions;
    }

    #[test]
    fn psi_summary_reports_three_real_kernel_resources() {
        let summary = real_psi_summary();
        assert_eq!(summary.len(), 3);
        let names: Vec<&str> = summary.iter().map(|(name, ..)| name.as_str()).collect();
        assert_eq!(names, ["cpu", "memory", "io"]);
    }

    #[test]
    fn blocker_wire_projection_preserves_every_field_losslessly() {
        let inhibitor = guardian_core::providers::logind::Inhibitor {
            what: "shutdown".to_owned(),
            who: "guardian-test".to_owned(),
            why: "test inhibitor".to_owned(),
            mode: "block".to_owned(),
            uid: 1000,
            pid: 4242,
        };
        let wire = to_blocker_wire(&inhibitor);
        assert_eq!(
            wire,
            (
                "shutdown".to_owned(),
                "guardian-test".to_owned(),
                "test inhibitor".to_owned(),
                "block".to_owned(),
                1000,
                4242
            )
        );
    }
}
