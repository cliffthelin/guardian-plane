//! Layer 1 structural contract for the PSI inherited-descriptor ingress
//! gate (`phase2-psi-inherited-descriptor-ingress`;
//! `P2-EVT-005`/`P2-EVT-006`/`P2-EVT-008`).
//!
//! **Why this file asserts against source text and unit-file text rather
//! than only behavior.** The defect this gate closes is *not* a behavioral
//! bug in a library: `guardian-core`'s `providers::psi` was complete,
//! correct, and covered by seven passing unit tests for three gates while
//! `guardian-daemon` never instantiated a single line of it. A behavioral
//! test can only observe code that something calls; it cannot observe that
//! production `main()` calls nothing. The gate TDD's R12 therefore requires
//! proof that the daemon's **own production wiring** stands the producer
//! up, and explicitly sanctions the technique used here: "`main()`'s wiring
//! factored into a testable function that `main()` then calls, with a test
//! asserting `main()` calls it". The behavioral half of that pair lives in
//! `guardian-daemon.rs`'s own `mod tests`, which drives the very same
//! functions; this file pins the call site those functions are reached
//! from, so a future refactor cannot quietly orphan the producer again.
//!
//! The same reasoning covers the unit-file assertions: the descriptors this
//! gate depends on exist only because `debian/guardian-daemon.service`
//! declares them, and the manifest's `[unit_file_constraints]` bound that
//! file to *additive `:graceful` `OpenFile=` lines and nothing else*.

use std::path::PathBuf;

/// The production daemon binary's own source. `include_str!` (not a
/// runtime read) so that deleting or moving the file is a compile error
/// rather than a silently skipped assertion.
const DAEMON_BIN_SOURCE: &str = include_str!("../src/bin/guardian-daemon.rs");

/// The real packaged unit — the only place PSI descriptors can come from.
const UNIT_FILE: &str = include_str!("../../../debian/guardian-daemon.service");

/// The daemon's D-Bus surface — `Capabilities1::psi_summary` in
/// particular. `include_str!`-ed for the same reason as `DAEMON_BIN_SOURCE`:
/// a future refactor that reintroduces a `/proc/pressure` pathname probe
/// here must fail to compile this test, not merely fail at runtime.
///
/// Final acceptance repair: an independent re-review reproduced, live,
/// `ListCapabilities` reporting `psi.pressure.cpu` `available`/`healthy`
/// while `PsiSummary` simultaneously reported `supported=false` for the
/// same resource on the same running daemon. The root cause was exactly
/// the class of defect `the_daemon_never_opens_psi_by_pathname` (below)
/// already guards against in `guardian-daemon.rs`/`psi_ingress.rs` — but
/// `dbus_surface.rs` was never included in that guard, so the guard did
/// not catch it. `dbus_surface_never_opens_psi_by_pathname` closes that
/// gap.
const DBUS_SURFACE_SOURCE: &str = include_str!("../src/dbus_surface.rs");

/// The daemon-library module that owns inherited-descriptor resolution and
/// the per-resource PSI monitor. Read at runtime rather than
/// `include_str!`-ed so that its absence surfaces as a named test failure
/// (the RED state this gate started from) instead of a compile error that
/// would take the whole file's other assertions down with it.
fn psi_ingress_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/psi_ingress.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "guardian-daemon must own a PSI inherited-descriptor ingress module at {}: {error}",
            path.display()
        )
    })
}

/// Everything before `#[cfg(test)]`, i.e. the code that actually ships.
/// Every "the daemon does/does not do X" assertion below is made against
/// this region only, so a test helper can never satisfy a production
/// claim — the precise confusion (capability present somewhere in the
/// crate vs. instantiated in production) that let this defect survive.
fn production_region(source: &str) -> &str {
    source
        .find("#[cfg(test)]")
        .map_or(source, |offset| &source[..offset])
}

/// The body of `fn main()`, from its opening brace to the first
/// column-zero `}`.
fn main_body(source: &str) -> &str {
    let start = source
        .find("\nfn main()")
        .expect("the daemon binary must define a top-level fn main()");
    let body_start = source[start..]
        .find('{')
        .expect("fn main() must have a body")
        + start;
    let end = source[body_start..]
        .find("\n}\n")
        .expect("fn main()'s body must be closed by a column-zero brace")
        + body_start;
    &source[body_start..end]
}

