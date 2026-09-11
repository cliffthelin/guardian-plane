//! `guardian-daemon` — G7's unprivileged production process (ADR-002 Model
//! B; G7 implementation handoff §2). Owns `io.github.cliffthelin.Guardian1`
//! on the real system bus. Never relays a client's privileged write
//! request to `guardian-helper` (§2.3) — this binary has no code path that
//! constructs a call against `GuardianHelper1`, and never will while
//! `GuardianHelper1` remains the sole privileged-mutation surface.
//!
//! **G8 update to the G7 "no D-Bus client/proxy construction" claim**: G7's
//! original doc comment stated this binary constructed no D-Bus client/
//! proxy of any kind. That was true at G7 and is no longer true: G8 adds
//! exactly six read-only provider client proxies
//! ([`guardian_core::providers`] — systemd1, `PSI`, login1, `UDisks2`,
//! `UPower`, `Accounts`), driven from [`capability_registry_tick`]. None of
//! the six is `GuardianHelper1`; none performs a write; the G7 invariant
//! this comment originally protected (never proxying to the privileged
//! helper) is unchanged and still holds. Only the broader, imprecise
//! restatement of that invariant needed correcting.
//!
//! **Repair of the independent audit's public-API-scope finding**: this
//! binary previously served an additive `Guardian1.Transactions1` object
//! with one Class B evidence method
//! (`AttemptProviderDelegatedWrite`). That method was not required by any
//! of the nine G7 normative IDs and was removed — `Guardian1` now serves
//! only the frozen G0 contract
//! ([`guardian_daemon::GuardianContract`], unchanged since G0). Class B's
//! architecture (provider-owned authorization, daemon-local transaction
//! ownership, no helper involvement) is proved instead by a disposable
//! prototype under `tests/vm/g7-class-b-prototype/`, following the same
//! precedent G2's Model B evidence used — never merged into production.
//! G8's Capability Registry population is internal state only (handoff
//! §12) — it adds no new `Guardian1` object or method either.
//!
//! **G9 update**: this binary now also serves three real, read-only
//! interfaces at their own object paths — `Capabilities1`, `Incidents1`,
//! `Transactions1` (`crates/guardian-daemon/src/dbus_surface.rs`, ADR-001's
//! own worked example for the shape). `Guardian1` remains exactly
//! `ContractVersion`/`ServiceState`; none of the three new interfaces
//! shares an object path or method with it. `Incidents1`/`Transactions1`
//! are genuinely, honestly empty in this gate (no incident producer, no
//! daemon-side transaction store exists) — see `dbus_surface`'s own doc
//! comment for the two paths this explicitly forbids for populating
//! `Transactions1` (reading `guardian-helper`'s state; any new
//! `guardian-daemon` -> `GuardianHelper1` call).
//!
//! **Repair of the independent audit's G5 FC-2 finding**: this binary
//! evaluates `guardian_core::budget::recorder_policy_for()` on a real,
//! periodic, no-privilege monitoring tick and records a real `Event` into
//! a real `BoundedRecorder` — genuine production wiring, not a fixture.
//! It does **not** claim FC-2 closed: no real spill/retention sink exists
//! yet for either policy branch. See the module doc on
//! [`monitoring_tick`].
//!
//! **Phase 2 Gate 2b update**: this binary now also owns the single
//! daemon-wide `guardian_core::correlation::IngressClock`/
//! `CorrelationEngine` admission point (gate TDD R2) — every event source
//! this binary produces (the existing monitoring-tick producer, and the
//! new provider-health snapshot-diff producer) feeds through it, never
//! constructing its own ingress clock/sequence. The debounce ring's
//! `CapacityRejected` outcome (Gate 2a, `guardian-core`, zero I/O there by
//! design) is logged here — the daemon-owned half of `P2-REC-003` — using
//! this file's existing `eprintln!("[guardian-daemon] ...")` convention,
//! never a new logging facility. `guardian-daemon`'s live PSI event
//! production (real kernel `poll()`-triggered events,
//! `guardian_core::providers::psi::PsiEventSource`) is **not** added by
//! this gate: no such production loop exists anywhere in this binary
//! today (it exists only as a library capability plus a standalone
//! evidence example, `crates/guardian-core/examples/
//! g8_psi_trigger_evidence.rs`), Gate 2b's own R7 only conditionally
//! requires PSI-specific glue code ("if daemon wiring adds any..."), and
//! no `P2-*` ID this gate owns requires it — wiring a real PSI production
//! thread into this process remains a future gate's scope.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use guardian_core::budget::{self, FreeSpaceState};
use guardian_core::correlation::{
    AdmitOutcome, CorrelationEngine, CorrelationPolicy, FreshHealthObservation, IngressClock,
};
use guardian_core::event::Event;
use guardian_core::providers::health::HealthTransitionProducer;
use guardian_core::providers::psi::{PsiAvailability, PsiResourceStatus};
use guardian_core::providers::udisks::{TopologyTracker, UdisksProvider};
use guardian_core::psi::{PressureSeverity, SeverityThresholds};
use guardian_core::recorder::BoundedRecorder;
use guardian_core::risk::Risk;
use guardian_daemon::psi_ingress::{self, PsiMonitorConfig, PsiResourceMonitor};
use guardian_daemon::{GuardianContract, dbus_surface};
use guardian_provider_api::{CapabilityRecord, EventId, ProviderId};

const WELL_KNOWN_NAME: &str = "io.github.cliffthelin.Guardian1";
const OBJECT_PATH: &str = "/io/github/cliffthelin/Guardian1";
const PROVIDER_ID: &str = "guardian.g7.daemon-monitor";
const CAPABILITY_REGISTRY_TICK_INTERVAL: Duration = Duration::from_secs(30);
const RECORDER_CAPACITY: usize = 256;
const MONITORING_TICK_INTERVAL: Duration = Duration::from_secs(30);

fn state_dir() -> PathBuf {
    std::env::var_os("GUARDIAN_DAEMON_STATE_DIR")
        .map_or_else(|| PathBuf::from("/var/lib/guardian/daemon"), PathBuf::from)
}

