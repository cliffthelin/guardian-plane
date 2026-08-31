# Guardian Phase 0 Implementation Handoff
## G3 — Core Data Models Only

**Audience:** Primary coding agent
**Scope:** **G3 — Core Data Models** only
**Stop condition:** the normative G3 tests below are green (or the gate is
honestly reported partial/blocked with the specific missing evidence
named). Do **not** begin G4 (transaction engine), providers, real
Capability Registry/Provider Arbitrator *runtime* behavior against real
hardware, clients, or packaging.
**Governing contract:** `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`
§10 (risk taxonomy), §11 (Capability Registry schema), §12 (Provider
interface contract), §13 (Provider Arbitrator), §16 (error model), §17
(Event schema), §18 (Incident schema), §36 (P0-REG-001/002 new, P0-REG-003/004
reused, P0-ARB-001..004, P0-EVT-001..004)
**Prerequisite:** G2 tagged at `phase0-g2-privilege-topology` (commit
`87502df8e41268aec4e94635d218c8b81c82189c`). Confirm this tag exists and
`HEAD` descends from it before starting.

---

# 1. Mission

G3 gives Guardian its internal typed language: what a capability is, who
provides it, who is allowed to write it right now, what an event is, and
how events correlate into an incident. This is a **deterministic, Layer 1
data-model gate** — no real provider, no real hardware, no transaction
execution, no VM required unless a coding agent introduces host-dependent
behavior that has no business being here (in which case, remove it or
defer it, per §14 of this handoff).

The desired result is a repository in which:

- `CapabilityRecord`, the `Provider` trait, and `ArbitrationDecision` exist
  as typed models per TDD contract §11–§13, in `guardian-core` and/or
  `guardian-provider-api` per the placement analysis required in §11 below;
- capability identity and provider identity are structurally distinct
  types — a capability's identity never depends on which provider currently
  realizes it;
- the Provider Arbitrator is deterministic: identical inputs (candidate
  providers, health, ownership, priority) always produce the identical
  `ArbitrationDecision`, proven by tests that specifically try to break
  this (reversed provider order, shuffled `HashMap` iteration, etc.);
- the single-writer rule (GP-04) is representable at the data-model level:
  observers vs. the current write owner are distinguishable, "no writer"
  is distinguishable from "ambiguous owner," and ambiguity fails closed
  (`write_permitted = false`), matching P0-ARB-002;
- authorization ownership is representable using G2's established
  three-way distinction (none / provider-owned authorization / Guardian
  polkit authorization) — never collapsed into a boolean "is Guardian
  privileged";
- `Event` and `Incident` exist per TDD contract §17/§18, with stable IDs,
  monotonic-plus-wall-clock timestamps, normalized correlation keys that
  still preserve the raw source reference, and incidents that link
  multiple events without deleting them;
- every model that crosses a serialization boundary has an explicit,
  tested representation for unknown/future enum values — Guardian fails
  closed on unknown, never silently maps unknown to safe/available/
  authorized;
- `P0-REG-001`, `P0-REG-002`, `P0-ARB-001..004`, `P0-EVT-001..004` are
  green; `P0-REG-003`/`P0-REG-004` are confirmed still green, reused
  unchanged from their existing `guardian-provider-api` implementation;
- no G4 transaction state machine, no real provider adapter, no GUI/TUI/CLI
  code, and nothing that weakens or reopens the G2 privilege topology
  exists anywhere in the result.

Then stop.

---

# 2. Read before changing code

1. `AGENTS.md`
2. `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`
   - §2 Governing principles (GP-01, GP-02, GP-04, GP-06, GP-08, GP-10 especially)
   - §4 Required repository layout
   - §9 polkit action taxonomy
   - §10 Risk taxonomy
   - §11 Capability Registry schema
   - §12 Provider interface contract
   - §13 Provider Arbitrator
   - §16 Error model
   - §17 Event schema
   - §18 Incident schema
   - §21 Boot availability model (feeds `CapabilityRecord.boot_availability`)
   - §36 P0-REG, P0-ARB, P0-EVT
