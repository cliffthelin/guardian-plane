# Guardian Phase 0 Implementation Handoff
## G4 — Transaction Engine Only

**Audience:** Primary coding agent
**Scope:** **G4 — Transaction Engine** only
**Stop condition:** the normative G4 tests below are green, or the gate is
honestly reported partial/blocked with the specific missing evidence named.
Do **not** begin G5, real providers, the privileged helper, GUI/TUI/CLI, or
packaging.
**Governing contract:** `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`
§2 (GP-04, GP-05, GP-06, GP-08, GP-10 especially), §14 (transaction state
machine), §15 (transaction observation contract), §16 (error model), §23
(persistent state layout), §36 (P0-TXN-001..012)
**Prerequisite:** G3 tagged at `phase0-g3-core-data-models` (commit
`d21d0c4f41f7032db969b0b50c20c72c17b836c5`). Confirm this tag exists and
`HEAD` descends from it before starting.

---

# 1. Mission

G4 turns Guardian's G3 data models into a real, deterministic transaction
lifecycle: every risky mutation becomes bounded, authorized, observable,
recoverable, and auditable, exactly as GP-05 requires. This is still not a
real-provider gate — everything is exercised against deterministic fixture
providers/adapters, matching G3's own testing discipline.

The desired result is a repository in which:

- a `TransactionRecord` and the canonical state machine (§18 below) exist
  as typed models in `guardian-core`, using exactly the contract's states
  and transitions — no invented states, no arbitrary `set_state`;
- `TransactionId` has its own deliberately-chosen identity semantics (§17),
  not inherited from `CapabilityId`'s domain-identity validator;
- every transaction genuinely follows Snapshot → Validate → Authorize →
  Apply → Observe → Confirm → Commit, or fails onto a defined path that
  reaches `REJECTED`/`FAILED`/`ROLLED_BACK`/`ROLLBACK_FAILED`, with real
  typed logic behind each step — not a rubber-stamp pipeline where every
  step trivially succeeds;
- G3's two carried-forward findings become real, resolved decisions: NB-1
  (unknown privilege/access blocks `Apply`, not silently ignored) and NB-2
  (a defined owner and mechanism for `revision`, not a caller-trusted
  number);
- TOCTOU revalidation immediately before `Apply` is real and tested, not
  merely validated once at transaction start;
- lost-response/duplicate-apply risk is addressed via `idempotency_key`
  (P0-TXN-009), not hand-waved;
- a persistence contract exists (schema, versioning, atomic write, restart
  recovery) sufficient to satisfy P0-TXN-011, without implementing
  production packaging;
- `P0-TXN-001..012` are green, or the gate is reported blocked with the
  specific missing evidence named;
- no G5 Diagnostic Budget Manager, no G7 production daemon integration, no
  G8 real provider, no client, no packaging exists anywhere in the result.

Then stop.

---

# 2. Read before changing code

1. `AGENTS.md`
2. `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`
   - §2 GP-04 (single-writer), GP-05 (every write is transactional), GP-06
     (fail closed), GP-08 (preserve evidence), GP-10 (explain decisions)
   - §4 Required repository layout (`transaction/` under `guardian-core`)
   - §10 Risk taxonomy (reused, not reinvented)
   - §13 Provider Arbitrator (G3, consumed here — do not modify it)
   - §14 Transaction state machine + transaction record
   - §15 Transaction observation contract
   - §16 Error model
   - §23 Persistent state layout
   - §36 P0-TXN-001..012
3. `docs/guardian/30_TDD/GUARDIAN_G3_IMPLEMENTATION_HANDOFF.md` and
   `GUARDIAN_G3_INDEPENDENT_REVIEW_HANDOFF.md` — G3's data models and the
   audit process this gate must survive the same way.
4. `docs/evidence/g3/G3_MILESTONE.md` — **read this in full before writing
   any code.** Its "Forward constraints for G4+" section (NB-1..NB-4) is
   binding on this handoff; §14/§15/§17/§20/§32 below are this handoff's
   resolution of those constraints and must not be silently reopened or
   ignored.
5. `docs/guardian/20_Control_Plane/Transaction_Engine.md`
6. `docs/guardian/20_Control_Plane/Privilege_and_Authorization.md`
7. `docs/adr/ADR-002-guardian-privilege-topology.md` — the G2 boundary
   this gate must not regress (§12 below).

---

# 3. Normative G4 contract IDs

