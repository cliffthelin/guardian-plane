# Guardian Phase 0/1 Implementation Handoff
## G7 — Production Daemon Only

# 1. Mission

Build the real, running production daemon: a systemd-managed service
registered on the system D-Bus, implementing the selected privilege
topology (ADR-002, Model B — unprivileged `guardian-core` + narrow
privileged `guardian-helper`), with persistent state that survives
daemon restart and forced termination without corruption.

G7 is mechanically derived from §38's own text:

```text
## G7 — Production daemon

Required:
- systemd unit;
- D-Bus service;
- persistent state;
- selected privilege topology implemented.

Tests:
P1-DMN-001..005
P1-SEC-001..004
```

Nine normative IDs total. Do not treat any other P1-* group (P1-SYS,
P1-PSI, P1-LGI, P1-UDS, P1-UPW, P1-ACC — all G8; P1-CLI/TUI/GUI/IND —
G9; P1-PKG — G9) as in scope here. §36's P0-* IDs are already
implemented (G0-G5, all tagged) and are not re-litigated by this gate.

# 2. G7 is primarily an integration/wiring gate, not a from-scratch logic gate

Read this before writing code. The hard business logic G7 depends on
already exists, is already tested, and is not to be redesigned:

- Transaction engine, persistence (`persist`/`load`/`load_all`), and
  crash recovery model: `crates/guardian-core/src/transaction/`
  (`engine.rs`, `persistence.rs`, `recovery.rs`, `apply.rs`,
  `observation.rs`, `rollback.rs`) — G4, tagged
  `phase0-g4-transaction-engine`.
- Diagnostic Budget Manager, PSI parsing model, bounded recorder:
  `crates/guardian-core/src/{budget,psi,recorder}.rs` — G5, tagged
  `phase0-g5-diagnostic-safety`.
- Capability Registry, Provider Arbitrator, event/incident model:
  `crates/guardian-core/src/{arbitration,event,incident}.rs` — G3,
  tagged `phase0-g3-core-data-models`.
- Caller identity resolution and polkit authorization:
  `crates/guardian-core/src/{identity,authorization}.rs` — G1, tagged
  `phase0-g1-identity-authorization`.
- Privilege topology decision and its full evidence base: `ADR-002`, G2
  prototypes referenced there (`crates/guardian-daemon/tests/
  g2_privilege_topology_contract.rs`) — G2, tagged
  `phase0-g2-privilege-topology`.

G7's job is to wire this already-tested logic into a real running
service and prove, under real systemd/D-Bus/crash conditions (not unit
tests alone), that the wiring itself is correct. Do not reimplement or
redesign any of the above modules "while you're in there."

# 3. Naming collision to resolve explicitly (read before creating binaries)

`crates/guardian-core` is the existing **library crate name** holding
the shared business logic listed in §2. `ADR-002`'s own prose uses "the
future production `guardian-core`" to mean the **unprivileged daemon
binary** Model B calls for. These are not the same thing, and G7 must
not conflate them. Concretely:

- The existing `crates/guardian-daemon` crate (currently a thin,
  31-line library exporting only the G0 read-only contract skeleton —
  `INTERFACE_NAME`, `OBJECT_PATH`, `GuardianContract` with
  `service_state() == "contract-only"`) is the natural home for the
  **unprivileged core daemon binary** ADR-002 describes. It already
  depends on `guardian-core` (the library) and already has the G0/G1/G2
  contract test suites in `tests/`.
- A **new** crate/binary is needed for `guardian-helper` (the narrow
  privileged component ADR-002 requires) — it does not exist yet.
- Pick and document real crate/binary names before writing systemd unit
  files; do not leave the "core daemon" vs. "core library" ambiguity
  unresolved in the unit files or D-Bus service names.

# 4. Required tests (§37, normative)

