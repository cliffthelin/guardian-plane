//! TDD-contract Phase 2, Gate 2b — Layer 2 (private/mocked D-Bus)
//! acceptance evidence for the manifest's owned normative IDs
//! (`P2-API-001..003`). See `docs/guardian/30_TDD/gates/
//! phase2-2b-manifest.toml` and `docs/guardian/30_TDD/gates/
//! phase2-2b-tdd.md`.
//!
//! This file proves the real, end-to-end path: a genuine
//! `capability_registry_tick`-shaped snapshot diff (via
//! `guardian_core::providers::health::HealthTransitionProducer`) feeds a
//! real event through the one shared `IngressClock`/`CorrelationEngine`
//! admission point, and `Incidents1::list_incidents` — served over a
//! private D-Bus connection exactly as `guardian-daemon`'s `main()` wires
//! it — returns that incident, correctly round-tripped. It also proves
//! Gate 2b adds no new public D-Bus surface (`P2-API-002`) by diffing
//! live introspection against the recorded G9 baseline.

use std::sync::{Arc, Mutex};

use guardian_core::correlation::{
    AdmitOutcome, CorrelationEngine, CorrelationPolicy, FreshHealthObservation, IngressClock,
};
use guardian_core::providers::health::HealthTransitionProducer;
use guardian_daemon::dbus_surface::{
    self, CAPABILITIES_OBJECT_PATH, Capabilities1, INCIDENTS_OBJECT_PATH, Incidents1,
    TRANSACTIONS_OBJECT_PATH, Transactions1,
};
use guardian_provider_api::{
    Availability, BootAvailability, CapabilityId, CapabilityRecord, DiagnosticCost, Health,
    InterfaceKind, Knowledge, PrivilegeRequirement, ProviderId,
};
use guardian_testkit::PrivateSessionBus;
use zbus::blocking::{Connection, Proxy, connection};

fn capability(cap_id: &str, availability: Availability, health: Health) -> CapabilityRecord {
    CapabilityRecord {
        capability_id: CapabilityId::new(cap_id).unwrap(),
        provider_id: ProviderId::new("guardian.g8.systemd").unwrap(),
        provider_version: None,
        availability,
        health,
        read_support: true,
        write_support: false,
        authorization_ownership: Knowledge::Unknown,
        privilege_requirement: PrivilegeRequirement::NoDirectPrivilege,
        boot_availability: [BootAvailability::SystemBus].into_iter().collect(),
        interface_kind: InterfaceKind::DBus,
        interface_name: None,
        interface_hash: None,
        diagnostic_cost: DiagnosticCost::default(),
        last_observed_at: "2026-09-06T00:00:00Z".to_owned(),
    }
}

fn with_private_connection(test: impl FnOnce(&Connection)) {
    let bus = PrivateSessionBus::launch().expect("private D-Bus must launch");
    let connection = connection::Builder::address(bus.address())
        .expect("parse private D-Bus address")
        .build()
        .expect("connect to private D-Bus");
    test(&connection);
}

fn introspect(connection: &Connection, path: &str) -> String {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().expect("unique bus name").as_str(),
        path,
        "org.freedesktop.DBus.Introspectable",
    )
    .expect("create introspection proxy");
    proxy
        .call("Introspect", &())
        .expect("live introspection works")
}