The **only** normative test IDs in scope for G4 are `P0-TXN-001..012`,
mechanically confirmed as the contract's complete transaction-test group
(`grep -oE "P0-[A-Z]+-[0-9]+" docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md
| sort -u` — the full ID list contains no `P0-CAP-*`/`P0-PROV-*`/`P0-INC-*`
group, confirming G3's boundary, and the transaction group is exactly
`P0-TXN-001..012`, nothing more). Do not invent new IDs.

```text
P0-TXN-001 — happy path                    Transaction reaches COMMITTED only through valid state transitions.
P0-TXN-002 — validation failure             Invalid precondition ends in REJECTED and no apply occurs.
P0-TXN-003 — authorization denied           Denied authorization performs no apply.
P0-TXN-004 — apply failure                  Apply error reaches FAILED or rollback path according to whether state changed.
P0-TXN-005 — observation failure            Provider call success followed by failed health observation does not commit.
P0-TXN-006 — rollback success               Failed observation with supported rollback ends ROLLED_BACK.
P0-TXN-007 — rollback failure               Rollback failure ends ROLLBACK_FAILED.
P0-TXN-008 — terminal immutability          A terminal transaction cannot re-enter an active state.
P0-TXN-009 — idempotent retry               Same idempotency key cannot perform the same write twice.
P0-TXN-010 — stale resource identity        Resource replacement between validation/apply blocks apply.
P0-TXN-011 — daemon restart                 Persisted nonterminal transaction is recovered into a defined state after daemon restart.
P0-TXN-012 — client disconnect              Client disappearance does not lose the audit record.
```

The governing contract is judged **sufficient** for this gate — it
specifies the exact state machine, the exact transaction record shape, the
exact observation-policy shape, and 12 concrete acceptance tests. Do not
report `G4 BLOCKED — GOVERNING CONTRACT INSUFFICIENT` merely because some
implementation details (exact `TransactionId` format, exact persistence
schema version, exact revision-ownership mechanism) are left to this
handoff to specify — that is normal implementation latitude, not a contract
gap. Only escalate that status if inspection reveals a genuine textual
contradiction, not an open design choice this handoff already resolves.

---

# 4. Transaction state machine (§14 of the governing contract, verbatim states)

```text
CREATED
VALIDATING
VALIDATED
AUTHORIZING
AUTHORIZED
APPLYING
OBSERVING
COMMITTED

ROLLING_BACK
ROLLED_BACK

REJECTED
FAILED
ROLLBACK_FAILED
EXPIRED
CANCELLED
```

Use exactly these states — do not rename, merge, or add states. Terminal
states (`COMMITTED`, `ROLLED_BACK`, `REJECTED`, `FAILED`,
`ROLLBACK_FAILED`, `EXPIRED`, `CANCELLED`) **must be immutable**
(P0-TXN-008): implement transitions as a function that rejects any attempt
to leave a terminal state, not as a bare public setter. No
`set_state(anything)` API may exist — only named transition methods
(`fn begin_validation(self) -> Result<...>`, etc.) that each encode their
own legal-predecessor check.

## 4.1 Legal transitions

Define the legal transition graph explicitly and reject illegal ones with
a typed error, not a panic:

```text
CREATED        → VALIDATING
VALIDATING     → VALIDATED | REJECTED
VALIDATED      → AUTHORIZING
AUTHORIZING    → AUTHORIZED | REJECTED
AUTHORIZED     → APPLYING
APPLYING       → OBSERVING | FAILED | ROLLING_BACK
OBSERVING      → COMMITTED | ROLLING_BACK | FAILED
ROLLING_BACK   → ROLLED_BACK | ROLLBACK_FAILED
(any nonterminal) → CANCELLED | EXPIRED   (explicit external cancellation / deadline)
```

`APPLYING → ROLLING_BACK` covers the case where `Apply` itself reports a
failure that is known to have partially mutated state (§7 below).
`OBSERVING → ROLLING_BACK` covers the case where the provider call
succeeded but observation determined the postcondition was not met
(P0-TXN-005/006). A required test must prove at least one illegal
transition (e.g. `COMMITTED → APPLYING`) is rejected, not merely that legal
ones succeed.

## 4.2 `TransactionRecord` (§14.1, verbatim fields — use guardian-core types
where a G3 type already exists for the concept)