fn provider_id() -> ProviderId {
    ProviderId::new(PROVIDER_ID).expect("fixed literal is a valid ProviderId")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// A real (not fixture) free-space signal, deliberately minimal: attempts
/// a small real write to `state`'s own probe file and observes whether the
/// OS reports `StorageFull`. This is a genuine host interaction — not a
/// full disk-usage provider (that remains G8 scope).
fn probe_free_space(state: &Path) -> FreeSpaceState {
    let probe_path = state.join(".free-space-probe");
    match fs::write(&probe_path, b"x") {
        Ok(()) => {
            let _ = fs::remove_file(&probe_path);
            FreeSpaceState::Sufficient
        }
        Err(error) if error.kind() == std::io::ErrorKind::StorageFull => FreeSpaceState::Critical,
        Err(_) => FreeSpaceState::Sufficient,
    }
}

fn humantime_wall() -> String {
    format!("{}s-since-epoch", now_secs())
}

/// The single daemon-owned `CorrelationIngress` admission point (gate TDD
/// R2): every event source this binary produces calls this and only this
/// function to reach the correlation engine — no producer constructs its
/// own `IngressClock`/sequence. The daemon-side half of `P2-REC-003`
/// (Gate 2a already performs the zero-I/O library half, `guardian-core`'s
/// `CorrelationEngine::reject_capacity()`) lives here: a `CapacityRejected`
/// outcome produces exactly one operational log line, using this file's
/// existing `eprintln!("[guardian-daemon] ...")` convention.
fn admit_event(
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
    event: Event,
) {
    let ingress = ingress_clock.lock().unwrap().admit(event);
    let result = engine.lock().unwrap().admit(&ingress);
    if let AdmitOutcome::CapacityRejected(rejected) = &result.outcome {
        eprintln!(
            "[guardian-daemon] correlation debounce ring at capacity: rejected new candidate capability_id={} (rejection_count={})",
            rejected.capability_id, rejected.rejection_count
        );
    }
}

/// Genuine, no-privilege, Class C periodic monitoring work (G7 handoff
/// §2.4: "the recorder is a Class C/monitoring concern and lives in
/// `guardian-daemon`"). Every tick: probes real free space, calls G5's
/// `recorder_policy_for()` on that real input, records a real `Event`
/// into the real bounded recorder, and logs which policy branch was
/// selected.
///
/// **What this closes and what it does not (G5 FC-2)**: `RecorderPolicy`
/// is genuinely evaluated against a real signal on a real, permanent
/// production runtime path (not merely constructed and discarded, and not
/// only exercised once at a client's request) — this is real
/// improvement over the original candidate, where the policy was only
/// ever evaluated inside the now-removed Class B evidence method. It does
/// **not** close FC-2: neither `RecorderPolicy::Normal` nor
/// `RecorderPolicy::MemoryFirst` yet drives a real spill/retention sink —
/// no such sink exists in this build. FC-2 remains open; closure is
/// assigned to the first gate that instantiates an actual spill/retention
/// path (see `docs/evidence/g7/G7_DAEMON_HELPER_EVIDENCE.md`'s corrected
/// disposition).
fn monitoring_tick(
    recorder: &Mutex<BoundedRecorder>,
    state: &Path,
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
) {
    let free_space = probe_free_space(state);
    let policy = budget::recorder_policy_for(free_space);

    let event_type = "daemon_monitoring_tick";
    let event = Event {
        event_id: EventId::new("guardian.g7.daemon-tick").expect("fixed literal is valid"),
        timestamp_monotonic: now_secs(),
        timestamp_wall: humantime_wall(),
        source_provider: provider_id(),
        event_type: event_type.to_owned(),
        resource_refs: Vec::new(),
        severity: Risk::Observe,
        normalized_key: guardian_core::event::normalize_key(event_type),
        raw_reference: event_type.to_owned(),
        attributes: std::collections::BTreeMap::new(),
    };

    // The same Event already produced for the recorder also feeds the
    // one shared correlation-ingress admission point (gate TDD R2) — no
    // second provider read, no second event construction.
    admit_event(ingress_clock, engine, event.clone());

    let mut guard = recorder.lock().unwrap();
    guard.record(event);
    eprintln!(
        "[guardian-daemon] monitoring tick: recorder len={} dropped={} policy={policy:?} free_space={free_space:?} (FC-2 not closed: no spill sink wired)",
        guard.len(),
        guard.dropped_count()
    );
}

/// Real, internal G8 Capability Registry population (implementation
/// handoff §11/§12): connects to the real system bus as a client — the
/// six read-only provider proxies described in the module doc — and logs
/// a genuine snapshot. This is internal `guardian-daemon` state only: no
/// `Guardian1` object or method exposes it to any client (handoff §12
/// assigns that to G9, when a real consumer exists). One provider being
/// unreachable never stops the others: [`guardian_core::providers::
/// registry::populate_registry`] treats each of the six independently.
fn capability_registry_tick(
    connection: &zbus::Connection,
    health_producer: &mut HealthTransitionProducer,
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
    psi_availability: &Mutex<PsiAvailability>,
) -> Vec<CapabilityRecord> {
    // Acceptance-repair Blocker 4: the registry observes the SAME live
    // per-resource PSI state `psi_ingress`'s production path maintains
    // (Blocker 3) -- it never re-probes `/proc/pressure` by pathname,
    // which the accepted `ProcSubset=pid` sandbox denies unconditionally
    // from inside this daemon. Snapshotted (cloned) here, outside the
    // lock, so the lock is never held across the `.await` below.
    let psi_snapshot = psi_availability.lock().unwrap().clone();
    let records = async_io::block_on(guardian_core::providers::registry::populate_registry(
        connection,
        Some(&psi_snapshot),
    ));
    let available = records
        .iter()
        .filter(|r| r.availability.is_usable())
        .count();
    eprintln!(
        "[guardian-daemon] capability registry tick: {available}/{} capabilities available",
        records.len()
    );

    // Gate 2b's new provider-health transition producer (gate TDD R1):
    // diffs this snapshot against the previous one and feeds any
    // resulting Events through the one shared ingress point (R2) — no
    // new provider read is performed here beyond the snapshot already
    // collected above.
    for event in health_producer.observe(&records) {
        admit_event(ingress_clock, engine, event);
    }

    // Gate 2b health-lifecycle integration repair R2: after admitting any
    // real diff-driven Events above, also advance Gate 2a's
    // fresh-observation dwell-promotion path once per capability in this
    // same fresh snapshot — no second provider/registry read, no
    // fabricated Event.
    advance_health_dwell_for_snapshot(&records, ingress_clock, engine);

    records
}

/// Gate 2b health-lifecycle integration repair R2: calls
/// [`guardian_core::correlation::CorrelationEngine::advance_health_dwell`]
/// once per [`CapabilityRecord`] in `records` — the same fresh snapshot
/// [`capability_registry_tick`] already collected, immediately after
/// admitting any real diff-driven `Event`s from it. Every call in one
/// invocation of this function shares a single `(Instant::now(),
/// IngressClock::sequence())` ingress-order pair, captured once here
/// (not once per capability), per the gate TDD's "one tick, one fresh
/// observation" framing. `advance_health_dwell` is a documented no-op
/// (`AdmitOutcome::Ignored`) for any `capability_id` with no pending
/// debounce candidate, so calling it unconditionally for every record in
/// the snapshot is safe: a capability absent from this snapshot is
/// simply not called this tick (never treated as an implicit `Good` or
/// `Bad`), and a capability present but unresolved
/// ([`FreshHealthObservation::Unresolved`]) never falsely advances a
/// pending candidate (Gate 2a's own invariant, consumed here unmodified).
fn advance_health_dwell_for_snapshot(
    records: &[CapabilityRecord],
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
) {
    let ingress_instant = std::time::Instant::now();
    let ingress_sequence = ingress_clock.lock().unwrap().sequence();
    let mut engine_guard = engine.lock().unwrap();
    for record in records {
        let observation = FreshHealthObservation::classify(record.availability, record.health);
        engine_guard.advance_health_dwell(
            &record.capability_id,
            observation,
            ingress_instant,
            ingress_sequence,
        );
    }
}

fn replace_registry_snapshot(
    snapshot: &Mutex<Vec<CapabilityRecord>>,
    records: Vec<CapabilityRecord>,
) {
    *snapshot.lock().unwrap() = records;
}

/// Production PSI trigger/classification parameters. `avg10` is the
/// percentage of the last ten seconds during which work was stalled on the
/// resource, so these are percentages: sustained 20% stall is elevated,
/// sustained 50% is critical. The kernel trigger asks to be woken when
/// 100ms of stall accumulates within a 2s window — frequent enough to
/// observe a real transition, far above the kernel's 500ms minimum window.
const PSI_MONITOR_CONFIG: PsiMonitorConfig = PsiMonitorConfig {
    thresholds: SeverityThresholds::new(20.0, 50.0),
    // `Critical`, not `Elevated` (2026-09-07 threshold repair; see
    // `docs/evidence/p2/PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md`
    // §F.15 Finding 1). `classify()` (`correlation.rs`, Gate 2a, forbidden
    // scope) opens a PSI incident only for `Risk::High`/`Critical`,  but
    // `ThresholdMonitor::observe` reports a crossing only relative to
    // `event_threshold` -- with `event_threshold = Elevated`, an
    // `Elevated -> Critical` transition emits nothing (both readings are
    // already above the Elevated threshold), so a `Risk::High` event could
    // only ever come from a single-step `Nominal -> Critical` observation.
    // `avg10` is a 10-second kernel EWMA sampled at most once per
    // `trigger_window_us` below (2s in production), so that single step is
    // arithmetically unreachable under sustained real stall (from `x =
    // 20`, one 2s step reaches at most `20 + 80*(1 - e^-0.2) = 34.5`, still
    // short of the 50.0 Critical threshold at any load) -- proven on the
    // real VM: 70s of CPU pressure peaking at `avg10=98.97` opened zero
    // incidents. Setting `event_threshold = Critical` makes the monitored
    // boundary match the boundary `classify()` actually cares about: any
    // transition crossing into or out of Critical emits, however
    // gradually pressure arrived there, without widening
    // `trigger_window_us` (which would only paper over the deeper
    // mismatch, not fix it -- see the evidence document's Finding 1 for
    // the full analysis of why widening the window was rejected).
    event_threshold: PressureSeverity::Critical,
    trigger_threshold_us: 100_000,
    trigger_window_us: 2_000_000,
};

/// A descriptor that reports readiness without `POLLPRI` this many times
/// in a row is not behaving like a kernel PSI trigger. The worker stops
/// rather than spinning: `P1-PSI-004` forbids a busy loop, and a silent
/// hot loop would be worse than an honest loss of PSI observability.
const PSI_MAX_CONSECUTIVE_IDLE_WAKES: u32 = 64;

/// One iteration's result for a PSI worker.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PsiStepOutcome {
    /// A real crossing was classified and admitted through the shared
    /// ingress. Carries the admitted event's id for operational logging.
    Admitted(EventId),
    /// A real wake that the accepted G5 monitor decided was not a crossing.
    NoCrossing,
    /// The wait returned without a `POLLPRI` event (timeout, or a
    /// descriptor that does not behave like a PSI trigger).
    Idle,
    /// A hard failure for this resource.
    Failed,
}

/// The third instance of this binary's established producer pattern
/// (`monitoring_tick`, `capability_registry_tick`): a worker thread per PSI
/// resource, each feeding the one shared `IngressClock`/`CorrelationEngine`
/// admission point through [`admit_event`].
///
/// Returns the number of monitors actually started (so startup logging and
/// tests can observe it — zero is a normal outcome, not an error: a
/// PSI-less kernel, a container, or `OpenFile=`'s `:graceful` option
/// dropping absent paths all produce it, and the daemon runs normally with
/// PSI simply reported unavailable rather than silently reported as "no
/// pressure") together with the **shared, truthful, per-resource**
/// [`PsiAvailability`] state (acceptance-repair Blocker 3) — the same
/// state `main()` threads into [`capability_registry_tick`] so the
/// Capability Registry (Blocker 4) observes exactly what this function's
/// own startup/runtime logging observes, never a second, independent
/// probe.
fn spawn_psi_ingress(
    plan: &psi_ingress::PsiDescriptorPlan,
    ingress_clock: &Arc<Mutex<IngressClock>>,
    engine: &Arc<Mutex<CorrelationEngine>>,
) -> (usize, Arc<Mutex<PsiAvailability>>) {
    let report = psi_ingress::start_psi_monitors(plan, PSI_MONITOR_CONFIG);

    for failure in &report.failures {
        eprintln!("[guardian-daemon] {failure}");
    }
    let started = report.monitors.len();

    // Acceptance-repair Blocker 3: truthful, individual per-resource
    // reporting -- never a single aggregate "N/M monitored" line that
    // cannot say *which* resource is missing. cpu/memory/io are each
    // reported explicitly, every startup, whether available or not.
    let availability = Arc::new(Mutex::new(psi_ingress::availability_from_startup(
        plan, &report,
    )));
    for (kind, name) in psi_ingress::PSI_MONITORED_RESOURCES {
        let status = availability
            .lock()
            .unwrap()
            .get(kind)
            .cloned()
            .unwrap_or(PsiResourceStatus::NoDescriptor);
        eprintln!(
            "[guardian-daemon] PSI {name}: {} ({})",
            if status.is_available() {
                "available"
            } else {
                "unavailable"
            },
            status.reason()
        );
    }
    eprintln!(
        "[guardian-daemon] PSI ingress: {started}/{} inherited descriptors monitored ({} failed)",
        report.attempts,
        report.failures.len()
    );

    for monitor in report.monitors {
        let ingress_clock = Arc::clone(ingress_clock);
        let engine = Arc::clone(engine);
        let availability = Arc::clone(&availability);
        std::thread::spawn(move || {
            psi_ingress_loop(monitor, &ingress_clock, &engine, &availability);
        });
    }
    (started, availability)
}

