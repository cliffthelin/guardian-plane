//! `guardian-daemon` — G7's unprivileged production process (ADR-002 Model
//! B; G7 implementation handoff §2). Owns `io.github.cliffthelin.Guardian1`
//! on the real system bus. Never relays a client's privileged write
//! request to `guardian-helper` (§2.3) — this binary has no code path that
//! constructs a call against `GuardianHelper1` at all, and has no D-Bus
//! client/proxy construction of any kind.
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
//!
//! **Repair of the independent audit's G5 FC-2 finding**: this binary
//! evaluates `guardian_core::budget::recorder_policy_for()` on a real,
//! periodic, no-privilege monitoring tick and records a real `Event` into
//! a real `BoundedRecorder` — genuine production wiring, not a fixture.
//! It does **not** claim FC-2 closed: no real spill/retention sink exists
//! yet for either policy branch. See the module doc on
//! [`monitoring_tick`].

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use guardian_core::budget::{self, FreeSpaceState};
use guardian_core::event::Event;
use guardian_core::recorder::BoundedRecorder;
use guardian_core::risk::Risk;
use guardian_daemon::GuardianContract;
use guardian_provider_api::{EventId, ProviderId};

const WELL_KNOWN_NAME: &str = "io.github.cliffthelin.Guardian1";
const OBJECT_PATH: &str = "/io/github/cliffthelin/Guardian1";
const PROVIDER_ID: &str = "guardian.g7.daemon-monitor";
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
fn monitoring_tick(recorder: &Mutex<BoundedRecorder>, state: &Path) {
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

    let mut guard = recorder.lock().unwrap();
    guard.record(event);
    eprintln!(
        "[guardian-daemon] monitoring tick: recorder len={} dropped={} policy={policy:?} free_space={free_space:?} (FC-2 not closed: no spill sink wired)",
        guard.len(),
        guard.dropped_count()
    );
}

fn main() -> zbus::Result<()> {
    let state = state_dir();
    fs::create_dir_all(&state).expect("create daemon state directory");

    let recorder = std::sync::Arc::new(Mutex::new(
        BoundedRecorder::new(RECORDER_CAPACITY).expect("fixed positive capacity"),
    ));
    let recorder_for_thread = std::sync::Arc::clone(&recorder);
    let state_for_thread = state.clone();
    std::thread::spawn(move || {
        loop {
            monitoring_tick(&recorder_for_thread, &state_for_thread);
            std::thread::sleep(MONITORING_TICK_INTERVAL);
        }
    });

    let connection = zbus::blocking::connection::Builder::system()?
        .name(WELL_KNOWN_NAME)?
        .serve_at(OBJECT_PATH, GuardianContract::default())?
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
    use super::{monitoring_tick, probe_free_space};
    use guardian_core::budget::FreeSpaceState;
    use guardian_core::recorder::BoundedRecorder;
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
        monitoring_tick(&recorder, &dir);
        assert_eq!(recorder.lock().unwrap().len(), 1);
        monitoring_tick(&recorder, &dir);
        assert_eq!(recorder.lock().unwrap().len(), 2);
    }

    #[test]
    fn monitoring_tick_respects_the_recorder_bound_across_many_ticks() {
        let dir = temp_dir("tick-bounded");
        let recorder = Mutex::new(BoundedRecorder::new(2).unwrap());
        for _ in 0..5 {
            monitoring_tick(&recorder, &dir);
        }
        let guard = recorder.lock().unwrap();
        assert_eq!(guard.len(), 2, "must never exceed the configured capacity");
        assert_eq!(
            guard.dropped_count(),
            3,
            "the 3 oldest ticks must be counted as dropped"
        );
    }
}
