# Guardian Phase 0 Independent Review Handoff
## G7 — Production Daemon Only

# 1. Baseline verification

```bash
git status
git rev-parse HEAD
git diff --name-status <accepted-G6-tag-SHA>..HEAD
git diff --stat <accepted-G6-tag-SHA>..HEAD
```

Baseline is `phase0-g6-indicator-decision` (985a04d9a9af24cf9201d9dbeb1
ebbbea762a139). Independently re-derive this SHA from `git rev-parse
phase0-g6-indicator-decision^{commit}` rather than trusting a pasted
value. Confirm `phase0-g6-indicator-decision` and every earlier
`phase0-g*` tag are unmoved. Review every actual changed file — do not
sample.

# 2. Governing material to read

```text
AGENTS.md
docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md  (§37, §38 G7
  entry, §39, §40, §41)
docs/guardian/30_TDD/GUARDIAN_G7_IMPLEMENTATION_HANDOFF.md
docs/adr/ADR-002-guardian-privilege-topology.md  (the accepted Model B
  decision G7 must implement)
docs/evidence/g6/G6_MILESTONE.md  (immediately prior accepted gate)
```

# 3. Required verdict

Exactly one:

```text
PASS — G7 PRODUCTION DAEMON ACCEPTED
PASS WITH NON-BLOCKING FINDINGS
FAIL — G7 REQUIRED EVIDENCE INCOMPLETE
FAIL — G7 PRIVILEGE TOPOLOGY NOT FAITHFULLY IMPLEMENTED
FAIL — G7 SCOPE BOUNDARY VIOLATION
```

# 4. Mechanically re-derive G7's normative scope — do not trust the candidate's own framing

Read §37 and §38's G7 entry directly. Confirm independently that G7's
normative IDs are exactly `P1-DMN-001..005` and `P1-SEC-001..004` (nine
total) and no others. If the candidate's own report cites a different
ID set, that is itself a finding.

# 5. The central audit question — is this real production wiring, or G0-G5's logic re-presented with a systemd unit wrapped around it?

G7 depends entirely on already-tested logic from G1-G5 (transaction
engine, persistence, budget, PSI, recorder, arbitration, identity,
authorization — see the implementation handoff §2 for exact file
locations). The real risk this audit exists to catch: a candidate that
builds a systemd unit and a D-Bus registration, but never actually
routes real requests through the real engine/persistence code paths —
i.e., a thin shell that *looks* like a production daemon in its unit
file but doesn't actually exercise the logic G0-G5 spent five gates
proving. For every one of the nine normative IDs, trace the actual code
path: does the evidence prove the real `guardian-core` library logic
ran, or only that a process started and a D-Bus name was claimed?

# 6. Privilege topology fidelity audit (ADR-002)

- Does a real `guardian-helper` binary exist, running as root, with
  narrow, individually-authorized, typed write methods — not a generic
  command broker?
- Does `guardian-core` (the daemon) genuinely run unprivileged? Verify
  via the same kind of `/proc/<pid>/status` capability inspection G2's
  own evidence used (`ADR-002` §"systemd hardening"), not by trusting
  the unit file's `User=` directive alone — confirm the *running*
  process's actual capability set.
- Does `guardian-helper` independently verify caller identity/
  authorization, or does it trust a claim forwarded by `guardian-core`?
  ADR-002 states this must never happen — verify it directly against
  the real running services (e.g., attempt a call to `guardian-helper`
  that bypasses `guardian-core` entirely and confirm it is still
  correctly authorized or correctly rejected).
- Check the naming-collision resolution from the implementation
  handoff's §3 was actually made and is not left ambiguous between the
  `guardian-core` library crate and whatever binary now serves as the
  unprivileged daemon.

# 7. Real-environment evidence audit (G1/G2/G6 evidentiary standard applies)

For each of the nine normative IDs, verify:

```text
- real disposable VM, not the primary workstation;
- real systemd (systemctl start/stop/restart, real kill -9), not
  simulated or scripted-around;
- real D-Bus calls against the real system bus, not a private test bus
  standing in for "real";
- real systemd-analyze security output as a genuine artifact, not a
  summary claiming a score;
- provenance on every screenshot/transcript (candidate/build identity,
  environment, timestamp/run identity), same standard as G6's evidence.
```

Specifically interrogate:

- **P1-DMN-002/005 (restart / crash recovery)**: was a real
  non-terminal transaction actually in flight at the moment of
  restart/kill, or was the daemon idle? An idle-daemon restart proves
  much less than the required claim.
- **P1-DMN-004 (clean stop)**: does the evidence show the persistence
  store's actual state (not just "the daemon exited 0") before and
  after stop?
- **P1-SEC-004 (privilege denial)**: is the "unauthorized client" a
  genuinely different, unprivileged identity, or the same privileged
  test harness merely omitting a flag?

# 8. Fail-closed / forbidden-shortcuts audit (§40)

Confirm none of §40's list was violated: no `sudo` from any client, no
GUI-as-root (not applicable yet — no GUI exists), no generic root
command broker anywhere in `guardian-helper`, no shell-out where a
stable D-Bus provider API was already selected, no direct `/etc` writes
from an unprivileged client.

# 9. Forward-constraint audit (G4/G5 carried into G7)

Verify the implementation handoff's §6 claims are actually true, not
just asserted:

- Is `RecorderPolicy`/`recorder_policy_for()` (G5) genuinely called by
  the running daemon's real recorder instance and its result actually
  acted upon — or merely constructed and left unused (repeating the
  exact class of gap G4's own independent audit caught and required
  fixed)?
- If G5's FC-1 (byte-boundedness) or G4's FC-3 (recorder/persistence
  relationship) became relevant during this gate's wiring work, was
  either explicitly addressed and documented, or explicitly and
  honestly deferred with a stated reason — not silently ignored?

# 10. Scope-leak audit (G8/G9)

Confirm no real provider (systemd unit inspection beyond what's needed
to prove the daemon's own liveness, PSI, logind, UDisks, UPower,
AccountsService), no CLI/TUI/GUI, no production indicator, and no
packaging work was implemented under cover of "needed to evidence the
daemon." `ksni` (G6's selection) should not appear wired into a real
indicator client anywhere in this diff — that is G9.

# 11. Regression audit

Verify unmoved: all `phase0-g0..g6` tags. Verify unchanged: G0-G5's
`guardian-core` library logic (any diff there must be a narrowly-scoped,
explicitly-documented integration bugfix per the implementation
handoff's §5 "out of scope" note — not a redesign). Confirm the public
D-Bus contract (`io.github.cliffthelin.Guardian1` namespace, ADR-001)
was extended additively if at all, never broken.

# 12. Validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Baseline before G7: 189 passed, 0 failed. Report the exact new total —
G7 is expected to add real integration tests, so an increase is
expected and should be explained, not merely reported as a number.

# 13. Required report

```text
1. Baseline/candidate SHAs, tag state
2. Full changed-file reconciliation (every file, not sampled)
3. P1-DMN-001..005 matrix: what was actually proven, with artifact
   references
4. P1-SEC-001..004 matrix: same
5. Privilege topology fidelity findings (§6)
6. Real-vs-simulated evidence findings (§7)
7. Forbidden-shortcuts findings (§8)
8. Forward-constraint findings (§9)
9. Scope-leak findings (§10)
10. Regression findings (§11)
11. Validation results
12. Blocking findings (or "None")
13. Non-blocking findings
14. Verdict
15. Recommended next action
```

Then STOP. Do not push. Do not tag. Do not begin G8/G9 work.