fn unit_directives() -> Vec<&'static str> {
    UNIT_FILE
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

// ---------------------------------------------------------------------
// P2-EVT-005 / R12 — production instantiation, not library capability
// ---------------------------------------------------------------------

/// R12. The single most important assertion in this gate: production
/// `main()` must actually call the PSI producer's startup function. The
/// original defect was a fully capable `PsiEventSource` that no production
/// call site ever constructed.
#[test]
fn main_stands_up_the_psi_producer_during_daemon_startup() {
    let body = main_body(DAEMON_BIN_SOURCE);
    assert!(
        body.contains("spawn_psi_ingress("),
        "fn main() must call spawn_psi_ingress(...) — a PSI producer that main() \
         never instantiates is exactly the defect this gate closes"
    );
}

/// R12. `spawn_psi_ingress` must be real production code. A
/// `#[cfg(test)]`-gated definition would make the assertion above true
/// while shipping a daemon with no PSI producer at all.
#[test]
fn the_psi_startup_function_is_production_code_not_test_only() {
    let production = production_region(DAEMON_BIN_SOURCE);
    assert!(
        production.contains("fn spawn_psi_ingress("),
        "spawn_psi_ingress must be defined outside any #[cfg(test)] region"
    );
    assert!(
        production.contains("fn psi_ingress_loop("),
        "the per-resource worker loop must be production code"
    );
    assert!(
        production.contains("fn dispatch_psi_wake("),
        "the wake -> classify -> Event -> admit step must be production code, \
         so the behavioral tests can drive the same function the worker loop calls"
    );
}

/// R12/R13. The producer must reach correlation through the daemon's
/// existing single admission point.
#[test]
fn the_psi_producer_admits_through_the_existing_shared_admission_point() {
    let production = production_region(DAEMON_BIN_SOURCE);
    let dispatch = production
        .find("fn dispatch_psi_wake(")
        .map(|offset| &production[offset..])
        .expect("dispatch_psi_wake must exist in production code");
    let dispatch_body = &dispatch[..dispatch.find("\n}\n").unwrap_or(dispatch.len())];
    assert!(
        dispatch_body.contains("admit_event("),
        "PSI events must be admitted through the same admit_event(...) call site \
         monitoring_tick and capability_registry_tick already use"
    );
}

/// `P2-EVT-005` required evidence, second item: exactly one admission
/// point daemon-wide. A PSI producer that built its own `IngressClock` or
/// `CorrelationEngine` would produce events that never share Gate 2b's
/// single ingress sequence, silently contradicting Gate 2b R2.
#[test]
fn the_daemon_constructs_exactly_one_ingress_clock_and_one_correlation_engine() {
    let production = production_region(DAEMON_BIN_SOURCE);
    assert_eq!(
        production.matches("IngressClock::new()").count(),
        1,
        "production code must construct exactly one IngressClock"
    );
    assert_eq!(
        production.matches("CorrelationEngine::new(").count(),
        1,
        "production code must construct exactly one CorrelationEngine"
    );
    let ingress_module = psi_ingress_source();
    let ingress_production = production_region(&ingress_module);
    assert!(
        !ingress_production.contains("IngressClock::new()")
            && !ingress_production.contains("CorrelationEngine::new("),
        "the PSI ingress module must consume the daemon's shared ingress, never build its own"
    );
}

// ---------------------------------------------------------------------
// P2-EVT-006 — descriptor discipline that only source can prove
// ---------------------------------------------------------------------

/// R2/defect 2. `:graceful` reorders the inherited FD list when a path is
/// absent, so any fixed-index resolution is a reproducible mis-binding
/// bug. The resolver must consult `LISTEN_FDNAMES`.
#[test]
fn descriptor_resolution_reads_listen_fdnames_and_validates_listen_pid() {
    let source = psi_ingress_source();
    let production = production_region(&source);
    for expected in ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"] {
        assert!(
            production.contains(expected),
            "the resolver must consume {expected} from the listen-fd protocol"
        );
    }
    assert!(
        production.contains("std::process::id()") || production.contains("process::id()"),
        "LISTEN_PID must be validated against the daemon's own PID, so a descriptor \
         set addressed to another process is never consumed"
    );
}

