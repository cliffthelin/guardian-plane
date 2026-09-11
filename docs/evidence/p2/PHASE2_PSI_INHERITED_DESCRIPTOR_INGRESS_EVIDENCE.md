# PSI inherited-descriptor ingress — Phase D/E evidence

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`.
Governing TDD:
`docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-tdd.md`.

Owned normative IDs: `P2-EVT-005`, `P2-EVT-006`, `P2-EVT-007`,
`P2-EVT-008`, `P2-VM-003`.

Baseline: `4d5df972a4fde6ae788f13e0aac68c38e295a411`, verified before any
change (`git rev-parse HEAD`), with the expected uncommitted governance
documents from the 2026-09-07 PSI production-wiring governance-repair pass
present in the working tree.

**Scope of this document.** This pass executed the TDD's **Phases A–E**
only. **`P2-VM-003` (Phase F, real-VM evidence from the real production
systemd unit) is deliberately not attempted here** and is recorded as
outstanding in the last section. Nothing below should be read as
satisfying `P2-VM-003`.

---

## 1. Baseline verification (execution protocol step 2)

Run on `guardian-g9` (multipass, Ubuntu 26.04, `libadwaita-1-dev`
present), from a tarball of the unmodified tree with `target/` and `.git/`
excluded and key files checksum-matched against the host first.

```text
cargo fmt --check                                              -> exit 0
cargo clippy --workspace --all-targets --all-features -D warnings -> exit 0
cargo test --workspace   -> passed=409 failed=0 ignored=3
```

Matches the manifest's `expected_baseline` exactly
(`409 passed, 0 failed, 3 ignored`).

---

## 2. RED evidence (execution protocol step 3)

Two waves, both observed failing against the unmodified baseline before
any production code was written.

### Wave 1 — behavioral RED (compiles at baseline, fails at baseline)

`crates/guardian-core/src/providers/psi.rs`, `P2-EVT-008` / R17:

```text
test providers::psi::tests::invalid_raw_values_can_never_produce_a_severity ... FAILED
test providers::psi::tests::out_of_range_raw_values_are_rejected_at_the_read_boundary ... FAILED
test providers::psi::tests::non_finite_raw_values_are_rejected_at_the_read_boundary ... FAILED

  invalid raw value must surface a typed parse error, got Ok(Nominal)
  negative: out-of-range PSI value must be a typed parse error,
    got Ok(Present(PsiResource { some: PsiLine { avg10: -1.0, ... } }))
  nan-avg10: non-finite PSI value must be a typed parse error,
    got Ok(Present(PsiResource { some: PsiLine { avg10: NaN, ... } }))

test result: FAILED. 8 passed; 3 failed
```

The first failure message is the substantive finding: at baseline a raw
`avg10=NaN` reached `classify()` and came back **`Nominal`** — a real
fail-open, since `NaN`'s comparison semantics make every threshold test
false. This was a genuine latent defect, not a hypothetical.

`crates/guardian-daemon/tests/phase2_psi_ingress_contract.rs`:

```text
test result: FAILED. 3 passed; 9 failed
```

RED (9): `main_stands_up_the_psi_producer_during_daemon_startup`,
`the_psi_startup_function_is_production_code_not_test_only`,
`the_psi_producer_admits_through_the_existing_shared_admission_point`,
`the_daemon_constructs_exactly_one_ingress_clock_and_one_correlation_engine`,
`descriptor_resolution_reads_listen_fdnames_and_validates_listen_pid`,
`no_proc_self_fd_path_is_ever_materialized_on_a_filesystem`,
`the_daemon_never_opens_psi_by_pathname`,
`the_daemon_never_hand_builds_a_psi_event_with_its_own_severity`,
`every_openfile_line_is_graceful_and_names_one_psi_resource`.

**Honest qualification.** Three of those nine
(`no_proc_self_fd_path_is_ever_materialized_on_a_filesystem`,
`the_daemon_never_opens_psi_by_pathname`,
`the_daemon_never_hand_builds_a_psi_event_with_its_own_severity`) are
*negative* assertions that failed at baseline only because the module they
also scan did not yet exist; their assertions about the daemon binary were
already satisfiable. They are honestly regression guards against
reintroduction, not proof of a behavior that was previously absent.
`the_daemon_constructs_exactly_one_ingress_clock_and_one_correlation_engine`
is a hybrid: its daemon-binary half was already green, its PSI-module half
was not.

Already-green regression guards (3, GREEN at baseline and still green):
`the_unit_never_enables_a_file_descriptor_store`,
`the_unit_adds_no_bind_mounts`,
`the_accepted_sandbox_directives_survive_verbatim_and_in_order`.

### Wave 2 — compile-failure RED

The behavioral production-path tests in `guardian-daemon.rs`'s `mod tests`
and the acquisition tests in `psi_ingress.rs` were written against an API
that did not exist:

```text
11 error[E0425]: cannot find function `resolve_psi_descriptors` in this scope
 3 error[E0425]: cannot find value `PSI_MONITORED_RESOURCES` in this scope
 1 error[E0432]: unresolved imports `super::PSI_MONITOR_CONFIG`, `super::PsiStepOutcome`,
                 `super::dispatch_psi_wake`, `super::psi_ingress_step`
 1 error[E0432]: unresolved imports `guardian_daemon::psi_ingress::PSI_FD_NAME_CPU`, ...
 ... 29 errors total; no production code existed to satisfy them
```

This wave is compile-failure RED, which is weaker evidence than a
behavioral failure. It is reported as such rather than as observed
behavioral RED.

---

## 3. Production call graph (`P2-EVT-005`, R12/R13)

```
main()                                    guardian-daemon.rs
  |-- IngressClock::new() / CorrelationEngine::new()   (the only pair)
  |-- thread: monitoring_tick ---------------\
  |-- spawn_psi_ingress(&ingress_clock, &engine)       <-- NEW
  |     |-- psi_ingress::psi_descriptor_plan_from_env()
  |     |     `-- resolve_psi_descriptors(LISTEN_PID, LISTEN_FDS, LISTEN_FDNAMES, own_pid)
  |     |-- psi_ingress::start_psi_monitors(&plan, PSI_MONITOR_CONFIG)
  |     |     `-- PsiResourceMonitor::register(kind, /proc/self/fd/N, config)
  |     |           |-- PsiEventDispatcher::new(...)   (accepted G8, unmodified)
  |     |           `-- PsiTrigger::register(...)      (accepted G8, unmodified)
  |     `-- thread per resource: psi_ingress_loop
  |           `-- psi_ingress_step  -> monitor.wait(POLLPRI)
  |                 `-- dispatch_psi_wake -> monitor.dispatch_wake()
  |                       `-- admit_event(&ingress_clock, &engine, event) ---\
  |-- thread: capability_registry_tick ------------------------------------->|
                                                                             v
                                          the one shared IngressClock / CorrelationEngine
```

`spawn_psi_ingress` is the third instance of this binary's established
producer pattern; it constructs no ingress state of its own and reaches
correlation only through the pre-existing `admit_event` call site.

---

## 4. Evidence by normative ID

### `P2-EVT-005` — production instantiation, one admission point

| Requirement | Proof |
|---|---|
| R12 production wiring instantiates the producer | `phase2_psi_ingress_contract.rs::main_stands_up_the_psi_producer_during_daemon_startup` (call site) + `guardian-daemon.rs::tests::psi_startup_registers_exactly_one_monitor_per_inherited_descriptor` (behavior, same function `main()` calls) |
| R12 not `#[cfg(test)]`-only | `phase2_psi_ingress_contract.rs::the_psi_startup_function_is_production_code_not_test_only` |
| R13 shared ingress sequence | `guardian-daemon.rs::tests::psi_events_share_the_one_ingress_sequence_with_the_other_producers` |
| Exactly one admission point | `phase2_psi_ingress_contract.rs::the_daemon_constructs_exactly_one_ingress_clock_and_one_correlation_engine`, `::the_psi_producer_admits_through_the_existing_shared_admission_point` |

### `P2-EVT-006` — inherited-descriptor acquisition and triggering