3. `docs/guardian/20_Control_Plane/Capability_Registry.md`
4. `docs/guardian/20_Control_Plane/Provider_Arbitrator.md`
5. `docs/guardian/20_Control_Plane/Event_and_Incident_Model.md`
6. `docs/guardian/20_Control_Plane/Privilege_and_Authorization.md` (G2's
   authorization-ownership distinction, which G3 must represent, not
   redesign)
7. `docs/adr/ADR-002-guardian-privilege-topology.md` — the accepted G2
   decision and its constraint on any future helper↔core coordination.
8. `docs/evidence/g2/G2_MILESTONE.md` and
   `docs/evidence/g2/PRIVILEGE_REQUIREMENT_INVENTORY.md` — the concrete
   authorization-ownership classifications G3's `authorization_mode` type
   must be able to express without loss.

---

# 3. G3 normative contract IDs

These are the **only** normative test IDs in scope for G3. Do not invent
new `P0-*` IDs — the contract's full ID list was mechanically extracted
(`grep -oE "P0-[A-Z]+-[0-9]+"` against the governing contract) and contains
no `P0-CAP-*`/`P0-PROV-*`/`P0-INC-*` group. Sections 11 (Capability
Registry schema), 12 (Provider interface contract), 16 (Error model), and
18 (Incident schema) are normative **schema** requirements without their
own dedicated test-ID group; they are exercised through the tests below
plus ordinary focused unit/serialization tests that do not need a `P0-*`
label of their own (see §13).

```text
P0-REG-001 — provider unavailable        NEW this gate
P0-REG-002 — degraded provider           NEW this gate
P0-REG-003 — contract provenance         ALREADY GREEN — crates/guardian-provider-api/tests/provenance_contract.rs.
                                          Confirm still passing; do not reimplement.
P0-REG-004 — drift detection             ALREADY GREEN — same file. Confirm still passing; do not reimplement.

P0-ARB-001 — single writer               NEW this gate
P0-ARB-002 — ambiguous owner             NEW this gate
P0-ARB-003 — owner change invalidates transaction   NEW this gate (data-model level only —
                                          this is "a stale arbitration input must be
                                          detectable," not G4's transaction machine)
P0-ARB-004 — rollback disclosure         NEW this gate

P0-EVT-001 — monotonic ordering          NEW this gate
P0-EVT-002 — normalized key              NEW this gate
P0-EVT-003 — raw reference preserved     NEW this gate
P0-EVT-004 — incident linking            NEW this gate
```

Out of scope for G3 (do not implement, do not write tests that assume
their runtime behavior exists): `P0-TXN-*` (G4), `P0-DIAG-*`/`P0-REC-*`
(G5), `P0-IND-*` (G6), `P0-PRIV-*` (G2, already tagged).

---

# 4. Capability vs. provider — the identity boundary (§10 of the requester's brief)

This is the single most important structural rule in this gate.

**A capability describes what Guardian understands can be done. A provider
describes how that capability is currently realized on this machine.**
`capability_id` and `provider_id` MUST be separate, independently stable
identifier types — never the same string, never one derived from the
other, never positional.

```text
capability_id:  storage.device.poweroff     (stable — never changes)
provider_id:    udisks2                     (can change if Guardian ever
                                              gains an alternative realization)
```

A future provider change for a capability must **not** make historical
incidents, transactions, or policy that reference `capability_id` appear
to describe a different capability. Write a test that proves this: change
which provider offers a capability mid-scenario and assert every reference
that used the old `capability_id` is unaffected.

Capability IDs use dotted, domain-first, stable naming
(`domain.resource.operation`), matching the pattern already implied by the
polkit action taxonomy (§9 of the governing contract:
`guardian.storage.power-off`, `guardian.service.pause`, etc.) — e.g.
`system.service.restart`, `storage.device.poweroff`,
`system.pressure.io.observe`, `power.profile.read`, `power.profile.hold`.
Do not derive capability identity from a provider's D-Bus interface name,
UI label, discovery order, or a randomly generated UUID (P0-scoped
adversarial question §39.8/§39.9 below).

---

# 5. `CapabilityRecord`

Per TDD contract §11, the canonical representation MUST contain at least:

```rust
CapabilityRecord {
    capability_id,
    provider_id,
    provider_version,
    availability,
    health,
    read_support,
    write_support,
    authorization_mode,
    boot_availability,
    interface_kind,
    interface_name,
    interface_hash,
    diagnostic_cost,
    last_observed_at,
}
```