/// One PSI resource's worker loop. Parks in the kernel on
/// `poll(POLLPRI)` — never a sleep-and-recheck loop — and exits on a hard
/// failure for its resource rather than retrying: a PSI trigger belongs to
/// its open file description, so re-arming in place is impossible and
/// re-registering on the same description would only return `EBUSY`. Every
/// exit path updates `availability` for **this resource only**
/// (acceptance-repair Blocker 3) before returning, so the daemon's
/// truthful per-resource state — and, through it, the Capability Registry
/// (Blocker 4) — reflects a later runtime failure, not only the startup
/// snapshot.
fn psi_ingress_loop(
    mut monitor: PsiResourceMonitor,
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
    availability: &Mutex<PsiAvailability>,
) {
    let kind = monitor.kind();
    let mut consecutive_idle = 0u32;
    loop {
        match psi_ingress_step(&mut monitor, None, ingress_clock, engine) {
            PsiStepOutcome::Admitted(_) | PsiStepOutcome::NoCrossing => consecutive_idle = 0,
            PsiStepOutcome::Idle => {
                consecutive_idle += 1;
                if consecutive_idle >= PSI_MAX_CONSECUTIVE_IDLE_WAKES {
                    let reason = "descriptor never reports POLLPRI; worker stopped rather than \
                                   spinning"
                        .to_owned();
                    eprintln!(
                        "[guardian-daemon] PSI {kind:?} {reason} (PSI observability lost \
                         for this resource until restart)"
                    );
                    availability
                        .lock()
                        .unwrap()
                        .set(kind, PsiResourceStatus::Degraded(reason));
                    return;
                }
            }
            PsiStepOutcome::Failed => {
                let reason =
                    "worker stopped after a hard failure; a PSI trigger cannot be re-armed \
                     in place"
                        .to_owned();
                eprintln!(
                    "[guardian-daemon] PSI {kind:?} {reason} (PSI observability lost for \
                     this resource until restart)"
                );
                availability
                    .lock()
                    .unwrap()
                    .set(kind, PsiResourceStatus::Degraded(reason));
                return;
            }
        }
    }
}

/// One worker iteration: wait in the kernel, then dispatch. Split out from
/// the loop so the dispatch half is drivable in a test without pretending a
/// fixture file supports `POLLPRI`.
fn psi_ingress_step(
    monitor: &mut PsiResourceMonitor,
    timeout: Option<Duration>,
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
) -> PsiStepOutcome {
    match monitor.wait(timeout) {
        Ok(true) => dispatch_psi_wake(monitor, ingress_clock, engine),
        Ok(false) => PsiStepOutcome::Idle,
        Err(error) => {
            eprintln!(
                "[guardian-daemon] PSI {:?} trigger wait failed: {error}",
                monitor.kind()
            );
            PsiStepOutcome::Failed
        }
    }
}

/// The daemon-owned half of the PSI path: re-read live pressure text
/// through this resource's own inherited descriptor, classify it with the
/// accepted, unmodified G5 classifier, and admit any resulting `Event`
/// through the **existing** single admission point.
///
/// Every authority field is daemon-owned. Nothing outside this process
/// supplies, influences, or self-reports a severity: the severity is
/// derived here from raw bytes this daemon read itself, and no IPC,
/// environment variable, file, or D-Bus message carries one in. The ingress
/// timestamp and ingress order come from the shared [`IngressClock`] at
/// [`admit_event`] time, never from a producer.
fn dispatch_psi_wake(
    monitor: &mut PsiResourceMonitor,
    ingress_clock: &Mutex<IngressClock>,
    engine: &Mutex<CorrelationEngine>,
) -> PsiStepOutcome {
    match monitor.dispatch_wake() {
        Ok(Some(event)) => {
            let event_id = event.event_id.clone();
            admit_event(ingress_clock, engine, event);
            PsiStepOutcome::Admitted(event_id)
        }
        Ok(None) => PsiStepOutcome::NoCrossing,
        Err(error) => {
            // A malformed, non-finite, or out-of-range reading lands here
            // as a typed error. It is reported and dropped -- it must never
            // become an admitted Event, and must never be silently treated
            // as "no pressure".
            eprintln!(
                "[guardian-daemon] PSI {:?} reading rejected: {error}",
                monitor.kind()
            );
            PsiStepOutcome::Failed
        }
    }
}