/// Extracts every `<method name="...">` belonging only to interfaces
/// under the `io.github.cliffthelin.` namespace from a raw introspection
/// XML blob — deliberately excluding the standard `org.freedesktop.DBus.*`
/// interfaces every object also exports (`Introspectable`, `Peer`,
/// `Properties`), since those are not part of this gate's "no new
/// Guardian surface" claim. Enough to compare a method surface without
/// pulling in a full XML parser dependency this crate does not already
/// have as a non-dev dependency (the existing `dbus_contract.rs` Layer-2
/// test already owns that heavier comparison for `Guardian1` itself; this
/// is a lighter, additive check scoped to the three G9 interfaces this
/// gate touches).
fn method_names(xml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_guardian_interface = false;
    for line in xml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<interface name=\"") {
            in_guardian_interface = rest.starts_with("io.github.cliffthelin.");
            continue;
        }
        if trimmed == "</interface>" {
            in_guardian_interface = false;
            continue;
        }
        if !in_guardian_interface {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("<method name=\"") {
            if let Some(end) = rest.find('"') {
                names.push(rest[..end].to_owned());
            }
        }
    }
    names.sort();
    names
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn record_check(failures: &mut Vec<String>, contract_id: &str, check: impl FnOnce()) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)) {
        failures.push(format!(
            "{contract_id}: {}",
            panic_message(payload.as_ref())
        ));
    }
}

/// Builds the real, end-to-end `P2-API-001` fixture (Gate 2b
/// health-lifecycle integration repair R3): a genuine snapshot diff (the
/// same shape `capability_registry_tick` produces) drives the real
/// provider-health producer; the one real resulting `Event` is admitted
/// through the shared `IngressClock`/`CorrelationEngine` admission point
/// -- exactly `guardian-daemon`'s own production wiring; a later,
/// unchanged-still-bad fresh snapshot proves the producer stays
/// edge-triggered (zero new `Event`s); and `CorrelationEngine::
/// advance_health_dwell`, fed a fresh re-observation of that same
/// snapshot, promotes the pending candidate and opens the incident. No
/// `Event` is ever cloned or has its `event_id` mutated anywhere in this
/// sequence -- the fabrication the pre-repair fixture used to compensate
/// for the daemon not yet calling `advance_health_dwell` at all.
fn build_engine_with_one_real_open_incident() -> Arc<Mutex<CorrelationEngine>> {
    let mut ingress_clock = IngressClock::new();
    let mut engine = CorrelationEngine::new(CorrelationPolicy::default());
    let mut health_producer = HealthTransitionProducer::new();

    // Tick 1: baseline snapshot -- the producer has nothing to diff
    // against yet, so (correctly) no Event is produced.
    let baseline = vec![capability(
        "systemd.unit.state",
        Availability::Available,
        Health::Healthy,
    )];
    assert!(health_producer.observe(&baseline).is_empty());

    // Tick 2: a real Health/Availability transition -- exactly the shape
    // `capability_registry_tick` would observe if the real
    // `systemd-logind.service` unit actually went unavailable. Carries
    // complete from/to provenance (R1) since the real producer now
    // populates it.
    let bad = vec![capability(
        "systemd.unit.state",
        Availability::Unavailable,
        Health::Error,
    )];
    let events = health_producer.observe(&bad);
    assert_eq!(events.len(), 1, "the real diff must emit exactly one Event");

    // The first Bad reading for a brand-new key only seeds the debounce
    // ring (§4.2's dwell rule) -- no incident yet.
    let base = std::time::Instant::now();
    let admitted = ingress_clock.admit_at(events.into_iter().next().unwrap(), base);
    let first_result = engine.admit(&admitted);
    assert!(
        matches!(first_result.outcome, AdmitOutcome::DebouncePending),
        "the first Bad reading for a brand-new key must only seed the debounce ring"
    );

    // Tick 3: a later, unchanged-still-bad fresh snapshot. The real
    // producer stays edge-triggered under repeated bad readings (R1) --
    // zero new Events -- so promotion below can only come from
    // fresh-observation dwell-advancement (R2), never a second Event.
    let still_bad_events = health_producer.observe(&bad);
    assert!(
        still_bad_events.is_empty(),
        "an unchanged snapshot pair must emit no Event, even under repeated bad readings"
    );

    // Gate 2a's `advance_health_dwell`, fed this same fresh
    // re-observation (R2): promotes the pending candidate past
    // `health_min_dwell` and opens exactly one Incident -- no second
    // Event constructed or cloned.
    let capability_id = CapabilityId::new("systemd.unit.state").unwrap();
    let observation = FreshHealthObservation::classify(Availability::Unavailable, Health::Error);
    let dwell = CorrelationPolicy::default().health_min_dwell;
    let open_result = engine.advance_health_dwell(
        &capability_id,
        observation,
        base + dwell,
        ingress_clock.sequence(),
    );
    assert!(
        matches!(open_result.outcome, AdmitOutcome::IncidentOpened(_)),
        "fresh-observation advancement alone must open exactly one incident once dwell has elapsed"
    );

    Arc::new(Mutex::new(engine))
}

