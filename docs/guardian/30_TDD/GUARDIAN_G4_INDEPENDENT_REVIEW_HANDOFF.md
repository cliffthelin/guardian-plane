# Guardian Phase 0 Independent Review Handoff
## G4 — Transaction Engine Only

**Audience:** Independent reviewer (a separate session/agent from the one
that implemented G4)
**Scope:** Read-only audit of the G4 implementation against
`GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` and the governing TDD contract.
**Constraints:** Do not modify files. Do not create commits. Do not move
tags. Do not create a G4 tag. Do not begin G5. Scratch mutations are
permitted only outside the repository (e.g. `/tmp`) and must not be
committed.

---

# 1. Baseline verification

Before judging anything, confirm:

- `main` branch, working tree clean, `origin/main` synchronized;
- `HEAD` descends from `phase0-g3-core-data-models` (target
  `d21d0c4f41f7032db969b0b50c20c72c17b836c5`);
- `phase0-g0-public-contracts`, `phase0-g1-identity-authorization`,
  `phase0-g2-privilege-topology`, `phase0-g3-core-data-models` all remain
  unmoved and are ancestors of `HEAD`;
- no `phase0-g4-*` tag exists yet;
- no G5/G8 implementation exists (real Diagnostic Budget Manager, real
  provider adapters, GUI/TUI/CLI code, packaging).

If any of these differ materially, report the discrepancy before
continuing.

---

# 2. Governing material to read