Design each field as a real type, not a raw string, wherever the contract
already defines the value set:

- `availability`: `AVAILABLE | DEGRADED | UNAVAILABLE | UNSUPPORTED |
  UNKNOWN` (§11). **`UNKNOWN` MUST NOT be rendered as healthy** — write a
  test that asserts this directly (any code path that treats `UNKNOWN` as
  equivalent to `AVAILABLE` must fail a test).
- `health`: `HEALTHY | WARNING | ERROR | STALE | UNKNOWN` (§11,
  "Recommended" — if the coding agent narrows this set, document why in
  the completion report; do not silently drop states).
- `read_support` / `write_support`: distinct booleans or a small typed
  pair — never a single `active` flag. A capability can be observable by
  many callers while writable by at most one (GP-04); the type must make
  "can observe" structurally different from "can mutate," and both
  different again from "currently owns mutation" (that last one lives on
  `ArbitrationDecision.current_owner`, not here — see §7).
- `authorization_mode`: a typed enum reflecting G2's established,
  audited distinction — **do not invent a fourth state without evidence,
  and do not collapse this into a boolean**:
  ```text
  NoAuthorizationRequired          (G2 inventory: "no privilege")
  ProviderOwnedAuthorization       (G2 inventory: "provider-owned authorization" —
                                     the provider itself performs its own polkit check;
                                     Guardian needs no elevated privilege)
  GuardianOwnedAuthorization       (G2 inventory: "Guardian polkit authorization" —
                                     Guardian's own action is polkit-gated)
  ```
  `docs/evidence/g2/PRIVILEGE_REQUIREMENT_INVENTORY.md`'s 24 rows are a
  direct, ready-made fixture set for testing that every classification in
  that table maps onto this enum without information loss. Use it.
  **Never represent `ProviderOwnedAuthorization` as "Guardian is
  privileged."** These remain different (governing brief §16).
- `boot_availability`: `EARLY_BOOT | SYSTEM_BUS | PRE_LOGIN | USER_SESSION
  | DESKTOP_ONLY | OPTIONAL` (§21). A capability may declare more than one
  level if it becomes available at different points.
- `diagnostic_cost`: may be a placeholder/minimal type this gate — the
  full `DiagnosticCost` structure (§19) belongs to G5's Diagnostic Budget
  Manager. Do not build G5's veto logic here; just ensure the field exists
  and is typed, not a raw number with no unit.

---

# 6. `Provider` trait and provider model

Per TDD contract §12:

```rust
trait Provider {
    fn identity(&self) -> ProviderIdentity;
    fn provenance(&self) -> ProviderProvenance;
    async fn probe(&self) -> ProbeResult;
    async fn capabilities(&self) -> Vec<CapabilityRecord>;
    async fn health(&self) -> ProviderHealth;
    async fn subscribe_events(&self) -> EventStream;
}
```

`ProviderProvenance` already exists in `guardian-provider-api` (P0-REG-003/
004) — reuse it unchanged. Build `ProviderIdentity`, `ProbeResult`, and
`ProviderHealth` alongside it, consistent with its existing style
(explicit `unknown` handling via `FromStr`/parse errors, not panics).

A mutable capability adapter MUST be able to express `inspect()`,
`validate(action)`, `snapshot(action)`, `apply(action)`, `observe
(expectation)`, `rollback(snapshot)` — as trait methods or an associated
trait. **Do not implement real bodies for these against real hardware in
G3.** Only the typed shape and an explicit `Unsupported` result for
operations a given provider doesn't support (§12, "Not every provider must
support every operation. Unsupported operations MUST return an explicit
typed `Unsupported` result.") — test this with a fixture provider that
deliberately supports only a subset.

Test fixtures (deterministic, in `guardian-testkit` or test-only modules)
should include at least two fixture providers ("provider A", "provider B")
offering overlapping and non-overlapping capabilities, with independently
controllable health/availability, so arbitration tests (§7) have something
real to arbitrate over. **Do not implement UDisks, NetworkManager,
systemd, NVML, UPower, or thermald adapters — those belong to G8 (§31 of
the governing brief).**

---

# 7. `ArbitrationDecision` and the Provider Arbitrator