/// `P2-API-001`: `Incidents1.ListIncidents()` returns the real,
/// non-empty incident the engine actually opened, correctly round-tripped
/// through `IncidentWire`.
fn assert_p2_api_001_list_incidents_returns_the_real_open_incident(connection: &Connection) {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().unwrap().as_str(),
        INCIDENTS_OBJECT_PATH,
        dbus_surface::INCIDENTS_INTERFACE,
    )
    .unwrap();
    let wires: Vec<dbus_surface::IncidentWire> = proxy
        .call("ListIncidents", &())
        .expect("ListIncidents call succeeds");

    assert_eq!(
        wires.len(),
        1,
        "ListIncidents() must return the real, non-empty incident the engine opened"
    );
    let incident = &wires[0];
    assert_eq!(incident.3, "open");
    assert_eq!(incident.6, "systemd.unit.state");
    assert_eq!(incident.5, "confirmed");
    assert!(
        !incident.0.is_empty(),
        "incident_id must be a real, non-empty identifier, not a placeholder"
    );
}

/// Test-only second object path this integration layer serves a
/// recovery-scenario `Incidents1` instance at, on the SAME private
/// connection `p2_2b_dbus_contract_suite` already builds -- never a
/// second `PrivateSessionBus`/blocking `zbus` connection (see that
/// test's own doc comment on the real concurrency hazard two such
/// connections triggered). This is not additional real production D-Bus
/// surface: `guardian-daemon`'s own `main()` never serves `Incidents1`
/// here -- it exists only so this private test bus can independently
/// exercise Gate 2b health-lifecycle integration repair R4's
/// closed-incident round trip alongside R3's already-open-incident
/// engine served at the real `INCIDENTS_OBJECT_PATH`.
const INCIDENTS_RECOVERY_TEST_OBJECT_PATH: &str =
    "/io/github/cliffthelin/Guardian1/IncidentsRecoveryTest";