/// R5/defect 6. `/proc/self/fd/N` resolves against the **reader's** fd
/// table, so materializing one as a symlink in a shared directory hands
/// another process a path that silently means something else. These paths
/// must be built in-process and never written anywhere.
#[test]
fn no_proc_self_fd_path_is_ever_materialized_on_a_filesystem() {
    let source = psi_ingress_source();
    let sources = [
        production_region(DAEMON_BIN_SOURCE),
        production_region(&source),
    ];
    for text in sources {
        for forbidden in ["symlink(", "soft_link(", "symlink_file(", "hard_link("] {
            assert!(
                !text.contains(forbidden),
                "PSI descriptor paths must be constructed in-process only; found {forbidden}"
            );
        }
    }
}

/// The daemon must reach PSI *only* through inherited descriptors.
/// `ProcSubset=pid` denies it any pathname access to `/proc/pressure`, so
/// a pathname fallback could only ever be a silent, permanent
/// `Unavailable` masquerading as a monitoring path.
#[test]
fn the_daemon_never_opens_psi_by_pathname() {
    let source = psi_ingress_source();
    for text in [
        production_region(DAEMON_BIN_SOURCE),
        production_region(&source),
    ] {
        assert!(
            !text.contains("PsiFileSource::real()"),
            "guardian-daemon must not build a /proc/pressure pathname source — \
             ProcSubset=pid denies it, and a silent Unavailable is not monitoring"
        );
    }
}

/// Final acceptance repair regression guard. `Capabilities1::psi_summary`
/// was, until this pass, the one production PSI reader this file's other
/// pathname assertions never covered — `dbus_surface.rs` was outside their
/// scope — and it was exactly the one still calling
/// `PsiFileSource::real()`, producing a live, reproducible contradiction
/// with `ListCapabilities` (which had already been repaired). This pins
/// the fix at the source-text level, the same way the sibling guard above
/// pins `guardian-daemon.rs`/`psi_ingress.rs`.
#[test]
fn dbus_surface_never_opens_psi_by_pathname() {
    let production = production_region(DBUS_SURFACE_SOURCE);
    assert!(
        !production.contains("PsiFileSource::real()"),
        "Capabilities1's D-Bus surface (list_capabilities, psi_summary) must never build \
         a /proc/pressure pathname source — ProcSubset=pid denies it, and doing so is \
         exactly how ListCapabilities and PsiSummary previously contradicted each other \
         live on the same running daemon"
    );
}

/// R9. Nothing outside `guardian-daemon` may supply a PSI severity. The
/// daemon must never hand-build a `psi_threshold_crossing` `Event`: every
/// one must come from the accepted G8 dispatcher, classified from bytes
/// the daemon read itself.
#[test]
fn the_daemon_never_hand_builds_a_psi_event_with_its_own_severity() {
    let source = psi_ingress_source();
    for text in [
        production_region(DAEMON_BIN_SOURCE),
        production_region(&source),
    ] {
        assert!(
            !text.contains("\"psi_threshold_crossing\""),
            "a PSI Event's shape and severity are the accepted G8/G5 path's output; \
             the daemon must not construct one with a severity of its own choosing"
        );
    }
}

// ---------------------------------------------------------------------
// debian/guardian-daemon.service — [unit_file_constraints]
// ---------------------------------------------------------------------

/// R16/defect 1. Without `:graceful`, a missing `OpenFile=` path aborts
/// the unit with `status=202/FDS` **before `ExecStart`** — a PSI-less
/// kernel or container would take the entire daemon down.
#[test]
fn every_openfile_line_is_graceful_and_names_one_psi_resource() {
    let open_files: Vec<&str> = unit_directives()
        .into_iter()
        .filter(|line| line.starts_with("OpenFile="))
        .collect();
    assert_eq!(
        open_files.len(),
        3,
        "exactly one OpenFile= line per monitored PSI resource (cpu, memory, io); \
         the `full` pressure class is parsed but never classified, so it gets no \
         descriptor of its own"
    );
    for (line, resource) in open_files.iter().zip(["cpu", "memory", "io"]) {
        assert_eq!(
            *line,
            format!("OpenFile=/proc/pressure/{resource}:psi-{resource}:graceful"),
            "each OpenFile= line must name a stable fd name and use :graceful"
        );
    }
}