```rust
TransactionRecord {
    transaction_id,       // new type, §17 below — NOT CapabilityId
    idempotency_key,      // §27 below
    action_type,          // typed, not a free string — see §22
    risk_class,            // guardian_core::risk::Risk, reused unchanged
    initiating_bus_name,
    initiating_session,
    provider_id,           // guardian_provider_api::ProviderId, reused unchanged
    capability_id,          // guardian_provider_api::CapabilityId, reused unchanged

    created_at,
    deadline,

    pre_state,              // snapshot, §19
    validation_results,     // §20
    arbitration_result,     // guardian_core::arbitration::ArbitrationDecision, reused unchanged
    authorization_result,   // §21 -- NOT a re-implementation of G1/G2's real authorization

    requested_change,       // typed per action_type, never opaque payload
    provider_request,
    provider_response,

    observation_policy,     // §23
    observations,

    commit_result,
    rollback_result,        // §25/§26

    incident_ids,           // Vec<guardian_provider_api::IncidentId>, reused unchanged
}
```

`arbitration_result` reusing G3's real `ArbitrationDecision` type directly
(not a copy/paraphrase) is required — this is exactly how NB-1/NB-2 get
resolved: the transaction record carries the *actual* G3 decision, including
its `revision`, so later code can check it against current state rather
than trusting a bare number.

---

# 5. G2 boundary preservation (read this before writing `authorization_result`)

G4 may include a transaction-visible state such as `Authorized` (it is
already in the canonical state machine, §4). This records **that the
transaction's authorization step concluded successfully** — it is not, and
must never become, a mechanism by which `guardian-core` performs real
authorization itself. Concretely:

- The transaction engine's "Authorize" step (§21) **coordinates** a request
  to whatever real authorizer exists (G1's `Authorizer` trait,
  `PolkitAuthorizer`, or — in this gate's deterministic fixtures — a test
  double implementing the same trait) and records the *outcome*. It does
  not reimplement or bypass real polkit `CheckAuthorization`.
