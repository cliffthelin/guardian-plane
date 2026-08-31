# Guardian Phase 0 Independent Review Handoff
## G3 — Core Data Models Only

**Audience:** Independent reviewer (a separate session/agent from the one
that implemented G3)
**Scope:** Read-only audit of the G3 implementation against
`GUARDIAN_G3_IMPLEMENTATION_HANDOFF.md` and the governing TDD contract.
**Constraints:** Do not modify files. Do not create commits. Do not move
tags. Do not create a G3 tag. Do not begin G4.

---

# 1. Baseline verification

Before judging anything, confirm:

- `main` branch, working tree clean, `origin/main` synchronized;
- `HEAD` descends from `phase0-g2-privilege-topology` (target
  `87502df8e41268aec4e94635d218c8b81c82189c`);
- `phase0-g0-public-contracts` (`15cdb787f99b4374f08a4c6bd3fe570f07f74960`)
  and `phase0-g1-identity-authorization`
  (`761bd4ae869c3e5d2168b8f9da47fbe797e89c62`) remain unmoved and are
  ancestors of `HEAD`;
- `phase0-g2-privilege-topology` remains unmoved and is an ancestor of
  `HEAD`;
- no `phase0-g3-*` tag exists yet;
- no G4/G8 implementation exists (real transaction state machine, real
  UDisks/NetworkManager/systemd/NVML/UPower/thermald adapters, GUI/TUI/CLI
  code).

If any of these differ materially, report the discrepancy before
continuing.

---

# 2. Governing material to read