fn main() -> zbus::Result<()> {
    let state = state_dir();
    fs::create_dir_all(&state).expect("create daemon state directory");

    let recorder = std::sync::Arc::new(Mutex::new(
        BoundedRecorder::new(RECORDER_CAPACITY).expect("fixed positive capacity"),
    ));

    // Phase 2 Gate 2b: the single daemon-owned `CorrelationIngress`
    // admission point (gate TDD R2) — every event source below feeds
    // through this one `IngressClock`/`CorrelationEngine` pair, never
    // constructing its own.
    let ingress_clock = std::sync::Arc::new(Mutex::new(IngressClock::new()));
    let engine = std::sync::Arc::new(Mutex::new(CorrelationEngine::new(
        CorrelationPolicy::default(),
    )));

    let recorder_for_thread = std::sync::Arc::clone(&recorder);
    let state_for_thread = state.clone();
    let ingress_clock_for_monitoring = std::sync::Arc::clone(&ingress_clock);
    let engine_for_monitoring = std::sync::Arc::clone(&engine);
    std::thread::spawn(move || {
        loop {
            monitoring_tick(
                &recorder_for_thread,
                &state_for_thread,
                &ingress_clock_for_monitoring,
                &engine_for_monitoring,
            );
            std::thread::sleep(MONITORING_TICK_INTERVAL);
        }
    });

    // Gate `phase2-psi-inherited-descriptor-ingress` (`P2-EVT-005`): the
    // live PSI producer. Until this call existed, `guardian-core`'s
    // complete and correct `providers::psi` capability was never
    // instantiated by any production call site, so no live PSI `Event` had
    // ever reached the shared correlation ingress. Its worker threads feed
    // the same `ingress_clock`/`engine` pair constructed above, through the
    // same `admit_event` point the two producers above use.
    // `psi_descriptor_plan_from_env` is called exactly once: as of the
    // final acceptance repair (Part B) it has the side effect of setting
    // real `FD_CLOEXEC` on every raw inherited descriptor via the
    // canonical `libsystemd` acquisition path (see `psi_ingress`'s module
    // doc), so it must not be called redundantly. `spawn_psi_ingress`'s
    // monitors and `psi_file_source` below (fed to `Capabilities1::
    // psi_summary` so it reads through the exact same inherited
    // descriptors `psi_ingress`'s own monitors read through, never a
    // second, independent `/proc/pressure` pathname probe — the root
    // cause of the live `ListCapabilities`/`PsiSummary` contradiction the
    // re-review reproduced) both derive from this one plan.
    let psi_plan = psi_ingress::psi_descriptor_plan_from_env();
    let psi_file_source = psi_plan.file_source();
    let (psi_monitors, psi_availability) = spawn_psi_ingress(&psi_plan, &ingress_clock, &engine);
    if psi_monitors == 0 {
        eprintln!(
            "[guardian-daemon] no PSI descriptors inherited; PSI is reported unavailable \
             (never 'no pressure') and every other producer is unaffected"
        );
    }

    // A second, genuine system-bus connection dedicated to the G8
    // Capability Registry's six read-only provider proxies — kept
    // entirely separate from the `Guardian1`-serving connection below, so
    // a provider read never shares a connection with (or can block) the
    // daemon's own served object.
    let registry_snapshot = std::sync::Arc::new(Mutex::new(Vec::<CapabilityRecord>::new()));
    let registry_snapshot_for_thread = std::sync::Arc::clone(&registry_snapshot);
    let ingress_clock_for_registry = std::sync::Arc::clone(&ingress_clock);
    let engine_for_registry = std::sync::Arc::clone(&engine);
    let psi_availability_for_registry = Arc::clone(&psi_availability);
    std::thread::spawn(move || {
        let mut topology_tracker = TopologyTracker::new();
        let mut health_producer = HealthTransitionProducer::new();
        loop {
            // Reconnect each bounded cycle. A missing bus or a connection
            // lost since the previous cycle therefore cannot permanently
            // kill this worker or retain a stale healthy snapshot.
            match async_io::block_on(zbus::Connection::system()) {
                Ok(registry_connection) => {
                    replace_registry_snapshot(
                        &registry_snapshot_for_thread,
                        capability_registry_tick(
                            &registry_connection,
                            &mut health_producer,
                            &ingress_clock_for_registry,
                            &engine_for_registry,
                            &psi_availability_for_registry,
                        ),
                    );
                    if let Ok(topology) =
                        async_io::block_on(UdisksProvider::new(&registry_connection).topology())
                    {
                        for event in topology_tracker.observe(&topology) {
                            eprintln!("[guardian-daemon] UDisks topology event: {event:?}");
                        }
                    }
                }
                Err(error) => {
                    replace_registry_snapshot(&registry_snapshot_for_thread, Vec::new());
                    eprintln!(
                        "[guardian-daemon] capability registry: system bus unavailable; retrying: {error}"
                    );
                }
            }
            std::thread::sleep(CAPABILITY_REGISTRY_TICK_INTERVAL);
        }
    });

    // G9's three read-only interfaces, each a separate interface major at
    // its own object path (per ADR-001's own worked example) — never
    // bolted onto the frozen `Guardian1` object above. `Capabilities1`
    // shares the same `registry_snapshot` the worker thread above already
    // maintains; it never opens a second registry connection of its own.
    let connection = zbus::blocking::connection::Builder::system()?
        .name(WELL_KNOWN_NAME)?
        .serve_at(OBJECT_PATH, GuardianContract::default())?
        .serve_at(
            dbus_surface::CAPABILITIES_OBJECT_PATH,
            dbus_surface::Capabilities1::new(
                std::sync::Arc::clone(&registry_snapshot),
                Arc::clone(&psi_availability),
                psi_file_source,
            ),
        )?
        .serve_at(
            dbus_surface::INCIDENTS_OBJECT_PATH,
            dbus_surface::Incidents1::new(std::sync::Arc::clone(&engine)),
        )?
        .serve_at(
            dbus_surface::TRANSACTIONS_OBJECT_PATH,
            dbus_surface::Transactions1,
        )?
        .build()?;
    eprintln!(
        "[guardian-daemon] serving {WELL_KNOWN_NAME} at {OBJECT_PATH}, unique_name={}",
        connection.unique_name().map_or("<none>", |n| n.as_str())
    );
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PSI_MONITOR_CONFIG, PsiStepOutcome, admit_event, advance_health_dwell_for_snapshot,
        dispatch_psi_wake, monitoring_tick, probe_free_space, psi_ingress_step,
        replace_registry_snapshot,
    };
    use guardian_core::budget::FreeSpaceState;
    use guardian_core::correlation::{
        AdmitOutcome, CorrelationEngine, CorrelationPolicy, HEALTH_AVAILABILITY_TO_ATTR,
        HEALTH_CAPABILITY_ID_ATTR, HEALTH_TRANSITION_EVENT_TYPE, IngressClock,
    };
    use guardian_core::event::Event;
    use guardian_core::providers::health::HealthTransitionProducer;
    use guardian_core::psi::{
        PressureSeverity, PsiLine, PsiReading, PsiResourceKind, SeverityThresholds, classify,
    };
    use guardian_core::recorder::BoundedRecorder;
    use guardian_core::risk::Risk;
    use guardian_daemon::psi_ingress::{
        PSI_FD_NAME_CPU, PSI_FD_NAME_IO, PSI_FD_NAME_MEMORY, PsiDescriptorPlan, PsiResourceMonitor,
        resolve_psi_descriptors, start_psi_monitors,
    };
    use guardian_provider_api::{
        Availability, BootAvailability, CapabilityId, CapabilityRecord, DiagnosticCost, EventId,
        Health, InterfaceKind, Knowledge, PrivilegeRequirement, ProviderId,
    };
    use std::fs::File;
    use std::os::fd::AsRawFd;
    use std::sync::Mutex;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "guardian-daemon-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn shared_ingress() -> (Mutex<IngressClock>, Mutex<CorrelationEngine>) {
        (
            Mutex::new(IngressClock::new()),
            Mutex::new(CorrelationEngine::new(CorrelationPolicy::default())),
        )
    }

    #[test]
    fn probe_free_space_reports_sufficient_for_a_real_writable_directory() {
        let dir = temp_dir("probe-sufficient");
        assert_eq!(probe_free_space(&dir), FreeSpaceState::Sufficient);
    }

    #[test]
    fn probe_free_space_does_not_leave_the_probe_file_behind() {
        let dir = temp_dir("probe-cleanup");
        probe_free_space(&dir);
        assert!(!dir.join(".free-space-probe").exists());
    }

    #[test]
    fn monitoring_tick_records_a_real_event_and_does_not_panic() {
        let dir = temp_dir("tick-records-event");
        let recorder = Mutex::new(BoundedRecorder::new(4).unwrap());
        let (ingress_clock, engine) = shared_ingress();
        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(recorder.lock().unwrap().len(), 1);
        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(recorder.lock().unwrap().len(), 2);
    }

    #[test]
    fn monitoring_tick_respects_the_recorder_bound_across_many_ticks() {
        let dir = temp_dir("tick-bounded");
        let recorder = Mutex::new(BoundedRecorder::new(2).unwrap());
        let (ingress_clock, engine) = shared_ingress();
        for _ in 0..5 {
            monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        }
        let guard = recorder.lock().unwrap();
        assert_eq!(guard.len(), 2, "must never exceed the configured capacity");
        assert_eq!(
            guard.dropped_count(),
            3,
            "the 3 oldest ticks must be counted as dropped"
        );
    }

    /// Gate TDD R2 evidence: every source feeding `admit_event` — here,
    /// several `monitoring_tick` calls plus a directly-admitted
    /// provider-health event — receives strictly increasing
    /// `ingress_sequence` values from the one shared admission point,
    /// regardless of which source produced the event.
    #[test]
    fn monitoring_tick_and_a_second_source_share_one_monotonically_increasing_ingress_sequence() {
        let dir = temp_dir("tick-shared-ingress");
        let recorder = Mutex::new(BoundedRecorder::new(8).unwrap());
        let (ingress_clock, engine) = shared_ingress();

        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(ingress_clock.lock().unwrap().sequence(), 1);

        let health_event = sample_health_event(0, "available");
        admit_event(&ingress_clock, &engine, health_event);
        assert_eq!(
            ingress_clock.lock().unwrap().sequence(),
            2,
            "a second source admitted through the same point must continue the same sequence"
        );

        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(
            ingress_clock.lock().unwrap().sequence(),
            3,
            "ingress_sequence keeps increasing regardless of which source produced the event"
        );
    }

    /// Gate TDD R6 / `P2-REC-003` daemon-side half: a `CapacityRejected`
    /// outcome consumed by `admit_event` does not panic and the engine's
    /// own rejection counter increments — the log line itself is asserted
    /// indirectly here (this test proves the outcome is real and handled,
    /// not silently dropped); the exact log text is documented at the
    /// `admit_event` call site.
    #[test]
    fn admit_event_handles_a_capacity_rejected_outcome_without_panicking() {
        let ingress_clock = Mutex::new(IngressClock::new());
        let policy = CorrelationPolicy {
            debounce_capacity: 1,
            ..CorrelationPolicy::default()
        };
        let engine = Mutex::new(CorrelationEngine::new(policy));

        admit_event(
            &ingress_clock,
            &engine,
            sample_health_event(0, "unavailable"),
        );
        admit_event(
            &ingress_clock,
            &engine,
            sample_health_event_for("b.cap", 1, "unavailable"),
        );

        assert_eq!(
            engine.lock().unwrap().rejection_count(),
            1,
            "the second, brand-new candidate key must be rejected once the ring is at capacity"
        );
    }

    fn sample_health_event(sequence: u64, availability_to: &str) -> Event {
        sample_health_event_for("a.cap", sequence, availability_to)
    }

    fn sample_health_event_for(capability_id: &str, sequence: u64, availability_to: &str) -> Event {
        let mut attributes = std::collections::BTreeMap::new();
        attributes.insert(
            HEALTH_CAPABILITY_ID_ATTR.to_owned(),
            capability_id.to_owned(),
        );
        attributes.insert(
            HEALTH_AVAILABILITY_TO_ATTR.to_owned(),
            availability_to.to_owned(),
        );
        Event {
            event_id: EventId::new(format!("guardian.health.test.event-{sequence}")).unwrap(),
            timestamp_monotonic: sequence,
            timestamp_wall: format!("sequence-{sequence}"),
            source_provider: ProviderId::new("guardian.p2.capability-health").unwrap(),
            event_type: HEALTH_TRANSITION_EVENT_TYPE.to_owned(),
            resource_refs: vec![capability_id.to_owned()],
            severity: Risk::Observe,
            normalized_key: guardian_core::event::normalize_key("test"),
            raw_reference: "test".to_owned(),
            attributes,
        }
    }

    #[test]
    fn registry_snapshot_is_replaced_not_retained_after_outage() {
        let snapshot = Mutex::new(vec![
            guardian_core::providers::registry::psi_capabilities(None).remove(0),
        ]);
        replace_registry_snapshot(&snapshot, Vec::new());
        assert!(snapshot.lock().unwrap().is_empty());
    }

    #[test]
    fn admit_outcome_ignored_for_unrecognized_event_type_still_advances_ingress() {
        // A "daemon_monitoring_tick" event matches no correlation rule
        // (`AdmitOutcome::Ignored`) but must still have been admitted
        // through the one shared point -- this is what proves the
        // daemon-tick producer is a real ingress source, not merely
        // recorder-only.
        let (ingress_clock, engine) = shared_ingress();
        let dir = temp_dir("ignored-still-advances");
        let recorder = Mutex::new(BoundedRecorder::new(4).unwrap());
        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        let engine_guard = engine.lock().unwrap();
        assert_eq!(engine_guard.admitted_ingress_count(), 1);
        assert_eq!(
            engine_guard.open_incidents().len(),
            0,
            "a daemon-tick event never opens an incident on its own"
        );
    }

    fn capability_record(
        cap_id: &str,
        availability: Availability,
        health: Health,
    ) -> CapabilityRecord {
        CapabilityRecord {
            capability_id: CapabilityId::new(cap_id).unwrap(),
            provider_id: ProviderId::new("guardian.g8.test").unwrap(),
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

    /// Gate TDD R2(a): a capability that goes bad and stays bad across
    /// enough fresh-snapshot ticks to satisfy `health_min_dwell` opens
    /// exactly one incident via `advance_health_dwell_for_snapshot` alone
    /// -- no second `Event` is ever constructed for the unchanged-bad
    /// snapshot.
    #[test]
    fn sustained_bad_across_fresh_snapshots_opens_exactly_one_incident_with_no_second_event() {
        let (ingress_clock, engine) = shared_ingress();
        let mut health_producer = HealthTransitionProducer::new();

        let baseline = vec![capability_record(
            "systemd.unit.state",
            Availability::Available,
            Health::Healthy,
        )];
        assert!(health_producer.observe(&baseline).is_empty());

        let bad = vec![capability_record(
            "systemd.unit.state",
            Availability::Unavailable,
            Health::Error,
        )];
        let events = health_producer.observe(&bad);
        assert_eq!(events.len(), 1, "the real diff must emit exactly one Event");
        for event in events {
            admit_event(&ingress_clock, &engine, event);
        }
        advance_health_dwell_for_snapshot(&bad, &ingress_clock, &engine);
        assert_eq!(
            engine.lock().unwrap().open_incidents().len(),
            0,
            "the first Bad reading only seeds the debounce ring"
        );

        // A later, unchanged-still-bad fresh snapshot: the producer stays
        // edge-triggered (zero new Events); the dwell-advance call is what
        // must do the promotion once real time has elapsed.
        std::thread::sleep(std::time::Duration::from_millis(2));
        let still_bad_events = health_producer.observe(&bad);
        assert!(
            still_bad_events.is_empty(),
            "an unchanged snapshot pair must emit no Event"
        );

        // Force the debounce candidate's dwell to have elapsed by driving
        // `advance_health_dwell` directly with an ingress instant beyond
        // `health_min_dwell` -- exactly what a later real tick would
        // naturally supply given enough wall-clock time between ticks.
        let capability_id = CapabilityId::new("systemd.unit.state").unwrap();
        let observation = guardian_core::correlation::FreshHealthObservation::classify(
            bad[0].availability,
            bad[0].health,
        );
        let dwell = CorrelationPolicy::default().health_min_dwell;
        let base = std::time::Instant::now();
        let outcome = engine.lock().unwrap().advance_health_dwell(
            &capability_id,
            observation,
            base + dwell,
            ingress_clock.lock().unwrap().sequence(),
        );
        assert!(
            matches!(outcome.outcome, AdmitOutcome::IncidentOpened(_)),
            "fresh-observation advancement alone must open exactly one incident once dwell elapses"
        );
        assert_eq!(engine.lock().unwrap().open_incidents().len(), 1);

        // A further call for the same, now-open capability must never
        // open a second incident (R2(b): unaffected once there is no
        // pending candidate left to advance).
        advance_health_dwell_for_snapshot(&bad, &ingress_clock, &engine);
        assert_eq!(
            engine.lock().unwrap().open_incidents().len(),
            1,
            "a capability with no pending candidate must be unaffected by being called every tick"
        );
    }

    /// Gate TDD R2(c): a capability that disappears from one tick's
    /// snapshot entirely must not have its pending candidate promoted on
    /// that tick -- `advance_health_dwell_for_snapshot` only calls
    /// `advance_health_dwell` for capabilities actually present in the
    /// snapshot it is given.
    #[test]
    fn a_capability_absent_from_a_snapshot_is_not_promoted_that_tick() {
        let (ingress_clock, engine) = shared_ingress();
        let mut health_producer = HealthTransitionProducer::new();

        let baseline = vec![capability_record(
            "systemd.unit.state",
            Availability::Available,
            Health::Healthy,
        )];
        assert!(health_producer.observe(&baseline).is_empty());

        let bad = vec![capability_record(
            "systemd.unit.state",
            Availability::Unavailable,
            Health::Error,
        )];
        let events = health_producer.observe(&bad);
        for event in events {
            admit_event(&ingress_clock, &engine, event);
        }
        assert_eq!(engine.lock().unwrap().open_incidents().len(), 0);

        // A fresh snapshot in which "systemd.unit.state" is entirely
        // absent -- e.g. its provider became unreachable this tick. Must
        // not promote the pending candidate.
        let snapshot_without_it: Vec<CapabilityRecord> = Vec::new();
        advance_health_dwell_for_snapshot(&snapshot_without_it, &ingress_clock, &engine);
        assert_eq!(
            engine.lock().unwrap().open_incidents().len(),
            0,
            "a capability absent from this tick's snapshot must not be promoted this tick"
        );
    }

    // -----------------------------------------------------------------
    // Gate `phase2-psi-inherited-descriptor-ingress`
    // (`P2-EVT-005`/`P2-EVT-006`/`P2-EVT-007`/`P2-EVT-008`)
    //
    // Every test below drives the *production* functions `main()` calls --
    // `start_psi_monitors`, `psi_ingress_step`, `dispatch_psi_wake` -- never
    // a test-only reimplementation. The complementary structural proof that
    // `main()` actually calls them lives in
    // `tests/phase2_psi_ingress_contract.rs`.
    // -----------------------------------------------------------------

    const PSI_NOMINAL: &str = "some avg10=0.10 avg60=0.05 avg300=0.01 total=1000\n";
    const PSI_ELEVATED: &str = "some avg10=31.00 avg60=12.00 avg300=4.00 total=2000\n";
    const PSI_CRITICAL: &str = "some avg10=81.00 avg60=44.00 avg300=17.00 total=3000\n";

    /// Builds the `LISTEN_PID`/`LISTEN_FDS`/`LISTEN_FDNAMES` triple systemd
    /// would set, naming a descriptor **this process really holds** at the
    /// index that maps to its real fd number (descriptors start at fd 3, in
    /// `LISTEN_FDNAMES` order).
    ///
    /// This is the "faithful local equivalent" the gate TDD's R6 permits:
    /// production `resolve_psi_descriptors` and everything downstream of it
    /// runs completely unmodified; only the descriptor's provenance differs
    /// (a test-opened file rather than PID 1's `OpenFile=`). Whether the
    /// *kernel* honours a trigger on a genuinely inherited PSI descriptor
    /// was proven empirically in the preflight and is re-proven on real
    /// hardware in Phase F.
    fn listen_fd_env(files: &[(&File, &str)]) -> (String, String, String) {
        let highest = files
            .iter()
            .map(|(file, _)| file.as_raw_fd())
            .max()
            .expect("at least one descriptor");
        assert!(highest >= 3, "inherited descriptors start at fd 3");
        let count = usize::try_from(highest - 2).unwrap();
        let mut names: Vec<String> = (0..count)
            .map(|index| format!("unrelated-{index}"))
            .collect();
        for (file, fd_name) in files {
            let index = usize::try_from(file.as_raw_fd() - 3).unwrap();
            names[index] = (*fd_name).to_owned();
        }
        (
            std::process::id().to_string(),
            count.to_string(),
            names.join(":"),
        )
    }

    fn plan_for(files: &[(&File, &str)]) -> PsiDescriptorPlan {
        let (pid, fds, names) = listen_fd_env(files);
        let plan =
            resolve_psi_descriptors(Some(&pid), Some(&fds), Some(&names), std::process::id());
        // Fail loudly rather than silently degrading into an empty plan
        // (which would make every downstream assertion in the caller read
        // as "the production path did nothing" for an unrelated reason --
        // e.g. the test binary's fd table having grown past the resolver's
        // fail-closed LISTEN_FDS bound while tests run in parallel).
        assert_eq!(
            plan.len(),
            files.len(),
            "the synthesized listen-fd environment must resolve every named descriptor \
             (LISTEN_FDS={fds}, LISTEN_FDNAMES={names})"
        );
        plan
    }

    /// Opens a fixture as a read/write descriptor, exactly as `OpenFile=`'s
    /// default `rw` mode does.
    fn psi_fixture(dir: &std::path::Path, name: &str, text: &str) -> (std::path::PathBuf, File) {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        let file = File::options()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open PSI fixture read/write");
        (path, file)
    }

    /// The kernel's own PSI decayed-average update
    /// (`Documentation/accounting/psi.rst`; `kernel/sched/psi.c`'s
    /// per-class EWMA), used below to build fixture trajectories a real
    /// `/proc/pressure` file could actually produce over time.
    /// `PSI_NOMINAL`/`PSI_ELEVATED`/`PSI_CRITICAL` above are deliberately
    /// instantaneous jumps -- fine for the shape/wiring tests that use
    /// them, but exactly the kind of fixture that let the Finding-1 defect
    /// (production PSI incidents unreachable under any real trajectory)
    /// survive undetected. `target_pct` is the stalled percentage
    /// sustained for `elapsed_secs` (100.0 under continuous full stall,
    /// 0.0 once stall stops); `tau_secs` is the averaging class's own time
    /// constant (10 / 60 / 300 for `avg10`/`avg60`/`avg300`).
    fn kernel_psi_ewma_step(
        previous: f64,
        target_pct: f64,
        elapsed_secs: f64,
        tau_secs: f64,
    ) -> f64 {
        let factor = 1.0 - (-elapsed_secs / tau_secs).exp();
        previous + (target_pct - previous) * factor
    }

    /// Ties fixture generation to the real production sampling cadence at
    /// compile time: the fastest a real kernel PSI trigger can wake this
    /// daemon for one resource is once per
    /// `PSI_MONITOR_CONFIG.trigger_window_us`. If that production constant
    /// ever changes, this assertion fails the build rather than silently
    /// leaving `TRAJECTORY_STEP_SECS` -- and therefore every derived
    /// fixture below -- physically stale.
    const _: () = assert!(PSI_MONITOR_CONFIG.trigger_window_us == 2_000_000);
    const TRAJECTORY_STEP_SECS: f64 = 2.0;

    /// All three decayed averages advanced together by one real production
    /// sampling interval. Only `avg10` is ever classified
    /// (`providers/psi.rs`'s `present_severity` calls
    /// `classify(resource.some, ..)`, and `classify` reads only `avg10`),
    /// but a fixture claiming to be physically realizable must be
    /// realizable on every field it writes, not merely the one field
    /// production happens to read.
    #[derive(Clone, Copy, Debug)]
    struct PsiAverages {
        avg10: f64,
        avg60: f64,
        avg300: f64,
    }

    impl PsiAverages {
        const fn steady(value: f64) -> Self {
            Self {
                avg10: value,
                avg60: value,
                avg300: value,
            }
        }

        /// Advances every average by one `TRAJECTORY_STEP_SECS` real
        /// sampling interval toward `target_pct`.
        fn step(self, target_pct: f64) -> Self {
            Self {
                avg10: kernel_psi_ewma_step(self.avg10, target_pct, TRAJECTORY_STEP_SECS, 10.0),
                avg60: kernel_psi_ewma_step(self.avg60, target_pct, TRAJECTORY_STEP_SECS, 60.0),
                avg300: kernel_psi_ewma_step(self.avg300, target_pct, TRAJECTORY_STEP_SECS, 300.0),
            }
        }

        fn line(self, total: u64) -> String {
            format!(
                "some avg10={:.2} avg60={:.2} avg300={:.2} total={total}\n",
                self.avg10, self.avg60, self.avg300
            )
        }

        fn severity(self, thresholds: SeverityThresholds) -> PressureSeverity {
            classify(
                PsiLine {
                    avg10: self.avg10,
                    avg60: self.avg60,
                    avg300: self.avg300,
                    total: 0,
                },
                thresholds,
            )
        }
    }

    /// `P2-EVT-005` R12 + `P2-EVT-006` R3/R6: the production startup
    /// function registers exactly one monitor per inherited descriptor,
    /// with exactly one registration attempt per resource (no retry loop --
    /// there is no in-place re-arm for a PSI trigger).
    #[test]
    fn psi_startup_registers_exactly_one_monitor_per_inherited_descriptor() {
        let dir = temp_dir("psi-startup-one-per-descriptor");
        let (_, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let (_, io) = psi_fixture(&dir, "io", PSI_NOMINAL);
        let plan = plan_for(&[(&cpu, PSI_FD_NAME_CPU), (&io, PSI_FD_NAME_IO)]);
        assert_eq!(plan.len(), 2, "one descriptor per monitored resource");

        let report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
        assert_eq!(
            report.attempts, 2,
            "exactly one registration attempt per planned resource -- never retried in place"
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert_eq!(report.monitors.len(), 2);
        let kinds: Vec<PsiResourceKind> = report
            .monitors
            .iter()
            .map(PsiResourceMonitor::kind)
            .collect();
        assert_eq!(kinds, [PsiResourceKind::Cpu, PsiResourceKind::Io]);
    }

    /// `P2-EVT-006` R7 -- the non-obvious requirement a trigger-only design
    /// would silently fail. `PsiEventDispatcher::dispatch_wake` re-reads the
    /// pressure text *by pathname*; under `ProcSubset=pid` a
    /// `/proc/pressure/...` pathname returns `Unavailable`, so the re-read
    /// must go through the same inherited descriptor the trigger was
    /// registered on. Here the trigger is registered first (which really
    /// writes the kernel ABI payload), the pressure text is then restored as
    /// a live kernel file would carry it, and the wake still classifies.
    #[test]
    fn one_inherited_descriptor_serves_both_the_trigger_and_the_content_reread() {
        let dir = temp_dir("psi-one-fd-both-roles");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let plan = plan_for(&[(&cpu, PSI_FD_NAME_CPU)]);
        let descriptor_path = plan.path(PsiResourceKind::Cpu).unwrap().to_path_buf();
        assert!(
            descriptor_path.starts_with("/proc/self/fd/"),
            "the resolved path must be the in-process descriptor path, got {}",
            descriptor_path.display()
        );

        // Role 1: content re-read through the descriptor, *before* the
        // trigger exists.
        let source = plan.file_source();
        assert!(matches!(
            source.read(PsiResourceKind::Cpu).unwrap(),
            PsiReading::Present(_)
        ));

        // Role 2: real trigger registration on that same descriptor.
        let mut report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let mut monitor = report.monitors.remove(0);

        // Role 1 again, *after* registration -- the case that fails
        // catastrophically and silently if the re-read used a pathname.
        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(matches!(
            source.read(PsiResourceKind::Cpu).unwrap(),
            PsiReading::Present(_)
        ));
        let event = monitor
            .dispatch_wake()
            .expect("the re-read must succeed through the inherited descriptor")
            .expect("a Nominal -> Critical crossing must produce an event");
        assert_eq!(event.severity, Risk::High);
    }

    /// Acceptance-repair Blocker 1/2, on the production `PsiResourceMonitor`
    /// type itself (not just the library primitives it composes): the
    /// monitor's trigger and dispatcher share the *exact* open file
    /// description of the single `OwnedPsiFile` `PsiResourceMonitor::
    /// register` opened, and that descriptor is close-on-exec. Before this
    /// repair, `PsiResourceMonitor::register` built a `PsiFileSource` from
    /// the descriptor path and let the dispatcher/trigger each open it
    /// independently (the dispatcher's own baseline read, the trigger's
    /// registration, and every subsequent `dispatch_wake` each performed
    /// their own fresh `open()` of `/proc/self/fd/N`) -- this test would
    /// have failed against that implementation, because independent
    /// reopens of the same magic-symlink path are independent open file
    /// descriptions on Linux.
    #[test]
    fn the_production_monitor_registers_trigger_and_dispatch_through_one_owned_descriptor() {
        let dir = temp_dir("psi-one-owned-descriptor");
        let (_, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let plan = plan_for(&[(&cpu, PSI_FD_NAME_CPU)]);
        let mut report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let monitor = report.monitors.remove(0);
        assert!(
            monitor.shares_one_open_file_description(),
            "the production PsiResourceMonitor must register its trigger and read every \
             dispatch through the exact same owned open file description"
        );
        assert!(
            monitor.is_close_on_exec(),
            "the production monitor's owned descriptor must be close-on-exec"
        );
    }

    /// `P2-EVT-007` R11/R10: a real crossing produces the accepted Guardian
    /// `Event` shape, with the daemon owning every authority field.
    ///
    /// Crosses straight to `PSI_CRITICAL` (not `PSI_ELEVATED`, as an
    /// earlier version of this test did) because the 2026-09-07
    /// `event_threshold` repair (`PSI_MONITOR_CONFIG`, Finding 1 in
    /// `docs/evidence/p2/
    /// PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md`) means a bare
    /// `Nominal -> Elevated` reading is no longer a reportable crossing at
    /// all -- `ThresholdMonitor` now only reports crossings of the
    /// Critical boundary, so this is the shape a real production crossing
    /// can actually take today. `gradual_ewma_trajectory_crosses_into_
    /// critical_only_at_the_real_boundary_and_opens_incident` above covers
    /// the gradual/multi-step shape of the same boundary.
    #[test]
    fn a_real_crossing_produces_the_accepted_psi_event_shape() {
        let dir = temp_dir("psi-event-shape");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);

        std::fs::write(&path, PSI_CRITICAL).unwrap();
        let event = monitor.dispatch_wake().unwrap().unwrap();

        assert_eq!(event.event_type, "psi_threshold_crossing");
        assert_eq!(event.resource_refs, ["/proc/pressure/cpu"]);
        assert_eq!(event.severity, Risk::High);
        assert_eq!(
            event.normalized_key,
            "psi cpu threshold crossing nominal->critical"
        );
        assert_eq!(event.attributes["from"], "Nominal");
        assert_eq!(event.attributes["to"], "Critical");
        assert_eq!(
            event.source_provider.as_str(),
            "guardian.g8.psi",
            "the accepted G8 provider identity, unchanged by this gate"
        );
    }

    /// `P2-EVT-007` R10: correlation identity is the stable kernel resource
    /// path, never the volatile `/proc/self/fd/N` descriptor path the daemon
    /// happens to have read through. Asserted for every resource kind.
    #[test]
    fn resource_identity_is_the_kernel_path_not_the_descriptor_path() {
        let dir = temp_dir("psi-resource-identity");
        for (fixture, fd_name, kind, expected) in [
            (
                "cpu",
                PSI_FD_NAME_CPU,
                PsiResourceKind::Cpu,
                "/proc/pressure/cpu",
            ),
            (
                "memory",
                PSI_FD_NAME_MEMORY,
                PsiResourceKind::Memory,
                "/proc/pressure/memory",
            ),
            (
                "io",
                PSI_FD_NAME_IO,
                PsiResourceKind::Io,
                "/proc/pressure/io",
            ),
        ] {
            let (path, file) = psi_fixture(&dir, fixture, PSI_NOMINAL);
            let plan = plan_for(&[(&file, fd_name)]);
            assert!(
                plan.path(kind).unwrap().starts_with("/proc/self/fd/"),
                "the read path is the descriptor path"
            );
            let mut report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
            let mut monitor = report.monitors.remove(0);
            std::fs::write(&path, PSI_CRITICAL).unwrap();
            let event = monitor.dispatch_wake().unwrap().unwrap();
            assert_eq!(
                event.resource_refs,
                [expected],
                "but the correlation identity is the stable kernel path"
            );
        }
    }

    /// `P2-EVT-005` R13: PSI events reach correlation through the **same**
    /// `admit_event` call site `monitoring_tick` and the provider-health
    /// producer use, sharing one strictly increasing ingress sequence. Same
    /// technique as
    /// `monitoring_tick_and_a_second_source_share_one_monotonically_increasing_ingress_sequence`.
    #[test]
    fn psi_events_share_the_one_ingress_sequence_with_the_other_producers() {
        let dir = temp_dir("psi-shared-ingress");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);
        let (ingress_clock, engine) = shared_ingress();
        let recorder = Mutex::new(BoundedRecorder::new(8).unwrap());

        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(ingress_clock.lock().unwrap().sequence(), 1);

        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(matches!(
            dispatch_psi_wake(&mut monitor, &ingress_clock, &engine),
            PsiStepOutcome::Admitted(_)
        ));
        assert_eq!(
            ingress_clock.lock().unwrap().sequence(),
            2,
            "the PSI producer must continue the one shared ingress sequence"
        );

        admit_event(&ingress_clock, &engine, sample_health_event(0, "available"));
        assert_eq!(ingress_clock.lock().unwrap().sequence(), 3);
        assert_eq!(engine.lock().unwrap().admitted_ingress_count(), 3);
    }

    /// `P2-EVT-007` R14 / `P2-COR-001` + `P2-COR-002`, regressed against
    /// Gate 2a's **unmodified** engine: a PSI `Critical` crossing opens an
    /// incident; a later `Critical` crossing for the same
    /// `resource_refs.first()` within the window links to it instead of
    /// opening a second one.
    #[test]
    fn a_psi_critical_crossing_opens_an_incident_and_the_next_one_links_to_it() {
        let dir = temp_dir("psi-correlation");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);
        let (ingress_clock, engine) = shared_ingress();

        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(matches!(
            dispatch_psi_wake(&mut monitor, &ingress_clock, &engine),
            PsiStepOutcome::Admitted(_)
        ));
        let opened = engine.lock().unwrap().open_incidents();
        assert_eq!(opened.len(), 1, "a Critical PSI crossing opens an incident");
        let incident_id = opened[0].incident_id.clone();
        assert_eq!(
            opened[0].primary_resource.as_deref(),
            Some("/proc/pressure/cpu")
        );

        // Recovery below the event threshold: a real crossing, but
        // `Risk::Observe` -- deliberately ignored by Gate 2a's rule.
        std::fs::write(&path, PSI_NOMINAL).unwrap();
        assert!(matches!(
            dispatch_psi_wake(&mut monitor, &ingress_clock, &engine),
            PsiStepOutcome::Admitted(_)
        ));
        assert_eq!(engine.lock().unwrap().open_incidents().len(), 1);

        // A second Critical crossing for the same resource, inside the
        // window.
        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(matches!(
            dispatch_psi_wake(&mut monitor, &ingress_clock, &engine),
            PsiStepOutcome::Admitted(_)
        ));
        let after = engine.lock().unwrap().open_incidents();
        assert_eq!(
            after.len(),
            1,
            "a same-resource Critical crossing inside the window must link, not open a second incident"
        );
        assert_eq!(after[0].incident_id, incident_id);
        assert!(
            after[0].event_ids.len() >= 2,
            "the second crossing must be linked to the existing incident, got {:?}",
            after[0].event_ids
        );
    }

    /// `P2-EVT-007`/`P2-COR-001` regression, encoding Finding 1's root
    /// cause directly (`docs/evidence/p2/
    /// PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md` §F.15): a
    /// **physically realizable** gradual trajectory (`Nominal -> Elevated
    /// -> higher Elevated -> Critical`, each step produced by
    /// `kernel_psi_ewma_step` at the real production sampling interval,
    /// never an instantaneous jump like `PSI_NOMINAL`->`PSI_CRITICAL`
    /// above) must still classify the `Elevated -> Critical` step as
    /// `Risk::High` and open exactly one incident, and the Elevated-entry
    /// step must never itself fabricate a Critical event or incident.
    ///
    /// Before the repair (`event_threshold: PressureSeverity::Elevated`)
    /// this is RED for the documented reason:
    /// `ThresholdMonitor::observe` never reports an `Elevated -> Critical`
    /// transition, because both readings are already above the Elevated
    /// threshold (`was_above == is_above`) -- so this exact, physically
    /// realizable trajectory produces zero `Risk::High` events and opens
    /// zero incidents, reproducing the real VM's 70-second/`avg10=98.97`
    /// zero-incident control run in a deterministic unit test.
    #[test]
    fn gradual_ewma_trajectory_crosses_into_critical_only_at_the_real_boundary_and_opens_incident()
    {
        let dir = temp_dir("psi-gradual-ewma-critical");

        let nominal = PsiAverages::steady(15.0);
        let elevated_entry = nominal.step(100.0);
        let elevated_higher = elevated_entry.step(100.0);
        let critical = elevated_higher.step(100.0);

        let thresholds = PSI_MONITOR_CONFIG.thresholds;
        assert_eq!(
            nominal.severity(thresholds),
            PressureSeverity::Nominal,
            "test setup: trajectory must start Nominal"
        );
        assert_eq!(
            elevated_entry.severity(thresholds),
            PressureSeverity::Elevated,
            "test setup: one real EWMA step from Nominal under sustained 100% stall must land \
             Elevated, never jump straight to Critical (Finding 1's arithmetic ceiling)"
        );
        assert_eq!(
            elevated_higher.severity(thresholds),
            PressureSeverity::Elevated,
            "test setup: the second step must still classify Elevated -- this is the exact \
             Elevated -> Elevated -> Critical shape ThresholdMonitor::observe cannot see across \
             under the old config"
        );
        assert_eq!(
            critical.severity(thresholds),
            PressureSeverity::Critical,
            "test setup: the third step must actually reach Critical"
        );

        let (path, cpu) = psi_fixture(&dir, "cpu", &nominal.line(1000));
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);
        let (ingress_clock, engine) = shared_ingress();

        // Nominal -> Elevated: whether this is itself reported depends on
        // `event_threshold`, but either way it must never be reported as
        // Critical/`Risk::High`, and it must never open an incident.
        std::fs::write(&path, elevated_entry.line(1001)).unwrap();
        if let Some(event) = monitor.dispatch_wake().unwrap() {
            assert_ne!(
                event.severity,
                Risk::High,
                "entering Elevated must never itself fabricate a Critical event"
            );
            admit_event(&ingress_clock, &engine, event);
        }
        assert!(
            engine.lock().unwrap().open_incidents().is_empty(),
            "no incident may exist before Critical is actually reached"
        );

        // Elevated -> higher Elevated: same severity class both times, so
        // `ThresholdMonitor` must report no crossing at all.
        std::fs::write(&path, elevated_higher.line(1002)).unwrap();
        assert_eq!(
            monitor.dispatch_wake().unwrap(),
            None,
            "no threshold crossing between two readings that both classify Elevated"
        );
        assert!(engine.lock().unwrap().open_incidents().is_empty());

        // Elevated -> Critical: the transition Finding 1 proved the
        // production config could never observe under a real trajectory.
        std::fs::write(&path, critical.line(1003)).unwrap();
        let event = monitor
            .dispatch_wake()
            .unwrap()
            .expect("a gradual Elevated -> Critical crossing must still produce an event");
        assert_eq!(
            event.severity,
            Risk::High,
            "the Elevated -> Critical crossing must classify as Critical/Risk::High"
        );
        assert_eq!(event.resource_refs, ["/proc/pressure/cpu"]);
        admit_event(&ingress_clock, &engine, event);
        let opened = engine.lock().unwrap().open_incidents();
        assert_eq!(
            opened.len(),
            1,
            "a gradual, physically realizable Elevated -> Critical crossing must open an \
             incident (P2-COR-001), reproducing the real VM's control run failing this exact \
             way before the repair"
        );
        assert_eq!(
            opened[0].primary_resource.as_deref(),
            Some("/proc/pressure/cpu")
        );
    }

    /// `P2-EVT-007` reverse-boundary regression: the recovery direction
    /// (`Critical -> Elevated`), decayed by the same real EWMA formula
    /// (`target_pct = 0.0`, stall relieved) rather than an instantaneous
    /// drop. The repaired `event_threshold: PressureSeverity::Critical`
    /// watches exactly this boundary, so recovery must remain observable
    /// as a real event -- and admitting it must not disturb the
    /// already-open incident: Gate 2a's engine (`correlation.rs`,
    /// forbidden scope) closes PSI incidents by its debounce window
    /// elapsing, never by an explicit recovery reading (any non-`Risk::
    /// High` PSI event is `Ignored` by `classify()`), so this also
    /// regresses that accepted dwell/close behaviour against a physically
    /// realizable recovery reading instead of an untested one.
    #[test]
    fn gradual_ewma_recovery_crossing_critical_to_elevated_remains_observable_without_disturbing_the_open_incident()
     {
        let dir = temp_dir("psi-gradual-ewma-recovery");

        let nominal = PsiAverages::steady(15.0);
        let critical = nominal.step(100.0).step(100.0).step(100.0);
        let recovered = critical.step(0.0);
        let thresholds = PSI_MONITOR_CONFIG.thresholds;
        assert_eq!(
            critical.severity(thresholds),
            PressureSeverity::Critical,
            "test setup: must actually reach Critical before testing recovery from it"
        );
        assert_eq!(
            recovered.severity(thresholds),
            PressureSeverity::Elevated,
            "test setup: one real decay step down from Critical must land Elevated, not \
             Nominal -- this is the boundary the repaired config watches"
        );

        let (path, cpu) = psi_fixture(&dir, "cpu", &nominal.line(2000));
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);
        let (ingress_clock, engine) = shared_ingress();

        std::fs::write(&path, critical.line(2001)).unwrap();
        let opening_event = monitor
            .dispatch_wake()
            .unwrap()
            .expect("Nominal -> Critical must open the incident");
        assert_eq!(opening_event.severity, Risk::High);
        admit_event(&ingress_clock, &engine, opening_event);
        let opened = engine.lock().unwrap().open_incidents();
        assert_eq!(opened.len(), 1);
        let incident_id = opened[0].incident_id.clone();

        std::fs::write(&path, recovered.line(2002)).unwrap();
        let recovery_event = monitor
            .dispatch_wake()
            .unwrap()
            .expect("the Critical -> Elevated recovery boundary must remain observable");
        assert_eq!(recovery_event.attributes["from"], "Critical");
        assert_eq!(recovery_event.attributes["to"], "Elevated");
        assert_ne!(
            recovery_event.severity,
            Risk::High,
            "recovery must never itself report Critical"
        );
        admit_event(&ingress_clock, &engine, recovery_event);

        let after = engine.lock().unwrap().open_incidents();
        assert_eq!(
            after.len(),
            1,
            "the recovery reading must not open a second incident or close the existing one"
        );
        assert_eq!(
            after[0].incident_id, incident_id,
            "the same incident must remain open, undisturbed by the recovery reading (Gate 2a \
             closes by window elapsing, not by a recovery signal)"
        );
    }

    /// `P2-EVT-007` R9: the severity that reaches `CorrelationEngine::
    /// classify()` is a pure function of the raw PSI bytes the daemon read
    /// through its own descriptor. Nothing else can influence it: the only
    /// input changed between these two runs is the file's contents.
    ///
    /// Uses one Nominal-baseline run crossing up into Critical and one
    /// Critical-baseline run crossing down into Elevated (rather than two
    /// Nominal-baseline runs, one of which used to stop at `PSI_ELEVATED`)
    /// because the 2026-09-07 `event_threshold` repair means a bare
    /// `Nominal -> Elevated` reading no longer reaches `dispatch_wake` as
    /// a reportable crossing at all -- see the comment on
    /// `a_real_crossing_produces_the_accepted_psi_event_shape` above. The
    /// two runs still differ only in the file contents each independently
    /// reads, which is exactly the property this test proves.
    #[test]
    fn psi_severity_is_derived_only_from_the_bytes_the_daemon_read_itself() {
        let dir = temp_dir("psi-severity-provenance");
        let mut severities = Vec::new();
        for (fixture, baseline, text) in [
            ("a", PSI_NOMINAL, PSI_CRITICAL),
            ("b", PSI_CRITICAL, PSI_ELEVATED),
        ] {
            let (path, file) = psi_fixture(&dir, fixture, baseline);
            let mut report =
                start_psi_monitors(&plan_for(&[(&file, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
            let mut monitor = report.monitors.remove(0);
            std::fs::write(&path, text).unwrap();
            severities.push(monitor.dispatch_wake().unwrap().unwrap().severity);
        }
        assert_eq!(
            severities,
            [Risk::High, Risk::Moderate],
            "severity must track the raw bytes alone -- there is no other input to track"
        );
    }

    /// `P2-EVT-006` R15: a restart re-acquires descriptors and re-registers
    /// from scratch. Each start produces an independent monitor with a fresh
    /// open file description and no carried-over threshold state -- the
    /// second start's first crossing is detected exactly as the first
    /// start's was, rather than being suppressed as "already Critical".
    #[test]
    fn restarting_the_psi_producer_re_registers_with_no_carried_over_state() {
        let dir = temp_dir("psi-restart");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);

        let mut first =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        assert!(first.failures.is_empty(), "{:?}", first.failures);
        let mut first_monitor = first.monitors.remove(0);
        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(first_monitor.dispatch_wake().unwrap().is_some());
        drop(first_monitor);

        // Restart: the old description is gone, so re-registration must
        // succeed (a stale reused description would be EBUSY on a real
        // kernel -- Phase F proves that half on real hardware).
        std::fs::write(&path, PSI_NOMINAL).unwrap();
        let mut second =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        assert!(
            second.failures.is_empty(),
            "re-registration after restart must succeed: {:?}",
            second.failures
        );
        let mut second_monitor = second.monitors.remove(0);
        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert!(
            second_monitor.dispatch_wake().unwrap().is_some(),
            "the restarted producer must observe its own fresh baseline, not inherit one"
        );
    }

    /// `P2-EVT-008` R16: with zero descriptors the daemon starts and runs
    /// normally -- no monitors, no failures, and every other producer
    /// unaffected.
    #[test]
    fn the_daemon_runs_normally_with_zero_psi_descriptors() {
        let empty = resolve_psi_descriptors(None, None, None, std::process::id());
        assert!(empty.is_empty());

        let report = start_psi_monitors(&empty, PSI_MONITOR_CONFIG);
        assert_eq!(report.attempts, 0);
        assert!(report.monitors.is_empty());
        assert!(report.failures.is_empty());

        let dir = temp_dir("psi-zero-descriptors");
        let recorder = Mutex::new(BoundedRecorder::new(4).unwrap());
        let (ingress_clock, engine) = shared_ingress();
        monitoring_tick(&recorder, &dir, &ingress_clock, &engine);
        assert_eq!(engine.lock().unwrap().admitted_ingress_count(), 1);
    }

    /// `P2-EVT-008` R16 / `P2-EVT-006` R2, the `:graceful` case: when only
    /// some `OpenFile=` paths existed, the absent resource must read as a
    /// truthful `Unavailable` -- never "no pressure", and never bound to a
    /// neighbouring resource's descriptor.
    #[test]
    fn a_partial_graceful_descriptor_set_leaves_the_absent_resource_unavailable() {
        let dir = temp_dir("psi-partial-graceful");
        let (_, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let (_, io) = psi_fixture(&dir, "io", PSI_CRITICAL);
        let plan = plan_for(&[(&cpu, PSI_FD_NAME_CPU), (&io, PSI_FD_NAME_IO)]);

        assert!(plan.path(PsiResourceKind::Memory).is_none());
        let source = plan.file_source();
        assert_eq!(
            source.read(PsiResourceKind::Memory).unwrap(),
            PsiReading::Unavailable,
            "an absent descriptor is truthfully Unavailable, never 'no pressure'"
        );
        assert!(matches!(
            source.read(PsiResourceKind::Cpu).unwrap(),
            PsiReading::Present(_)
        ));

        let report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
        assert_eq!(report.monitors.len(), 2);
        assert!(report.failures.is_empty());
    }

    /// `P2-EVT-008` R17: malformed, non-finite and out-of-range raw PSI
    /// values are rejected at the read/classification boundary, so none of
    /// them can ever become an admitted `Event`.
    #[test]
    fn invalid_raw_psi_values_never_produce_an_admitted_event() {
        let dir = temp_dir("psi-invalid-values");
        for (name, text) in [
            ("malformed", "not a psi line at all\n"),
            ("nan", "some avg10=NaN avg60=0 avg300=0 total=9\n"),
            ("infinite", "some avg10=inf avg60=0 avg300=0 total=9\n"),
            ("negative", "some avg10=-3.0 avg60=0 avg300=0 total=9\n"),
            ("above-range", "some avg10=140.0 avg60=0 avg300=0 total=9\n"),
        ] {
            let (path, file) = psi_fixture(&dir, name, PSI_NOMINAL);
            let mut report =
                start_psi_monitors(&plan_for(&[(&file, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
            let mut monitor = report.monitors.remove(0);
            let (ingress_clock, engine) = shared_ingress();

            std::fs::write(&path, text).unwrap();
            assert_eq!(
                dispatch_psi_wake(&mut monitor, &ingress_clock, &engine),
                PsiStepOutcome::Failed,
                "{name}: an invalid raw value must surface as a typed failure"
            );
            assert_eq!(
                engine.lock().unwrap().admitted_ingress_count(),
                0,
                "{name}: no Event may be admitted from an invalid raw PSI value"
            );
            assert_eq!(ingress_clock.lock().unwrap().sequence(), 0);
        }
    }

    /// `P2-EVT-008` R18 / `P2-EVT-006` R8: a PSI resource whose descriptor
    /// cannot be registered is reported as a hard, observable failure and is
    /// simply not monitored -- it never becomes a silently-unpollable source,
    /// it never stops the other PSI resources, and it never touches the
    /// provider-health path.
    #[test]
    fn a_psi_registration_failure_degrades_psi_observability_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("psi-registration-failure");
        let (_, good) = psi_fixture(&dir, "cpu", PSI_NOMINAL);

        // A descriptor that is readable but NOT trigger-capable: the kernel
        // PSI ABI requires O_RDWR (the trigger is written to the same fd
        // that is later polled), so a read-only descriptor reads fine and
        // then fails registration for real. This is a genuine kernel-level
        // failure, and unlike closing a descriptor it cannot be perturbed by
        // fd-number reuse from a concurrently running test.
        let bad_path = dir.join("io");
        std::fs::write(&bad_path, PSI_NOMINAL).unwrap();
        std::fs::set_permissions(&bad_path, std::fs::Permissions::from_mode(0o444)).unwrap();
        let bad = File::options().read(true).open(&bad_path).unwrap();

        let plan = plan_for(&[(&good, PSI_FD_NAME_CPU), (&bad, PSI_FD_NAME_IO)]);
        let report = start_psi_monitors(&plan, PSI_MONITOR_CONFIG);
        assert_eq!(
            report.attempts, 2,
            "one attempt per resource, never retried"
        );
        assert_eq!(
            report.monitors.len(),
            1,
            "the healthy resource still starts"
        );
        assert_eq!(
            report.failures.len(),
            1,
            "the failure is observable, not swallowed"
        );
        assert_eq!(report.failures[0].resource(), PsiResourceKind::Io);

        // Provider-health production is entirely unaffected.
        let (ingress_clock, engine) = shared_ingress();
        let mut health_producer = HealthTransitionProducer::new();
        let baseline = vec![capability_record(
            "systemd.unit.state",
            Availability::Available,
            Health::Healthy,
        )];
        assert!(health_producer.observe(&baseline).is_empty());
        let bad_snapshot = vec![capability_record(
            "systemd.unit.state",
            Availability::Unavailable,
            Health::Error,
        )];
        let events = health_producer.observe(&bad_snapshot);
        assert_eq!(events.len(), 1);
        for event in events {
            admit_event(&ingress_clock, &engine, event);
        }
        assert_eq!(engine.lock().unwrap().admitted_ingress_count(), 1);
    }

    /// `P2-EVT-006` R8, the classification half: `EBUSY` at trigger
    /// registration means a trigger already exists on that open file
    /// description. Triggers are per-description, so there is no in-place
    /// re-arm -- it must be reported as a hard error for that resource, never
    /// as a benign retry condition. (The real kernel `EBUSY` itself was
    /// proven empirically in the preflight and is re-proven in Phase F; what
    /// is under test here is that the daemon classifies it as hard.)
    #[test]
    fn ebusy_at_registration_is_classified_as_a_hard_error() {
        use guardian_daemon::psi_ingress::PsiRegistrationError;
        let busy = PsiRegistrationError::from_io(
            PsiResourceKind::Cpu,
            std::io::Error::from_raw_os_error(16),
        );
        assert!(busy.is_busy());
        assert!(
            busy.is_hard(),
            "EBUSY must never be treated as retryable in place"
        );

        let other = PsiRegistrationError::from_io(
            PsiResourceKind::Io,
            std::io::Error::from(std::io::ErrorKind::NotFound),
        );
        assert!(!other.is_busy());
        assert!(
            other.is_hard(),
            "every registration failure is hard for its resource -- PSI never retries in place"
        );
    }

    /// `P2-EVT-005` R12/R13: the production worker-loop *step* -- the exact
    /// function `psi_ingress_loop` calls each iteration -- is exercised end
    /// to end, including its kernel wait. A regular-file descriptor never
    /// reports `POLLPRI`, so the step reports `Idle` rather than
    /// manufacturing an event: the daemon only ever emits on a real kernel
    /// wake.
    #[test]
    fn the_production_worker_step_never_manufactures_an_event_without_a_wake() {
        let dir = temp_dir("psi-worker-step");
        let (path, cpu) = psi_fixture(&dir, "cpu", PSI_NOMINAL);
        let mut report =
            start_psi_monitors(&plan_for(&[(&cpu, PSI_FD_NAME_CPU)]), PSI_MONITOR_CONFIG);
        let mut monitor = report.monitors.remove(0);
        let (ingress_clock, engine) = shared_ingress();

        std::fs::write(&path, PSI_CRITICAL).unwrap();
        assert_eq!(
            psi_ingress_step(
                &mut monitor,
                Some(std::time::Duration::from_millis(1)),
                &ingress_clock,
                &engine,
            ),
            PsiStepOutcome::Idle,
            "no POLLPRI wake means no event, even though the pressure text crossed"
        );
        assert_eq!(engine.lock().unwrap().admitted_ingress_count(), 0);
    }
}