```text
P1-DMN-001 — boot start
  Daemon starts successfully under systemd before graphical login.

P1-DMN-002 — restart
  Daemon restart preserves required persistent state.

P1-DMN-003 — no desktop dependency
  Daemon runs without GNOME/Xfce/session bus.

P1-DMN-004 — clean stop
  Stop completes without leaving corrupt transaction persistence.

P1-DMN-005 — crash recovery
  Forced daemon termination does not corrupt persisted state.

P1-SEC-001 — hardening review
  `systemd-analyze security` artifact exists.

P1-SEC-002 — path access
  Service cannot write outside declared writable paths except through
  authoritative external provider APIs.

P1-SEC-003 — no arbitrary shell API
  Public interface remains free of generic command execution.

P1-SEC-004 — privilege denial
  Unauthorized client cannot use a test write action.
```

Every one of these is a real-system claim (systemd behavior, real
process kill/restart, real `systemd-analyze security` output, a real
unauthorized D-Bus call denied) — none is provable by `cargo test`
alone. This gate's evidentiary standard is G1/G2/G6's precedent, not
G3/G4/G5's: real disposable-VM execution, screenshots/transcripts with
provenance, no mocks substituting for the real systemd/D-Bus/polkit
stack. Follow the `*-vm-setup.sh` + `*_EVIDENCE.md` pattern already
established in `docs/evidence/g1/`, `docs/evidence/g2/`, and
`docs/evidence/g6/`.

# 5. Scope boundary

## In scope

- `guardian-core` daemon binary (unprivileged, per ADR-002): systemd
  unit, D-Bus service registration on the real system bus (not just a
  private test bus), wiring the transaction engine + persistence +
  budget + recorder + arbitration modules into a real running process.
- `guardian-helper` binary (privileged, per ADR-002): systemd unit,
  narrow, individually-authorized, typed write methods only — no
  generic command execution, no shell-out (§40's forbidden-shortcuts
  list applies in full).
- Real crash/restart/stop persistence behavior, tested against the real
  running service (kill -9, systemctl restart, systemctl stop — not
  simulated).
- `systemd-analyze security` hardening pass and its artifact, for both
  units.
- Real polkit-backed authorization denial test against a live service
  (P1-SEC-004) — this is the production-wiring counterpart to G1's
  already-tested authorization logic, not a new authorization design.

## Out of scope (do not implement here)

- Any real provider (systemd unit inspection, PSI, logind, UDisks,
  UPower, AccountsService) beyond what's needed to prove the daemon
  itself boots/runs/persists — all real providers are G8.
- CLI, TUI, GUI, or the production indicator — all G9. `ksni` (G6's
  selection) is not wired into a real production indicator here; G7
  produces the daemon `ksni`-based client will eventually talk to, not
  the client itself.
- Packaging (`.deb`, install/uninstall/purge semantics) — G9.
- Any change to `guardian-core`'s (library) G0-G5 modules' own logic
  unless a genuine, narrowly-scoped integration bug is found while
  wiring them in — if so, document the bug and the minimal fix
  separately from the wiring work, and do not use "wiring" as cover for
  unrelated refactoring.
- Re-deciding the privilege topology. `ADR-002` (Model B) is accepted
  and tagged; G7 implements it, it does not re-litigate it.

# 6. Forward constraints from G4-G6 that apply here

Carried forward deliberately, not dragged in wholesale — only the ones
this gate can actually act on:

- **G5's FC-2 (`RecorderPolicy` runtime wiring)**: G5 built and tested
  `RecorderPolicy`/`recorder_policy_for()` as an isolated decision type,
  explicitly *not* wired into a real recorder+budget runtime path, with
  G5's own milestone naming the first gate that builds such a runtime
  path as the one responsible for ensuring the policy decision is
  actually consumed, not merely present. **G7 is that gate** for the
  daemon-level wiring (the recorder instance the real daemon runs must
  actually call `recorder_policy_for()` and act on its result, not just
  hold an unused `RecorderPolicy` type in scope).
- **G5's FC-1 (recorder byte boundedness)**: G5 proved a bounded
  *queue* (capacity-bounded), not a byte-size bound. If G7's real daemon
  wiring introduces long-running production capture with variable-size
  payloads, this gate must determine whether a byte-level bound is now
  required and make it explicit — do not silently assume G5 already
  proved this.