/// R5/defect 4. With the FD store enabled, a stale already-triggered
/// descriptor is returned *first* under a duplicated name, so name lookup
/// picks it and trigger registration fails `EBUSY`.
#[test]
fn the_unit_never_enables_a_file_descriptor_store() {
    assert!(
        !UNIT_FILE.contains("FileDescriptorStoreMax"),
        "FileDescriptorStoreMax must never be set on this unit"
    );
}

/// `[unit_file_constraints]`: no `Bind*Paths=` of any kind. The whole
/// point of `OpenFile=` is that it adds no mount and relocates no path.
#[test]
fn the_unit_adds_no_bind_mounts() {
    for forbidden in ["BindPaths=", "BindReadOnlyPaths="] {
        assert!(
            !UNIT_FILE.contains(forbidden),
            "{forbidden} must not appear — OpenFile= adds no mount by design"
        );
    }
}

/// `[unit_file_constraints]`: the only permitted delta is additive
/// `OpenFile=` lines. Every directive accepted at G7/G9/Gate 2c must still
/// be present, unchanged, and in its original relative order.
#[test]
fn the_accepted_sandbox_directives_survive_verbatim_and_in_order() {
    const ACCEPTED: [&str; 42] = [
        "[Unit]",
        "Description=Guardian production daemon (unprivileged core/monitoring process)",
        "After=dbus.socket",
        "Wants=dbus.socket",
        "[Service]",
        "Type=simple",
        "User=guardiand",
        "Group=guardiand",
        "BusName=io.github.cliffthelin.Guardian1",
        "Environment=GUARDIAN_DAEMON_STATE_DIR=/var/lib/guardian/daemon",
        "StateDirectory=guardian/daemon",
        "RuntimeDirectory=guardian-daemon",
        "ExecStart=/usr/bin/guardian-daemon",
        "Restart=on-failure",
        "NoNewPrivileges=yes",
        "CapabilityBoundingSet=",
        "AmbientCapabilities=",
        "PrivateTmp=yes",
        "PrivateDevices=yes",
        "ProtectSystem=strict",
        "ReadWritePaths=/var/lib/guardian/daemon",
        "ProtectHome=yes",
        "ProtectKernelTunables=yes",
        "ProtectKernelModules=yes",
        "ProtectKernelLogs=yes",
        "ProtectControlGroups=yes",
        "RestrictAddressFamilies=AF_UNIX",
        "RestrictNamespaces=yes",
        "RestrictRealtime=yes",
        "RestrictSUIDSGID=yes",
        "LockPersonality=yes",
        "MemoryDenyWriteExecute=yes",
        "SystemCallFilter=@system-service",
        "DevicePolicy=closed",
        "ProtectClock=yes",
        "SystemCallArchitectures=native",
        "ProtectHostname=yes",
        "ProtectProc=invisible",
        "ProcSubset=pid",
        "PrivateNetwork=yes",
        "PrivateUsers=yes",
        "UMask=0077",
    ];

    let actual = unit_directives();
    let mut cursor = 0usize;
    for expected in ACCEPTED {
        let found = actual[cursor..]
            .iter()
            .position(|line| *line == expected)
            .unwrap_or_else(|| {
                panic!(
                    "accepted directive `{expected}` was removed, re-valued, or reordered — \
                     this gate may only ADD OpenFile= lines"
                )
            });
        cursor += found + 1;
    }

    // Nothing but `OpenFile=` and the `[Install]` stanza may be new.
    let accepted: std::collections::BTreeSet<&str> = ACCEPTED.into_iter().collect();
    for line in actual {
        assert!(
            accepted.contains(line)
                || line.starts_with("OpenFile=")
                || line == "[Install]"
                || line == "WantedBy=multi-user.target",
            "unexpected new unit directive `{line}` — the only permitted delta is \
             additive OpenFile= lines"
        );
    }
}