1. `AGENTS.md`
2. `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §10–§13, §16,
   §18, §21, §36 (P0-REG, P0-ARB, P0-EVT groups)
3. `docs/guardian/30_TDD/GUARDIAN_G3_IMPLEMENTATION_HANDOFF.md` (the
   operative instruction for this pass)
4. `docs/guardian/20_Control_Plane/Capability_Registry.md`,
   `Provider_Arbitrator.md`, `Event_and_Incident_Model.md`,
   `Privilege_and_Authorization.md`
5. `docs/adr/ADR-002-guardian-privilege-topology.md` and
   `docs/evidence/g2/G2_MILESTONE.md` — the accepted G2 constraints this
   gate must not violate
6. The actual implemented code and tests, not just the completion report.

---

# 3. Required verdict

Return exactly one:

```text
PASS
PASS WITH NON-BLOCKING FINDINGS
FAIL — CONTRACT VIOLATION
FAIL — TEST INSUFFICIENT
FAIL — NONDETERMINISTIC CORE MODEL
FAIL — CAPABILITY/PROVIDER IDENTITY CONFLATED
FAIL — SINGLE-WRITER MODEL UNSAFE
FAIL — AUTHORIZATION / PRIVILEGE DIMENSIONS CONFLATED
FAIL — G2 PRIVILEGE BOUNDARY REGRESSION
FAIL — G4/G8 SCOPE LEAK
```

`FAIL — AUTHORIZATION / PRIVILEGE DIMENSIONS CONFLATED` replaces the
narrower `FAIL — AUTHORIZATION OWNERSHIP CONFLATED` concept from an
earlier draft of this handoff. It covers both: (a) authorization ownership
itself being conflated with something else, and (b) — the specific defect
this handoff was repaired to catch — a single field/enum being used to
represent *both* "who owns authorization" and "what OS privilege/access is
required," which are two independent dimensions per the implementation
handoff §5.

A `PASS` means the core data models are sound enough to become Guardian's
G3 milestone.

---

# 4. Normative contract audit

For each of `P0-REG-001`, `P0-REG-002`, `P0-ARB-001..004`, `P0-EVT-001..004`:

- confirm the test genuinely exercises the described behavior (not a
  trivial assertion against a mock that can't fail);
- confirm `P0-REG-003`/`P0-REG-004` are unmodified and still green —
  they were already accepted at G0/G1-era work and must not have been
  "reimplemented" or drifted;
- confirm no `P0-*` ID was invented that doesn't exist in the governing
  contract (mechanically re-verify: `grep -oE "P0-[A-Z]+-[0-9]+"` against
  `GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` and diff against what the
  implementation claims to satisfy).

---

# 5. Capability/provider identity separation

This is the audit's highest-priority structural check.

- Is `capability_id` ever derived from, or interchangeable with,
  `provider_id`?
- Does changing which provider realizes a capability change the
  capability's own identity anywhere in the model or tests?
- Is there a real test that swaps a capability's provider mid-scenario and
  asserts every historical reference by `capability_id` is unaffected?
- Are capability IDs stable, dotted, domain-first strings (or an
  equivalent structured stable identifier) — never positional, never a
  provider's D-Bus interface name, never a randomly regenerated UUID?

If capability identity and provider identity are conflated anywhere this
is `FAIL — CAPABILITY/PROVIDER IDENTITY CONFLATED`.

---

# 6. Determinism audit

- Locate the arbitration function/logic. Manually construct two inputs
  that are logically identical but differently ordered (reversed provider
  list, differently-ordered map construction) and confirm — by reading the
  code, and ideally by running the existing tests plus one of your own —
  that the decision is identical.
- Check for any `HashMap`/`HashSet` iteration whose order could influence
  a decision without an explicit tie-breaking rule keyed on stable IDs.
- Check for any randomness (`rand`, UUID generation used as a *decision*
  input rather than an opaque record ID) in the arbitration path.

If a genuine ordering- or iteration-dependent nondeterminism is found,
verdict is `FAIL — NONDETERMINISTIC CORE MODEL`.

---

# 7. Single-writer / arbitration audit

Check each arbitration invariant from TDD contract §13 against real tests,
not just prose claims:

1. two providers never simultaneously receive write ownership for one
   exclusively-owned capability (P0-ARB-001) — find the test, confirm it
   would actually fail if this were violated (mutate the implementation
   in your head/scratch copy and check the test would catch it);
2. ambiguous ownership fails closed (`write_permitted = false`,
   P0-ARB-002) — confirm no code path defaults ambiguity to `true`;
3. provider absence produces degraded read-only capability, never a
   guessed write owner;
4. `decision_reason` is genuinely populated and inspectable, not an empty
   placeholder;
5. a provider-ownership change is mechanically detectable as invalidating
   stale precondition state (P0-ARB-003), at the data-model level only —
   confirm this was **not** used as an excuse to build actual G4
   transaction-state transitions.

Also check: is `current_owner` able to represent no-writer,
Guardian-owned-writer, provider-owned-writer, external-writer, and
conflict as genuinely distinct states? Is "provider exists" ever treated
as equivalent to "provider owns writes" anywhere?

If the single-writer model cannot actually prevent two simultaneous
writers, or silently resolves ambiguity to a writer, verdict is
`FAIL — SINGLE-WRITER MODEL UNSAFE`.

---

# 8. Authorization ownership and privilege/access requirement audit

This gate's central structural check is that these are **two independent
dimensions**, each with its own field/type, never one field representing
both.

## 8.1 Authorization ownership (Dimension A — `authorization_mode`)

- Confirm `authorization_mode` (or equivalent) has exactly the three
  states G2 established: `NoAuthorizationRequired` /
  `ProviderOwnedAuthorization` / `GuardianOwnedAuthorization` — not
  collapsed into a boolean, not expanded with an unevidenced fourth state.
- Confirm this field answers only "who performs/owns authorization?" —
  never "what OS privilege is required?" and never "is the current caller
  authorized?"
- Confirm `ProviderOwnedAuthorization` is never rendered or documented as
  "Guardian is privileged" anywhere (code, doc comments, tests).
- Confirm an unrecognized wire value for this specific field fails closed
  via a typed parse/deserialization error, and does **not** become any of
  the three governed states, and does **not** silently gain a fourth
  runtime `Unknown` variant unless the governing contract is shown to
  require one (implementation handoff §10).

## 8.2 Privilege/access requirement (Dimension B — `privilege_requirement`
or equivalent)

- Confirm a **separate** typed field/model exists for "what OS-level
  privilege/access does this operation require, independent of who
  authorizes it?" — reusing the G2 inventory's own category names (no
  direct privilege / specific file-device access / specific Linux
  capability / root-system privilege / unknown-requires-host-research).
- Confirm this dimension carries a real runtime `Unknown` variant (not
  merely a parse-error fallback) — the G2 inventory has 8 genuinely
  unresearched rows, and that is legitimate information the model must be
  able to hold, not an error state to reject.
- Confirm `RootOrSystemPrivilege` is never rendered or documented as
  implying `GuardianOwnedAuthorization`, and vice versa.

## 8.3 Cross-check against the G2 inventory (both dimensions, separately)

- Cross-check `authorization_mode` against all 24 rows of
  `docs/evidence/g2/PRIVILEGE_REQUIREMENT_INVENTORY.md`: does every row's
  authorization-ownership classification map onto this enum without loss?
- **Separately**, cross-check `privilege_requirement` against the same 24
  rows: does every row's privilege/access classification (including all 8
  `unknown — requires host research` rows remaining `Unknown`) map onto
  this model without loss?
- Confirm these are reported as two distinct mappings in the completion
  report, not one combined claim.
- Spot-check the `power-profiles-daemon (HoldProfile)` row specifically:
  it must land on `authorization_mode = GuardianOwnedAuthorization` *and*
  `privilege_requirement = NoDirectPrivilege` simultaneously — if the
  implementation cannot represent this combination, that is the exact
  defect this handoff was repaired to catch.

## 8.4 Independence proof

- Confirm a test exists proving the two fields serialize/deserialize and
  vary independently (implementation handoff §16.1, item 5) — changing one
  dimension's value must not affect the other's stored value or its own
  round-trip correctness.
- Confirm changing `privilege_requirement` never alters `capability_id`,
  and changing `authorization_mode` never alters `provider_id`
  (implementation handoff §16.1, items 7–8).

If any of the above shows the two dimensions collapsed into one field, one
dimension silently implying a value for the other, or an unknown state on
one dimension forcing a false-known or false-unknown state on the other,
verdict is `FAIL — AUTHORIZATION / PRIVILEGE DIMENSIONS CONFLATED`.

---

# 9. G2 privilege-boundary regression audit

This is the audit's second-highest-priority structural check, since G3
runs entirely in the domain G2 designated unprivileged.

- Does any G3 type or method resemble an authorization result (e.g. an
  `authorized: bool` field on a core-owned decision struct) that a future
  privileged helper could plausibly be tempted to trust as proof of caller
  identity or authorization?
- Is there any D-Bus-exposed method introduced in this gate? (There should
  be none — G3 is internal typed models only.)
- Does anything in G3 grant, imply, or prepare privilege for
  `guardian-core` itself, in tension with ADR-002's "the core itself must
  not become elevated"?
- Does `authorization_mode` describe *what kind of authorization a
  capability needs* (correct) rather than *whether the current caller
  currently has it* (would be a category error and a potential regression
  vector)? Does `privilege_requirement` describe *what OS privilege the
  operation needs* (correct) rather than *what privilege the current
  process holds* (also a category error)?

If G3 introduces anything that could let a core-owned value be mistaken
for privileged authorization/identity proof, verdict is
`FAIL — G2 PRIVILEGE BOUNDARY REGRESSION`.

---

# 10. Scope-leak audit

Search for and rule out:

- a real G4 transaction state machine (`VALIDATING`→`APPLYING`→...
  runtime transitions, real `Apply`/`Rollback` execution) — only ID/
  reference plumbing should exist;
- real provider adapters for UDisks, NetworkManager, systemd, NVML,
  UPower, thermald, fwupd, or any other G8-scope integration;
- GUI/TUI/CLI/indicator/packaging code;
- any public D-Bus surface change beyond G0's `ContractVersion`/
  `ServiceState` and G2's helper's bounded write method.

If any of these exist, verdict is `FAIL — G4/G8 SCOPE LEAK`.

---

# 11. Unknown-handling audit

Confirm the two authorization/privilege dimensions follow their own,
different rules (implementation handoff §10) — do not accept one blanket
"unknown handling" claim covering both:

- `authorization_mode` specifically: an unrecognized wire value produces a
  typed parse/deserialization error and fails closed — it does **not**
  become one of the three governed states, and does **not** gain a runtime
  `Unknown` fourth variant unless the governing contract is shown to
  require one. If the implementation added an `Unknown` variant to this
  specific enum without such justification, that is a finding — flag it.
- `privilege_requirement` and every other governed enum
  (`availability`, `health`, `boot_availability`, `rollback_kind`,
  `current_owner`, incident `status`, event `severity`): confirm a real
  test exists that feeds an unrecognized value and asserts an explicit
  `Unknown`/parse-failure result — not a panic, not a silent default to a
  safe/available/authorized value.

Spot-check at least three enums by direct inspection rather than trusting
the completion report's claim, and confirm the implementation and review
handoffs' descriptions of `authorization_mode`'s specific rule actually
match what was built (§10 requires the two handoff documents to describe
the *same* rule — if the code disagrees with either document, that is
itself a finding).

---

# 12. Serialization audit

- Confirm round-trip tests exist for every model expected to cross a
  boundary.
- Confirm no internal G3 model was exposed on the public D-Bus surface.
- Confirm enum wire representations are explicit (not relying on
  `Debug`-format strings as a de facto wire format).

---

# 13. Adversarial questions (mirror the implementation handoff's §16, verify each independently)

## 13.1 Authorization/privilege dimension questions (mirrors implementation handoff §16.1)

a. does an unrecognized/future `privilege_requirement` wire value silently
   become `NoDirectPrivilege`?
b. is `authorization_mode = ProviderOwnedAuthorization` ever conflated
   with "Guardian holds elevated privilege"?
c. does `authorization_mode = GuardianOwnedAuthorization` imply
   `privilege_requirement = RootOrSystemPrivilege` anywhere (check the
   `power-profiles-daemon HoldProfile` fixture specifically — it must
   prove the opposite)?
d. does `privilege_requirement = RootOrSystemPrivilege` imply
   `authorization_mode = GuardianOwnedAuthorization` anywhere?
e. do `authorization_mode` and `privilege_requirement` round-trip
   independently through serialization (test every combination of known/
   unknown across both)?
f. does an unrecognized wire value for `authorization_mode` specifically
   fail closed via a typed parse error, rather than becoming
   `NoAuthorizationRequired` or gaining an unjustified runtime `Unknown`
   variant?
g. does changing `privilege_requirement` on a capability ever alter its
   `capability_id`?
h. does changing `authorization_mode` on a capability ever alter its
   `provider_id`?

Report each as confirmed safe / confirmed unsafe / not applicable, with
the specific file/test supporting the conclusion — same as §13.2 below.

## 13.2 General adversarial questions

1. reversed provider order — same decision?
2. same capability, two providers — deterministic, explained authority
   selection?
3. active writer disappears — `current_owner` correctly becomes no-writer?
4. two providers claim exclusive ownership — explicit conflict, not
   silently resolved?
5. read-only observer ever mistakable for a writer?
6. provider-owned authorization ever mistakable for Guardian privilege?
7. unknown privilege value ever silently becomes "safe"?
8. any capability/event/incident ID regenerates randomly on reload?
9. capability ID changes when its provider changes?
10. event loses provenance/source_provider when normalized?
11. incident stores a mutable reference/array position instead of a
    stable event_id?
12. unknown future enum value crashes deserialization instead of
    producing an explicit unknown?
13. unavailable provider silently falls back to a lower-authority writer
    without that being visible in `decision_reason`?
14. external writer state — does Guardian's arbitrator ever try to
    silently claim ownership over it?
15. does anything let a future privileged helper trust an
    `ArbitrationDecision` (or similar) as authorization proof?
16. does any core data model gain a privileged-mutation method?
17. did transaction Apply/Rollback runtime logic leak in beyond ID/
    reference plumbing?
18. did a real provider call leak into a test fixture instead of a
    deterministic fake?

Report each as: confirmed safe / confirmed unsafe / not applicable, with
the specific file/test that supports the conclusion.

---

# 14. Validation

Run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Report exact pass/fail counts. Confirm the pre-G3 baseline (23 tests, 0
failed) still passes unmodified, plus the new G3 tests, all green.

---

# 15. Required report

1. Verdict
2. Exact candidate (HEAD / branch / origin/main / working tree / G0 tag /
   G1 tag / G2 tag / G3 tag [must be absent] / ancestry)
3. Normative contract audit (§4)
4. Capability/provider identity separation (§5)
5. Determinism audit (§6)
6. Single-writer/arbitration audit (§7)
7. Authorization ownership and privilege/access requirement audit (§8),
   reporting §8.1–§8.4 separately, including the two distinct G2-inventory
   mapping tables (one per dimension) and the independence-proof result
8. G2 privilege-boundary regression audit (§9)
9. Scope-leak audit (§10)
10. Unknown-handling audit (§11)
11. Serialization audit (§12)
12. Adversarial questions (§13), one line each
13. Blocking findings (file/test ID/problem/evidence/why it matters/
    required correction)
14. Non-blocking findings (same structure)
15. Validation results
16. Git state (confirm no changes made by this review)
17. Recommended next action — exactly one of: Tag G3 and prepare G4 gate.
    / Repair G3 and re-review. / Reconsider a specific model's design. /
    Resolve architecture ambiguity before G4.

Then STOP. Do not create a G3 tag. Do not begin G4.