- `authorization_result` on `TransactionRecord` is a record of what
  happened, not a capability that can be forged/replayed to skip real
  authorization on a later attempt. Each new `Apply` attempt against a
  privileged operation must still pass through the real, independent
  privileged-boundary check — see GP-05/§14.2 ("Validation is repeated at
  the privileged boundary immediately before apply").
- `ArbitrationDecision.write_permitted` (G3) continues to mean only
  "control-plane policy permits proceeding." Nothing in G4 may promote it,
  or `authorization_ownership = Known(GuardianOwnedAuthorization)`, to mean
  "the caller is authorized." Both remain what G2/G3 already established.
- If any G4 type has a field or method whose name or shape could plausibly
  be read as "authorization proof forwarded to the privileged helper"
  (e.g. anything the helper would be tempted to trust instead of
  performing its own real `CheckAuthorization`), that is a **G2 regression**
  — do not build it. The privileged helper's independent-authorization
  invariant, established at G1/G2 and unchanged through G3, is unchanged
  by G4 too.
- Background transactions cannot unexpectedly prompt for interactive
  authentication if G1's P0-AUTH-003 rule prohibits it in that context —
  reuse G1's existing `interactive` flag semantics on
  `AuthorizationRequest`, do not invent a second one.

---

# 6. NB-1 resolution — unknown privilege/access blocks `Apply`

**Required conceptual invariant:** `PrivilegeRequirement::Unknown` on the
capability a transaction targets → **no `Apply`.** Concretely: the
`Validate` step (§20) MUST check the target capability's
`privilege_requirement` (from its `CapabilityRecord`, obtained via the
snapshot, §19) and produce `REJECTED` (not `FAILED`, since this is a
precondition failure, matching P0-TXN-002's semantics) if it is `Unknown`.

Do not invent an exception where a provider-owned operation is assumed to
make Guardian-held privilege irrelevant, and therefore skip this check.
`authorization_ownership = Known(ProviderOwnedAuthorization)` and
`privilege_requirement = Unknown` are independent facts (G3, unchanged) —
a provider performing its own authorization does not tell you what OS-level
access the operation itself needs, and no G3 evidence proves that
"provider-owned authorization" implies "privilege requirement doesn't
matter." If a future gate discovers a real, typed, evidence-backed reason
this exception is safe for a specific capability class, that is a separate,
explicitly-justified decision for that gate to make — G4 does not manufacture
it here.

Required test: a transaction targeting a capability with
`privilege_requirement = Unknown` reaches `REJECTED` at `Validate`, never
reaches `APPLYING`, regardless of `authorization_ownership`'s value.

---

# 7. NB-2 resolution — revision ownership

**G4 owns `revision` generation, not the transaction caller.** Concretely:

- The party that computes `ArbitrationDecision` (G3's `arbitrate()`, called
  from wherever G4's transaction engine invokes arbitration) is the only
  legitimate source of a `revision` value for a given `capability_id` at a
  given moment. A transaction's `Snapshot` step captures the
  `ArbitrationDecision` (including its `revision`) as part of `pre_state`
  (§19) — it does not accept a caller-supplied `revision` as input.
- `Validate` (§20) MUST re-run arbitration and compare the freshly-computed
  `revision` against the one captured in `pre_state`. A mismatch is a
  precondition failure → `REJECTED` (P0-TXN-002), and, if the resource
  identity itself changed, specifically exercises P0-TXN-010.
- `Apply` (§22) MUST re-check `revision` **immediately before mutation**
  (§16's TOCTOU requirement) — not rely on the `Validate`-time check alone.
  A changed `revision` between `Validate` and `Apply` blocks `Apply` and
  transitions to `FAILED` or `ROLLING_BACK` per §4.1, never proceeds.
- For this gate's deterministic fixtures, `revision` MAY be implemented as
  a monotonic counter owned by a small in-memory "capability registry"
  fixture that G4's own tests control directly (bump it between `Validate`
  and `Apply` to prove the recheck actually blocks the write) — this is
  the concrete, minimal mechanism this handoff requires; it does not need
  to be the production capability-registry implementation (that belongs to
  whichever future gate builds the real registry service). What matters is
  that **G4's own code**, not a test's hand-constructed input, is what
  produces and checks `revision`.
- Restart/recovery (§26): a recovered transaction must re-derive current
  `revision` from the (fixture) registry at recovery time — it must never
  trust a `revision` value read back from disk as still current without
  re-comparison.

Required tests: (a) `revision` unchanged between `Validate` and `Apply` →
transaction proceeds; (b) `revision` bumped between `Validate` and `Apply`
(simulating a concurrent ownership change) → `Apply` is blocked, not
silently proceeded with.

---

# 8. `TransactionId` semantics (NB-4 resolution)

Do not reuse `CapabilityId`'s dotted-domain validator. `TransactionId` is a
**generated record identity**, not a semantic domain identity — it answers
"which specific transaction attempt is this," not "what capability/provider
concept does this name." Required properties: unique, persistent, stable
once created, serializable, safe across restart, never derived from `Vec`
position or discovery order.

Implement `TransactionId` as its own newtype (not the `stable_id!` macro
used for `CapabilityId`/`ProviderId`/`EventId`/`IncidentId` in G3) wrapping
a ULID or UUIDv4-shaped string — pick one and justify it in the completion
report (a ULID is lexicographically sortable by creation time, which is
useful for audit-log ordering; a UUIDv4 has no such property but is simpler
and has no host-clock dependency; either is acceptable, do not agonize).
Validate only that the wrapped string matches the chosen format's shape —
do not impose `CapabilityId`'s letter-led/lowercase/dotted rule on it.

**While implementing this, also resolve whether `EventId`/`IncidentId`
should be corrected to use the same generated-record-identity pattern
instead of the `stable_id!` domain-identity macro (G3 NB-4).** This handoff
requires an explicit decision, documented in the completion report, choosing
one of:

- (a) leave `EventId`/`IncidentId` exactly as G3 built them (acceptable if
  this gate's fixtures never need to construct one from a generated
  UUID/ULID-shaped value — confirm this is actually true before choosing
  it, don't assume it);
- (b) correct `EventId`/`IncidentId` in a small, separate, explicitly-labeled
  commit (not silently folded into the main G4 feature commits) to use the
  same generated-record-identity pattern as `TransactionId`.

Do not modify `crates/guardian-provider-api/src/ids.rs` as an incidental
side effect of building `TransactionId` — if (b) is chosen, it is a
deliberate, separately-reviewable change with its own justification, per
`G3_MILESTONE.md`'s explicit instruction not to let this spread by
accident.

---

# 9. Transaction preconditions (§14.2, §36 P0-TXN-002/010)

`Validate` must check, at minimum, before allowing `AUTHORIZING`:

- capability still exists and is not `Availability::Unavailable`;
- provider identity matches what `Snapshot` captured (else P0-TXN-010);
- `ArbitrationDecision.write_permitted` is (still, freshly-computed) true
  for this capability/provider, including the `revision` recheck (§7);
- `authorization_ownership` is `Known(...)` — not `Unknown` (an unknown
  authorization architecture already fails closed at G3's own arbitration
  layer, but `Validate` must not rely solely on a `pre_state` snapshot that
  could be stale; recompute or re-verify);
- `privilege_requirement` is not `Unknown` (§6);
- rollback capability is disclosed (`RollbackKind` present on the decision)
  — required by §14.2 ("Rollback capability is disclosed before
  authorization"), not merely present in a struct field nobody reads;
- the provider (fixture) actually supports the requested action type
  (`MutableCapabilityAdapter` returns something other than `Unsupported`
  for a dry inspect/validate call, per G3's typed shape).

Missing or unknown safety-critical prerequisites fail closed → `REJECTED`.
Not every field needs to be independently mandatory if the governing
contract doesn't require it (e.g. `initiating_session` may legitimately be
absent for a non-interactive background transaction) — use judgment, but
the safety-relevant items above are non-negotiable per GP-06.

---

# 10. TOCTOU — explicit revalidation points

`Snapshot → Validate → Authorize → time passes → Apply` — state may change
at every step. This handoff requires:

- a **fresh** re-check of `revision`/ownership immediately before `Apply`
  begins (§7) — not the `Validate`-time value reused;
- a defined behavior for each of: provider disappears between `Validate`
  and `Apply`; ownership changes between `Validate` and `Apply`; an
  external writer appears between `Validate` and `Apply`; the resource
  identity itself changes (P0-TXN-010); the authorization context becomes
  stale (e.g. a long deadline elapses — see `deadline` on
  `TransactionRecord`); the (fixture) helper "restarts" mid-transaction
  (simulate this as a fixture behavior, not a real process restart);
  the (fixture) core "restarts" mid-transaction (§11 below covers the
  real recovery-on-restart case; this item is about a fixture explicitly
  modeling loss of in-memory helper state between steps).
- each of the above must be a real test with a real, specific expected
  outcome (block `Apply`, or transition to a specific failure state) — not
  a single generic "TOCTOU is handled" claim.

---

# 11. Snapshot contract (§19 informal name; §14.1's `pre_state`)

`Snapshot` must capture enough to support later rollback/recovery:
`transaction_id`, `capability_id`, `provider_id`, the `ArbitrationDecision`
(including `revision`), prior resource state (whatever the fixture provider
reports via `inspect()`), `rollback_kind`, `risk_class`, and a timestamp.
Snapshot failure (fixture configured to fail) MUST prevent `Apply` — no
fake empty snapshot where rollback is later claimed as supported. Required
test: a snapshot-fails fixture never reaches `APPLYING`.

---

# 12. Validate contract

Established in §9 above. Validation is **not** authorization — it
establishes that transaction assumptions remain true; it does not decide
whether the caller may proceed. Keep the two steps and their typed results
distinct on `TransactionRecord` (`validation_results` vs.
`authorization_result`).

---

# 13. Authorize contract

Preserve G1/G2 ordering: `Authorize` strictly before `Apply` — no mutation
occurs before a real authorization outcome is obtained. The transaction
engine coordinates the request (§5); it does not replace the privileged
boundary. Denied authorization (P0-TXN-003) performs no apply and reaches
`REJECTED`. Interactive-vs-background behavior must remain explicit and
reuse G1's existing `interactive` flag — no new prompting mechanism.

---

# 14. Apply contract

`Apply` must be typed, bounded, and provider-specific through G3's typed
`MutableCapabilityAdapter` shape — never `RunCommand`/`RunShell`/arbitrary
argv/arbitrary file or sysfs path/opaque privileged payload, exactly as G2's
forbidden-pattern list already establishes and G3's `ActionRequest` already
enforces structurally (it is a typed placeholder, not a generic string
carrying arbitrary meaning — if G4 needs richer typed action payloads than
G3's placeholder `ActionRequest(String)`, define a real typed
`action_type`-specific payload here, but keep each variant narrow and
enumerable, never a generic "opaque bytes" field).

Apply failure (P0-TXN-004): the transition target depends on whether state
is known to have changed — reaches `FAILED` if nothing changed, or enters
`ROLLING_BACK` if the fixture reports a partial/uncertain mutation. This
distinction must be a real, tested fixture behavior (configurable "apply
fails cleanly" vs. "apply fails after partially mutating"), not asserted
in prose only.

Use only deterministic fixture adapters (§34 below) — no real provider
integration.

---

# 15. Observe contract (§15 of the governing contract)

After `Apply`, the engine must **not** assume success merely because the
provider call returned success (§15, verbatim: "A provider returning
'method call succeeded' MUST NOT automatically mean the transaction
succeeded"). Define, per transaction, an `observation_policy`:
`expected_properties`, `forbidden_properties`, `minimum_observation_duration`,
`maximum_observation_duration`, `health_checks[]`, `commit_condition`,
`rollback_condition` — all present as typed fields, not a free-form check
function with no inspectable shape. `observe()` on the fixture adapter
returns a real, distinguishable match/mismatch/ambiguous outcome (P0-TXN-005).
Do not implement G5's Diagnostic Budget Manager here — no PSI-driven cost
scheduling, no diagnostic escalation logic; `Observe`'s job in G4 is purely
"did the expected state occur," using the fixture's reported state.

---

# 16. Confirm contract

`Confirm` answers whether the observed result satisfies the transaction's
postcondition strongly enough to commit — keep "provider call succeeded"
structurally distinct from "desired state confirmed" (two different fields/
states, not one boolean). If confirmation fails, the rollback policy (§17
below) must be explicit and reached, not left ambiguous.

---

# 17. Rollback contract (§25/§26 of the requester's brief, using G3's `RollbackKind`)

Reuse G3's `RollbackKind` (`Native | Emulated | BestEffort | None`)
unchanged. Define behavior for each:

- **Native**: the fixture provider genuinely restores prior state from the
  snapshot; rollback is expected and testable (P0-TXN-006 uses this).
- **Emulated**: Guardian synthesizes restoration from the captured
  snapshot itself (e.g. re-`Apply`ing the inverse typed action) — model
  this as a real, distinct code path in the fixture, not silently identical
  to Native.
- **BestEffort**: rollback is attempted but the outcome may remain
  ambiguous — the resulting state must be representable as "rollback
  attempted, success unconfirmed," not forced into either `ROLLED_BACK` or
  `ROLLBACK_FAILED` when the fixture genuinely can't tell.
- **None**: the transaction must expose, structurally, that rollback
  cannot be guaranteed — do not claim full safety for `RollbackKind::None`.
  A `VERY_HIGH`-risk action (TDD contract §10) with `RollbackKind::None`
  is exactly the case §10's "never automated by default; explicit user
  acknowledgement required" language exists for — this handoff does not
  build that policy layer (that's a client/UI concern, later gates), but
  the transaction *model* must make the fact ("no rollback guarantee")
  inspectable so a later gate can enforce policy on it.

**Rollback failure is a first-class state** (P0-TXN-007, `ROLLBACK_FAILED`).
Do not collapse "Apply failed, rollback succeeded" with "Apply failed,
rollback failed," or "Apply succeeded, Confirm failed, rollback failed" —
the last of these represents an unresolved incident requiring recovery, and
must be distinguishably represented (e.g. via `rollback_result`'s own typed
outcome plus a linked `incident_ids` entry) so a future Event/Incident
integration (§18 below) can surface it, without G4 itself implementing that
surfacing.

---

# 18. Lost response / duplicate apply (P0-TXN-009)

Address explicitly: helper/provider applies a mutation, the response is
lost, the caller retries. Required mechanism for this gate:
`idempotency_key` on `TransactionRecord` — the engine must refuse to
`Apply` a second time for a transaction whose `idempotency_key` already has
a recorded `APPLYING`/post-`APPLYING` outcome, returning the existing
result instead of re-executing. Do not invent a universal cross-provider
idempotency guarantee beyond this — real providers may differ (that's G8's
problem to solve per-provider); this gate's obligation is that **Guardian's
own transaction engine** never re-applies the same `idempotency_key` twice
against its own fixture adapters. Required test: retry with the same
`idempotency_key` after a simulated lost response does not cause a second
`Apply` call to the fixture.

---

# 19. Persistence contract (§23 of the governing contract)

Define, without implementing production packaging:

- **When first persisted**: at minimum, a transaction must be durably
  recorded before `Apply` begins (so a crash during/after `Apply` can be
  recovered/detected, §20) — persisting only at `COMMITTED` is insufficient.
- **Atomic write strategy**: write-to-temp-file-then-rename, or an
  equivalent atomicity guarantee — no partial-record risk from a crash
  mid-write.
- **Schema/version**: every persisted record carries an explicit schema
  version field; deserializing a record from a newer/unknown schema
  version must fail closed (typed error), never silently misinterpret
  fields (mirrors G3's Rule-1/Rule-3 unknown-handling discipline).
- **Restart load**: on startup, all persisted nonterminal transactions are
  read back (P0-TXN-011).
- **Corrupt record behavior**: a corrupt/unparseable persisted record must
  not be treated as safe-to-resume — it surfaces as a distinct
  "requires recovery/human handling" outcome (§20), never silently
  discarded or silently treated as committed/rolled-back.
- **Partial write behavior**: covered by the atomic-write requirement
  above — a partial write must never be readable as a seemingly-valid
  record.
- For this gate, a local filesystem fixture under a test-controlled
  temporary directory is sufficient — do not wire up real
  `/var/lib/guardian` paths or systemd `StateDirectory=` (that's packaging,
  out of scope), but the *contract* (schema, atomicity, versioning) must be
  real and tested, not simulated only in memory with no serialization at
  all.

---

# 20. Crash/restart recovery (P0-TXN-011)

G2's real-host evidence established that privileged-helper memory resets
across restart — transaction truth cannot depend solely on in-memory state.
On recovery, Guardian must be able to distinguish, for each recovered
nonterminal transaction:

```text
safe to resume            (nonterminal, no Apply attempted yet — re-run Validate/Authorize)
must observe               (Apply attempted, no Observe result recorded — re-run Observe only)
must rollback               (Apply attempted, Observe failed/ambiguous, rollback not yet attempted)
already committed            (Apply + Observe + Confirm all recorded successful — nothing to do)
state ambiguous                (Apply attempted, response lost, no Observe possible from fixture)
requires human/recovery handling (corrupt record, or ROLLBACK_FAILED with no further automated path)
```

This gate does not need to solve every real-provider recovery scenario —
fixture adapters are sufficient — but the **state model** must support
these six distinctions as real, representable outcomes of a recovery
function, each with its own test using a persisted record constructed to
represent that exact scenario (not just the general "restart recovers
transactions" claim). Required test set: at minimum one test per
distinction above (6 tests), each constructing a specific persisted-record
fixture and asserting the recovery function classifies it correctly.

---

# 21. Event/Incident integration (using G3 models, unchanged)

Transaction state changes may emit `guardian_core::event::Event`s — e.g.
transaction started, authorization denied, apply started, apply failed,
observation mismatch, rollback started, rollback failed, transaction
committed. Reuse G3's `Event`/`normalize_key` unchanged; do not fork them.
Transaction failures (especially `ROLLBACK_FAILED`) may reference or create
incident-compatible evidence (an `Incident` linking the relevant events) —
but do not implement a correlation *engine* here (that's G5/G7 scope); a
direct, explicit link from a known failure to a known incident is
acceptable, an inferred/fuzzy correlation system is not.

---

# 22. Provenance

A `TransactionRecord` should retain enough provenance to explain which
capability, which provider, which arbitration decision (the actual
`ArbitrationDecision`, not a summary), which snapshot, and which
observations led to the outcome. `initiating_bus_name`/`initiating_session`
may be present per §14.1, but do not duplicate G1's sensitive caller
identity into the transaction record beyond what the contract's own field
list already specifies — preserve the privacy-minimization discipline G1
established (real caller resolution stays inside the authorization
boundary; the transaction record is an audit trail, not a caller-identity
store).

---

# 23. Serialization boundaries (NB-3 resolution)

Identify which G4 types genuinely cross a real boundary in this gate:
`TransactionRecord` (persistence, §19) definitely does. `Event`/`Incident`
references inside it reuse G3's existing (partial) serialization — do not
expand G3's enum serialization coverage merely for symmetry unless a
specific G4 type actually needs it (e.g. if `TransactionRecord` persists a
`RollbackKind` value, and `RollbackKind` currently has `Display` but no
`FromStr` per G3's NB-3, **this gate must add the missing `FromStr`**,
since persistence is exactly the real boundary NB-3 anticipated — this is
not "adding for symmetry," it is closing a real gap this gate's own
persistence contract exposes). For every type that does cross a boundary:
explicit stable wire representation, schema version awareness, round-trip
tests, typed parse failures for unknown values (matching each type's
established unknown-handling rule from G3, do not invent a new rule per
type). No `Debug`-format persistence anywhere.

---

# 24. Fail-closed checklist

At minimum, verify and test that none of the following silently becomes
"safe"/"authorized"/"valid"/"committed":

```text
unknown authorization ownership       (G3, reused — reconfirm still fails closed through G4's Validate)
unknown privilege requirement          (§6 — new G4 invariant)
unknown provider health
ambiguous writer                        (G3, reused)
external writer                          (G3, reused)
stale revision                            (§7)
provider unavailable
unsupported operation
snapshot failure                            (§11)
authorization failure                        (§13)
observation ambiguous                          (§15)
corrupt persistent state                        (§20)
```

---

# 25. Provider fixtures only

All G4 tests use deterministic fake providers/adapters implementing G3's
`Provider`/`MutableCapabilityAdapter` traits, with configurable behavior
for: snapshot succeeds/fails; validate succeeds/fails; authorize succeeds/
fails (via a fixture `Authorizer`, reusing G1's trait); apply succeeds/
fails; apply succeeds but response is lost (for §18's idempotency test);
observe matches/mismatches/is ambiguous; rollback succeeds/fails/is
best-effort-ambiguous; provider disappears mid-transaction; arbitration
revision changes mid-transaction. **No real UDisks/systemd/NetworkManager/
NVML/UPower/thermald/fwupd integration anywhere.**

---

# 26. TDD sequence

Strict: failing test → minimal implementation → focused pass → full
workspace pass, for each normative item. Every `P0-TXN-*` test must have a
real load-bearing failure mode — prove this the same way G3's independent
audit did: after implementing, deliberately mutate the implementation in a
`/tmp` scratch copy (never the tracked tree) for at least the adversarial
items in §27, confirm the relevant test(s) fail, then discard the mutation.
Report which mutations were performed and what they caught in the
completion report, matching G3's evidentiary standard.

---

# 27. Adversarial self-check before reporting done

1. mutation before authorization — does anything reach `Apply` without
   passing through `Authorize`?
2. stale arbitration revision accepted — does `Apply` proceed if `revision`
   changed since `Validate`?
3. caller-controlled revision trusted — can a test construct a
   `TransactionRecord` with an arbitrary `revision` and have it accepted
   without re-derivation?
4. unknown privilege permitted — does a `PrivilegeRequirement::Unknown`
   capability ever reach `APPLYING`?
5. unknown authorization permitted — does a `Knowledge::Unknown`
   capability ever reach `APPLYING`?
6. ambiguous ownership permitted — does `Ownership::Conflict` ever reach
   `APPLYING`?
7. snapshot omitted — can `Apply` proceed with no recorded `pre_state`?
8. rollback claimed but impossible — does a `RollbackKind::None` decision
   ever get silently treated as though rollback succeeded?
9. apply-success/reply-loss/retry duplicate — does retrying an
   `idempotency_key` whose response was lost cause a second real `Apply`
   call to the fixture?
10. provider disappears between `Validate` and `Apply` — is this detected,
    or does `Apply` proceed against a vanished provider?
11. core "crashes" after `Apply` (simulated) — does recovery correctly
    classify the transaction per §20, not silently drop it?
12. helper "crashes" after `Apply` (simulated) — same question from the
    privileged-boundary side.
13. restart loses a transaction — does every persisted nonterminal
    transaction actually get re-loaded, or can one silently vanish?
14. corrupt persisted record treated as safe — does a deliberately corrupted
    record ever classify as "already committed" or "safe to resume"?
15. illegal state transition — does the state machine reject
    `COMMITTED → APPLYING` (or similar) rather than silently allowing it?
16. rollback failure hidden — does `ROLLBACK_FAILED` ever get collapsed
    into `FAILED` or `ROLLED_BACK` in any code path?
17. best-effort rollback reported as success — does `RollbackKind::BestEffort`
    ever force a `ROLLED_BACK` result the fixture couldn't actually confirm?
18. `Debug`-format persistence — does any persisted record rely on `Debug`
    output as its wire format?
19. real provider leakage — does any fixture or test accidentally call a
    real system D-Bus service?
20. G5/G8 scope leakage — does any code implement diagnostic-budget logic,
    a real provider adapter, or client-facing surface?

---

# 28. Completion states

Report exactly one, honestly:

```text
G4 CANDIDATE — TRANSACTION ENGINE READY FOR INDEPENDENT AUDIT
G4 PARTIAL — CONTRACT TESTS INCOMPLETE
G4 BLOCKED — GOVERNING CONTRACT INSUFFICIENT
G4 BLOCKED — TRANSACTION SAFETY MODEL CONFLICT
```

Do not report `CANDIDATE` if any `P0-TXN-*` test is red, skipped, or
asserts against a fixture that cannot fail.

---

# 29. Completion report

Include, at minimum: which normative tests are green with exact names/IDs
(all 12 `P0-TXN-*`); the state-machine transition table actually
implemented, compared against §4.1; `TransactionId` format chosen and why
(§8); the `EventId`/`IncidentId` decision made (§8, option a or b, and
why); the exact revision-ownership mechanism built (§7); where
`PrivilegeRequirement::Unknown` is checked and by which function (§6); the
persistence schema version and atomic-write mechanism (§19); the six
recovery-classification tests and their results (§20); the §27 adversarial
self-check results item-by-item, including which were executed as real
scratch mutations vs. reasoned by inspection (mirror G3's audit-survivable
standard, don't just claim it); full `cargo fmt --check` / `cargo clippy
--workspace --all-targets --all-features -- -D warnings` / `cargo test
--workspace` output; and an explicit statement of what was deferred to G5
or G8 and why.

Then stop. Do not begin G5. Do not tag G4 — independent review happens
first, exactly as it did for G0 through G3.