/// Gate 2b health-lifecycle integration repair R4 (symmetric recovery):
/// mirrors `build_engine_with_one_real_open_incident`'s real opening
/// sequence, then drives the closure counterpart the same way -- a real
/// recovery `Event` seeds the recovery debounce candidate, a later fresh
/// snapshot proves the producer stays edge-triggered under repeated good
/// readings (zero new `Event`s), and `advance_health_dwell` alone (no
/// second `Event` manufactured, no clone/mutation anywhere) closes the
/// incident once recovery dwell has elapsed.
fn build_engine_with_one_real_closed_incident() -> Arc<Mutex<CorrelationEngine>> {
    let mut ingress_clock = IngressClock::new();
    let mut engine = CorrelationEngine::new(CorrelationPolicy::default());
    let mut health_producer = HealthTransitionProducer::new();
    let capability_id = CapabilityId::new("systemd.unit.state").unwrap();
    let dwell = CorrelationPolicy::default().health_min_dwell;

    // -- Open the incident (identical shape to
    // `build_engine_with_one_real_open_incident` above). --
    let baseline = vec![capability(
        "systemd.unit.state",
        Availability::Available,
        Health::Healthy,
    )];
    assert!(health_producer.observe(&baseline).is_empty());

    let bad = vec![capability(
        "systemd.unit.state",
        Availability::Unavailable,
        Health::Error,
    )];
    let events = health_producer.observe(&bad);
    assert_eq!(events.len(), 1, "the real diff must emit exactly one Event");
    let base = std::time::Instant::now();
    let admitted = ingress_clock.admit_at(events.into_iter().next().unwrap(), base);
    let first_result = engine.admit(&admitted);
    assert!(
        matches!(first_result.outcome, AdmitOutcome::DebouncePending),
        "the first Bad reading for a brand-new key must only seed the debounce ring"
    );

    let still_bad_events = health_producer.observe(&bad);
    assert!(
        still_bad_events.is_empty(),
        "an unchanged snapshot pair must emit no Event"
    );

    let bad_observation =
        FreshHealthObservation::classify(Availability::Unavailable, Health::Error);
    let open_result = engine.advance_health_dwell(
        &capability_id,
        bad_observation,
        base + dwell,
        ingress_clock.sequence(),
    );
    assert!(
        matches!(open_result.outcome, AdmitOutcome::IncidentOpened(_)),
        "fresh-observation advancement alone must open exactly one incident once dwell has elapsed"
    );

    // -- Recovery: a real diff back to Good is one real Event -- still no
    // clone/mutation, a genuinely new transition -- which seeds the
    // recovery debounce candidate
    // (`admit_health_with_open_incident`'s existing Good-direction
    // path, untouched by this repair). --
    let good = vec![capability(
        "systemd.unit.state",
        Availability::Available,
        Health::Healthy,
    )];
    let recovery_events = health_producer.observe(&good);
    assert_eq!(
        recovery_events.len(),
        1,
        "the real diff back to Good must emit exactly one Event"
    );
    let recovery_time = base + dwell + std::time::Duration::from_millis(1);
    let recovery_admitted =
        ingress_clock.admit_at(recovery_events.into_iter().next().unwrap(), recovery_time);
    let recovery_result = engine.admit(&recovery_admitted);
    assert!(
        matches!(recovery_result.outcome, AdmitOutcome::DebouncePending),
        "the first Good reading only seeds the recovery debounce candidate"
    );

    // -- Sustained good: a later fresh snapshot, unchanged from the
    // last-observed Good state -- the producer emits zero new Events,
    // even during recovery, proving R1's edge-triggered property holds
    // symmetrically. --
    let still_good_events = health_producer.observe(&good);
    assert!(
        still_good_events.is_empty(),
        "an unchanged snapshot pair must emit no Event, even during recovery"
    );

    // `advance_health_dwell` alone -- no second Event -- closes the
    // incident once recovery dwell has elapsed.
    let good_observation =
        FreshHealthObservation::classify(Availability::Available, Health::Healthy);
    let close_time = recovery_time + dwell;
    let close_result = engine.advance_health_dwell(
        &capability_id,
        good_observation,
        close_time,
        ingress_clock.sequence(),
    );
    assert!(
        matches!(close_result.outcome, AdmitOutcome::IncidentClosed(_)),
        "fresh-observation advancement alone must close the incident once recovery dwell has elapsed"
    );

    Arc::new(Mutex::new(engine))
}

/// Gate 2b health-lifecycle integration repair R4 evidence:
/// `Incidents1::list_incidents`, served over the private test bus at
/// [`INCIDENTS_RECOVERY_TEST_OBJECT_PATH`], reflects the real closure --
/// present in the closed-incident set, status `"closed"`.
fn assert_gate_2b_repair_r4_list_incidents_reflects_the_real_closure(connection: &Connection) {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().unwrap().as_str(),
        INCIDENTS_RECOVERY_TEST_OBJECT_PATH,
        dbus_surface::INCIDENTS_INTERFACE,
    )
    .unwrap();
    let wires: Vec<dbus_surface::IncidentWire> = proxy
        .call("ListIncidents", &())
        .expect("ListIncidents call succeeds");

    assert_eq!(wires.len(), 1, "the closed incident must still be listed");
    let incident = &wires[0];
    assert_eq!(
        incident.3, "closed",
        "ListIncidents must reflect the closure"
    );
    assert_eq!(incident.6, "systemd.unit.state");
}