1. `AGENTS.md`
2. `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §2 (GP-04/05/
   06/08/10), §14, §15, §16, §23, §36 (P0-TXN group)
3. `docs/guardian/30_TDD/GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` (the
   operative instruction for this pass)
4. `docs/evidence/g3/G3_MILESTONE.md` — the four forward constraints
   (NB-1..NB-4) this gate is required to resolve
5. `docs/guardian/20_Control_Plane/Transaction_Engine.md`,
   `Privilege_and_Authorization.md`
6. `docs/adr/ADR-002-guardian-privilege-topology.md`
7. The actual implemented code and tests, not just the completion report.

---

# 3. Required verdict

Return exactly one:

```text
PASS
PASS WITH NON-BLOCKING FINDINGS
FAIL — CONTRACT VIOLATION
FAIL — TEST INSUFFICIENT
FAIL — TRANSACTION ORDERING UNSAFE
FAIL — STALE PRECONDITION NOT DETECTABLE
FAIL — UNKNOWN PRIVILEGE EXECUTION ALLOWED
FAIL — AUTHORIZATION BOUNDARY REGRESSION
FAIL — ROLLBACK MODEL UNSAFE
FAIL — CRASH RECOVERY MODEL INSUFFICIENT
FAIL — PERSISTENCE CONTRACT UNSAFE
FAIL — DUPLICATE APPLY RISK UNCONTROLLED
FAIL — G5/G8 SCOPE LEAK
```

A `PASS` means the transaction engine is sound enough to become Guardian's
G4 milestone.

---

# 4. Normative contract audit

For each `P0-TXN-001..012`: confirm the test genuinely exercises the
described behavior against real transaction-engine code (not a mock that
can't fail), and confirm no `P0-*` ID was invented that doesn't exist in
the governing contract (re-verify mechanically:
`grep -oE "P0-[A-Z]+-[0-9]+" docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md
| sort -u`, confirm the transaction group is exactly `P0-TXN-001..012`).

---

# 5. Transaction ordering audit (mutation before authorization)

This is the audit's highest-priority structural check, since it is the
direct extension of G1/G2's core invariant.

- Locate every code path that can reach `Apply`. For each, trace backward:
  is `Authorize` (with a real, non-bypassable outcome check) unconditionally
  on that path?
- Attempt a scratch mutation (in `/tmp`, never committed) that skips
  `Authorize` for some transaction shape and calls `Apply` directly — does
  a test catch it?
- Confirm `authorization_result` is a *record* of what happened, not a
  forgeable/replayable capability that could let a second `Apply` attempt
  skip real authorization.

If mutation before authorization is possible anywhere, verdict is
`FAIL — TRANSACTION ORDERING UNSAFE`.

---

# 6. Stale precondition detectability audit (NB-2 resolution)

- Confirm `revision` is produced by G4's own code (arbitration re-run,
  or an explicit fixture registry G4 controls), never accepted as a bare
  caller-supplied input with no derivation.
- Confirm `Validate` re-runs arbitration and compares `revision` against
  `pre_state`'s captured value.
- Confirm `Apply` re-checks `revision` **immediately before mutation**, not
  only at `Validate` time — locate the specific test that bumps `revision`
  between `Validate` and `Apply` and confirms `Apply` is blocked.
- Confirm restart/recovery re-derives current `revision` rather than
  trusting a disk-persisted value as still current.

If `revision` is still a bare caller-trusted number with no G4-owned
derivation or TOCTOU recheck, verdict is
`FAIL — STALE PRECONDITION NOT DETECTABLE`.

---

# 7. Unknown privilege/access execution audit (NB-1 resolution)

- Locate the exact `Validate`-step check for
  `CapabilityRecord.privilege_requirement == PrivilegeRequirement::Unknown`.
- Confirm it produces `REJECTED`, not `FAILED` or a silent pass-through.
- Confirm there is no exception path that treats
  `authorization_ownership = Known(ProviderOwnedAuthorization)` (or any
  other known authorization state) as license to skip this check — attempt
  a scratch mutation removing the check specifically for provider-owned
  capabilities and confirm a test catches it.
- Confirm the check happens for every transaction that reaches `Validate`,
  not only ones explicitly constructed to test it (i.e., it's in the real
  `Validate` code path, not a separate opt-in validator).

If a write can reach `APPLYING` while the target's `privilege_requirement`
is `Unknown`, verdict is `FAIL — UNKNOWN PRIVILEGE EXECUTION ALLOWED`.

---

# 8. Authorization boundary regression audit (G2 preservation)

- Does any G4 type have a field/method resembling "authorization proof"
  that a privileged helper could be tempted to trust instead of performing
  its own real `CheckAuthorization`?
- Is `ArbitrationDecision.write_permitted` (G3, reused) still documented
  and treated strictly as control-plane policy, never caller-authorization
  proof?
- Does `authorization_result` on `TransactionRecord` ever get promoted to
  mean "this caller is authorized" for a *different* subsequent request,
  rather than describing what happened for *this* transaction's own
  authorize step?
- Is G1's `interactive` flag semantics reused unchanged (no second
  prompting mechanism invented)?
- Was any G1/G2 production code (`guardian-core/src/{authorization,identity}`,
  the G2 helper prototype) modified? (It should not have been — confirm via
  `git diff phase0-g3-core-data-models..HEAD -- crates/guardian-core/src/authorization crates/guardian-core/src/identity`
  is empty, and confirm no privileged-helper file changed.)

If G4 introduces anything that could let a core-owned or transaction-owned
value be mistaken for real caller-authorization proof, verdict is
`FAIL — AUTHORIZATION BOUNDARY REGRESSION`.

---

# 9. Rollback model audit

- Confirm all four `RollbackKind` variants (`Native`/`Emulated`/
  `BestEffort`/`None`) have distinct, tested behavior per the
  implementation handoff §17.
- Confirm `RollbackKind::None` never gets silently treated as though
  rollback succeeded, and confirm the model exposes "no rollback guarantee"
  as an inspectable fact.
- Confirm `RollbackKind::BestEffort`'s ambiguous outcome is representable
  and not forced into `ROLLED_BACK` when the fixture can't actually confirm
  success.
- Confirm `ROLLBACK_FAILED` (P0-TXN-007) is a real, distinct, reachable
  state — locate the test, and confirm by scratch mutation that collapsing
  it into `FAILED` or `ROLLED_BACK` would be caught.
- Confirm the "Apply succeeded, Confirm failed, rollback failed" case is
  distinguishably represented (not silently indistinguishable from a
  cleaner failure).

If any of these is false, verdict is `FAIL — ROLLBACK MODEL UNSAFE`.

---

# 10. Crash recovery model audit

Per the implementation handoff §20, confirm all six recovery
classifications are real, tested outcomes of a recovery function, each
with its own dedicated test using a purpose-built persisted-record
fixture:

```text
safe to resume
must observe
must rollback
already committed
state ambiguous
requires human/recovery handling
```

Attempt a scratch mutation feeding the recovery function a deliberately
corrupted record — confirm it does NOT classify as "safe to resume" or
"already committed." If any classification is missing, untested, or a
corrupt record can be misclassified as safe, verdict is
`FAIL — CRASH RECOVERY MODEL INSUFFICIENT`.

---

# 11. Persistence contract audit

- Confirm every persisted record carries an explicit schema version.
- Confirm atomic-write behavior (temp-file-then-rename or equivalent) —
  locate the actual write-path code, not just a doc claim.
- Confirm a record from an unrecognized future schema version fails closed
  (typed error) rather than being silently misinterpreted.
- Confirm no persisted record relies on `Debug` formatting as its wire
  representation.
- Confirm P0-TXN-011's restart-recovery test actually reads persisted
  records from disk (or an equivalent durable-storage fixture), not merely
  from in-memory state carried across a simulated "restart" that never
  left memory.

If persistence is only simulated in memory with no real serialization/
durability contract, or a corrupt/future-schema record could be
misinterpreted as valid, verdict is `FAIL — PERSISTENCE CONTRACT UNSAFE`.

---

# 12. Duplicate-apply / idempotency audit

- Locate the `idempotency_key` check in the `Apply` path.
- Confirm a retry with the same `idempotency_key` after a simulated lost
  response does not cause a second real call to the fixture adapter's
  `apply()` — attempt this as a real test/scratch check, not by reading
  the code alone.
- Confirm the mechanism is scoped honestly (Guardian's own engine never
  double-applies against its own fixtures) without an overclaimed universal
  cross-provider guarantee the handoff didn't require.

If the same `idempotency_key` can cause two real `apply()` calls, verdict
is `FAIL — DUPLICATE APPLY RISK UNCONTROLLED`.

---

# 13. `TransactionId` / `EventId` / `IncidentId` audit (NB-4 resolution)

- Confirm `TransactionId` is its own newtype with generated-record-identity
  semantics (ULID/UUID-shaped), not reusing `CapabilityId`'s
  dotted-domain validator.
- Confirm the completion report makes an explicit, justified decision about
  `EventId`/`IncidentId` (leave as-is vs. correct to match) — and if
  corrected, confirm it landed as a separate, clearly-labeled commit, not
  folded silently into a general feature commit.
- Confirm `guardian-provider-api/src/ids.rs` was not modified as an
  unlabeled side effect.

---

# 14. State machine audit

- Confirm the implemented states match §14 of the governing contract
  exactly — no renamed, merged, or invented states.
- Confirm terminal states are immutable — attempt a scratch mutation
  allowing `COMMITTED → APPLYING` (or similar) and confirm a test (P0-TXN-008)
  catches it.
- Confirm there is no generic `set_state(anything)` API — only named
  transition methods, each with its own legal-predecessor check.

---

# 15. TOCTOU revalidation audit

For each of the scenarios listed in the implementation handoff §10
(provider disappears, ownership changes, external writer appears, resource
identity changes, authorization context stale, fixture helper/core
"restart" between steps): confirm a real, specific test exists with a
specific expected outcome — not one generic "TOCTOU is handled" test.

---

# 16. Event/Incident integration audit

- Confirm `guardian_core::event::Event`/`Incident` are reused unchanged
  (no fork) — check via `git diff phase0-g3-core-data-models..HEAD -- crates/guardian-core/src/event.rs crates/guardian-core/src/incident.rs`.
- Confirm no correlation *engine* was built (only direct, explicit
  failure-to-incident links, if any).

---

# 17. Scope-leak audit (G5/G8)

Search for and rule out: G5's Diagnostic Budget Manager (cost-class veto
logic, PSI-driven scheduling), real provider adapters (`org.freedesktop.UDisks2`,
`NetworkManager`, `systemd1`, NVML, UPower, thermald, fwupd — distinguish
real integration from doc/test-label mentions), privileged-helper changes,
public D-Bus surface expansion, GUI/TUI/CLI code, packaging artifacts,
production `/var/lib/guardian` wiring (only a test-controlled temp
directory should exist).

If any of these exist as real functionality (not doc mentions), verdict is
`FAIL — G5/G8 SCOPE LEAK`.

---

# 18. G0/G1/G2/G3 regression audit

Run the full pre-existing test suite (should remain at least the 75 tests
from G3, unmodified) and confirm:

- `crates/guardian-daemon` untouched;
- `crates/guardian-core/src/{authorization,identity}` untouched;
- `crates/guardian-provider-api/tests/provenance_contract.rs` still
  byte-identical to its G0 baseline;
- G3's arbitration/event/incident modules unchanged except where this
  handoff explicitly required an addition (e.g. adding `FromStr` to
  `RollbackKind` per NB-3, §23 of the implementation handoff) — confirm any
  such change is narrow, additive, and doesn't alter G3's existing tested
  behavior.

---

# 19. Adversarial questions (mirror the implementation handoff's §27, verify each independently)

Report each as confirmed safe / confirmed unsafe / not applicable, with the
specific file/test supporting the conclusion, for all 20 items listed in
the implementation handoff's §27.

---

# 20. Validation

Run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Report exact pass/fail counts. Confirm the pre-G4 baseline (75 tests, 0
failed) still passes unmodified, plus the new G4 tests, all green.

---

# 21. Required report

1. Verdict
2. Exact candidate (HEAD / branch / origin/main / working tree / G0-G3 tags
   / G4 tag [must be absent] / ancestry)
3. Normative contract audit (§4)
4. Transaction ordering audit (§5)
5. Stale precondition detectability audit (§6)
6. Unknown privilege/access execution audit (§7)
7. Authorization boundary regression audit (§8)
8. Rollback model audit (§9)
9. Crash recovery model audit (§10)
10. Persistence contract audit (§11)
11. Duplicate-apply audit (§12)
12. TransactionId/EventId/IncidentId audit (§13)
13. State machine audit (§14)
14. TOCTOU revalidation audit (§15)
15. Event/Incident integration audit (§16)
16. Scope-leak audit (§17)
17. G0-G3 regression audit (§18)
18. Adversarial questions (§19), one line each
19. Blocking findings (file/test ID/problem/evidence/why it matters/
    required correction)
20. Non-blocking findings (same structure)
21. Validation results
22. Git state (confirm no changes made by this review)
23. Recommended next action — exactly one of: Tag G4 and prepare G5 gate.
    / Repair G4 and re-review. / Reconsider transaction/rollback model. /
    Resolve G4 model ambiguity before G5.

Then STOP. Do not create a G4 tag. Do not begin G5.