- **G4's FC-3 (Flight Recorder has no relationship to G4's persistence
  module)**: relevant if G7's daemon wiring brings the recorder and the
  transaction persistence module into the same running process for the
  first time — check whether they need to interact (e.g., recorder
  evidence referencing a transaction ID) or must remain deliberately
  independent, and document the decision either way.

**Explicitly not carried into G7** (considered and excluded, not
overlooked): G6's FC-G6-1 (simplicity-rule underspecification) concerns
future *candidate-selection* gates, not daemon wiring — not applicable
here. G6's FC-G6-2 (session-scoped production launch) concerns the
future *indicator client's* launch mechanism — that is G9's concern when
the production indicator is built on top of `ksni`, not G7's.

# 7. Fail-closed checklist

- A daemon crash or forced kill must never leave a transaction in a
  state that looks committed but wasn't, or applied-but-unrecorded —
  P1-DMN-005 must be evidenced against a real `kill -9`, not a graceful
  shutdown standing in for it.
- `guardian-helper` must reject an unauthorized write even if
  `guardian-core` is compromised or malicious — per ADR-002's own stated
  invariant ("`guardian-helper` must never depend on `guardian-core`
  forwarding the client's identity or authorization claim"). Verify this
  is actually true of the real wiring, not just asserted.
- No component may call `sudo`, run as root wholesale, or expose a
  generic command-execution method — §40's forbidden-shortcuts list
  applies without exception.

# 8. TDD sequence

1. Resolve the naming question in §3 and record the decision (a short
   note in this gate's own evidence, not a new ADR unless the decision
   is genuinely contested).
2. Build the minimal `guardian-core` daemon binary: real systemd unit,
   real D-Bus service registration on the system bus, wiring in the
   already-tested transaction engine + persistence.
3. Build the minimal `guardian-helper` binary: real systemd unit, one or
   two narrow typed write methods for evidence purposes, real polkit
   `CheckAuthorization` call (not simulated).
4. Set up a real disposable Ubuntu 26.04.1 VM (matching G1/G2/G6
   precedent) and gather real evidence for P1-DMN-001..005 and
   P1-SEC-001..004 in that order.
5. Run `systemd-analyze security` against both real units; capture the
   artifact.
6. Write the completion report per §13 below, then hand off to
   independent review.

# 9. Adversarial self-check before reporting done

- Does the daemon actually survive `systemctl restart` with a
  non-terminal transaction in flight, or was this only tested with an
  idle daemon?
- Does `kill -9` really get tested, or was `systemctl stop` (graceful)
  substituted for it?
- Does `guardian-helper` actually verify the caller's identity itself,
  or does it trust a claim forwarded by `guardian-core`?
- Is `RecorderPolicy` actually consumed by the running daemon's recorder
  instance, or merely constructed and discarded (repeating G4's own
  audit-caught mistake of a precondition type existing in isolation
  without being wired into the real call path)?
- Does `systemd-analyze security`'s score reflect the units as they will
  actually ship, or a looser development-only configuration?
- Did any G8/G9 material (a real provider, a CLI/TUI/GUI, packaging)
  get implemented under cover of "needed to test the daemon"?

# 10. Completion states

Report exactly one, honestly:

```text
G7 CANDIDATE — READY FOR INDEPENDENT AUDIT
G7 PARTIAL — REQUIRED EVIDENCE INCOMPLETE
G7 BLOCKED — GOVERNING CONTRACT INSUFFICIENT
```

# 11. Completion report

State plainly:

1. What was built (crate/binary names, systemd unit paths, D-Bus
   service name).
2. Real evidence for each of the nine normative IDs, with VM
   setup/reproduction script, environment details, and provenance-
   labeled transcripts/artifacts — no ID may be marked PASS on unit-test
   evidence alone.
3. Which G4/G5/G6 forward constraints (§6) were addressed and how.
4. Explicit confirmation that G8 providers, G9 clients/packaging, and
   the production indicator were not implemented here.
5. `cargo fmt --check` / `cargo clippy --workspace --all-targets
   --all-features -- -D warnings` / `cargo test --workspace` results,
   with the exact before/after passed count (189 passed, 0 failed is
   the pre-G7 baseline).