| Requirement | Proof (`psi_ingress::tests::` unless noted) |
|---|---|
| R1 `LISTEN_PID` absent / non-numeric / mismatched | `an_absent_listen_pid_yields_no_descriptors`, `a_non_numeric_listen_pid_yields_no_descriptors_and_never_panics`, `a_mismatched_listen_pid_never_takes_another_processs_descriptors` |
| R2 name-based, permuted, partial, duplicated | `descriptors_are_resolved_by_name_not_by_position`, `a_partial_graceful_set_binds_present_names_and_reports_the_absent_one`, `a_duplicated_fd_name_resolves_to_nothing_rather_than_an_arbitrary_entry`, `unknown_fd_names_are_ignored_not_mistaken_for_psi_descriptors`, `an_fdnames_count_that_disagrees_with_listen_fds_fails_closed`, `an_absurd_listen_fds_count_is_rejected` |
| R3 one descriptor per resource, none for `full` | `exactly_one_fd_name_exists_per_monitored_resource_and_none_for_full`, `a_resource_never_resolves_to_more_than_one_descriptor`, `guardian-daemon.rs::tests::psi_startup_registers_exactly_one_monitor_per_inherited_descriptor` |
| R4 `FD_CLOEXEC` | `every_descriptor_this_daemon_opens_is_close_on_exec` (reads the kernel's own `/proc/self/fdinfo/N` `flags:`) |
| R5 no FD store, no symlink farm | `phase2_psi_ingress_contract.rs::the_unit_never_enables_a_file_descriptor_store`, `::no_proc_self_fd_path_is_ever_materialized_on_a_filesystem`, `descriptor_paths_are_built_in_process_and_never_written_anywhere` |
| R6 registration through an inherited descriptor | `guardian-daemon.rs::tests::psi_startup_registers_exactly_one_monitor_per_inherited_descriptor` (drives the full env-string -> resolve -> `/proc/self/fd/N` -> register chain against a descriptor the test opened, not a path the production code chose) |
| R7 one descriptor, both roles | `guardian-daemon.rs::tests::one_inherited_descriptor_serves_both_the_trigger_and_the_content_reread` |
| R8 `EBUSY` hard, never retried | `registration_failures_are_hard_for_their_resource`, `guardian-daemon.rs::tests::ebusy_at_registration_is_classified_as_a_hard_error`, `::a_psi_registration_failure_degrades_psi_observability_only` (`attempts == plan.len()`) |
| R15 restart / fresh description | `guardian-daemon.rs::tests::restarting_the_psi_producer_re_registers_with_no_carried_over_state` |

### `P2-EVT-007` — daemon-owned classification and `Event`

| Requirement | Proof (`guardian-daemon.rs::tests::`) |
|---|---|
| R9 severity derived from bytes the daemon read | `psi_severity_is_derived_only_from_the_bytes_the_daemon_read_itself`; `phase2_psi_ingress_contract.rs::the_daemon_never_hand_builds_a_psi_event_with_its_own_severity` |
| R10 authority fields; `resource_refs` per kind | `a_real_crossing_produces_the_accepted_psi_event_shape`, `resource_identity_is_the_kernel_path_not_the_descriptor_path` |
| R11 full `Event` shape | `a_real_crossing_produces_the_accepted_psi_event_shape` — `event_type "psi_threshold_crossing"`, `resource_refs ["/proc/pressure/cpu"]`, `severity Moderate`, `normalized_key "psi cpu threshold crossing nominal->elevated"` (matches the preflight §9.6 reference shape exactly) |
| R14 `P2-COR-001`/`P2-COR-002` regression | `a_psi_critical_crossing_opens_an_incident_and_the_next_one_links_to_it` |

### `P2-EVT-008` — degradation, invalid input, non-interference

| Requirement | Proof |
|---|---|
| R16 zero and partial descriptors | `guardian-daemon.rs::tests::the_daemon_runs_normally_with_zero_psi_descriptors`, `::a_partial_graceful_descriptor_set_leaves_the_absent_resource_unavailable`, `phase2_psi_ingress_contract.rs::every_openfile_line_is_graceful_and_names_one_psi_resource` |
| R17 malformed / non-finite / out-of-range | `providers::psi::tests::non_finite_raw_values_are_rejected_at_the_read_boundary`, `::out_of_range_raw_values_are_rejected_at_the_read_boundary`, `::invalid_raw_values_can_never_produce_a_severity`, `guardian-daemon.rs::tests::invalid_raw_psi_values_never_produce_an_admitted_event` |
| R18 provider health unaffected | all Gate 2b / 2b-repair tests green and untouched in the diff; `guardian-daemon.rs::tests::a_psi_registration_failure_degrades_psi_observability_only` |

---

## 5. `debian/guardian-daemon.service` delta

Purely additive; 12 lines added, 0 removed, 0 re-valued, 0 reordered:

```
+OpenFile=/proc/pressure/cpu:psi-cpu:graceful
+OpenFile=/proc/pressure/memory:psi-memory:graceful
+OpenFile=/proc/pressure/io:psi-io:graceful
```

plus a nine-line explanatory comment block. `ProcSubset=pid` and
`ProtectProc=invisible` are unchanged; no `FileDescriptorStoreMax`; no
`Bind*Paths=`. Enforced continuously by
`phase2_psi_ingress_contract.rs::the_accepted_sandbox_directives_survive_verbatim_and_in_order`,
which pins all 42 accepted directives in order and rejects any new
directive that is not an `OpenFile=` line.

---

## 6. Post-change validation

Same environment and method as §1 (fresh tarball, `target/`/`.git/`
excluded, five key files checksum-matched against the host before the run):

```text
cargo fmt --check                                                 -> exit 0
cargo clippy --workspace --all-targets --all-features -D warnings -> exit 0
cargo test --workspace   -> passed=454 failed=0 ignored=3   (4 consecutive runs)
```

Baseline was `409 passed, 0 failed, 3 ignored`; the deterministic new
tests add **+45 passing**, with no test deleted, ignored, or weakened.

### A real flakiness defect found and fixed during validation

The first full-workspace VM run failed one test
(`a_psi_registration_failure_degrades_psi_observability_only`) that passed
in isolation and passed on the host. It was reproduced on the host at
roughly 1-in-3 under the full parallel suite. Two genuine defects, both in
this gate's own work, not pre-existing:

1. `MAX_LISTEN_FDS` was set to `64`, an arbitrary number chosen from this
   unit's three `OpenFile=` lines. The test helper synthesizes a
   `LISTEN_FDS` value from a real fd number, and a test binary running 24
   tests in parallel pushes fd numbers past 66 — so the resolver correctly
   failed closed and returned an empty plan, and the test's assertions then
   failed for an unrelated reason. The bound is now `1024`, sized to a
   process fd table, with the rationale that the substantive consistency
   check is the `LISTEN_FDNAMES`/`LISTEN_FDS` count equality (which still
   rejects `LISTEN_FDS=100000`), not this bound. The helper now asserts its
   own precondition so a future occurrence is diagnosable rather than
   silent.
2. The test induced its registration failure by closing a descriptor and
   deleting the file. Under parallel execution the freed fd number was
   reused by another test's file, so `/proc/self/fd/N` resolved to a valid
   PSI fixture and registration *succeeded*. Replaced with a read-only
   (`0o444`) descriptor: readable, but the kernel PSI ABI requires
   `O_RDWR`, so registration fails for real and the outcome cannot be
   perturbed by fd-number reuse.

Verified stable afterwards: 12 consecutive host runs of the daemon binary
suite and 4 consecutive full-workspace VM runs, all green.

---

## 7. Deferred to Phase F (`P2-VM-003`) — NOT satisfied by this document

Every item below requires the real production systemd unit on real
hardware and is untouched by this pass. Three of them are the parts of
Phases A–E that a unit test structurally cannot reach:

- **Real kernel trigger registration and a real `poll(POLLPRI)` wake**
  under bounded `stress-ng` load. Unit tests use regular files, which
  never report `POLLPRI`; `the_production_worker_step_never_manufactures_an_event_without_a_wake`
  asserts the daemon emits nothing in that case, which is the correct
  behavior but is not proof the kernel path wakes.
- **A genuinely inherited descriptor.** Tests synthesize the listen-fd
  protocol strings for a descriptor the test process opened itself. Every
  line of production resolution and registration code runs unmodified, but
  the descriptor's provenance is not PID 1.
- **Real `EBUSY` at registration.** The `EBUSY` *disposition* is pinned by
  test; producing a real one needs a real PSI file with an existing
  trigger on the same description. Preflight §9.4 proved the kernel
  behavior empirically; Phase F must re-prove it inside the real unit.
- **Restart obtaining a fresh open file description** across a real
  `systemctl restart`.
- Phase F items 1, 2, 7, 8, 9, 10, 12, 13 in full: `systemd-analyze
  security` parity, the descriptors systemd actually supplied and their
  `fdinfo`, a real incident over the system bus via
  `Incidents1.ListIncidents()`, `/proc/pressure` still `ENOENT` by
  pathname from inside the running unit, unrelated procfs still hidden, no
  new capabilities, PSI failure degrading PSI only, and the
  provider-health/UPower path still functional.

---

# Phase F — `P2-VM-003` real-VM evidence

**This section was produced by a separate, later pass** (2026-09-07/08)
than everything above it. Sections 1–7 above are the implementation
pass's own record and are left exactly as that pass wrote them —
nothing above this line has been deleted, rewritten, or reflowed.

**Provenance rule used throughout this section.** Every command output
below was captured directly by this pass, on the VM, at the timestamps
shown. Where this section refers to a fact established by an earlier
pass (the implementation pass's unit-test results, or the preflight's
`OpenFile=` experiment), it says so explicitly and does not present it
as newly observed. Nothing here is reconstructed from memory or
inferred: what could not be captured is stated as not captured, in
§F.14.

Governing checklist: the gate TDD's "Phase F — real-VM evidence
(`P2-VM-003`)" 13-item list. Each item below is numbered to match it.

---

## F.0 Method, environment, and tree provenance

### VM identity (directly observed)

```text
$ uname -a
Linux guardian-g9 7.0.0-30-generic #30-Ubuntu SMP PREEMPT_DYNAMIC Fri Jul 31 18:22:54 UTC 2026 x86_64 GNU/Linux
$ lsb_release -a
Distributor ID: Ubuntu
Description:    Ubuntu 26.04 LTS
Release:        26.04
Codename:       resolute
$ systemctl --version | head -1
systemd 259 (259.5-0ubuntu3.4)
$ nproc
4
$ free -m | head -2
               total        used        free      shared  buff/cache   available
Mem:            5402        1248        1483         146        3112        4153
$ ls /proc/pressure
cpu  io  memory
```

`guardian-g9` is the project's existing disposable multipass Ubuntu
26.04 VM.

### How the tree was synced, and the checksum verification performed

The host working tree (`/home/Cliff/SysProjects/Guardian`, HEAD
`4d5df972a4fde6ae788f13e0aac68c38e295a411`, uncommitted implementation
work present) was tarred with `target/` and `.git/` excluded,
transferred with `multipass transfer`, and extracted to
`/home/ubuntu/phasef` on the VM. Eight key files were then verified
against a host-generated `sha256sum` list, on the VM, with
`sha256sum -c`:

```text
=== VM CHECKSUM VERIFY (sha256sum -c against the host list) ===   [2026-09-07 22:45:07 UTC]
crates/guardian-daemon/src/psi_ingress.rs: OK
crates/guardian-daemon/src/bin/guardian-daemon.rs: OK
crates/guardian-core/src/providers/psi.rs: OK
debian/guardian-daemon.service: OK
crates/guardian-daemon/src/lib.rs: OK
crates/guardian-daemon/tests/phase2_psi_ingress_contract.rs: OK
crates/guardian-core/src/correlation.rs: OK
crates/guardian-core/src/psi.rs: OK
```

Host reference digests:

```text
23a43132444460a010ebec64181c8c30f22ab5b1574e59e3ff289ff387f7a20d  crates/guardian-daemon/src/psi_ingress.rs
286ab370abb6fb27132eb3a12e5f576b392656595a747bcf03bfb09988ab7333  crates/guardian-daemon/src/bin/guardian-daemon.rs
77399ae64746b7f79c9f5a8cbb617a2b5a7dd50293bec5a43d5cde7e4007ac7e  crates/guardian-core/src/providers/psi.rs
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  debian/guardian-daemon.service
77245f7d632c390cc8e1d1ad4b523a08c884115f427bc8319fb4eafa07c66e2f  crates/guardian-daemon/src/lib.rs
4317a553b3361cbf0da0ee503a035fa203c27128fc24dfea9a283748cd080ff5  crates/guardian-daemon/tests/phase2_psi_ingress_contract.rs
928e462c31d779b06f02932acb233e8d4a2abf9740920a11fe6d9208d613c24e  crates/guardian-core/src/correlation.rs
a51638559d0629cda8e0801e35ad4321516c1a4df4f6429be8972acd902e5e0c  crates/guardian-core/src/psi.rs
```

The same eight digests were re-verified **after** the package build, to
confirm the build had not altered any source file. All eight still `OK`.

### The daemon under test is the real packaged service, never `cargo run`

The evidence below comes exclusively from `guardian-daemon` running as
the real systemd service, started by PID 1 from the packaged unit. The
binary was produced by a real `dpkg-buildpackage` run over the synced
tree and installed with `apt-get install`:

```text
$ DEB_BUILD_OPTIONS=nocheck dpkg-buildpackage -us -uc -b -d
   ... (debian/rules override_dh_auto_build: cargo build --release --workspace --locked)
   dpkg-deb: building package 'guardian' in '../guardian_0.1.0-1_amd64.deb'

$ dpkg-deb --fsys-tarfile guardian_0.1.0-1_amd64.deb | tar -xO ./usr/lib/systemd/system/guardian-daemon.service | sha256sum
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  -
$ sha256sum debian/guardian-daemon.service
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  debian/guardian-daemon.service
```

The unit inside the package is byte-identical to the host worktree unit.
After installation:

```text
$ sha256sum /usr/lib/systemd/system/guardian-daemon.service
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  /usr/lib/systemd/system/guardian-daemon.service
$ sha256sum /usr/bin/guardian-daemon
30b049a39f4238eb6e99260dad26b11a082079d034716eca28a3d0fea6fc1aed  /usr/bin/guardian-daemon
```

`DEB_BUILD_OPTIONS=nocheck` was used only to skip `debian/rules`'
`override_dh_auto_test` re-run of the suite, which this pass had already
run in full (§F.13); `-d` was needed because `cargo`/`rustc` are
installed via rustup on this VM rather than as apt packages, so
`dpkg-checkbuilddeps` cannot see them. Neither flag changes the built
artifacts.

### Provisioning corrections required (NOT part of the tested change)

- **None were needed for D-Bus.** The bus policy file
  `/etc/dbus-1/system.d/io.github.cliffthelin.Guardian1.conf` was already
  present (dated 2026-09-07 11:10), so the provisioning gap a previous
  pass hit did not recur. Directly verified:

  ```text
  $ ls -la /etc/dbus-1/system.d/ | grep -i guardian
  -rw-r--r-- 1 root root  366 Sep  7 11:10 io.github.cliffthelin.Guardian1.conf
  -rw-r--r-- 1 root root  373 Sep  4 21:31 io.github.cliffthelin.GuardianHelper1.conf
  ```

  The daemon does own its well-known name (§F.7), which is the
  behavioural confirmation.
- **Disk housekeeping only.** The VM was at 79% full, so two stale build
  trees from earlier passes (`/home/ubuntu/Guardian/target`,
  `/home/ubuntu/gate-psi-work2`) were removed and the latter's `target/`
  was reused as this pass's cargo cache. This affects build time only,
  not any artifact: `cargo` re-verifies fingerprints, and `fmt`,
  `clippy`, and the full suite were all re-run from scratch (§F.13).

Both are recorded here as **provisioning**, explicitly not as part of
the change under test.

---

## F.1 The accepted sandbox is still active and unchanged

### Unit-file delta against the accepted G9/Gate-2c baseline

The accepted baseline unit (`git show HEAD:debian/guardian-daemon.service`,
sha256 `a74466c82f5c91bdedb38083a578eb57e029d23fd8fe6de72efd7aeee7120a99`)
was the unit installed on this VM before this pass touched anything —
directly confirmed, same digest. Diff against the installed
PSI-enabled unit:

```text
$ diff -u /home/ubuntu/baseline-unit.service /usr/lib/systemd/system/guardian-daemon.service
@@ -43,5 +43,17 @@
 PrivateUsers=yes
 UMask=0077

+# PSI ingress (P2-EVT-006). systemd opens these paths in PID 1's own
+# namespace and passes the descriptors in, so guardian-daemon can read
+# pressure information while ProcSubset=pid above continues to deny it any
+# /proc/pressure pathname -- no directive is relaxed and no mount is added.
+# :graceful is required: without it a missing path aborts the unit with
+# status=202/FDS before ExecStart, so a PSI-less kernel or container would
+# take the whole daemon down. The daemon resolves these by fd name, never
+# by index, because :graceful renumbers the descriptors that remain.
+OpenFile=/proc/pressure/cpu:psi-cpu:graceful
+OpenFile=/proc/pressure/memory:psi-memory:graceful
+OpenFile=/proc/pressure/io:psi-io:graceful
+
 [Install]
 WantedBy=multi-user.target
```

12 lines added, 0 removed, 0 re-valued, 0 reordered. Exactly the
`[unit_file_constraints].allowed_delta` the manifest permits.

### `systemd-analyze security` — before vs. after, same machine, same invocation

The pre-change table was captured at **22:45:58 UTC**, while the
accepted baseline unit was the installed one. The post-change table was
captured after installing the PSI-enabled unit, with the identical
command and invocation style:

```text
$ diff sec-baseline.txt sec-post-nosudo.txt
IDENTICAL: all 84 lines byte-for-byte equal

$ grep "Overall exposure" sec-baseline.txt sec-post-nosudo.txt
sec-baseline.txt:→ Overall exposure level for guardian-daemon.service: 0.6 SAFE :-}
sec-post-nosudo.txt:→ Overall exposure level for guardian-daemon.service: 0.6 SAFE :-}
```

**No regression: the entire 84-line table, including the overall
exposure level, is byte-for-byte identical before and after.**

### The *running* service's actual properties

```text
$ systemctl show guardian-daemon -p ProcSubset -p ProtectProc -p User -p Group \
      -p NoNewPrivileges -p CapabilityBoundingSet -p AmbientCapabilities \
      -p PrivateUsers -p PrivateNetwork -p ProtectSystem -p ProtectHome \
      -p MemoryDenyWriteExecute -p SystemCallArchitectures -p RestrictNamespaces \
      -p MainPID -p FragmentPath
FragmentPath=/usr/lib/systemd/system/guardian-daemon.service
MainPID=755643
CapabilityBoundingSet=
AmbientCapabilities=
User=guardiand
Group=guardiand
PrivateNetwork=yes
PrivateUsers=yes
ProtectHome=yes
ProtectSystem=strict
NoNewPrivileges=yes
SystemCallArchitectures=native
MemoryDenyWriteExecute=yes
RestrictNamespaces=yes
ProtectProc=invisible
ProcSubset=pid

$ systemctl show guardian-daemon -p OpenFile
OpenFile=/proc/pressure/cpu:psi-cpu:graceful
OpenFile=/proc/pressure/memory:psi-memory:graceful
OpenFile=/proc/pressure/io:psi-io:graceful

$ systemctl show guardian-daemon -p FileDescriptorStoreMax -p BindPaths -p BindReadOnlyPaths
FileDescriptorStoreMax=0
BindPaths=
BindReadOnlyPaths=
```

`ProcSubset=pid` and `ProtectProc=invisible` are live on the running
service; `FileDescriptorStoreMax` is 0 (R5) and there are no bind
mounts of any kind.

---

## F.2 The exact PSI descriptors systemd supplied, and their provenance

Captured from the live daemon at 2026-09-07 22:52:14 UTC, MainPID 755643:

```text
$ sudo cat /proc/755643/environ | tr '\0' '\n' | grep '^LISTEN_'
LISTEN_PID=755643
LISTEN_PIDFDID=749398
LISTEN_FDS=3
LISTEN_FDNAMES=psi-cpu:psi-memory:psi-io
```

`LISTEN_PID` equals the daemon's own MainPID (R1), `LISTEN_FDS` is
exactly 3 — one per monitored resource, none for the `full` class (R3) —
and the names are the unit file's, in declaration order.

```text
$ sudo ls -l /proc/755643/fd
lr-x------ 1 guardiand guardiand 64 Sep  7 16:50 0 -> /dev/null
lrwx------+1 guardiand guardiand 64 Sep  7 16:50 1 -> socket:[2766980]
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 10 -> anon_inode:[eventfd]
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 11 -> anon_inode:[timerfd]
lrwx------+1 guardiand guardiand 64 Sep  7 16:50 12 -> socket:[2760038]
lrwx------+1 guardiand guardiand 64 Sep  7 16:50 2 -> socket:[2766980]
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 3 -> /proc/pressure/cpu
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 4 -> /proc/pressure/memory
lrwx------ 1 guardiand guardiand 64 Sep  7 16:51 5 -> /proc/pressure/io
lrwx------ 1 guardiand guardiand 64 Sep  7 16:51 6 -> anon_inode:[eventpoll]
lrwx------ 1 guardiand guardiand 64 Sep  7 16:51 7 -> /proc/pressure/cpu
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 8 -> /proc/pressure/memory
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 9 -> /proc/pressure/io
```

fds 3/4/5 are systemd's inherited descriptors; fds 7/8/9 are the
daemon's own `/proc/self/fd/N` reopens that carry the kernel triggers.

```text
$ for n in 3 4 5; do sudo cat /proc/755643/fdinfo/$n; done
### fdinfo/3 -> /proc/pressure/cpu       pos: 0   flags: 0100002   mnt_id: 53   ino: 4026532064
### fdinfo/4 -> /proc/pressure/memory    pos: 0   flags: 0100002   mnt_id: 53   ino: 4026532063
### fdinfo/5 -> /proc/pressure/io        pos: 0   flags: 0100002   mnt_id: 53   ino: 4026532062

$ for n in 7 8 9; do sudo cat /proc/755643/fdinfo/$n; done
### fdinfo/7 -> /proc/pressure/cpu       pos: 0   flags: 02100002  mnt_id: 53   ino: 4026532064
### fdinfo/8 -> /proc/pressure/memory    pos: 0   flags: 02100002  mnt_id: 53   ino: 4026532063
### fdinfo/9 -> /proc/pressure/io        pos: 0   flags: 02100002  mnt_id: 53   ino: 4026532062
```

Two things are directly readable from those flags:

- **inherited descriptors arrive without `FD_CLOEXEC`**: `0100002` is
  `O_RDWR|O_LARGEFILE`; `O_CLOEXEC` (`02000000`) is absent — confirming
  preflight defect 3 on the real unit;
- **the daemon's own reopens are close-on-exec by construction**:
  `02100002` includes `O_CLOEXEC`. This is R4's alternative acceptance
  path: the raw inherited descriptor is never retained, and every
  descriptor the daemon itself opens is `CLOEXEC`. The matching `ino:`
  values show each reopen reaches the same inode as its inherited source.

### Provenance: the descriptors really come from PID 1's namespace

```text
$ sudo awk '$5=="/proc"{print "mnt_id="$1" mountpoint="$5" fstype="$9" opts="$6}' /proc/1/mountinfo
mnt_id=53 mountpoint=/proc fstype=proc opts=rw,nosuid,nodev,noexec,relatime

$ sudo grep " /proc " /proc/755643/mountinfo
384 381 0:60 / /proc rw,nosuid,nodev,noexec,relatime shared:481 - proc proc rw,hidepid=invisible,subset=pid

$ sudo awk '{print $1}' /proc/755643/mountinfo | grep -x 53
NOT FOUND -> the PSI descriptors' mnt_id 53 belongs to a mount the daemon cannot see: PID 1's namespace

$ sudo readlink /proc/1/ns/mnt ; sudo readlink /proc/755643/ns/mnt
mnt:[4026531832]
mnt:[4026532392]
```

This is the decisive check. The PSI descriptors carry `mnt_id 53`, which
is **PID 1's own `/proc` mount**. The daemon's `/proc` is a different
mount (`mnt_id 384`, with `hidepid=invisible,subset=pid`) in a different
mount namespace, and `mnt_id 53` appears nowhere in the daemon's
`mountinfo`. The descriptors were therefore opened in PID 1's namespace
and passed in — exactly the `OpenFile=` mechanism the gate relies on,
now confirmed against the live daemon rather than argued from
documentation.

---

## F.3 Real kernel trigger registration through an inherited descriptor

### The daemon's own report

Every start of the real unit logs the outcome of `start_psi_monitors`,
which counts one registration attempt per planned resource:

```text
Sep 07 16:50:39.900193 guardian-g9 guardian-daemon[755643]: [guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
Sep 07 16:50:39.901498 guardian-g9 guardian-daemon[755643]: [guardian-daemon] serving io.github.cliffthelin.Guardian1 at /io/github/cliffthelin/Guardian1, unique_name=:1.22199
```

`3/3 ... (0 failed)` means `PsiTrigger::register` returned `Ok` for all
three resources — i.e. the kernel accepted a real trigger write on each
descriptor derived from systemd's inherited fds.

### Independent kernel-side confirmation (`pidfd_getfd`)

The daemon's own log is its own claim, so it was corroborated against
the kernel directly. `pidfd_open(2)` + `pidfd_getfd(2)` yield a
duplicate referring to the **same open file description** the daemon
holds — re-opening `/proc/<pid>/fd/N` would create a *new* description
and prove nothing. Writing the production trigger string to that
duplicate then asks the kernel whether a trigger already exists on that
description.

Probe source: `/home/ubuntu/fdprobe.py` on the VM (written by this pass,
not a repository file). Trigger payload is exactly
`PSI_MONITOR_CONFIG`'s: `b"some 100000 2000000\x00"`.

```text
[2026-09-07 22:55:21 UTC]  daemon MainPID=755643
pidfd_open(755643) ok
PSI descriptors held by the live daemon: [(3, '/proc/pressure/cpu'), (4, '/proc/pressure/memory'),
 (5, '/proc/pressure/io'), (7, '/proc/pressure/cpu'), (8, '/proc/pressure/memory'), (9, '/proc/pressure/io')]

=== daemon fd 3 -> /proc/pressure/cpu  [inherited from systemd (LISTEN_FDS)] ===
  LIVE READ through the daemon's own open file description:
      some avg10=0.00 avg60=0.00 avg300=0.87 total=503691150
      full avg10=0.00 avg60=0.00 avg300=0.00 total=0
  TRIGGER WRITE: skipped (would create a trigger on a description the daemon still holds)

=== daemon fd 4 -> /proc/pressure/memory  [inherited from systemd (LISTEN_FDS)] ===
  LIVE READ through the daemon's own open file description:
      some avg10=0.00 avg60=0.00 avg300=0.00 total=9697960
      full avg10=0.00 avg60=0.00 avg300=0.00 total=8740325
  TRIGGER WRITE: skipped (would create a trigger on a description the daemon still holds)

=== daemon fd 5 -> /proc/pressure/io  [inherited from systemd (LISTEN_FDS)] ===
  LIVE READ through the daemon's own open file description:
      some avg10=0.00 avg60=0.00 avg300=0.07 total=187041270
      full avg10=0.00 avg60=0.00 avg300=0.06 total=149266853
  TRIGGER WRITE: skipped (would create a trigger on a description the daemon still holds)

=== daemon fd 7 -> /proc/pressure/cpu  [daemon's own /proc/self/fd reopen] ===
  LIVE READ through the daemon's own open file description:
      some avg10=0.00 avg60=0.00 avg300=0.87 total=503691150
      full avg10=0.00 avg60=0.00 avg300=0.00 total=0
  TRIGGER WRITE: FAILED errno=16 (EBUSY) Device or resource busy
      => EBUSY: a PSI trigger ALREADY EXISTS on this open file description.

=== daemon fd 8 -> /proc/pressure/memory  [daemon's own /proc/self/fd reopen] ===
  ... (identical shape)
  TRIGGER WRITE: FAILED errno=16 (EBUSY) Device or resource busy

=== daemon fd 9 -> /proc/pressure/io  [daemon's own /proc/self/fd reopen] ===
  ... (identical shape)
  TRIGGER WRITE: FAILED errno=16 (EBUSY) Device or resource busy

=== CONTROL: a FRESH open file description of the same file ===
  trigger write on a fresh description: SUCCEEDED
  => the trigger string is valid and accepted by this kernel; the EBUSY above is
     scoped to the open file description, not to the inode and not to permissions.
  second write on the SAME fresh description: errno=16 (EBUSY) -> per-description confirmed
```

**What this proves, from the kernel rather than from the daemon's own
logging:**

1. **Item 3 / R6** — all three descriptions the daemon derived from
   systemd's inherited descriptors carry a live kernel PSI trigger.
2. **Real `EBUSY` at registration (deferred item, now closed)** — the
   `EBUSY` disposition is no longer only pinned by a synthesised
   `io::Error`; the real kernel returned `EBUSY` on the real production
   daemon's real descriptions. The control shows the same trigger string
   succeeds on a fresh description and then `EBUSY`s on a second write to
   that same description, so `EBUSY` is per-open-file-description, and
   there is no in-place re-arm (R8).
3. **Item 8's precondition** — the daemon is holding *working*
   descriptors: live PSI text was read through them.
4. **R7, one inherited descriptor serving both roles** — fd 7 both
   carries the trigger and yields live pressure text on read.

---

## F.4 A real `poll(POLLPRI)` wake under bounded `stress-ng` load

`strace` was attached to the live service (never to a `cargo run`
process) and a bounded, CPU-only `stress-ng` workload was applied.
Bounding: CPU class only, no memory/IO stressors, 45 s, on an otherwise
idle disposable VM.

```text
[2026-09-07 22:55:52 UTC]  daemon MainPID=755643
$ sudo strace -f -tt -p 755643 -e trace=ppoll,poll,pread64,read,lseek,openat,write -o psi.strace &
$ stress-ng --cpu 16 --timeout 45s

--- pressure before load ---
some avg10=0.00 avg60=0.00 avg300=0.77 total=503706160
t+5s  22:56:00 | some avg10=32.47 avg60=6.34  avg300=2.10  total=508748378
t+10s 22:56:05 | some avg10=62.62 avg60=15.17 avg300=4.08  total=513785014
t+20s 22:56:15 | some avg10=84.86 avg60=27.80 avg300=7.24  total=523717889
t+30s 22:56:25 | some avg10=93.59 avg60=38.68 avg300=10.33 total=533724348
t+45s 22:56:40 | some avg10=96.74 avg60=51.02 avg300=14.44 total=548367091
```

The PSI worker thread's syscall trace — a complete wake → re-read cycle,
repeated:

```text
755649 16:55:52.181660 ppoll([{fd=7, events=POLLPRI}], 1, NULL, NULL, 8) = 1 ([{fd=7, revents=POLLPRI}])
755649 16:55:57.141923 openat(AT_FDCWD, "/proc/self/fd/3", O_RDONLY|O_CLOEXEC) = 13
755649 16:55:57.142046 read(13, "some avg10=17.75 avg60=3.20 avg3", 32) = 32
755649 16:55:57.142106 read(13, "00=1.44 total=505653452\nfull avg", 32) = 32
755649 16:55:57.142144 read(13, "10=0.00 avg60=0.00 avg300=0.00 t"..., 64) = 39
755649 16:55:57.142180 read(13, "", 25) = 0
755649 16:55:57.142237 ppoll([{fd=7, events=POLLPRI}], 1, NULL, NULL, 8) = 1 ([{fd=7, revents=POLLPRI}])
755649 16:55:59.189934 openat(AT_FDCWD, "/proc/self/fd/3", O_RDONLY|O_CLOEXEC) = 13
755649 16:55:59.190059 read(13, "some avg10=32.47 avg60=6.34 avg3", 32) = 32
...
755649 16:56:01.239716 <... ppoll resumed>) = 1 ([{fd=7, revents=POLLPRI}])
755649 16:56:01.240314 read(13, "some avg10=44.52 avg60=9.37 avg3", 32) = 32
755649 16:56:03.286127 read(13, "some avg10=54.57 avg60=12.33 avg", 32) = 32
755649 16:56:07.190267 read(13, "some avg10=69.03 avg60=17.88 avg", 32) = 32
755649 16:56:09.241127 read(13, "some avg10=73.91 avg60=20.43 avg", 32) = 32
```

```text
$ grep -c "revents=POLLPRI" psi.strace
27
```

**Item 4 is proven at the syscall level.** A regular file never reports
`POLLPRI`; a PSI file reports it only when a registered trigger fires.
`ppoll` returning `revents=POLLPRI` on fd 7 is therefore the kernel
signalling a real threshold crossing on a real registered trigger.

Two further facts fall directly out of this trace:

- **The re-read goes through the inherited descriptor, by fd, not by
  pathname.** After each wake the daemon opens `/proc/self/fd/3` — the
  magic link to systemd's inherited descriptor — and reads live pressure
  text. It never opens `/proc/pressure/*`, which §F.8 shows would fail
  with `ENOENT` anyway. This is R7 observed on the real service: the one
  inherited descriptor (fd 3) serves both the trigger (via the fd 7
  reopen) and the dispatch-time content re-read.
- **Only `cpu` woke.** `memory` and `io` were under no pressure and their
  workers stayed parked in `ppoll`, which is the correct behaviour and
  incidentally confirms the workers are genuinely per-resource.

---

## F.5–F.7 Event construction, shared ingress, and correlation

### F.5/F.7 A real PSI Guardian `Event` and a real PSI-driven incident

Directly observed on the real system bus, from the real unit:

```text
[2026-09-07 23:06:21 UTC]  pressure at thaw: some avg10=75.13 avg60=33.91 avg300=21.49 total=634481795

$ busctl call io.github.cliffthelin.Guardian1 \
      /io/github/cliffthelin/Guardian1/Incidents \
      io.github.cliffthelin.Guardian.Incidents1 ListIncidents
a(sssssss) 1 "guardian.correlation.incident-000000" "ingress-1" "" "open" \
  "PSI critical pressure for /proc/pressure/cpu (direct kernel observation)" \
  "confirmed" "/proc/pressure/cpu"
```

Before the run the same call returned `a(sssssss) 0`. The incident's
fields carry the `Event`'s identity through:

| Wire field | Value | What it proves |
|---|---|---|
| `incident_id` | `guardian.correlation.incident-000000` | Gate 2a's engine minted it |
| `opened_at` | `ingress-1` | the shared `IngressClock` sequence at admission (`format!("ingress-{ingress_sequence}")`, `correlation.rs:769`) |
| `status` | `open` | `P2-COR-001` opened an incident |
| `summary` | `PSI critical pressure for /proc/pressure/cpu (direct kernel observation)` | the event was classified `Risk::High`; `classify()` returns `None` for anything else |
| `confidence` | `confirmed` | Gate 2a's PSI confidence rule |
| `primary_resource` | `/proc/pressure/cpu` | `resource_refs.first()`, i.e. `resource_refs == ["/proc/pressure/cpu"]` (R10) |

Because `classify()` (`correlation.rs:493-507`) admits a
`psi_threshold_crossing` event only when `event.severity == Risk::High`
and keys it on `event.resource_refs.first()`, an incident with this
summary and this `primary_resource` **can only** have been produced by a
real Guardian `Event` with `event_type == "psi_threshold_crossing"`,
`severity == Risk::High`, and `resource_refs == ["/proc/pressure/cpu"]`,
admitted through `admit_event`. The severity was derived by the daemon
from the raw PSI bytes captured in the §F.4 trace — nothing outside the
process supplies or influences it (R9).

**Honest scope note.** The individual `Event` struct's field values were
not dumped directly — the daemon emits no per-event log line on
admission, and this pass added no instrumentation to production code.
What is directly observed is the raw PSI text the daemon read (§F.4) and
the incident the resulting `Event` produced (above); the `Event`'s shape
is inferred from those two endpoints plus the unmodified `classify()`
predicate, not observed field-by-field.

### F.6 The `Event` reaches the shared Phase 2 ingress, not a separate path

A single daemon instance was driven with **both** producers, in order:
the provider-health producer first (upower masked), then a real PSI
crossing.

```text
=== single daemon instance MainPID=768073  [2026-09-07 23:19:19 UTC] ===
[guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
[guardian-daemon] serving io.github.cliffthelin.Guardian1 at /io/github/cliffthelin/Guardian1, unique_name=:1.22612
incidents at start: a(sssssss) 0

=== provider-health fault first (upower masked) ===   [23:19:19 UTC]
  [23:20:19] a(sssssss) 2
    "guardian.correlation.incident-000000" "ingress-5" "" "open"
      "capability upower.display-device debounced transition to Unavailable (direct provider report)"
      "unknown" "upower.display-device"
    "guardian.correlation.incident-000001" "ingress-5" "" "open"
      "capability upower.battery-presence debounced transition to Unavailable (direct provider report)"
      "unknown" "upower.battery-presence"

=== now a real PSI Critical crossing in the SAME instance ===   [23:20:19 UTC]
  avg10 at thaw=64.25  [23:20:31 UTC]

=== FINAL: incidents from BOTH producers  [23:20:35 UTC] ===
a(sssssss) 3
  "guardian.correlation.incident-000002" "ingress-5" "" "open"
    "PSI critical pressure for /proc/pressure/cpu (direct kernel observation)"
    "confirmed" "/proc/pressure/cpu"
  "guardian.correlation.incident-000000" "ingress-5" "" "open"
    "capability upower.display-device debounced transition to Unavailable (direct provider report)"
    "unknown" "upower.display-device"
  "guardian.correlation.incident-000001" "ingress-5" "" "open"
    "capability upower.battery-presence debounced transition to Unavailable (direct provider report)"
    "unknown" "upower.battery-presence"
```

**The argument.** `opened_at` is `format!("ingress-{ingress_sequence}")`
where `ingress_sequence` comes from the one `IngressClock` a
`CorrelationIngress` was minted by (`correlation.rs:720-738`,
`769`). A fresh `IngressClock` starts at sequence 0 (`P2-EVT-004`).

- In the PSI-only runs (§F.5), where no provider-health transition had
  occurred first, the PSI incident opened at **`ingress-1`**.
- In this combined run, where the provider-health producer had already
  driven the shared counter forward, the PSI incident opened at
  **`ingress-5`**, and it shares that counter's numbering with the
  provider-health incidents that preceded it.

If the PSI producer held its own `IngressClock`, its ordinal would be
insensitive to how much provider-health work happened first — it would
have been the same low number in both scenarios. It was not. The PSI
producer's ordinals are drawn from the same counter the other producers
advance, which is the observable signature of the single shared
`admit_event` admission point (R13).

The three incidents also demonstrate `CorrelationEngine`'s single
in-memory store serving both producers through one `Incidents1` surface,
with no new D-Bus member, interface, or bus name (`P2-API-002` untouched).

### F.7 Correlation executes — and a material finding about when it can

Correlation demonstrably executes against real PSI events: the incident
above was opened by Gate 2a's unmodified engine from a real crossing.

However, producing that crossing required an intervention, and the
reason is a substantive finding rather than a testing inconvenience. It
is written up in full in **§F.15 (Finding 1)**. In summary: a naturally
ramping CPU pressure trajectory — the ordinary case — produces PSI
`Event`s that reach the shared ingress but can **never** carry
`Risk::High`, so no incident opens. The incident above was obtained by
freezing the unit's cgroup with `systemctl freeze` while pressure
climbed, then thawing it, so that the worker's next observation was
separated from its previous one by more than one trigger window. That is
the delayed-observation condition a severely overloaded machine
produces; no code, unit, or configuration was modified to obtain it.

The unperturbed control run is recorded here because its negative result
is itself the evidence for the finding:

```text
=== load run: workers=24 secs=70 scope CPUWeight=10000 daemon MainPID=760945  [23:03:32 UTC] ===
--- guardian-daemon.service CPUWeight (unchanged, default) ---
CPUWeight=[not set]
--- incidents before --- a(sssssss) 0
23:03:34 cpu:some avg10=11.05 | incidents=a(sssssss) 0
23:03:40 cpu:some avg10=50.71 | incidents=a(sssssss) 0
23:04:00 cpu:some avg10=92.51 | incidents=a(sssssss) 0
23:04:20 cpu:some avg10=98.31 | incidents=a(sssssss) 0
23:04:41 cpu:some avg10=98.97 | incidents=a(sssssss) 0
--- incidents after ---   [23:04:47 UTC]
a(sssssss) 0
```

Seventy seconds of sustained CPU pressure peaking at **98.97 %** — far
above the 50 % Critical threshold — produced **zero** incidents.

### `P2-COR-002` (second same-resource crossing links) — NOT demonstrated on the VM

Four instrumented attempts were made to produce two Critical crossings
for `/proc/pressure/cpu` inside the engine's 30 s `psi_window`. None
succeeded, and the `strace`-captured observation sequences show exactly
why. Attempt #2's sequence, for example:

```text
### OBSERVATION SEQUENCE the daemon actually read  (link2.strace)
764032 17:10:59.277416 "some avg10=51.68     -> Critical   (previous = seed Nominal 0.10) => CROSSING, incident opens
764032 17:10:59.606039 "some avg10=57.17     -> Critical   (previous Critical)            => no event
764032 17:11:13.623905 "some avg10=25.88     -> Elevated   (previous Critical)            => no event (both >= Elevated)
764032 17:11:15.669961 "some avg10=36.23     -> Elevated                                  => no event
764032 17:11:24.407511 "some avg10=61.89     -> Critical   (previous Elevated)            => no event (both >= Elevated)
764032 17:11:25.654141 "some avg10=57.56     -> Critical                                  => no event
```

A second Critical *event* requires the worker to first observe a reading
below the Elevated threshold (`ThresholdMonitor::observe` emits only when
`was_above != is_above`). Reaching `avg10 < 20` from a Critical reading
requires ≥ 9 s of decay, but a wake only occurs while stall is
happening, and by the time the wake is delivered `avg10` has already
climbed back over 20. Attempt #3 landed at `avg10=20.94` — 0.94 above
the threshold. The timing budget needed (decay below ~6, a wake within
~1 s, then ≥ 5 s of delayed observation) does not fit inside the 30 s
`psi_window`.

This is reported as **not proven on the VM**. It is covered by
`guardian-daemon.rs::tests::a_psi_critical_crossing_opens_an_incident_and_the_next_one_links_to_it`
(implementation pass, §4 above) and by Gate 2a's own `P2-COR-002`
contract tests, and the failure to reproduce it live is a consequence of
the same root cause as Finding 1, not an independent problem.

---

## F.8–F.10 Sandbox containment while the descriptors are working

### Probe method

Two probes were used, and the difference between them matters:

1. **`nsenter` into the live daemon's namespaces.** Faithful for the
   procfs *mount* options (`subset=pid`), but **not** faithful for
   `hidepid=invisible`: the probe process retained a root supplementary
   group and a full capability bounding set, so it could see `/proc/1/*`
   where the daemon cannot. This is recorded as a **probe artifact**, not
   a sandbox weakness — see the second probe.
2. **A transient systemd unit carrying the identical sandbox
   directives**, run as `guardiand` by PID 1 exactly as the real service
   is. This reproduces the daemon's credentials and namespaces
   faithfully, and is the result relied on below. A matching control
   unit — same user, no sandbox directives — establishes the contrast.

The probe script (`/usr/local/bin/guardian-procfs-probe`) is a VM-only
file created by this pass; no repository file was added or changed.

### F.8 `/proc/pressure` is unavailable by pathname, while descriptors work

Sandboxed run, `[2026-09-07 22:54:04 UTC]`, executed while the live
daemon held three working PSI descriptors (§F.3 read live text through
them at 22:55:21):

```text
probe identity: uid=115(guardiand) gid=118(guardiand) groups=118(guardiand)
CapPrm: 0000000000000000
CapEff: 0000000000000000
CapBnd: 0000000000000000
CapAmb: 0000000000000000
NoNewPrivs: 1
===== ITEM 8: /proc/pressure by pathname =====
/proc/pressure           ENOENT/denied
/proc/pressure/cpu       ENOENT/denied
/proc/pressure/memory    ENOENT/denied
/proc/pressure/io        ENOENT/denied
```

Corroborated inside the live unit's own namespaces (`nsenter -m -p`,
22:52:48), where the error text is visible verbatim:

```text
/proc/pressure             DENIED rc=1 : cat: /proc/pressure: No such file or directory
/proc/pressure/cpu         DENIED rc=1 : cat: /proc/pressure/cpu: No such file or directory
/proc/pressure/memory      DENIED rc=1 : cat: /proc/pressure/memory: No such file or directory
/proc/pressure/io          DENIED rc=1 : cat: /proc/pressure/io: No such file or directory
```

Unsandboxed control, **same user**, `[2026-09-07 22:54:16 UTC]`:

```text
probe identity: uid=115(guardiand) gid=118(guardiand) groups=118(guardiand)
CapBnd: 000001ffffffffff      NoNewPrivs: 0
===== ITEM 8: /proc/pressure by pathname =====
/proc/pressure           ENOENT/denied         <- cat(1) on a directory; see note
/proc/pressure/cpu       READABLE <-- FAILURE
/proc/pressure/memory    READABLE <-- FAILURE
/proc/pressure/io        READABLE <-- FAILURE
```

The `guardiand` user can read all three pressure files perfectly well
when the sandbox is not applied. Under the production sandbox all three
are `ENOENT` — while the daemon is concurrently reading them through
inherited descriptors. That is the whole point of the design, now
demonstrated on the real unit.

*(Note: the `/proc/pressure` directory line reads `ENOENT/denied` in the
control too, because the probe uses `cat` and `cat` cannot read a
directory. The three file probes are the meaningful ones.)*

### F.9 Unrelated procfs stays hidden exactly as before; `/proc/self/*` visible

Sandboxed vs. control, same probe, same user:

```text
                               SANDBOXED      CONTROL (no sandbox)
/proc/sys/kernel/hostname      HIDDEN         VISIBLE
/proc/meminfo                  HIDDEN         VISIBLE
/proc/uptime                   HIDDEN         VISIBLE
/proc/net                      HIDDEN         HIDDEN
/proc/1/status                 HIDDEN         VISIBLE
/proc/1/cmdline                HIDDEN         VISIBLE
/proc/cpuinfo                  HIDDEN         VISIBLE
/proc/stat                     HIDDEN         VISIBLE
/proc/loadavg                  HIDDEN         VISIBLE
/proc/kallsyms                 HIDDEN         VISIBLE

===== control: /proc/self/* =====   (sandboxed run)
/proc/self/status        VISIBLE
/proc/self/cmdline       VISIBLE
/proc/self/mountinfo     VISIBLE
/proc/self/stat          VISIBLE
/proc/self/fd            VISIBLE
```

Every unrelated procfs surface named in the TDD's item 9 is hidden, and
`/proc/self/*` remains fully visible — which is exactly why the
`/proc/self/fd/N` reopen strategy works at all under `ProcSubset=pid`.

`/proc/net` is hidden in both columns because `PrivateNetwork=yes`
already removes it independently of `ProcSubset=`.

### F.10 The daemon remains unprivileged with no new capabilities

Read from the live daemon's own `/proc/<pid>/status`,
`[2026-09-07 22:52:14 UTC]`:

```text
$ sudo grep -E '^(Name|Pid|Uid|Gid|Groups|CapInh|CapPrm|CapEff|CapBnd|CapAmb|NoNewPrivs|Seccomp)' /proc/755643/status
Name:   guardian-daemon
Pid:    755643
Uid:    115     115     115     115
Gid:    118     118     118     118
Groups: 118
CapInh: 0000000000000000
CapPrm: 0000000000000000
CapEff: 0000000000000000
CapBnd: 0000000000000000
CapAmb: 0000000000000000
NoNewPrivs:     1
Seccomp:        2
Seccomp_filters:        31

$ id guardiand
uid=115(guardiand) gid=118(guardiand) groups=118(guardiand)
```

All five capability masks are `0000000000000000`, the process runs as
the `guardiand` UID, `NoNewPrivs` is set, and seccomp is in filter mode
with 31 filters loaded. Adding `OpenFile=` granted the daemon no
capability of any kind — descriptors are passed in by PID 1, which is
precisely why no privilege was needed.

---

## F.11 Restart obtains fresh descriptors and re-registers successfully

A real `systemctl restart` of the real unit, `[2026-09-07 23:03:02 UTC]`:

```text
PID before restart: 755643
lrwx------ 1 guardiand guardiand 64 Sep  7 16:50 /proc/755643/fd/3 -> /proc/pressure/cpu
lrwx------ 1 guardiand guardiand 64 Sep  7 16:51 /proc/755643/fd/7 -> /proc/pressure/cpu

$ sudo systemctl restart guardian-daemon
PID after restart:  760945

Sep 07 17:03:02.636636 systemd[1]: Stopping guardian-daemon.service...
Sep 07 17:03:02.664499 systemd[1]: guardian-daemon.service: Deactivated successfully.
Sep 07 17:03:02.667373 systemd[1]: Started guardian-daemon.service - Guardian production daemon.
Sep 07 17:03:02.702830 guardian-daemon[760945]: [guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
Sep 07 17:03:02.703878 guardian-daemon[760945]: [guardian-daemon] serving io.github.cliffthelin.Guardian1 at /io/github/cliffthelin/Guardian1, unique_name=:1.22296

$ sudo cat /proc/760945/environ | tr '\0' '\n' | grep '^LISTEN_'
LISTEN_PID=760945
LISTEN_PIDFDID=763059
LISTEN_FDS=3
LISTEN_FDNAMES=psi-cpu:psi-memory:psi-io
```

`LISTEN_PID` tracks the new MainPID, and all three triggers registered
again (`3/3 ... (0 failed)`). Kernel-side confirmation on the new
instance, `[2026-09-07 23:03:27 UTC]`:

```text
PSI descriptors held by the live daemon: [(3, '/proc/pressure/cpu'), (4, '/proc/pressure/memory'),
 (5, '/proc/pressure/io'), (6, '/proc/pressure/cpu'), (8, '/proc/pressure/memory'), (9, '/proc/pressure/io')]
=== daemon fd 6 -> /proc/pressure/cpu  [daemon's own /proc/self/fd reopen] ===
  TRIGGER WRITE: FAILED errno=16 (EBUSY) => a PSI trigger ALREADY EXISTS on this open file description.
=== daemon fd 8 -> /proc/pressure/memory ... EBUSY
=== daemon fd 9 -> /proc/pressure/io ... EBUSY
```

**Why this proves freshness.** Triggers are per-open-file-description
and cannot be re-armed in place. If the restarted daemon had somehow
been handed the *previous* instance's descriptions, its own registration
writes would have failed `EBUSY` and the journal would have read
`0/3 ... (3 failed)`. It read `3/3 ... (0 failed)`, and the kernel now
reports live triggers on the new descriptions. Note also that the
daemon's reopen landed on **fd 6** this time rather than fd 7, so the
implementation's name-based, position-independent resolution is doing
real work rather than relying on stable numbering.

No PSI state carried over: each restart re-seeds its `ThresholdMonitor`
from a fresh live reading and the correlation store starts empty
(`incidents at start: a(sssssss) 0` after every restart in this pass).

---

## F.12 PSI failure degrades PSI observability only

Three fault cases were induced on the real unit using **temporary
systemd drop-ins** under `/etc/systemd/system/guardian-daemon.service.d/`.
These are VM-only files created and removed by this pass; the packaged
unit was never edited, and §F.12.4 shows the unit restored and verified
by digest afterwards.

### F.12.1 Case A — zero PSI descriptors (`OpenFile=` list reset)

Drop-in: `[Service]\nOpenFile=\n` (an empty assignment resets the list).

```text
[2026-09-07 23:16:11 UTC]
  MainPID=765859 Active=active/running
  OpenFile as parsed: (empty)

Sep 07 17:16:12.018021 guardian-daemon[765859]: [guardian-daemon] PSI ingress: 0/0 inherited descriptors monitored (0 failed)
Sep 07 17:16:12.018021 guardian-daemon[765859]: [guardian-daemon] no PSI descriptors inherited; PSI is reported unavailable (never 'no pressure') and every other producer is unaffected
Sep 07 17:16:12.018245 guardian-daemon[765859]: [guardian-daemon] monitoring tick: recorder len=1 dropped=0 policy=Normal free_space=Sufficient (FC-2 not closed: no spill sink wired)
Sep 07 17:16:12.019287 guardian-daemon[765859]: [guardian-daemon] serving io.github.cliffthelin.Guardian1 at /io/github/cliffthelin/Guardian1, unique_name=:1.22489
Sep 07 17:16:12.025449 guardian-daemon[765859]: [guardian-daemon] capability registry tick: 8/11 capabilities available

  --- LISTEN_* ---            (none: systemd passed no descriptors)
  --- fds pointing at /proc/pressure ---   (none)
  --- D-Bus still served? ---   PID=765859   UniqueName=:1.22489
  --- Capabilities1 still answering? ---
a(sssssbbssss) 11 "systemd.unit.state" "guardian.g8.systemd" "" "available" "healthy" true false ...
     "psi.pressure.cpu" "guardian.g8.psi" "" "unsupported" "unknown" true false ...
  --- Incidents1 still answering? ---  a(sssssss) 0
```

The daemon **starts and runs normally with zero PSI descriptors**. It
reports PSI unavailable in its own words — "never 'no pressure'" — and
every other producer keeps working: the monitoring tick runs, the
capability registry tick runs (`8/11 capabilities available`), the
well-known bus name is owned, and both `Capabilities1` and `Incidents1`
answer live queries. **R16 and item 12 satisfied.**

### F.12.2 Case B — `:graceful` partial descriptor set

Drop-in replacing the middle path with one that does not exist:

```text
OpenFile=
OpenFile=/proc/pressure/cpu:psi-cpu:graceful
OpenFile=/proc/pressure/does-not-exist:psi-memory:graceful
OpenFile=/proc/pressure/io:psi-io:graceful
```

```text
[2026-09-07 23:16:17 UTC]
  MainPID=766065 Active=active/running
  OpenFile as parsed: /proc/pressure/cpu:psi-cpu:graceful
                      /proc/pressure/does-not-exist:psi-memory:graceful
                      /proc/pressure/io:psi-io:graceful

Sep 07 17:16:17.323450 guardian-daemon[766065]: [guardian-daemon] PSI ingress: 2/2 inherited descriptors monitored (0 failed)
Sep 07 17:16:17.324964 guardian-daemon[766065]: [guardian-daemon] serving io.github.cliffthelin.Guardian1 ... unique_name=:1.22500

  --- LISTEN_* (note the renumbering :graceful causes) ---
LISTEN_PID=766065
LISTEN_FDS=2
LISTEN_FDNAMES=psi-cpu:psi-io

  --- fds pointing at /proc/pressure ---
lrwx------ 1 guardiand guardiand 64 Sep  7 17:16 3 -> /proc/pressure/cpu
lrwx------ 1 guardiand guardiand 64 Sep  7 17:16 4 -> /proc/pressure/io
lrwx------ 1 guardiand guardiand 64 Sep  7 17:16 5 -> /proc/pressure/cpu
lrwx------ 1 guardiand guardiand 64 Sep  7 17:16 6 -> /proc/pressure/io

  --- D-Bus / Incidents1 ---  PID=766065  UniqueName=:1.22500   a(sssssss) 0
```

**This is the `:graceful` renumbering hazard, live.** `psi-memory` was
dropped and `psi-io` **moved from index 2 to index 1** — so an
index-based resolver would have bound `io`'s descriptor to `memory`.
The daemon bound both present resources correctly (fds 5 and 6 are its
reopens of cpu and io respectively), started 2/2 monitors, reported the
absent resource as absent, and kept serving D-Bus. **R2's partial-set
requirement confirmed on the real unit.**

### F.12.3 Case C — a missing path *without* `:graceful` aborts before `ExecStart`

```text
Drop-in: OpenFile=/proc/pressure/does-not-exist:psi-cpu     (no :graceful)

[2026-09-07 23:16:22 UTC]
  Result=exit-code   ExecMainStatus=202   Active=failed
Sep 07 17:16:23.693105 systemd[1]: guardian-daemon.service: Failed with result 'exit-code'.
Sep 07 17:16:23.937266 systemd[1]: guardian-daemon.service: Scheduled restart job, restart counter is at 5.
Sep 07 17:16:23.937413 systemd[1]: guardian-daemon.service: Start request repeated too quickly.
Sep 07 17:16:23.937460 systemd[1]: Failed to start guardian-daemon.service - Guardian production daemon.
```

`status=202/FDS`, the unit never reaches `ExecStart`, and `Restart=on-failure`
then exhausts its rate limit. This confirms on the real unit why every
`OpenFile=` line **must** carry `:graceful`: without it a PSI-less
kernel or container would take the whole daemon down. R16's rationale is
now evidence rather than citation.

### F.12.4 Restoration verified

```text
[2026-09-07 23:16:49 UTC]
$ sudo rm -f /etc/systemd/system/guardian-daemon.service.d/zz-phasef-*.conf
$ sudo systemctl daemon-reload && sudo systemctl reset-failed guardian-daemon && sudo systemctl start guardian-daemon

Active=active MainPID=766622
Sep 07 17:16:45.497933 guardian-daemon[766622]: [guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
Sep 07 17:16:45.498986 guardian-daemon[766622]: [guardian-daemon] serving io.github.cliffthelin.Guardian1 ... unique_name=:1.22520

$ systemctl cat guardian-daemon | head -1
# /usr/lib/systemd/system/guardian-daemon.service
$ ls -A /etc/systemd/system/guardian-daemon.service.d
no drop-in directory
$ sha256sum /usr/lib/systemd/system/guardian-daemon.service
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  /usr/lib/systemd/system/guardian-daemon.service
```

Back to exactly the packaged production unit, 3/3 monitored.

---

## F.13 The provider-health / UPower path remains functional (Gate 2c regression)

Gate 2c's accepted scenario was **re-run as a regression check** against
the PSI-enabled unit. No Gate 2c file was read, modified, or
re-derived — only the scenario was repeated.

```text
=== preconditions ===   [2026-09-07 23:17:13 UTC]
guardian-daemon: active MainPID=766622
[guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
upower.service: active / disabled
incidents at start: a(sssssss) 0
upower capability now: "upower.display-device" "guardian.g8.upower" "" "available" "healthy"

=== STEP 1: mask + stop upower.service ===   [23:17:13 UTC]
Created symlink '/etc/systemd/system/upower.service' → '/dev/null'.
upower.service: inactive
  23:17:19 upower_cap="upower.display-device" "guardian.g8.upower" "" "degraded" "error"
  ... (debounce dwell) ...
  23:17:49 incidents=a(sssssss) 2
      "guardian.correlation.incident-000000" "ingress-5" "" "open"
        "capability upower.display-device debounced transition to Unavailable (direct provider report)"
        "unknown" "upower.display-device"
      "guardian.correlation.incident-000001" "ingress-5" "" "open"
        "capability upower.battery-presence debounced transition to Unavailable (direct provider report)"
        "unknown" "upower.battery-presence"

=== STEP 2: unmask + start upower.service ===   [23:17:49 UTC]
Removed '/etc/systemd/system/upower.service'.
upower.service: active
  23:18:19 upower_cap="upower.display-device" "guardian.g8.upower" "" "available" "healthy"
  ... (recovery dwell) ...
  23:18:49 incidents=a(sssssss) 2
      "guardian.correlation.incident-000000" "ingress-5" "ingress-5" "closed" ...
      "guardian.correlation.incident-000001" "ingress-5" "ingress-5" "closed" ...

=== FINAL ===   [23:18:49 UTC]
upower.service: active / disabled
guardian-daemon still: active
PSI: [guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
```

The full Gate 2c cycle works unchanged on the PSI-enabled unit:
capability health degrades to `degraded`/`error`, two real incidents
**open** after the debounce dwell, recovery returns the capability to
`available`/`healthy`, and both incidents **close** — all observed
through `Incidents1.ListIncidents()` on the real system bus, with the
PSI producer running concurrently at 3/3. `P2-VM-001`/`P2-VM-002`
behaviour is not regressed by this gate.

---

## F.14 What this pass could NOT prove

Stated plainly, with the reason for each.

1. **`P2-COR-002` link behaviour on the real VM.** Four instrumented
   attempts, all recorded in §F.7. The `strace` observation sequences
   show why it could not be produced: the required "Critical crossing →
   observed sub-Elevated reading → second Critical crossing" sequence
   does not fit inside the engine's 30 s `psi_window` given the kernel's
   10 s `avg10` time constant. Covered by unit tests, not by VM evidence.
2. **A field-by-field dump of a live PSI `Event`.** The daemon emits no
   per-event log line on admission and this pass added no instrumentation
   to production code. The `Event`'s shape is established at both
   endpoints instead — the raw PSI bytes the daemon read (§F.4) and the
   incident those bytes produced (§F.5) — plus the unmodified
   `classify()` predicate. See the honest scope note in §F.5.
3. **Real trigger registration observed at the moment of `ExecStart`.**
   `strace` cannot be attached before PID 1 execs the daemon without
   wrapping `ExecStart`, which would no longer be the production unit.
   Registration is instead proven after the fact, from the kernel, via
   the `pidfd_getfd` `EBUSY` probe (§F.3) and by the `POLLPRI` wakes
   themselves (§F.4) — an unregistered PSI descriptor never reports
   `POLLPRI`.
4. **A naturally-occurring PSI incident.** See Finding 1 below; the
   incident in §F.5 required a `systemctl freeze`/`thaw` delayed-observation
   intervention. The unperturbed 70-second control run produced none.
5. **`memory` and `io` crossings.** Only `cpu` pressure was exercised.
   Memory-pressure stress was judged unsafe on a 5.4 GB VM within the
   task's "bounded, safe, do not destabilise" constraint. Registration
   and live reads through the `memory` and `io` descriptors *are* proven
   (§F.2, §F.3, §F.11); their wake-to-`Event` path is not.

---

## F.15 Findings

### Finding 1 (blocking) — the production PSI configuration cannot open an incident under a real pressure trajectory

**Observed.** Seventy seconds of sustained CPU pressure peaking at
`avg10=98.97` produced **zero** incidents (§F.7). The incident in §F.5
was obtained only by suspending the unit's cgroup so the worker's next
observation was separated from its previous one by more than one trigger
window.

**Mechanism.** Three accepted facts compose into an unreachable state:

1. `CorrelationEngine::classify()` (`correlation.rs:493-507`, Gate 2a,
   forbidden scope) opens a PSI incident only when
   `event.severity == Risk::High`, i.e. only for `PressureSeverity::Critical`.
2. `ThresholdMonitor::observe()` (`psi.rs:296-311`) emits a crossing only
   when `was_above != is_above` against `event_threshold`, which
   `PSI_MONITOR_CONFIG` sets to `Elevated`. An `Elevated → Critical`
   transition therefore emits **nothing** — both are above the threshold.
3. Consequently a `Risk::High` event requires a **single-step
   `Nominal → Critical` observation**: `avg10 < 20` at one wake and
   `avg10 >= 50` at the next.

**Why that step is unreachable.** `avg10` is a 10-second EWMA, and the
kernel delivers at most one trigger notification per window
(`trigger_window_us = 2_000_000`). During sustained stall, consecutive
observations are therefore ~2 s apart, and over 2 s of *continuous 100 %*
stall `avg10` can rise only from `x` to `x + (100 - x)(1 - e^{-0.2})`:

```
maximum reachable from just under the Nominal ceiling:
    x = 20  ->  20 + 80 x 0.1813 = 34.5      (< 50)
```

**34.5 is the ceiling**, so the required step is not merely unlikely — it
is arithmetically impossible for any pressure trajectory, at any load,
with a 2-second window. The observed sequences confirm it exactly:
`17.75 → 32.47 → 44.52 → 54.57` (§F.4) and
`51.68 → 57.17 → 25.88 → 36.23 → 61.89` (§F.7). The daemon always passes
*through* `Elevated`, where the crossing is consumed and no further event
is emitted.