Per TDD contract §13, the arbitrator answers: *"Which provider is
authoritative for this capability right now, and is Guardian allowed to
write it?"*

```rust
ArbitrationDecision {
    capability_id,
    candidate_providers,
    authoritative_provider,
    current_owner,
    ownership_basis,
    conflicts,
    write_permitted,
    rollback_kind,
    risk_class,
    decision_reason,
}
```

- `rollback_kind`: `NATIVE | EMULATED | BEST_EFFORT | NONE` (§13). This
  must be disclosed as part of the decision itself (P0-ARB-004) — not
  something a caller has to separately ask the provider for.
- `risk_class`: reuse the risk taxonomy directly (`OBSERVE | LOW |
  MODERATE | HIGH | VERY_HIGH`, §10) — do not create a second risk enum.
  If a shared `Risk` type does not yet exist in `guardian-core`, create it
  there and have this field use it; anything downstream (events,
  incidents, future transactions) reuses the same type (governing brief
  §18: "Do not create competing risk enums in different crates").
- `current_owner`: must be able to express **no writer**,
  **Guardian-owned writer**, **provider-owned writer** (e.g. thermald
  owning CPU thermal control while Guardian only observes), **external
  writer** (something outside Guardian's model entirely), and **conflict**
  — not collapsed into "provider exists ⇒ provider owns writes"
  (governing brief §14). A capability having a candidate provider is not
  the same claim as that provider currently holding write ownership.

## Arbitration invariants (§13, all five are testable requirements, not prose)

1. Two providers MUST NOT simultaneously receive write ownership for the
   same exclusively-owned capability — **P0-ARB-001**.
2. Ambiguous ownership fails closed: `write_permitted = false` —
   **P0-ARB-002**. Do not silently pick one candidate arbitrarily when
   ambiguity exists; that would be exactly the "unknown silently becomes
   safe" failure the governing brief's §28 forbids.
3. Provider absence can produce a degraded read-only capability but never
   a guessed write owner. Test: remove the sole candidate provider and
   assert the resulting `ArbitrationDecision` has `write_permitted =
   false`, not a stale/guessed owner.
4. `decision_reason` must be inspectable by clients — assert it is a real,
   non-empty, structured value (not just "reason: string" with no
   guarantee it's populated).
5. A provider ownership change invalidates stale arbitration/precondition
   state — **P0-ARB-003**, at the data-model level only. This gate does
   **not** implement G4's transaction machine; it must only prove that an
   `ArbitrationDecision` (or whatever state a future transaction would
   have captured as its precondition) carries enough identity/versioning
   that "this precondition is now stale because ownership moved" is
   mechanically detectable, e.g. via a monotonic arbitration
   epoch/generation number or an equivalent. Do not build `VALIDATING` →
   `APPLYING` runtime transitions here.

## Determinism (governing brief §26/§27)

Given identical candidate-provider input, arbitration MUST produce the
identical decision every run. Required adversarial tests:

- reverse the order candidate providers are supplied — decision must not
  change;
- if internal bookkeeping uses a `HashMap`, prove iteration order
  differences don't change the outcome (e.g. run the arbitration function
  many times with a hasher that randomizes each run, or explicitly
  construct two logically-identical-but-differently-ordered inputs);
- no random selection, no "first in discovery order wins," no similarity
  matching on display names — everything resolves via explicit
  `capability_id`/`provider_id` values only.

---

# 8. `Event` and `Incident`

Per TDD contract §17:

```rust
Event {
    event_id,
    timestamp_monotonic,
    timestamp_wall,
    source_provider,
    event_type,
    resource_refs,
    severity,
    normalized_key,
    raw_reference,
    attributes,
}
```

- Monotonic time MUST be used wherever ordering/duration matters
  (**P0-EVT-001**) — test that wall-clock adjustment (simulate the clock
  jumping backward) does not reorder events whose monotonic timestamps are
  correctly ordered.
- `normalized_key` lets equivalent volatile log/event variants share one
  key (**P0-EVT-002**) — e.g. two slightly different raw journald lines
  for "the same" disk-full condition normalize to one key — **without**
  destroying the link back to each event's own `raw_reference`
  (**P0-EVT-003**). Write a test with two distinctly-worded raw sources
  that share a normalized key and assert both raw references remain
  independently retrievable.
- `severity` should reuse the risk taxonomy type (§10) where the concepts
  align, or a clearly distinct, explicitly justified severity type if the
  governing docs distinguish them — do not silently conflate risk (what an
  *action* costs) with severity (what an *observation* means) without
  checking `docs/guardian/20_Control_Plane/Event_and_Incident_Model.md`
  first.

Per TDD contract §18:

```rust
Incident {
    incident_id,
    opened_at,
    closed_at,
    status,
    summary,
    confidence,
    primary_resource,
    event_ids,
    evidence,
    candidate_causes,
    recommended_actions,
    transaction_ids,
    outcome,
}
```

Incident invariants (§18, each individually testable):

- an incident does not replace source events — `event_ids` are
  **references**, not copies; the underlying events remain independently
  retrievable (governing brief §24: "typed references such as event IDs...
  not... pointer-by-array-position or fragile implicit relationships");
- correlation can be updated as evidence arrives — appending an
  `event_id` to an existing open incident must not require destroying and
  recreating the incident or losing its `incident_id`;
- confidence changes are recorded, not silently overwritten — if
  `confidence` changes, the change itself should be attributable/traceable
  (does not require a full audit-log implementation this gate, but the
  model must not make "what did we believe before" unrecoverable);
- user actions and Guardian transactions can be linked via
  `transaction_ids` — this gate only needs the reference *shape* to exist;
  G4 populates it for real;
  incidents can close (`status`, `closed_at`) without a known root cause —
  test that `candidate_causes` may legitimately be empty at close time and
  this is not treated as an error state.

**P0-EVT-004** ("Multiple correlated events can be linked into one
incident without deletion") is the direct test anchor for the incident
correlation invariants above — write it against real fixture events, not
against a trivial one-event incident.

## Confidence model (governing brief §23)

Prefer a small, deterministic, explainable representation over an
arbitrary floating-point percentage unless
`docs/guardian/20_Control_Plane/Event_and_Incident_Model.md` or the TDD
contract explicitly calls for a numeric scale. If no governing document
specifies the concrete representation, propose a minimal ordered enum
(e.g. `Hypothesis | Probable | Confirmed`, or whatever terminology the
control-plane doc already uses) and justify the choice in the completion
report rather than inventing floating-point precision nothing downstream
needs.

---

# 9. Provenance (feeds capability/provider/event/incident evidence, builds on G0)

`docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §6 already
establishes the source-authority hierarchy (D-Bus/library → kernel
interface → structured CLI → scraped CLI). G3 must make this reusable
wherever a model needs to answer "where did this fact come from" —
`CapabilityRecord.interface_*` fields, `Event.source_provider`/
`raw_reference`, and `Incident.evidence` all need it. **Rank indicates
authority/preference, not automatic correctness** (governing brief §22) —
do not write a test or a comment implying a lower-ranked source is
presumed false.

Reuse `ProviderProvenance` (already implemented, P0-REG-003/004) as the
provider-level provenance type; extend or compose it for
event/incident-level evidence references rather than building a second,
incompatible provenance type from scratch.

---

# 10. Unknown handling (governing brief §28)

Unknown is a valid state, not an error to paper over. For every governed
enum in this gate (`availability`, `health`, `authorization_mode`,
`boot_availability`, `rollback_kind`, `current_owner`, incident `status`,
event `severity`):

- deserializing a value the current binary doesn't recognize MUST produce
  an explicit `Unknown`/parse-failure outcome, never silently default to a
  "safe" or "available" or "authorized" variant (unless the governing
  contract explicitly defines that specific mapping — none currently do
  for these enums);
- write at least one test per governed enum that feeds it an unrecognized
  string/discriminant and asserts the result is the explicit unknown case,
  not a panic and not a silent default.

This mirrors `guardian-provider-api`'s existing `FromStr`-with-explicit-
unknown pattern (`(value != "unknown").then(...)` for optional provenance
fields) — follow that established style rather than inventing a new one.

---

# 11. Crate boundaries

TDD contract §4's illustrative tree places `capability/`, `arbitration/`,
`transaction/`, `incident/`, `diagnostics/`, `errors/`, and `identity/` as
submodules of `guardian-core`, with `guardian-provider-api` as the
separate provider-facing crate. The contract explicitly allows deviation
("exact crate names may change... but the architecture MUST preserve
separation equivalent to") — and the repository already has one
precedent: `ProviderProvenance` (conceptually provider-facing) lives in
`guardian-provider-api`, not under a `guardian-core` submodule, because it
is a provider-contract concern, not shared internal state.

Apply the same judgment, not the tree verbatim:

- `guardian-core`: `Risk` (shared, §10), `Event`, `Incident`,
  `ArbitrationDecision` (the *decision* type — internal, cross-cutting,
  used by anything that needs to know "who owns this write right now"),
  the shared identifier types (`CapabilityId`, `ProviderId`, `EventId`,
  `IncidentId`), and whatever error-model additions §16 requires beyond
  what `guardian-core::error` already has.
- `guardian-provider-api`: `CapabilityRecord`, the `Provider` trait,
  `ProviderIdentity`, `ProbeResult`, `ProviderHealth`, and anything a
  *provider implementation* needs to speak — provider-facing contract
  types, consistent with `ProviderProvenance`'s existing placement.

Before writing code, review the current contents of both crates (`find
crates/guardian-core/src crates/guardian-provider-api/src -type f`) and
justify the actual placement chosen against this repository's existing
structure in the completion report — do not silently follow this
handoff's suggestion if inspection reveals a better fit; do not silently
deviate without explaining why either.

Do not create new tiny crates merely for modularity theater (governing
brief §30).

---

# 12. G2 is a constraint on G3, not background context

G3 must not reopen or weaken anything G2 established. Specifically:

- `ArbitrationDecision`/Provider Arbitrator data lives in the unprivileged
  control-plane domain (`guardian-core`) — nothing built in this gate runs
  in, or grants privilege to, a helper process.
- A future privileged helper (per ADR-002) may eventually *consult*
  arbitration/ownership state produced here for non-authoritative
  coordination. **This gate must not build that consultation mechanism,
  and must not build anything that lets an `ArbitrationDecision` or
  similar core-owned value be mistaken for proof of caller identity or
  authorization.** If any G3 type has a field or method whose name or
  shape could plausibly be read as "authorization result" (e.g. anything
  resembling `authorized: bool` on a core-owned decision struct), that is
  a **G2 regression** — rename or restructure it. `authorization_mode` on
  `CapabilityRecord` (§5) describes *what kind of authorization a
  capability needs*, not *whether the current caller has it* — keep that
  distinction sharp in both the type and its doc comment.
- Do not implement any D-Bus-exposed method in this gate. G3 is internal
  typed models only; nothing here changes the G0 public surface
  (`ContractVersion`, `ServiceState`) or the G2 helper's bounded write
  surface.

---

# 13. Non-`P0-*` focused tests are expected and required

Sections 11, 12, 16, and 18 of the governing contract are schema
requirements without their own `P0-*` test-ID group (§3 above). Do not
treat "no ID exists" as "no test required." Write ordinary, clearly-named
focused tests for:

- serialization round-trips for every model that crosses a boundary
  (§14 below);
- the capability/provider identity separation proof (§4);
- the unknown-handling proof for each governed enum (§10);
- the `authorization_mode` ⇄ G2-inventory-classification mapping proof
  (§5).

Name these tests descriptively (matching this repository's existing style,
e.g. `capability_identity_survives_provider_change`,
`unrecognized_availability_value_is_explicit_unknown_not_a_panic`) so a
reviewer can see what's being proven without needing a `P0-*` cross-
reference for every single test.

---

# 14. Serialization and host evidence

G3 models will eventually cross process/D-Bus/persistence/log boundaries.
This gate requires:

- a stable representation for each governed enum (explicit
  discriminants/strings, not derive-default `Debug` formatting relied on
  as a wire format);
- explicit unknown-value handling on deserialization (§10);
- round-trip tests (serialize → deserialize → equal) for every model that
  is expected to cross a boundary.

**Do not expose these internal models on the public D-Bus surface in this
gate** — internal typed models and any future public wire contract remain
separate unless a later gate's governing contract explicitly says
otherwise (governing brief §25).

**No VM is required for this gate.** G3 is pure data-model semantics —
Layer 1 only. If, during implementation, a coding agent finds itself
writing behavior that depends on real host semantics (real D-Bus, real
polkit, real filesystem state), that is a signal the work has drifted out
of G3's scope — either remove it, or explicitly defer it to the gate that
actually owns it (G8 for real providers, G4 for real transactions) and say
so in the completion report. Do not quietly add a VM requirement to make
such code "pass."

---

# 15. TDD sequence (governing brief §35)

For each normative contract item in §3, follow strict red→green:

1. write the failing test for one specific normative behavior;
2. run it, confirm it fails for the right reason (not a compile error
   unrelated to the behavior under test);
3. implement the smallest correct model/logic that makes it pass;
4. run the focused test, confirm green;
5. run `cargo test --workspace`, confirm no regression;
6. move to the next item.

Do not build every model first and backfill tests. Preserve evidence of
this sequence in commit history (small, test-then-implementation-shaped
commits) or, at minimum, in the completion report's TDD-order narrative.

---

# 16. Adversarial self-check before reporting done

Before writing the completion report, deliberately try to break your own
implementation against each of these (this list is shared verbatim with
the independent reviewer — do not skip any):

1. reverse candidate-provider order — does arbitration choose differently?
2. same capability offered by two providers — is authority selection
   deterministic and explained via `decision_reason`?
3. the active writer disappears — does `current_owner` correctly become
   "no writer," not a stale reference?
4. two providers each claim exclusive write ownership of the same
   capability — is the conflict explicit (P0-ARB-001), not silently
   resolved to whichever was processed last?
5. is a read-only observer ever representable as, or mistakable for, a
   writer?
6. is `ProviderOwnedAuthorization` ever representable as, or mistakable
   for, "Guardian is privileged"?
7. does an unknown privilege/authorization value silently become "safe"
   anywhere?
8. does any capability/event/incident ID regenerate randomly on reload
   instead of being stable?
9. does a capability's ID change when its provider changes?
10. does an `Event` ever lose its `source_provider`/provenance when
    normalized?
11. does an `Incident` ever store a mutable object reference or array
    position instead of a stable `event_id`?
12. does deserializing an enum value from a newer, hypothetical future
    version crash instead of yielding an explicit unknown?
13. does an unavailable provider ever cause silent fallback to a
    lower-authority writer without that being visible in
    `decision_reason`?
14. if you modeled "external writer" at all, does Guardian's arbitrator
    ever try to silently claim ownership over it?
15. does anything resembling a future privileged helper trust an
    `ArbitrationDecision` (or any other core-owned value) as authorization
    proof? (Should be structurally impossible — confirm by inspection,
    not just by test, since this is a design property as much as a
    runtime one.)
16. does any core data model gain a privileged-mutation method? (Should be
    structurally impossible — G3 has no privileged code path at all.)
17. did any `Transaction`/`Apply`/`Rollback` *runtime* logic leak in,
    beyond the ID/reference plumbing explicitly required by §7/§8?
18. did a real provider call (UDisks, NetworkManager, systemd, etc.) leak
    into a test fixture instead of using a deterministic fake?

---

# 17. Completion states

Report exactly one, honestly:

```text
G3 CANDIDATE — CORE MODELS READY FOR INDEPENDENT AUDIT
G3 PARTIAL — CONTRACT TESTS INCOMPLETE
G3 BLOCKED — GOVERNING CONTRACT INSUFFICIENT
G3 BLOCKED — MODEL CONFLICT DISCOVERED
```

Do not report `CANDIDATE` if any normative test from §3 is red, skipped,
or asserts against a mock that doesn't actually exercise the behavior
under test.

---

# 18. Completion report

Include, at minimum: which normative tests are green with exact names/IDs;
crate-placement decisions and justification (§11); the
`authorization_mode` enum's mapping against all 24 Privilege Requirement
Inventory rows; determinism-test results (§7); the §16 adversarial
self-check results item-by-item; confirmation that `P0-REG-003`/`004`
remain green unmodified; full `cargo fmt --check` / `cargo clippy
--workspace --all-targets --all-features -- -D warnings` / `cargo test
--workspace` output; and an explicit statement of what was deferred to G4
or G8 and why.

Then stop. Do not begin G4. Do not tag G3 — independent review happens
first, exactly as it did for G0, G1, and G2.