/// `P2-API-002`: no new `Incidents1`/`Capabilities1`/`Transactions1`
/// method is added beyond what G9 already shipped (`docs/evidence/g9/
/// g9-{incidents,capabilities,transactions}-introspect.txt`) -- Gate 2b
/// only populates `ListIncidents`'s return value, it adds no member.
fn assert_p2_api_002_no_new_dbus_surface(connection: &Connection) {
    let capabilities_methods = method_names(&introspect(connection, CAPABILITIES_OBJECT_PATH));
    let mut expected_capabilities = vec![
        "ListBlockers".to_owned(),
        "ListCapabilities".to_owned(),
        "PsiSummary".to_owned(),
    ];
    expected_capabilities.sort();
    assert_eq!(
        capabilities_methods, expected_capabilities,
        "Capabilities1 must gain no new method beyond the recorded G9 baseline"
    );

    let incidents_methods = method_names(&introspect(connection, INCIDENTS_OBJECT_PATH));
    assert_eq!(
        incidents_methods,
        vec!["ListIncidents".to_owned()],
        "Incidents1 must gain no new method beyond the recorded G9 baseline"
    );

    let transactions_methods = method_names(&introspect(connection, TRANSACTIONS_OBJECT_PATH));
    assert_eq!(
        transactions_methods,
        vec!["ListTransactions".to_owned()],
        "Transactions1 must gain no new method beyond the recorded G9 baseline"
    );
}

/// A single private-bus test combining `P2-API-001`/`002`'s evidence in
/// one `#[test]` (following this crate's own `dbus_contract.rs`
/// precedent for a private-bus contract suite), deliberately never two
/// independent `#[test]` functions each launching their own
/// `PrivateSessionBus`/blocking `zbus` connection: doing that was tried
/// first and reliably deadlocked under the default parallel test harness
/// (two `zbus::blocking` connections racing to build their first
/// connection concurrently in one process) -- a real concurrency hazard
/// discovered during this gate's own RED→GREEN pass, not a theoretical
/// one, and not safely worked around by forcing `--test-threads=1` since
/// the manifest's own validation command runs plain
/// `cargo test --workspace`.
#[test]
fn p2_2b_dbus_contract_suite() {
    with_private_connection(|connection| {
        let engine = build_engine_with_one_real_open_incident();
        let recovery_engine = build_engine_with_one_real_closed_incident();
        let capabilities_snapshot = Arc::new(Mutex::new(Vec::<CapabilityRecord>::new()));

        connection
            .object_server()
            .at(
                CAPABILITIES_OBJECT_PATH,
                Capabilities1::new(Arc::clone(&capabilities_snapshot)),
            )
            .unwrap();
        connection
            .object_server()
            .at(INCIDENTS_OBJECT_PATH, Incidents1::new(Arc::clone(&engine)))
            .unwrap();
        connection
            .object_server()
            .at(
                INCIDENTS_RECOVERY_TEST_OBJECT_PATH,
                Incidents1::new(Arc::clone(&recovery_engine)),
            )
            .unwrap();
        connection
            .object_server()
            .at(TRANSACTIONS_OBJECT_PATH, Transactions1)
            .unwrap();

        let mut failures = Vec::new();
        record_check(&mut failures, "P2-API-001", || {
            assert_p2_api_001_list_incidents_returns_the_real_open_incident(connection);
        });
        record_check(&mut failures, "P2-API-002", || {
            assert_p2_api_002_no_new_dbus_surface(connection);
        });
        record_check(&mut failures, "gate-2b-repair-R4", || {
            assert_gate_2b_repair_r4_list_incidents_reflects_the_real_closure(connection);
        });
        assert!(
            failures.is_empty(),
            "Gate 2b private-bus contract failures:\n{}",
            failures.join("\n")
        );
    });
}