The same arithmetic makes `P2-COR-002`'s second crossing unreachable, so
Finding 1 also explains §F.14 item 1.

**Why the unit tests do not catch it.** They write fixture files whose
contents jump instantaneously from `PSI_NOMINAL` (`avg10=0.10`) to
`PSI_CRITICAL` (`avg10=81.00`). A file rewritten in place can make that
jump; a kernel EWMA cannot.

**Scale of the parameter gap.** With the kernel's maximum window
(`10_000_000` µs), the factor becomes `1 - e^{-1} = 0.632`, and from
`x = 19` one step reaches `19 + 81 x 0.632 = 70.2` — comfortably
Critical. The reachability boundary is therefore in the window
parameter, which this gate owns
(`PSI_MONITOR_CONFIG`, `guardian-daemon.rs:306-312`), not in Gate 2a's
`classify()` rule.

**Disposition.** This requires a production code change — to
`trigger_window_us`, to `event_threshold`, or to how a sustained-Critical
state is surfaced. Per the assigning task's instruction, **this pass
stopped and reported rather than fixing it.** No production Rust, test,
or unit file was modified. Candidate remedies are deliberately not
implemented or chosen here; they belong to a repair pass with its own
review.

### Finding 2 (non-blocking) — `Capabilities1` reports PSI `unsupported` while PSI ingress is working

With all three descriptors working and 3/3 monitors running, the
capability registry still reports the PSI capabilities as unavailable:

```text
[2026-09-07 23:16:45 UTC, 3/3 descriptors monitored]
"psi.pressure.cpu"    "guardian.g8.psi" "" "unsupported" "unknown" true false ... "kernel_interface"
"psi.pressure.memory" "guardian.g8.psi" "" "unsupported" "unknown" true false ... "kernel_interface"
"psi.pressure.io"     "guardian.g8.psi" "" "unsupported" "unknown" true false ... "kernel_interface"
```

The registry probes `/proc/pressure` **by pathname**, which the sandbox
denies (§F.8), so `unsupported` is truthful from that probe's own
vantage point and is **not** a fail-open. It is nonetheless an
observability inconsistency: the daemon can simultaneously report PSI as
unsupported on `Capabilities1` and open a PSI incident on `Incidents1`.

This is **pre-existing capability-registry behaviour, outside this
gate's scope** (the registry probe is not in `allowed_scope`), and it is
recorded here for routing to `GUARDIAN_HARDENING_BACKLOG.md` per
execution-protocol step 10 — not as a blocker and not as scope creep on
this gate.

---

## F.16 Validation, on the VM, against the synced tree

Run by this pass on `guardian-g9` against `/home/ubuntu/phasef`, the
checksum-verified tree of §F.0:

```text
$ cargo fmt --check
fmt exit=0

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Checking guardian-testkit / guardian-provider-api / guardian-client / guardian-core
    Checking guardian-cli / guardian-gui / guardian-indicator / guardian-daemon
    Checking guardian-helper / guardian-tui
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.10s
clippy exit=0

$ cargo test --workspace
=== AGGREGATE ===
passed=454 failed=0 ignored=3
```

**454 passed, 0 failed, 3 ignored** — matching the implementation pass's
reported post-change state exactly (baseline before this gate was
409/0/3). All three commands were run and observed by this pass; none of
these totals is carried from an earlier pass.

### Repository changes made by this pass

This pass modified exactly one repository file — this evidence document.
No production Rust, no test, and no `debian/guardian-daemon.service`
edit was made; no Gate 2c file was read or touched; nothing was
committed, pushed, or tagged, on host or VM.

### Phase F checklist status

| # | Item | Status |
|---|---|---|
| 1 | Sandbox still active and unchanged; `systemd-analyze security` parity | **PROVEN** (§F.1) |
| 2 | Exact descriptors supplied; PID 1 namespace provenance | **PROVEN** (§F.2) |
| 3 | Real kernel trigger registration through an inherited descriptor | **PROVEN** (§F.3) |
| 4 | Real `poll(POLLPRI)` wake under bounded load | **PROVEN** (§F.4) |
| 5 | Daemon constructs a real PSI Guardian `Event` | **PROVEN at both endpoints** (§F.5; scope note there) |
| 6 | That `Event` reaches the shared Phase 2 ingress | **PROVEN** (§F.6) |
| 7 | Correlation executes; incident via `ListIncidents()` | **PROVEN, but only under a delayed observation** (§F.5, §F.7, Finding 1) |
| 8 | `/proc/pressure` unavailable by pathname while descriptors work | **PROVEN** (§F.8) |
| 9 | Unrelated procfs hidden; `/proc/self/*` visible | **PROVEN** (§F.9) |
| 10 | Unprivileged, no new capabilities | **PROVEN** (§F.10) |
| 11 | Restart obtains fresh descriptors; re-registration succeeds | **PROVEN** (§F.11) |
| 12 | PSI failure degrades PSI observability only | **PROVEN** (§F.12) |
| 13 | Provider-health / UPower path still functional | **PROVEN** (§F.13) |
| + | Real `EBUSY` at registration (deferred item) | **PROVEN** (§F.3) |
| + | `:graceful` partial-descriptor case (deferred item) | **PROVEN** (§F.12.2) |
| + | `P2-COR-002` link on the real VM | **NOT PROVEN** (§F.7, §F.14) |

**Overall: `P2-VM-003`'s evidence chain is complete and the mechanism
works end to end on the real systemd sandbox, but Finding 1 is a
blocking defect in this gate's own `PSI_MONITOR_CONFIG` and must be
resolved by a separate repair pass before the gate can be accepted.**

---

# Phase F re-run — Critical-threshold repair (2026-09-08)

Everything above this line (§F.0–§F.16, Findings 1–2) is the **original**
Phase F pass, preserved **untouched** per this pass's supersede-don't-erase
instruction. Nothing above was edited. This section is additive and
covers only the items whose truth depends on the changed
`PSI_MONITOR_CONFIG.event_threshold`: the natural-trajectory incident-open
proof (Finding 1's own repro target), its recovery counterpart, and a
fresh `P2-COR-002` reproduction attempt. Items 1, 2, 3, 8, 9, 10, 12, 13
(sandbox, descriptor provenance, trigger registration, `EBUSY`, restart,
`:graceful`, Gate 2c/UPower regression) are **not** re-derived here
because their truth does not depend on `event_threshold`; §G.6 below only
spot-checks that they still hold.

## G.0 The repair under test

`crates/guardian-daemon/src/bin/guardian-daemon.rs`, `PSI_MONITOR_CONFIG`
(originally lines 306–312, now with an explanatory comment block): `event_threshold`
changed from `PressureSeverity::Elevated` to `PressureSeverity::Critical`.
`trigger_threshold_us` (100 000) and `trigger_window_us` (2 000 000)
**unchanged**, per the assigning task's binding decision not to widen the
trigger window as the primary repair. No other file's production code
changed. Two pre-existing unit tests
(`a_real_crossing_produces_the_accepted_psi_event_shape`,
`psi_severity_is_derived_only_from_the_bytes_the_daemon_read_itself`)
were updated to exercise a crossing shape that is still reachable under
the new threshold (a bare `Nominal -> Elevated` reading is no longer a
reportable crossing at all); this is a necessary consequence of the
config change, not a weakened assertion.

Two new regression tests were added to
`crates/guardian-daemon/src/bin/guardian-daemon.rs::tests` before the fix,
confirmed genuinely RED against the unmodified baseline, then GREEN after:
`gradual_ewma_trajectory_crosses_into_critical_only_at_the_real_boundary_and_opens_incident`
and
`gradual_ewma_recovery_crossing_critical_to_elevated_remains_observable_without_disturbing_the_open_incident`.
Both build their fixture trajectories from a real kernel PSI decayed-average
helper (`kernel_psi_ewma_step`, `x_new = x_old + (target_pct - x_old) *
(1 - e^(-elapsed/tau))`, the same formula Finding 1's arithmetic-ceiling
analysis used) sampled at `PSI_MONITOR_CONFIG.trigger_window_us`'s own
2-second cadence (tied to it by a compile-time `const` assertion), never
an instantaneous fixture jump — see §2 below for the RED output and the
derived values.

## G.1 Method (same rigor as §F.0, condensed)

Host tree (HEAD `4d5df972a4fde6ae788f13e0aac68c38e295a411` plus this pass's
changes) tarred with `target/`/`.git/` excluded, transferred to
`guardian-g9` via `multipass transfer`, extracted to
`/home/ubuntu/phasef-repair-tree`. Eight key files (the same set §F.0
checksummed, now re-hashed) verified `sha256sum -c` **OK** on the VM
before any build step:

```text
crates/guardian-daemon/src/psi_ingress.rs: OK
crates/guardian-daemon/src/bin/guardian-daemon.rs: OK
crates/guardian-core/src/providers/psi.rs: OK
debian/guardian-daemon.service: OK
crates/guardian-daemon/src/lib.rs: OK
crates/guardian-daemon/tests/phase2_psi_ingress_contract.rs: OK
crates/guardian-core/src/correlation.rs: OK
crates/guardian-core/src/psi.rs: OK
```

Of these eight, only `guardian-daemon.rs`'s digest differs from §F.0's
recorded value (`8efd772c...` vs the original pass's `286ab370...`) — the
other seven, including `correlation.rs` (forbidden scope) and
`debian/guardian-daemon.service` (additive-only scope, untouched this
pass), are byte-identical to the original Phase F pass's tree.

Built with the real packaged toolchain, never `cargo run`:

```text
$ DEB_BUILD_OPTIONS=nocheck dpkg-buildpackage -us -uc -b -d
dpkg-deb: building package 'guardian' in '../guardian_0.1.0-1_amd64.deb'.
$ sudo apt-get install -y ./guardian_0.1.0-1_amd64.deb
Setting up guardian (0.1.0-1) ...
$ sha256sum /usr/lib/systemd/system/guardian-daemon.service
f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb  (unchanged from §F.0/accepted G9 baseline)
$ sha256sum /usr/bin/guardian-daemon
5a4afb2b3d39f62eaee9851fc4a94b86d27ddba2e97d7623d7f31cf9d5477051  (new binary -- different from §F.0's, as expected: PSI_MONITOR_CONFIG changed)
$ sudo systemctl restart guardian-daemon.service
$ systemctl status guardian-daemon.service
Active: active (running) ... Main PID: 813590 (guardian-daemon)
guardian-daemon[813590]: [guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
```

This single daemon instance (PID 813590) ran continuously through every
experiment below.

## G.2 RED evidence for the two new regression tests (pre-fix, host-run)

Against the unmodified baseline (`event_threshold: PressureSeverity::Elevated`),
run before any production edit:

```text
$ cargo test -p guardian-daemon --bin guardian-daemon gradual_ewma
running 2 tests
test tests::gradual_ewma_trajectory_crosses_into_critical_only_at_the_real_boundary_and_opens_incident ... FAILED
test tests::gradual_ewma_recovery_crossing_critical_to_elevated_remains_observable_without_disturbing_the_open_incident ... FAILED

---- tests::gradual_ewma_trajectory_crosses_into_critical_only_at_the_real_boundary_and_opens_incident stdout ----
thread '...' panicked at crates/guardian-daemon/src/bin/guardian-daemon.rs:1438:14:
a gradual Elevated -> Critical crossing must still produce an event

---- tests::gradual_ewma_recovery_crossing_critical_to_elevated_remains_observable_without_disturbing_the_open_incident stdout ----
thread '...' panicked at crates/guardian-daemon/src/bin/guardian-daemon.rs:1514:14:
the Critical -> Elevated recovery boundary must remain observable

test result: FAILED. 0 passed; 2 failed; 0 ignored
```

Both fail for exactly the mechanism Finding 1 describes: with
`event_threshold = Elevated`, `ThresholdMonitor::observe` reports no
crossing when both the previous and current reading are already
`>= Elevated`, so an `Elevated -> Critical` step (test 1) and a
`Critical -> Elevated` step (test 2) both classify as "no crossing" and
`dispatch_wake` returns `None`, panicking the `.expect(...)` calls that
require `Some`. Neither failure is a compile error, a panic in test
setup, or an assertion of the wrong property — both are the documented
defect, reproduced deterministically.

**Trajectory values, derived from the real kernel EWMA formula, not
hand-picked:** starting `avg10 = 15.0` (Nominal) under continuous 100%
stall, sampled at the production `trigger_window_us` cadence (2 s),
`τ = 10 s` (the real `avg10` time constant):

| Step | Formula | avg10 | Severity (thresholds 20/50) |
|---|---|---|---|
| 0 (start) | — | 15.00 | Nominal |
| 1 | `15 + (100-15)(1-e^-0.2)` | 30.41 | Elevated |
| 2 | `30.41 + (100-30.41)(1-e^-0.2)` | 43.02 | Elevated (higher) |
| 3 | `43.02 + (100-43.02)(1-e^-0.2)` | 53.35 | **Critical** |
| 4 (recovery, target 0%) | `53.35 + (0-53.35)(1-e^-0.2)` | 43.68 | Elevated |

These are exactly the values the test's `PsiAverages::step` helper
computes at runtime (not transcribed by hand into the fixture text) and
match, in shape and magnitude, the real VM's own naturally observed
climb in §G.3 below (e.g. `12.50 -> 28.17 -> 41.00 -> 51.51`, §G.5's
strace) — confirming the synthetic unit-test trajectory and the real
kernel's actual behaviour agree.

## G.3 Natural-trajectory incident-open proof (real VM, no freeze/thaw)

Bounded, ordinary CPU load (`stress-ng --cpu 12` on a 4-core VM; no
cgroup freeze, no sampling interruption, no artificial intervention of
any kind), monitored every 2s via a real `busctl` call to
`Incidents1.ListIncidents()` over the real system bus, from a completely
idle baseline (`incidents: 0`):

```text
[03:52:47] some avg10=0.00  || incidents: a(sssssss) 0
[03:52:49] some avg10=8.33  || incidents: a(sssssss) 0
[03:52:51] some avg10=24.75 || incidents: a(sssssss) 0
[03:52:53] some avg10=38.20 || incidents: a(sssssss) 0
[03:52:55] some avg10=48.85 || incidents: a(sssssss) 0
[03:52:57] some avg10=57.58 || incidents: a(sssssss) 1 "guardian.correlation.incident-000000" "ingress-3" "" "open" \
    "PSI critical pressure for /proc/pressure/cpu (direct kernel observation)" "confirmed" "/proc/pressure/cpu"
```

The incident opens the moment `avg10` first reads `>= 50` (the last
pre-crossing sample, `48.85`, is below the Critical threshold; the very
next sample is above it and the incident already exists) — **this is
Finding 1's exact reproduction scenario from §F.7 (which produced zero
incidents over 70s at up to `avg10=98.97`), now succeeding on the first
ordinary rising trajectory attempted, with no freeze/thaw and no
manipulation of the monitoring loop.** This directly answers Finding 1:
the production PSI configuration can now open an incident under a real,
unperturbed pressure trajectory.

Timestamps are real wall-clock (`date -u`), UTC; the daemon was already
running (PID 813590, started 2026-09-07 21:51:53 MDT) for this and every
experiment below.

## G.4 Natural recovery and window-elapsed closure (real VM)

**What "recovery closes the incident" means for this engine, precisely.**
`CorrelationEngine::admit_psi` (`correlation.rs`, forbidden scope,
unmodified) only ever runs when `classify()` sees a `Risk::High` event —
a non-Critical reading is `Ignored` and touches no incident state at all
(documented in `classify()`'s own comment: "closure is driven by the
window elapsing, not by an explicit recovery reading"). So a PSI
incident does not close purely because `avg10` decays; it closes
**lazily**, the next time a *new* Critical crossing for the same resource
arrives more than `psi_window` (30s default) after the incident's last
Critical event. This pass reproduced exactly that mechanism, four
independent times, entirely through natural load management (stopping
and restarting `stress-ng`, never freezing a cgroup or otherwise
interrupting the monitoring loop):

| Natural closure # | Opened | Closed (next crossing, elapsed) | New incident |
|---|---|---|---|
| 1 | `incident-000000` @ 03:52:57 | @ 03:53:48 (~51s) | `incident-000001` |
| 2 | `incident-000001` @ 03:53:48 | @ 03:54:23 (~35s) | `incident-000002` |
| 3 | `incident-000002` @ 03:54:23 | @ 03:55:24 (~61s) | `incident-000003` |
| 4 | `incident-000006` @ 04:00:47 | @ 04:06:20 (~333s) | `incident-000007` |

Representative raw sample (closure #2, real natural decay then real
natural re-climb, `stress-ng` stopped and restarted, never frozen):

```text
[03:53:46] some avg10=46.72 || incidents: 1 open (incident-000000, still "open")
[03:53:48] some avg10=56.19 || incidents: 2 -- incident-000001 "open", incident-000000 "closed"
```

Each closed incident's wire record carries the documented backreference
reason internally (`correlation.rs:873`, `"closed: correlation window
elapsed with no further Critical event"`) — not itself on the
`IncidentWire` tuple (frozen shape, `P2-API-003`), but confirmed by code
reading against the unmodified, forbidden-scope `admit_psi` this pass
consumes as-is.

**Recovery is directly observable at the raw-pressure and per-event
level** even though it does not itself close an incident: e.g. closure
#1's decay was captured naturally after `stress-ng`'s bounded timeout
elapsed, with no restart or intervention:

```text
[03:53:13] some avg10=84.00
[03:53:15] some avg10=68.78
[03:53:17] some avg10=56.32
[03:53:19] some avg10=46.12   (incident-000000 still "open" -- correctly unaffected, per Gate 2a's own rule)
```

## G.5 `P2-COR-002` reproduction — a real within-window link (real VM)

The original Phase F pass could not reproduce this at all: §F.7/§F.14
showed the required "Critical → observed sub-Elevated → Critical again"
*shape* was arithmetically impossible within the 30s `psi_window` under
the old `Elevated`-threshold config (recall Finding 1: from `x=20`, a real
2s step reaches at most `34.5`). With `event_threshold = Critical`, the
shape needed is far less extreme — only a dip **below 50**, not all the
way below 20 — so this pass attempted it again.

**Method.** `IncidentWire` deliberately carries no event/link count
(frozen 7-field shape, `P2-API-003`), so a successful *link*
(`AdmitOutcome::IncidentUpdated`) is invisible on `ListIncidents()` alone
— it looks identical to "nothing happened". To get unambiguous ground
truth, `strace` was attached to the single, specifically-identified PSI
CPU worker thread (TID 813596, found via its `ppoll([{fd=6,
events=POLLPRI}])` signature) while a real load sequence ran: full load
(`stress-ng --cpu 12`, 10s) → auto-expiring into mild load (`--cpu 5`,
8s, chosen so *some* stall keeps the kernel trigger firing during the
dip — a full stop produces zero new stall and the trigger simply never
fires, so the daemon never observes the trough at all, which is what
undermined this pass's first two attempts) → auto-expiring back to full
load (`--cpu 12`, 10s). No `pkill`/manual kill during the traced window.

**The daemon's own classification sequence, read directly from its own
`read()` syscalls on its own inherited descriptor** (local VM time,
MDT = UTC−6, continuous with §G.1–G.4's UTC timestamps):

```text
22:06:14.426951  avg10=12.13  Nominal
22:06:16.471945  avg10=27.87  Elevated
22:06:18.518067  avg10=40.75  Elevated
22:06:20.566066  avg10=51.30  CRITICAL   <- crossing #1 (incident-000007 opens, elapsed since
                                             incident-000006's last Critical: ~333s > 30s window,
                                             so this is itself closure #4 from §G.4)
22:06:22.614039  avg10=59.94  Critical   (no new event: same severity as previous)
22:06:24.662049  avg10=56.87  Critical
22:06:26.710022  avg10=51.10  Critical
22:06:28.758035  avg10=46.37  ELEVATED   <- real down-crossing, observed directly by the daemon
22:06:30.806064  avg10=42.50  Elevated
22:06:32.857854  avg10=51.46  CRITICAL   <- crossing #2, elapsed since crossing #1 = 12s <= 30s window
22:06:34.903863  avg10=60.26  Critical   (no new event)
22:06:36.950853  avg10=66.91  Critical
22:06:38.997889  avg10=72.72  Critical
22:06:41.045961  avg10=77.12  Critical
```

This is unambiguous, direct ground truth (not inferred from D-Bus): the
daemon's own `ThresholdMonitor` genuinely observed a `Critical ->
Elevated -> Critical` cycle, with the second Critical crossing occurring
**12 seconds** after the first — well inside the engine's 30s
`psi_window`. Per `admit_psi`'s own (forbidden-scope, unmodified) logic,
`elapsed <= psi_window` **must** produce `AdmitOutcome::IncidentUpdated`
(link), not a new incident.

**Confirmed on the real system bus** — `ListIncidents()` before this
sequence started and again after `avg10` had fully decayed back to
baseline (`2.11`, several minutes later):

```text
$ busctl call ... ListIncidents      # immediately after crossing #2 completed the climb
a(sssssss) 8 "guardian.correlation.incident-000007" "ingress-47" "" "open" ...    <- same id, same opened_at
$ grep '^some' /proc/pressure/cpu
some avg10=2.11 ...                   # full natural recovery, no intervention
$ busctl call ... ListIncidents      # unchanged: still 8 total, incident-000007 still open, same id
a(sssssss) 8 "guardian.correlation.incident-000007" "ingress-47" "" "open" ...
```

Total incident count stayed at 8 across both crossings (it was 7 before
crossing #1's reopen, 8 after) — no ninth incident ever appeared, and
`incident-000007`'s `incident_id`/`opened_at` never changed. Combined
with the strace ground truth proving a real second Critical crossing
genuinely occurred 12s later, this rules out "no event at all" as the
explanation for the unchanged count: **`P2-COR-002`'s within-window link
behaviour is reproduced on the real VM for the first time**, via a
completely natural load sequence (no freeze/thaw, no cgroup
manipulation, no interruption of the monitoring loop — only ordinary
`stress-ng` start/stop, which is exactly the kind of oscillating real
pressure a busy machine produces).

## G.6 Spot-check: unaffected items still hold

Not re-derived in full (their truth does not depend on `event_threshold`)
but spot-checked on this pass's build to confirm no incidental
regression:

```text
$ diff <(sha256sum crates/.../guardian-daemon.rs) <(§F.0's recorded hash)
  differs (expected -- production config changed)
$ sha256sum /usr/lib/systemd/system/guardian-daemon.service
  f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb   (identical to §F.0/accepted G9)
$ systemd-analyze security guardian-daemon.service
  Overall exposure level for guardian-daemon.service: 0.6 SAFE       (identical to §F.1's score)
$ grep Cap /proc/813590/status
  CapPrm/CapEff/CapBnd/CapAmb: 0000000000000000                      (unchanged, unprivileged)
$ busctl call ... Capabilities1 ListCapabilities
  8 non-PSI capabilities: all "available"/"healthy" (systemd, logind, upower x2, accounts x2, udisks x2)
  3 PSI capabilities: "unsupported" (pre-existing Finding 2, unrelated cosmetic gap, unchanged)
```

Sandbox, descriptor provenance, capability set, and the Gate 2c/UPower
provider-health path are all consistent with §F.1/§F.8–F.10/§F.13's
original findings; nothing regressed.

## G.7 Validation, on the VM, against the synced (repaired) tree

```text
$ cargo fmt --check
exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.63s
exit 0
$ cargo test --workspace
=== AGGREGATE ===
passed=456 failed=0 ignored=3
```

**456 passed, 0 failed, 3 ignored** — exactly the pre-repair baseline
(454/0/3, §F.16) plus the two new regression tests, with no other count
change. All three commands were run and observed directly by this pass.

## G.8 Repository changes made by this pass

- `crates/guardian-daemon/src/bin/guardian-daemon.rs`: `PSI_MONITOR_CONFIG.event_threshold`
  changed `Elevated` -> `Critical` (with an explanatory comment); two new
  regression tests added; two pre-existing tests updated to exercise a
  still-reachable crossing shape (documented in each test's own comment).
- `docs/evidence/p2/PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md`:
  this section appended; nothing above it edited.

No other file changed. `correlation.rs`, `correlation_contract.rs`, all
`phase2-2c-*` files, `GATE2C_VM_EVIDENCE.md`, `psi_ingress.rs`'s
descriptor-acquisition logic, `debian/guardian-daemon.service`,
`guardian-helper/**`, `restart_capability.rs`, the client crates,
`IncidentWire`'s shape, and `Incident::link_event`'s signature are all
untouched — confirmed by `git status`/`git diff --stat` on the host
(§8 of this pass's report) and by the unchanged digests in §G.1/§G.6.

## G.9 Updated Phase F checklist status (supersedes only rows 7 and the `P2-COR-002` row)

| # | Item | Status |
|---|---|---|
| 7 | Correlation executes; incident via `ListIncidents()` | **PROVEN under an ordinary, unperturbed natural trajectory** (§G.3) — supersedes §F.7/§F.16's "only under a delayed observation" |
| + | Natural recovery / window-elapsed closure | **PROVEN**, reproduced 4 times naturally (§G.4) |
| + | `P2-COR-002` link on the real VM | **PROVEN**, real within-window link, strace ground truth (§G.5) — supersedes §F.16's "NOT PROVEN" |

All other rows (1–6, 8–13, and the `EBUSY`/`:graceful` deferred items)
remain as recorded in §F.16, unaffected by this repair.

**Overall: Finding 1 is resolved. The production PSI configuration now
opens an incident under an ordinary, unperturbed real pressure
trajectory, correctly links a second same-window Critical crossing
instead of duplicating the incident, and correctly performs its
documented lazy window-elapsed closure — all reproduced on the real
packaged systemd unit, with no freeze/thaw, no cgroup manipulation, and
no other artificial interruption of the monitoring loop.**

---

# Phase H — Acceptance repair, five owner-adjudicated blockers (2026-09-08)

Everything above this line (§F, §G, Findings 1–2) is preserved
**untouched**, per supersede-don't-erase. This section is additive and
covers the five specific acceptance blockers the project owner raised
after reading two independent whole-repair audits (both `PASS WITH
NON-BLOCKING FINDINGS`): (1) inherited-descriptor FD ownership/lifetime
(`FD_CLOEXEC` + no unintentional raw-fd liveness), (2) one owned open
file description across register/poll/reread (gate TDD R7), (3) truthful
per-resource PSI availability, (4) Capability Registry truthfulness, and
(5) direct live-`Event` evidence for `P2-VM-003`. Finding 2 (§F.15) is
resolved by Blocker 4 below. Finding 1 remains resolved as of §G (not
reopened here).

## H.0 Governance basis

The owner's dated 2026-09-08 governance act (recorded identically in
`TDD_CONTRACT.md` §51's revision history, `GUARDIAN_PHASE2_IMPLEMENTATION_
HANDOFF.md` §19's fifth revision note, the gate manifest, the gate TDD's
"Normative ID decision" section, and `ADR-009`'s revision history)
accepts `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`/`P2-VM-003` as minted and
demotes `P2-EVT-006` to acceptance criteria under those three IDs, content
preserved verbatim. This section's evidence is organized by blocker, not
by ID, since that is how the task was adjudicated; the ID-level mapping
is recorded in the four inventories above.

## H.1 Method (same rigor as §F.0/§G.1, condensed)

Host tree (HEAD `4d5df972a4fde6ae788f13e0aac68c38e295a411` plus this
pass's changes) tarred (`target/`, `.git/` excluded) and transferred via
`multipass transfer` to `guardian-g9` three times over this pass (initial
implementation sync, an evidence-instrumentation sync, and a final
post-revert sync), each verified `sha256sum` **byte-identical** to the
host before any build step. `crates/guardian-core/src/correlation.rs`
checksum (`928e462c31d779b06f02932acb233e8d4a2abf9740920a11fe6d9208d613c24e`)
and `debian/guardian-daemon.service` checksum
(`f1e5fab9a2547841bbe598cc876bb4c9a7d5d7ae4f98f348bcf6035ce3e5eaeb`) are
**identical** to the values recorded in §F.0/§G.1 above — both files are
genuinely untouched by this pass. Every build used the real packaged
toolchain (`dpkg-buildpackage` → `.deb` → `apt-get install`), never
`cargo run`. Disk space on `guardian-g9` was reclaimed before this pass
by deleting three prior audit passes' `target/` build-cache directories
only (never their source trees or evidence logs) — 91% → 46% used.

## H.2 Blocker 1 — inherited-descriptor FD ownership and lifetime

**The defect.** Before this pass, `PsiResourceMonitor::register`
(`crates/guardian-daemon/src/psi_ingress.rs`) built a `PsiFileSource`
from the `/proc/self/fd/N` path and let the dispatcher's baseline read,
the trigger registration, and **every subsequent `dispatch_wake` call**
each independently call `open()` on that same magic-symlink path. Each
such `open()` is a fresh syscall producing a genuinely distinct open
file description (verified empirically, not assumed — see H.3). The raw
systemd-supplied descriptor itself was consequently held open for the
daemon's entire lifetime without `FD_CLOEXEC` ever being set on it, and
was never the descriptor actually read from after the first reopen.

**The fix.** `guardian_core::providers::psi::OwnedPsiFile::open` (new
type, `crates/guardian-core/src/providers/psi.rs`) reopens
`/proc/self/fd/N` **exactly once** per resource, producing one owned
`std::fs::File`, and immediately sets `FD_CLOEXEC` on it explicitly via
`rustix::io::fcntl_getfd`/`fcntl_setfd` over the `BorrowedFd`
`std::os::fd::AsFd::as_fd` safely exposes — no `unsafe` anywhere;
`unsafe_code = "forbid"` (`Cargo.toml:30`) is unchanged, confirmed by
`cargo clippy --workspace --all-targets --all-features -- -D warnings`
passing on both host and VM (§H.6). `PsiResourceMonitor::register`
(`psi_ingress.rs`) now calls `OwnedPsiFile::open` once and never again
references the raw fd number or the `/proc/self/fd/N` path.

**FD_CLOEXEC proof — real, running production daemon, `/proc/<pid>/
fdinfo/N`, not a unit-test fixture:**

```text
$ PID=$(systemctl show -p MainPID --value guardian-daemon.service); echo $PID
920840
$ for fd in $(sudo ls /proc/$PID/fd/); do
    t=$(sudo readlink /proc/$PID/fd/$fd)
    case "$t" in *pressure*) echo "fd $fd -> $t"; sudo cat /proc/$PID/fdinfo/$fd | grep flags;; esac
  done
fd 3 -> /proc/pressure/cpu
flags:	0100002
fd 4 -> /proc/pressure/memory
flags:	0100002
fd 5 -> /proc/pressure/io
flags:	0100002
fd 6 -> /proc/pressure/cpu
flags:	02100002
fd 8 -> /proc/pressure/memory
flags:	02100002
fd 9 -> /proc/pressure/io
flags:	02100002
```

`0100002` = `O_RDWR` (no `O_CLOEXEC` = `02000000` octal). `02100002` =
`0100002 | 02000000` — the arithmetic difference is exactly the
`O_CLOEXEC`/`FD_CLOEXEC` bit. fd 3/4/5 are the **raw**
systemd-inherited descriptors (unchanged, non-`CLOEXEC`, exactly as
documented and expected — see "what is deliberately not closed" below);
fd 6/8/9 are the **owned** descriptors `OwnedPsiFile::open` produced —
**exactly one per resource**, confirming the single-reopen design (had
the pre-repair reopen-per-read behavior still been present, `dispatch_wake`
firing repeatedly under the `stress-ng` load in §H.4 below would have
accumulated additional fd numbers over the run; it did not — the fd set
was stable at 3/4/5/6/8/9 for the entire daemon lifetime). `pos:` on fd
6/8/9 was observed non-zero and changing across reads (`OwnedPsiFile`
seeks to 0 before every read, so `pos` lands at that read's content
length), confirming the same fd is genuinely reused for repeated reads,
not reopened.

**What is deliberately not closed, and why that is safe.** The raw
inherited descriptors (fd 3/4/5 above) remain open and non-`CLOEXEC` for
the daemon's lifetime. Rust cannot safely adopt or close an arbitrary raw
fd number without `unsafe` (`OwnedFd::from_raw_fd`/`BorrowedFd::
borrow_raw` are both `unsafe fn`), and the workspace forbids `unsafe`.
This carries no security consequence here because `guardian-daemon` never
execs — no subprocess is spawned anywhere in this binary (unchanged from
the accepted architecture) — so there is no exec boundary across which a
non-`CLOEXEC` fd could leak. The substantive property Blocker 1 asked
for — "the resulting **owned** descriptor must be close-on-exec" — is
proven above; "raw inherited anchors must not remain unintentionally
live" is satisfied by construction: after the single reopen, the raw fd
number is never read from, written to, or referenced again anywhere in
`psi_ingress.rs` or `providers/psi.rs` — it is inert, not "unintentionally"
anything, and this is now documented explicitly in `OwnedPsiFile`'s own
doc comment (`providers/psi.rs`).

**Unit tests** (all passing, host + VM, §H.6):
`an_owned_psi_file_is_close_on_exec`,
`an_owned_psi_file_open_on_a_missing_path_fails_closed`,
`the_production_monitor_registers_trigger_and_dispatch_through_one_owned_descriptor`
(asserts `monitor.is_close_on_exec()` on the actual production
`PsiResourceMonitor` type, not a standalone fixture).

## H.3 Blocker 2 — one owned open file description across register/poll/reread

**R7's current wording** (gate TDD, "Phase B" — acceptance criteria under
`P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008` as of the 2026-09-08 demotion):
"the same descriptor that carries the trigger also serves
`PsiEventDispatcher::dispatch_wake`'s pressure-text re-read." **No
wording change was needed.** This design (one owned File, opened once,
used for register+poll+reread) is achievable exactly as R7's text already
states, and is what this pass implements — verified, not assumed, per the
task's own instruction to test empirically rather than trust the magic
symlink's reopen semantics.

**Empirical verification that `/proc/self/fd/N` reopens are genuinely
distinct open file descriptions** (this is the fact the pre-repair code's
defect rested on, and R7's intended safety property depends on avoiding):
a new regression test,
`independent_reopens_of_the_same_path_are_different_open_file_descriptions`
(`crates/guardian-core/src/providers/psi.rs`), and — using the real
kernel PSI ABI's own per-open-file-description trigger scoping as an
independent oracle, per the task's own suggested mechanism —
`real_kernel_psi_triggers_are_scoped_per_open_file_description_not_per_inode`
(`#[ignore]`d by default; requires the **opening** process to hold real
PSI-trigger privilege — see below):

```text
$ sudo target/debug/deps/guardian_core-1b2376ee17457978 \
    providers::psi::tests::real_kernel_psi_triggers --ignored --nocapture --test-threads=1
running 1 test
test providers::psi::tests::real_kernel_psi_triggers_are_scoped_per_open_file_description_not_per_inode ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 77 filtered out; finished in 0.01s
```

This test proves, against the **real** kernel PSI ABI on `guardian-g9`
(not a fixture): (a) two independent `OwnedPsiFile::open` calls against
the identical real `/proc/pressure/cpu` path both succeed at trigger
registration **independently** — impossible if they shared one open file
description, since a second trigger on the same description is rejected;
(b) a second trigger registration attempt against the **same**
already-triggered open file description fails, `raw_os_error() ==
Some(16)` (`EBUSY`) — the real kernel enforcement this repair's identity
mechanism (`OwnedPsiFile::same_open_file_description`,
`PsiTrigger::shares_open_file_description_with`,
`PsiEventDispatcher::reads_through_owned_file`, all `Arc::ptr_eq`-based)
is proven to track a genuine kernel-observable property, not merely an
in-process bookkeeping artifact.

**Why this test is privilege-gated (a real, verified finding, not a
guess).** Registering a trigger on a descriptor requires the descriptor's
**opener** to hold PSI-trigger privilege — verified directly by
comparing an unprivileged open+write (fails `EINVAL`) against a
`sudo`-privileged open+write (succeeds) against the identical real
`/proc/pressure/cpu`, on both the authoring host and `guardian-g9`:

```text
$ python3 -c "import os; fd=os.open('/proc/pressure/cpu', os.O_RDWR); os.write(fd, b'some 10000 1000000\0')"
OSError: [Errno 22] Invalid argument
$ sudo python3 -c "import os; fd=os.open('/proc/pressure/cpu', os.O_RDWR); os.write(fd, b'some 10000 1000000\0')"
(no error)
```

World-writable (`0666`) permissions on `/proc/pressure/cpu` are
**necessary but not sufficient**. This is not a defect in this repair —
it is the exact kernel property the accepted `OpenFile=` architecture
(ADR-009) already depends on and exploits: systemd's `OpenFile=` opens
the path as **PID 1** (privileged), and the kernel's later trigger-write
check is against the descriptor's **opener's** credentials, not the
current writer's — which is precisely why the unprivileged `guardiand`
process (confirmed zero capabilities, §H.4) can register a real trigger
through a descriptor it could never have opened itself. §H.4 below
confirms this happening for real in the actual production daemon.

**End-to-end production proof** (not just the library primitives in
isolation):
`the_production_monitor_registers_trigger_and_dispatch_through_one_owned_descriptor`
(`crates/guardian-daemon/src/bin/guardian-daemon.rs`) asserts
`monitor.shares_one_open_file_description()` — true only when the
monitor's `PsiTrigger` and `PsiEventDispatcher` both read through the
exact same `Arc<std::fs::File>` as the single `OwnedPsiFile` `register`
constructed. Would have failed to compile against the pre-repair
`PsiResourceMonitor` (these methods did not exist before this pass) and
would have failed at runtime against a hypothetical implementation that
still performed independent reopens.

## H.4 Blocker 3 — truthful per-resource PSI availability

**Before this pass**, the only startup signal was one aggregate line:
`"[guardian-daemon] PSI ingress: {started}/{attempts} inherited
descriptors monitored ({failed} failed)"` — computed from `attempts`
(only resources that *were* in the descriptor plan), so a resource with
**no** descriptor at all (silently dropped by `:graceful`) never appeared
in the denominator and was invisible: `"2/2 monitored"` looked identical
to a fully-healthy 3-resource daemon.

**After this pass**, `guardian_core::providers::psi::{PsiAvailability,
PsiResourceStatus}` (new types) track each of cpu/memory/io individually
(`Available` / `Degraded(reason)` / `NoDescriptor`), populated at startup
by `psi_ingress::availability_from_startup` and updated at runtime by
`psi_ingress_loop` on either of its two termination paths (idle-timeout,
hard failure) — so a *later* fault is also reflected, not only the
startup snapshot. Real startup output, per-resource, from the running
production daemon:

```text
$ sudo journalctl -u guardian-daemon.service | grep "PSI psi-"
[guardian-daemon] PSI psi-cpu: available (monitor registered and running)
[guardian-daemon] PSI psi-memory: available (monitor registered and running)
[guardian-daemon] PSI psi-io: available (monitor registered and running)
[guardian-daemon] PSI ingress: 3/3 inherited descriptors monitored (0 failed)
```

The aggregate line is kept **in addition to**, never **instead of**, the
per-resource lines. New unit/integration tests exercise the partial
case directly (`availability_from_startup` given a plan where one
resource has no descriptor reports that resource `NoDescriptor` while its
siblings report `Available`), consistent with the existing
`a_partial_graceful_descriptor_set_leaves_the_absent_resource_unavailable`
regression test already in the suite.

**Non-interference confirmed unchanged**: `capability_registry_tick`,
`Capabilities1`, and provider-health-driven `Incidents1` entries are
unaffected by a PSI failure — proven by the existing, still-passing
`a_psi_registration_failure_degrades_psi_observability_only` test and,
live, by the `11/11 capabilities available` / `8/11` lines the daemon
logs every 30s regardless of PSI state throughout this pass's VM session.

## H.5 Blocker 4 — Capability Registry truthfulness

**The defect (this pass's own re-confirmation of §F.15 Finding 2).**
`registry::psi_capabilities()` (`crates/guardian-core/src/providers/
registry.rs:173-197` in the pre-repair candidate) unconditionally
constructed `PsiFileSource::real()` and read `/proc/pressure` **by
pathname** — which `ProcSubset=pid` denies unconditionally from inside
`guardian-daemon`, so this function could never observe a live inherited
descriptor and always reported PSI `Unsupported`, even while a real,
working, kernel-triggered monitor ran on the same descriptor
`psi_ingress` held. Directly reproduced by this pass before the fix
(same `guardian-g9` daemon, 3/3 monitors already running):

```text
$ busctl call ... Capabilities1 ListCapabilities | grep psi
"psi.pressure.cpu"    ... "unsupported" "unknown" ...
"psi.pressure.memory" ... "unsupported" "unknown" ...
"psi.pressure.io"     ... "unsupported" "unknown" ...
```

**The fix.** `psi_capabilities` now takes `Option<&PsiAvailability>` —
the **same shared state** Blocker 3 maintains, wired through
`main()` → `spawn_psi_ingress` (returns `Arc<Mutex<PsiAvailability>>`) →
(`Arc::clone`) → `capability_registry_tick` → `populate_registry`. No new
public API and no new D-Bus surface: the wiring is entirely internal
production plumbing, one narrow `Arc<Mutex<_>>`. The registry **never**
opens `/proc/pressure` itself when live state is available; the original
pathname probe (`psi_capabilities_by_pathname`) is preserved unmodified
as the `None` fallback for callers with no live daemon state (e.g. this
module's own pre-existing standalone unit test).

**Classification rule** (preserves the registry's existing
Available/Healthy, Unavailable/Degraded/Error, Unsupported/Unknown
distinctions, applied to PSI's new live signal): `Available` →
`Available`/`Healthy`; `Degraded(reason)` → `Degraded`/`Error` (a real
fault on a resource a descriptor genuinely exists for); `NoDescriptor`/
unobserved → `Unavailable`/`Error` **if** a sibling resource in the same
snapshot is `Available` (positive proof PSI is kernel-supported here, so
this resource's own absence is a truthful gap, not an unsupported
kernel) — otherwise `Unsupported`/`Unknown` (zero live signal anywhere is
structurally indistinguishable from genuine kernel absence).

**Live, real proof — same VM, same daemon, after the fix, 3/3 monitors
running:**

```text
$ busctl call io.github.cliffthelin.Guardian1 /io/github/cliffthelin/Guardian1/Capabilities \
    io.github.cliffthelin.Guardian.Capabilities1 ListCapabilities | tr '(' '\n' | grep -i psi
"psi.pressure.cpu"    "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T06:56:07Z"
"psi.pressure.memory" "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T06:56:07Z"
"psi.pressure.io"     "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T06:56:07Z"
```

Directly closes §F.15 Finding 2: a live inherited PSI descriptor is now
reported `available`/`healthy` on `Capabilities1`, matching what
`Incidents1` simultaneously shows (§H.7 below).

**New deterministic tests** (`crates/guardian-core/src/providers/
registry.rs`, all passing host + VM): `inherited_cpu_psi_available_is_
never_reported_unsupported`; `a_missing_graceful_descriptor_alongside_a_
working_sibling_is_truthfully_unavailable`;
`genuinely_absent_kernel_psi_support_is_reported_unsupported`;
`partial_availability_across_resources_is_represented_individually`
(cpu `Available`, memory `Degraded`, io `Unavailable` — all three
distinct in one snapshot). The pre-existing
`psi_capabilities_never_marks_a_genuinely_absent_resource_as_a_hard_error`
test is updated only to pass `None` (the unchanged pathname-probe
fallback), not reworded in substance.

## H.6 Validation (host and VM, both full workspace)

Host (this pass's own runs, `cargo 1.98.0`):

```text
$ cargo fmt --check
exit 0
$ cargo clippy --workspace --exclude guardian-gui --all-targets --all-features -- -D warnings
exit 0
$ cargo test --workspace --exclude guardian-gui
passed=463 failed=0 ignored=4
```

(`guardian-gui` excluded on the host only: the authoring host lacks
`libadwaita-1-dev`, exactly as the gate manifest's own note anticipates.)

`guardian-g9` (full workspace, including `guardian-gui`), run against the
**final**, post-instrumentation-revert tree, checksum-verified identical
to the host first:

```text
$ cargo fmt --check
exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.82s
exit 0
$ cargo test --workspace
passed=467 failed=0 ignored=4
```

**467 passed, 0 failed, 4 ignored** on the VM (full workspace) — the
baseline this pass started from was **456 passed, 0 failed, 3 ignored**
(per the assigning task); the new-test delta is net positive and the one
additional ignored test is the real-kernel `EBUSY`/identity proof
(`real_kernel_psi_triggers_are_scoped_per_open_file_description_not_per_inode`,
§H.3), `#[ignore]`d by default for the same reason this workspace already
ignores other real-kernel/real-bus tests, and run explicitly (passing)
under `sudo` in §H.3 above. No test required by any prior gate was
deleted, weakened, or newly `#[ignore]`d to obtain these totals.

## H.7 Blocker 5 — direct live `Event` evidence for `P2-VM-003`

**Why the prior methodology was insufficient.** §F.14 item 2 recorded:
"The daemon emits no per-event log line on admission and this pass added
no instrumentation to production code. The `Event`'s shape is established
at both endpoints instead" (raw PSI bytes in, incident out) — inference,
not direct observation of the `Event` itself.

**Bounded, temporary, evidence-only instrumentation.** One `eprintln!`
call was added to `dispatch_psi_wake`
(`crates/guardian-daemon/src/bin/guardian-daemon.rs`), gated behind
`std::env::var_os("GUARDIAN_PSI_EVIDENCE_DEBUG").is_some()` so it could
never fire in normal production operation, printing every field of the
constructed `Event` before it was handed to `admit_event`. The env var
was supplied **only** via a `systemctl edit --runtime`-style drop-in at
`/run/systemd/system/guardian-daemon.service.d/evidence-override.conf`
(`/run`, never `/etc` — gone on reboot even if not explicitly removed),
never by editing `debian/guardian-daemon.service` itself.

**Real production `.deb` built and installed with the instrumentation**,
daemon restarted under the runtime override, real load applied
(`stress-ng --cpu 12`, 40s, the same tool/shape §G.3 used), journal
followed live:

```text
$ sudo journalctl -u guardian-daemon.service -f | grep EVIDENCE-ONLY
[guardian-daemon][EVIDENCE-ONLY][P2-VM-003] PSI Event fields: event_id=EventId("guardian.psi.cpu.event-1") \
  timestamp_monotonic=1 timestamp_wall="sequence-1" source_provider=ProviderId("guardian.g8.psi") \
  event_type="psi_threshold_crossing" resource_refs=["/proc/pressure/cpu"] severity=High \
  normalized_key="psi cpu threshold crossing elevated->critical" \
  raw_reference="PSI cpu threshold crossing Elevated->Critical" \
  attributes={"from": "Elevated", "to": "Critical"}
```

Every field the gate TDD requires is directly present: `event_type ==
"psi_threshold_crossing"`; `resource_refs == ["/proc/pressure/cpu"]`
(the corrected §4.1 identity, preserved); `severity == High` (`Risk::High`,
the `Elevated -> Critical` mapping); a daemon-owned `EventId`
(`guardian.psi.cpu.event-1`); `normalized_key`/`raw_reference` produced by
the existing, unmodified `normalize_key` path; `attributes` recording the
real `from`/`to` transition. `timestamp_monotonic`/`timestamp_wall` are
the accepted, documented provenance-only values (§51's ingress-ordering
model — `ThresholdMonitor`'s own sequence counter, never consulted for
correlation decisions; real ingress order comes from the shared
`IngressClock` at `admit_event`, unchanged by this pass).

**Reaches the shared ingress, not a separate path** — confirmed by tying
this exact captured `Event` to the resulting incident over the real
system bus, `resource_refs` matching exactly:

```text
$ busctl call ... Incidents1 ListIncidents
a(sssssss) 1 "guardian.correlation.incident-000000" "ingress-2" "" "open" \
  "PSI critical pressure for /proc/pressure/cpu (direct kernel observation)" "confirmed" "/proc/pressure/cpu"
```

**Instrumentation removed before this pass's completion, confirmed by
diff, not merely by claim.** `sha256sum` of
`crates/guardian-daemon/src/bin/guardian-daemon.rs` **before** the
instrumentation was added and **after** it was removed are
byte-identical
(`9670a390f0bfd05e5983e768dfcef18cdc075b7f05834994aebf7c419e29174e`),
confirming an exact, residue-free revert — not merely a claim that the
block was deleted. `grep -rn "EVIDENCE-ONLY\|GUARDIAN_PSI_EVIDENCE_DEBUG"`
across `crates/`, `debian/`, `docs/` on the final tree returns zero
matches for this pass's instrumentation (the only hits are unrelated,
pre-existing G6 spike-evidence doc text using the same English phrase for
a different, already-accepted gate). The runtime env-var override was
removed (`rm` under `/run`, `daemon-reload`, `systemctl restart`) and
confirmed absent (`DropInPaths=` empty) before the final `.deb` was even
built. A **second, independent** real-load run
(`stress-ng --cpu 12`, 30s) against the **final, reverted** binary
produced a **new** real incident (`0` → `1`, `incident-000000`) with
**zero** `EVIDENCE-ONLY` lines in the journal for that daemon's PID,
proving the production PSI path still works correctly with the
instrumentation gone, not merely that the grep for it is clean.

## H.8 Regression spot-checks (final, reverted, installed build)

```text
$ diff <(cat /usr/lib/systemd/system/guardian-daemon.service) <(repo debian/guardian-daemon.service)
(no output -- IDENTICAL)
$ sudo systemd-analyze security guardian-daemon.service
Overall exposure level for guardian-daemon.service: 0.6 SAFE   (unchanged from §F.1/§G.6)
$ grep -E '^Cap|^Uid' /proc/<pid>/status
Uid: 115 115 115 115
CapPrm/CapEff/CapBnd/CapAmb: 0000000000000000               (unchanged, unprivileged)
$ sudo nsenter -t <pid> -m -p -- cat /proc/pressure/cpu
cat: /proc/pressure/cpu: No such file or directory           (still unavailable by pathname)
```

All consistent with §F.1/§F.8-F.10/§G.6 — no regression from this
acceptance repair.

## H.9 Repository changes made by this pass

Modified: `crates/guardian-core/src/providers/psi.rs` (`OwnedPsiFile`,
`PsiTrigger::register_on_owned_file`,
`PsiEventDispatcher::new_on_owned_file`, `PsiAvailability`/
`PsiResourceStatus`, new tests); `crates/guardian-core/src/providers/
registry.rs` (`psi_capabilities` signature + classification,
`psi_capabilities_by_pathname` fallback, `populate_registry` signature,
new tests); `crates/guardian-daemon/src/psi_ingress.rs`
(`PsiResourceMonitor` now owns one `OwnedPsiFile`,
`availability_from_startup`); `crates/guardian-daemon/src/bin/
guardian-daemon.rs` (`spawn_psi_ingress`/`psi_ingress_loop`/
`capability_registry_tick` threading `Arc<Mutex<PsiAvailability>>`,
per-resource startup logging, new tests; temporary evidence
instrumentation added and fully removed, confirmed by identical
before/after checksum); `crates/guardian-core/examples/g8_real_evidence.rs`
(one-line `populate_registry` call-site update for the new parameter).
Governance docs updated (see the four-inventory reconciliation recorded
in `TDD_CONTRACT.md` §51, `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md`
§19, the gate manifest, and the gate TDD). `ADR-009` revised in place
with a dated 2026-09-08 entry.

**Untouched, confirmed by checksum**: `crates/guardian-core/src/
correlation.rs`, `crates/guardian-core/tests/correlation_contract.rs`,
all `phase2-2c-*` files, `GATE2C_VM_EVIDENCE.md`,
`debian/guardian-daemon.service`, `crates/guardian-helper/**`,
`crates/guardian-daemon/src/restart_capability.rs`, client/GUI/TUI/CLI/
indicator crates, `IncidentWire`'s shape, `Incident::link_event`'s
signature, `PSI_MONITOR_CONFIG`'s threshold values (Critical-threshold
trajectory from §G, unchanged).

Nothing committed, tagged, or pushed — host or VM — per this task's
commit policy.

## H.10 Updated status

All five acceptance blockers are closed. `P2-EVT-005`/`P2-EVT-007`/
`P2-EVT-008`/`P2-VM-003` are owner-confirmed accepted; `P2-EVT-006` is
owner-confirmed demoted to acceptance criteria under those three IDs,
content preserved. `PHASE 2 PSI ACCEPTANCE REPAIR — READY FOR RE-REVIEW`.

---

# Phase I — Final acceptance repair (2026-09-08, post-re-review)

An independent focused re-review of Phase H returned `FAIL — PSI
CAPABILITY SURFACES STILL CONTRADICT`: it reproduced, live, on the real
installed daemon, `Capabilities1.ListCapabilities: psi.pressure.cpu =
"available"/"healthy"` simultaneously with `Capabilities1.PsiSummary: cpu
... supported=false`. This section closes that one blocking item (Part A)
and revisits the raw inherited-FD `FD_CLOEXEC` disposition on its own
merits, not merely its wording (Part B). Everything in §F–§H is preserved
untouched; nothing in it is redesigned here.

## I.1 Part A — the `PsiSummary`/`ListCapabilities` contradiction

**Root cause.** `dbus_surface.rs::real_psi_summary()` was never touched
by Phase H's Blocker 4 fix — it still called `PsiFileSource::real()`
(the standard-kernel-path constructor), an unconditional `/proc/pressure`
**pathname** read. Under the accepted `ProcSubset=pid` sandbox that
always fails, so `PsiSummary` unconditionally reported
`supported=false` for every resource regardless of how the live
inherited-descriptor monitor was actually doing — a structural
contradiction with `ListCapabilities`, which Blocker 4 had already
repaired to read the shared, governed `PsiAvailability` state.

**The fix.** `Capabilities1` now holds the same two governed production
sources `main()` already threads into `capability_registry_tick`/
`registry::psi_capabilities`: the `Arc<Mutex<PsiAvailability>>`
`spawn_psi_ingress` returns, and a `PsiFileSource` built from the same
`/proc/self/fd/N` inherited-descriptor paths `psi_ingress`'s own
monitors read through (`psi_plan.file_source()` in `main()`). `psi_summary`
never opens `/proc/pressure` by pathname again. `real_psi_summary` is now
a pure, testable function of `(&PsiAvailability, &PsiFileSource)`: a
resource is `available=true` with real numbers only when the governed
state says `Available` **and** a live read through that same descriptor
succeeds this call; every other case (never observed, `Degraded`,
`NoDescriptor`, or a narrow race where the state says `Available` but the
read momentarily fails) reports the wire's existing `(0.0, 0.0, 0.0,
false)` "no measurement" row — the same idiom the wire already used, not
a new representation.

**Wire-shape preservation.** `PsiSummaryWire = (String, f64, f64, f64,
bool)` is byte-for-byte unchanged; `PsiSummary`'s D-Bus member name,
interface, and object path are unchanged (confirmed by
`p2_2b_dbus_contract_suite` in `phase2_2b_contract.rs`, which diffs live
introspection against the recorded G9 baseline and still passes
unmodified). No `P2-API-002` exception is requested or needed.

**New tests** (`crates/guardian-daemon/src/dbus_surface.rs`):
`psi_summary_reports_three_resources_by_name`,
`psi_summary_reports_real_numbers_when_governed_state_says_available`,
`psi_summary_and_list_capabilities_never_disagree_on_the_same_governed_state`
(the direct regression proof: for one shared `PsiAvailability` snapshot,
`registry::psi_capabilities`'s `Availability::Available` classification
and `real_psi_summary`'s `available` boolean are asserted equal for every
resource), `psi_summary_never_reports_available_for_a_resource_with_no_governed_descriptor`,
`psi_summary_is_truthfully_unavailable_when_never_observed`,
`psi_summary_represents_partial_availability_individually`. Plus a
source-text regression guard (`crates/guardian-daemon/tests/
phase2_psi_ingress_contract.rs`): `dbus_surface_never_opens_psi_by_pathname`
— the same technique the existing `the_daemon_never_opens_psi_by_pathname`
guard already used for `guardian-daemon.rs`/`psi_ingress.rs`, extended to
cover `dbus_surface.rs`, which is exactly the file that guard never
covered and exactly where the defect lived.

**Live dual-call reproduction — real installed `.deb`, real systemd
unit, same running daemon (PID 953116), both D-Bus methods issued back to
back over the real system bus:**

```text
$ busctl call io.github.cliffthelin.Guardian1 \
    /io/github/cliffthelin/Guardian1/Capabilities \
    io.github.cliffthelin.Guardian.Capabilities1 ListCapabilities | grep -i psi
"psi.pressure.cpu"    "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T07:41:26Z"
"psi.pressure.memory" "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T07:41:26Z"
"psi.pressure.io"     "guardian.g8.psi" "" "available" "healthy" true false ... "kernel_interface" "2026-09-08T07:41:26Z"

$ busctl call io.github.cliffthelin.Guardian1 \
    /io/github/cliffthelin/Guardian1/Capabilities \
    io.github.cliffthelin.Guardian.Capabilities1 PsiSummary
a(sdddb) 3 "cpu" 0.3 2.88 1.44 true "memory" 0 0 0 true "io" 0.53 0.64 0.59 true
```

Every resource `ListCapabilities` reports `available`/`healthy`,
`PsiSummary` reports `available=true` with real, live numbers (`memory`'s
`0 0 0` is a genuine idle-system reading, backed by `available=true`, not
the wire's `false`-accompanied sentinel) — the exact contradiction the
re-review reproduced is gone.

**Also reproduced in the degraded (`:graceful`-partial) case**, via a
`systemctl edit --runtime` drop-in dropping the `memory` `OpenFile=` line
(never editing `debian/guardian-daemon.service` itself; removed and
`DropInPaths=` confirmed empty before returning to the 3-descriptor
state):

```text
$ busctl call ... ListCapabilities | grep -i psi
"psi.pressure.cpu"    ... "available"   "healthy" ...
"psi.pressure.memory" ... "unavailable" "error"   ...
"psi.pressure.io"     ... "available"   "healthy" ...

$ busctl call ... PsiSummary
a(sdddb) 3 "cpu" 0.06 1.27 1.21 true "memory" 0 0 0 false "io" 0 0.27 0.49 true
```

`memory` is `unavailable`/`error` in `ListCapabilities` and
`available=false` in `PsiSummary` — still consistent, never
contradictory, and the descriptor renumbering `:graceful` performs (`io`
lands on fd 4, not fd 5) resolves correctly through the unchanged
name-based logic.

## I.2 Part B — raw inherited-FD `FD_CLOEXEC`, revisited on canonical semantics

**What was weak about the prior wording.** §H.2 judged the raw,
non-`CLOEXEC` inherited descriptors (fd 3/4/5) safe *because
`guardian-daemon` never execs a child process* — true, but a property of
today's call graph, not a structural guarantee. The re-review asked this
pass to evaluate systemd's own canonical acquisition API
(`sd_listen_fds_with_names()`) rather than settle for that wording alone.

**Dependency evaluation.** Three real candidates were evaluated against
the actual constraint (Guardian's `unsafe_code = "forbid"` means setting
`FD_CLOEXEC` on a bare, externally-supplied raw fd number — which
requires an `unsafe` `BorrowedFd`/`OwnedFd` construction by definition —
can only be done inside a dependency's own, externally-maintained
implementation):

1. **[`libsystemd`](https://crates.io/crates/libsystemd) (`lucab/
   libsystemd-rs`, pinned `0.7.2`) — adopted.** Pure Rust, dual
   `MIT`/`Apache-2.0`, ~9.7M downloads, actively maintained (last
   published 2025-04-30). Its `activation::receive_descriptors` sets
   `FD_CLOEXEC` on every resolved raw descriptor via `nix::fcntl::fcntl`
   — read directly from the pinned upstream source as part of this
   evaluation, not assumed. Does not implement `LISTEN_PIDFDID`
   validation (see "residual gap" below).
2. **[`systemd`](https://crates.io/crates/systemd) (`codyps/rust-systemd`)
   — evaluated, rejected.** Real FFI to the actual `libsystemd.so`, so it
   *does* get genuine `LISTEN_PIDFDID` validation (confirmed by reading
   `sd_listen_fds()`'s real upstream C implementation,
   `src/libsystemd/sd-daemon/sd-daemon.c`), but requires a build-time
   `pkg-config`/`libsystemd-dev` dependency and a runtime dynamic link
   against an `LGPL`-licensed library, pulls in unrelated D-Bus/journal
   FFI surface by default, and binds only unnamed `sd_listen_fds()` (not
   `_with_names`) — this module's own `LISTEN_FDNAMES` resolution would
   still be needed on top, for no gain beyond the one `LISTEN_PIDFDID`
   check. Judged disproportionate for that single check, per `AGENTS.md`'s
   "avoid a large framework for a small capability."
3. **[`sd-listen-fds`](https://crates.io/crates/sd-listen-fds)
   (`Ralith/sd-listen-fds`) — evaluated, rejected.** Read directly from
   source: it `mem::transmute`s the raw fd number into an `OwnedFd` and
   **never calls `fcntl` at all** — it would not close the `CLOEXEC` gap.
   Also stale (last published 2023-08-27).

Full write-up, including the exact source lines read from each candidate,
lives in `crates/guardian-daemon/src/psi_ingress.rs`'s module doc
("Canonical FD acquisition" section) — not duplicated here.

**The fix.** `psi_ingress::acquire_canonical_listen_fds()` calls
`libsystemd::activation::receive_descriptors(false)` from
`psi_descriptor_plan_from_env()`, whenever this module's own
`LISTEN_PID`/`LISTEN_FDS`/`LISTEN_FDNAMES` resolution
(`resolve_psi_descriptors`, entirely **unchanged**) finds a non-empty
descriptor set. This is a second, independent `LISTEN_PID` re-validation
that also sets real `FD_CLOEXEC` on every raw descriptor in `[3,
3+LISTEN_FDS)` as a side effect; a disagreement (canonical acquisition
errors, or reports zero while this module's own parser found some) fails
the whole plan closed to empty. `resolve_psi_descriptors`'s own
name-based resolution, duplicate handling, and `:graceful` handling are
untouched — `libsystemd` is used only for the raw-descriptor `CLOEXEC`
side effect and the independent PID cross-check.

**Residual gap, honestly reported.** `LISTEN_PIDFDID` validation
(systemd 255+/259) is **not** performed by the adopted `libsystemd`
crate, so this pass does not close that specific sub-gap — only a genuine
FFI binding to `libsystemd.so` implements it, and that dependency was
judged disproportionate for this one check alone (candidate 2 above). The
residual exposure is narrow: it requires an attacker who can already
cause this specific unprivileged, sandboxed process to be replaced by
another process reusing the exact same PID number within the same
activation window — `LISTEN_PID` alone (independently validated twice
now) already defeats every case except that specific PID-reuse race.

**Real, live proof — same installed `.deb`, same unit, `/proc/<pid>/
fdinfo/N`, before/after comparison against §H.2's own recorded baseline:**

```text
$ PID=$(systemctl show -p MainPID --value guardian-daemon.service); echo $PID
953116
$ for fd in 3 4 5 6 8 9; do
    t=$(sudo readlink /proc/$PID/fd/$fd)
    echo "fd $fd -> $t"; sudo cat /proc/$PID/fdinfo/$fd | grep flags
  done
fd 3 -> /proc/pressure/cpu     flags:  02100002
fd 4 -> /proc/pressure/memory  flags:  02100002
fd 5 -> /proc/pressure/io      flags:  02100002
fd 6 -> /proc/pressure/cpu     flags:  02100002
fd 8 -> /proc/pressure/memory  flags:  02100002
fd 9 -> /proc/pressure/io      flags:  02100002
```

Every one of `02100002` includes the `O_CLOEXEC`/`FD_CLOEXEC` bit
(`02000000` octal). §H.2's own baseline recorded fd 3/4/5 as `0100002`
(no `O_CLOEXEC`) before this pass; they are `02100002` now. fd 6/8/9
(the owned descriptors `OwnedPsiFile::open` produces) are unchanged from
§H.2, still `CLOEXEC` as before.

**`LISTEN_PIDFDID` confirmed present in the real environment** (systemd
259 on `guardian-g9`, matching this task's framing):

```text
$ sudo cat /proc/953116/environ | tr '\0' '\n' | grep ^LISTEN_
LISTEN_PID=953116
LISTEN_PIDFDID=958923
LISTEN_FDS=3
LISTEN_FDNAMES=psi-cpu:psi-memory:psi-io
```

`journalctl -u guardian-daemon.service` for this boot contains no
"failing closed"/canonical-acquisition-disagreement message, confirming
`libsystemd`'s independent `LISTEN_PID` check agreed with this module's
own parser and the plan was not rejected.

**Restart re-proof (R15).** `systemctl restart guardian-daemon`: new PID
(953521), fresh trigger registration succeeded (`3/3 inherited
descriptors monitored (0 failed)` — a stale description would fail
`EBUSY`), raw fd 3/4/5 `CLOEXEC` confirmed again post-restart.

**`:graceful`/reordering re-proof, through the canonical path.** A
runtime drop-in (`/run/systemd/system/guardian-daemon.service.d/
graceful-test.conf`, never `debian/guardian-daemon.service`) dropping the
`memory` `OpenFile=` line: `2/2 inherited descriptors monitored (0
failed)`, `memory: unavailable (no descriptor was inherited for this
resource)`, `LISTEN_FDS=2`/`LISTEN_FDNAMES=psi-cpu:psi-io` as delivered,
fd 3→cpu/fd 4→io (the renumbered set) both `CLOEXEC`
(`flags: 02100002`) — the exact `:graceful`-renumbering property R2
already tests, now proven live through the new canonical-acquisition
layer combined with the unchanged name resolution. Drop-in removed,
daemon restarted back to the normal 3-descriptor state, `DropInPaths=`
confirmed empty before proceeding.

**New unit tests** (`crates/guardian-daemon/src/psi_ingress.rs`):
`canonical_acquisition_agrees_with_no_systemd_activation_in_a_test_process`
(in an ordinary `cargo test` process, no `LISTEN_PID` is ever set to the
test process's own pid, so `acquire_canonical_listen_fds()` must return
`Ok(0)` and the combined plan must remain empty — never an error, never a
panic); `psi_descriptor_plan_from_env_never_panics`. The existing R1–R5
unit tests (`resolve_psi_descriptors`, called directly, bypassing the new
canonical layer) all pass **unmodified** — confirming the canonical
acquisition is additive around that pure function, never a change to its
logic.

## I.3 Sandbox regression spot-check (this pass)

```text
$ diff <(cat /usr/lib/systemd/system/guardian-daemon.service) \
       <(repo debian/guardian-daemon.service)
(no output -- IDENTICAL; this pass made no unit-file change)
$ sudo systemd-analyze security guardian-daemon.service
Overall exposure level for guardian-daemon.service: 0.6 SAFE   (unchanged)
$ grep -E '^Cap|^Uid' /proc/<pid>/status
Uid: 115 115 115 115; CapPrm/CapEff/CapBnd/CapAmb: 0000000000000000
$ sudo nsenter -t <pid> -m -p -- cat /proc/pressure/cpu
cat: /proc/pressure/cpu: No such file or directory   (still unavailable by pathname)
```

`/proc/sys/kernel/hostname`, `/proc/meminfo`, `/proc/uptime`, `/proc/net`
all `ENOENT` via the same probe. `/proc/1/status`/`/proc/1/cmdline`
showed as visible under this `nsenter -m -p` probe — this is the exact,
already-documented probe artifact §F.8's methodology note records (the
probe runs as root with a full capability set, unlike the daemon's own
restricted `guardiand` identity), not a regression; §F.9's own
transient-unit probe already established the faithful result.

## I.4 Validation (this pass, `guardian-g9`, full workspace)

```text
$ cargo fmt --check
exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
exit 0
$ cargo test --workspace
passed=475 failed=0 ignored=4
```

Baseline for this pass (per the assigning task): **467 passed, 0 failed,
4 ignored**. New total: **475 passed, 0 failed, 4 ignored** — a net `+8`
new tests (6 in `dbus_surface.rs` net of the one renamed pre-existing
test, 1 new source-text contract guard, 2 new `psi_ingress.rs` tests;
`psi_summary_reports_three_real_kernel_resources` was renamed to
`psi_summary_reports_three_resources_by_name` and re-parameterized to
match the corrected signature, not deleted). The ignored count is
unchanged at 4: Part B's canonical-acquisition fix needed no new
`sudo`-gated real-kernel test (its evidence is entirely live-VM/systemd
evidence, §I.2 above), so no new ignored test was added.

Host (`guardian-daemon`/`guardian-core`, `guardian-gui` excluded — the
authoring host lacks `libadwaita-1-dev`): `cargo fmt --check` exit 0,
`cargo clippy -p guardian-daemon -p guardian-core --all-targets
--all-features -- -D warnings` exit 0, `cargo test --workspace --exclude
guardian-gui` → 471 passed, 0 failed, 4 ignored (matches the VM total
minus `guardian-gui`'s own 4 tests, consistent).

## I.5 Repository changes made by this pass

Modified: `crates/guardian-daemon/src/dbus_surface.rs` (`Capabilities1`
now holds `psi_availability`/`psi_source`; `real_psi_summary` takes
`(&PsiAvailability, &PsiFileSource)`; six new/renamed tests);
`crates/guardian-daemon/src/bin/guardian-daemon.rs` (`spawn_psi_ingress`
takes `&PsiDescriptorPlan`; `main()` computes `psi_plan` once and derives
both `spawn_psi_ingress`'s monitors and `Capabilities1`'s `psi_source`
from it); `crates/guardian-daemon/src/psi_ingress.rs`
(`acquire_canonical_listen_fds`, `psi_descriptor_plan_from_env` now calls
it; module doc's "Canonical FD acquisition" section; two new tests);
`crates/guardian-daemon/Cargo.toml` (`libsystemd = "0.7.2"` dependency,
with an inline comment pointing at the module doc's full evaluation);
`crates/guardian-daemon/tests/phase2_psi_ingress_contract.rs`
(`dbus_surface_never_opens_psi_by_pathname`, new); `crates/
guardian-daemon/tests/phase2_2b_contract.rs` (one call-site update for
`Capabilities1::new`'s new arity — no assertion weakened; the same test
still diffs live introspection against the recorded G9 baseline);
`Cargo.lock` (regenerated for the new dependency: `libsystemd` and its
transitive `nix`/`cfg_aliases`/`hmac`/`nom`/`subtle`/`thiserror`/
`thiserror-impl`).

**Untouched, confirmed by empty `git diff`**: `crates/guardian-core/src/
correlation.rs`, `crates/guardian-core/tests/correlation_contract.rs`,
all `phase2-2c-*` files, `GATE2C_VM_EVIDENCE.md`, `debian/
guardian-daemon.service`, `crates/guardian-helper/**`, `crates/
guardian-daemon/src/restart_capability.rs`, client/GUI/TUI/CLI/indicator
crates, `IncidentWire`'s shape, `Incident::link_event`'s signature,
`PSI_MONITOR_CONFIG`'s threshold values.

Nothing committed, tagged, or pushed — host or VM — per this task's
commit policy.

## I.6 Updated status

Both items this pass was scoped to close are closed: `Capabilities1` and
`PsiSummary` no longer contradict each other (proven live, dual-call, on
the same daemon instance, both the healthy and the degraded case); raw-FD
handling is genuinely improved (`FD_CLOEXEC` now set on the raw
descriptors themselves, proven live, before/after) with one honestly
reported residual gap (`LISTEN_PIDFDID`, and why closing it was judged
disproportionate). No accepted sandbox/security invariant was weakened.
`PHASE 2 PSI FINAL ACCEPTANCE REPAIR — READY FOR RE-REVIEW`.
