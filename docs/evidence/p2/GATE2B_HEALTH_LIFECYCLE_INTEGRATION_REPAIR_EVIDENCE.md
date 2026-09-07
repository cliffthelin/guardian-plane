# Gate 2b health-lifecycle integration repair — evidence

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-2b-health-lifecycle-integration-repair-manifest.toml`.
Governing TDD:
`docs/guardian/30_TDD/gates/phase2-2b-health-lifecycle-integration-repair-tdd.md`.

Reopened normative ID: `P2-API-001` (acceptance evidence corrected to
prove the real production integration path). `P2-API-002`/`P2-API-003`
are explicitly not reopened; regression-only in this repair.

Baseline: `0d071e194c353ffb42afd566be958a2a40925cd0`. Executed on top of
the current, uncommitted working tree, which additionally carries the
already-implemented, implementation-complete Gate 2a
transition-confidence repair candidate
(`crates/guardian-core/src/correlation.rs`,
`crates/guardian-core/tests/correlation_contract.rs`,
`docs/evidence/p2/GATE2A_TRANSITION_CONFIDENCE_REPAIR_EVIDENCE.md`) —
untouched by this repair.

Produced on a fresh tar-transfer of the working tree (uncommitted changes
included) into the disposable `guardian-g9` Ubuntu 26.04 multipass VM
(full `libadwaita-1-dev` workspace build, `guardian-gui` included),
independent of any stale prior copies on that VM. A stale ~3.7GB
`target/` directory from an unrelated earlier session
(`/home/ubuntu/guardian-gate2a-tc-repair/target`) and a stale, non-git
`/home/ubuntu/Guardian` copy were removed to free disk space before this
transfer (9.3GiB free before cleanup, 13GiB after).

## Root cause

Two related gaps, both closed here:

1. `HealthTransitionProducer::transition_event`
   (`crates/guardian-core/src/providers/health.rs`) populated only
   `HEALTH_AVAILABILITY_TO_ATTR` on every real `Event` it emitted — never
   `HEALTH_HEALTH_TO_ATTR`, and (see "Discovered Contract Collision"
   below) never `HEALTH_AVAILABILITY_FROM_ATTR`/`HEALTH_HEALTH_FROM_ATTR`
   either. Every real production `Event` therefore reached Gate 2a's
   `classify()`/`transition_confidence()` with no "from" provenance at
   all, so `transition_confidence()` always fell through to
   `Confidence::Unknown` regardless of the real transition — the two
   pre-existing failing tests this repair closes
   (`crates/guardian-daemon/src/dbus_surface.rs:481`,
   `crates/guardian-daemon/tests/phase2_2b_contract.rs:202`).
2. `capability_registry_tick`
   (`crates/guardian-daemon/src/bin/guardian-daemon.rs`) fed
   `HealthTransitionProducer::observe`'s output `Event`s through
   `admit_event` and stopped there — it never called Gate 2a's
   `CorrelationEngine::advance_health_dwell`. The real pipeline could
   therefore never open (or close) a provider-health incident
   end-to-end without a second, fabricated `Event`, which is exactly
   what the pre-repair `P2-API-001` fixture
   (`build_engine_with_one_real_open_incident`) did: clone the one real
   `Event` and mutate the clone's `event_id`.

## Discovered Contract Collision — not a STOP, resolved and reported here

The manifest's own Contract Collision Table (in its linked TDD) reasons
only about `health_direction()` (destination-only) and concludes only
`HEALTH_HEALTH_TO_ATTR` needs to be added — "No 'previous' state needs to
be added to the Event shape." That conclusion is accurate for
`health_direction()` but **incomplete relative to the actual, already-
landed `correlation.rs`**: the uncommitted Gate 2a transition-confidence
repair candidate (already in the working tree, already reviewed
separately, explicitly not to be modified by this repair) changed
`classify()` to recompute a `Bad` direction's `Confidence` via
`transition_confidence(availability_from, availability_to, health_from,
health_to)`, reading `HEALTH_AVAILABILITY_FROM_ATTR`/
`HEALTH_HEALTH_FROM_ATTR` from the `Event`'s attributes. The Gate 2b
repair manifest/TDD predate or do not reference this change and so never
mention those two attributes.

This is not a genuine collision (no two authoritative sources assign
conflicting ownership/responsibility here) — it is the manifest's R1
being under-scoped relative to ground truth. Per the execution protocol's
instruction to verify against the actual current code rather than only
the manifest's prose, and per the task's own explicit direction, this
repair populates all four attributes
(`availability_from`/`availability_to`/`health_from`/`health_to`) from
data `HealthTransitionProducer` already holds (`self.previous`/current
snapshot), not just the two `_to` attributes. `HEALTH_AVAILABILITY_FROM_ATTR`/
`HEALTH_HEALTH_FROM_ATTR` are pre-existing, already-exported `pub`
constants in `correlation.rs` (added by the Gate 2a transition-confidence
repair, forbidden scope for this repair) meant for exactly this producer
to populate; populating them is squarely within this repair's allowed
scope (`crates/guardian-core/src/providers/health.rs`) and required no
edit to any forbidden file. Flagged here for the record rather than
silently reconciled without comment.

## R1 — Event provenance implementation

`crates/guardian-core/src/providers/health.rs`:

- `transition_event` now takes `prev_availability`/`prev_health` in
  addition to the existing `availability`/`health`, and sets all four
  attributes: `HEALTH_AVAILABILITY_FROM_ATTR`, `HEALTH_AVAILABILITY_TO_ATTR`,
  `HEALTH_HEALTH_FROM_ATTR`, `HEALTH_HEALTH_TO_ATTR`.
- `HealthTransitionProducer::observe` passes the prior snapshot's
  `(Availability, Health)` pair through — already held in `self.previous`,
  diffed against the current snapshot; no new provider I/O.
- The producer remains edge-triggered: `unchanged_snapshot_pair_emits_no_event`
  is unchanged and still green.

Evidence (all in `crates/guardian-core/src/providers/health.rs`, `#[cfg(test)]
mod tests`):

- `availability_change_for_one_capability_emits_exactly_one_correlatable_event`
  — asserts a real `Available`+`Healthy` → `Unavailable`+`Error` transition
  carries `availability_from="available"`, `health_from="healthy"`,
  `health_to="error"` (alongside the pre-existing `availability_to="unavailable"`
  assertion).
- `health_only_change_with_stable_availability_still_emits_an_event` —
  asserts the `Healthy→Warning` row (the other named §4.2 transition)
  carries `availability_from="available"`, `availability_to="available"`,
  `health_from="healthy"`, `health_to="warning"`.
- `unchanged_snapshot_pair_emits_no_event` — unmodified, still passes:
  the edge-triggered property is unaffected by adding the attributes.

## R2 — daemon fresh-snapshot wiring implementation

`crates/guardian-daemon/src/bin/guardian-daemon.rs`:

- New function `advance_health_dwell_for_snapshot(records, ingress_clock,
  engine)`: captures one shared `(Instant::now(), IngressClock::sequence())`
  ingress-order pair per call (not per capability), then calls
  `CorrelationEngine::advance_health_dwell` once per `CapabilityRecord` in
  `records`, using `FreshHealthObservation::classify(record.availability,
  record.health)`.
- `capability_registry_tick` now calls this immediately after admitting
  any diff-driven `Event`s from `HealthTransitionProducer::observe`, using
  the same fresh snapshot already collected — no second provider/registry
  read.

Evidence (`crates/guardian-daemon/src/bin/guardian-daemon.rs`,
`#[cfg(test)] mod tests`):

- `sustained_bad_across_fresh_snapshots_opens_exactly_one_incident_with_no_second_event`
  — proves (a): a real diff-driven `Event` seeds the debounce ring; a
  later unchanged-bad fresh snapshot emits zero new `Event`s (asserted);
  `advance_health_dwell` (called with an ingress instant past
  `health_min_dwell`) alone opens exactly one incident; a further call
  with the same still-bad snapshot leaves the open-incident count
  unchanged — proves (b), a capability with no pending candidate left is
  unaffected by being called every tick.
- `a_capability_absent_from_a_snapshot_is_not_promoted_that_tick` —
  proves (c): a capability entirely absent from a given tick's snapshot
  (an empty `Vec<CapabilityRecord>`) is simply not called that tick, and
  its already-pending candidate is not promoted.

## R3 — real `P2-API-001` sequence

`crates/guardian-daemon/tests/phase2_2b_contract.rs`'s
`build_engine_with_one_real_open_incident` no longer clones an `Event` or
mutates an `event_id`. It now drives: baseline snapshot (producer seeded,
no `Event`) → bad-transition snapshot (`HealthTransitionProducer` emits
exactly one real `Event`, admitted; asserted `AdmitOutcome::DebouncePending`)
→ a later unchanged-still-bad fresh snapshot (asserted: the producer
emits zero new `Event`s) → `CorrelationEngine::advance_health_dwell`, fed
`FreshHealthObservation::classify(Availability::Unavailable, Health::Error)`
and an ingress instant past `health_min_dwell` (asserted:
`AdmitOutcome::IncidentOpened`) → the resulting engine is served at
`INCIDENTS_OBJECT_PATH` over the same private D-Bus connection
`guardian-daemon`'s own `main()` uses.

Evidence: `p2_2b_dbus_contract_suite`'s `record_check(&mut failures,
"P2-API-001", ...)` block, running
`assert_p2_api_001_list_incidents_returns_the_real_open_incident`
unmodified against the real-sequence-built engine — asserts `ListIncidents()`
returns exactly one incident, status `"open"`, `primary_resource ==
"systemd.unit.state"`, **`confidence == "confirmed"`** (now reachable
because the real `Event` carries `availability_from="available"` — the
named `Available→Unavailable` transition, per §4.2/`transition_confidence()`),
and a non-empty `incident_id`.

## R4 — symmetric recovery evidence

New helper `build_engine_with_one_real_closed_incident` (same file)
mirrors R3's opening sequence, then: a real diff back to `Available`+
`Healthy` is one real recovery `Event`, admitted (asserted
`AdmitOutcome::DebouncePending` — the first Good reading only seeds the
recovery candidate) → a later, unchanged-still-good fresh snapshot
(asserted: the producer emits zero new `Event`s, proving R1's
edge-triggered property holds symmetrically in recovery) →
`advance_health_dwell`, fed `FreshHealthObservation::classify(Availability::Available,
Health::Healthy)` and an ingress instant past the recovery dwell (asserted
`AdmitOutcome::IncidentClosed`) — no second `Event` manufactured in either
direction.

The resulting engine is served at a second, test-only object path
(`INCIDENTS_RECOVERY_TEST_OBJECT_PATH =
"/io/github/cliffthelin/Guardian1/IncidentsRecoveryTest"`) on the **same**
private D-Bus connection `p2_2b_dbus_contract_suite` already builds —
deliberately not a second `PrivateSessionBus`/blocking `zbus` connection,
per that test's own pre-existing doc comment on the real concurrency
hazard two such connections triggered during Gate 2b's original RED→GREEN
pass. This path is test-only: `guardian-daemon`'s own `main()` never
serves `Incidents1` there, so `P2-API-002`'s "no new D-Bus surface" check
(which only inspects the three real, known object paths) is unaffected.

Evidence: `p2_2b_dbus_contract_suite`'s `record_check(&mut failures,
"gate-2b-repair-R4", ...)` block, running
`assert_gate_2b_repair_r4_list_incidents_reflects_the_real_closure` —
asserts `ListIncidents()` (called against the recovery path) returns
exactly one incident, status `"closed"`, `primary_resource ==
"systemd.unit.state"`.

## R5 — regression validation

`assert_p2_api_002_no_new_dbus_surface` (P2-API-002) and
`incident_wire_shape_is_locked_to_a_seven_field_string_tuple`
(P2-API-003, in `crates/guardian-daemon/src/dbus_surface.rs`) are both
unmodified and green — see the final validation run below.

## Stale-fixture correction — `dbus_surface.rs:481`

`crates/guardian-daemon/src/dbus_surface.rs`'s
`incidents_list_reflects_a_real_incident_the_engine_actually_opened`
directly constructs two admitted `Event`s (not via the producer) to prove
`Incidents1::list_incidents` round-trips a real, engine-opened incident.
Its `make_event` closure previously set only
`HEALTH_CAPABILITY_ID_ATTR`/`HEALTH_AVAILABILITY_TO_ATTR="unavailable"` —
no "from" provenance — so under the corrected (candidate-A) confidence
rule the resulting incident's confidence was honestly `"unknown"`, not
the test's hardcoded `"confirmed"` expectation. This was Gate 2a's
correction working exactly as intended (missing "from" provenance is
`Confidence::Unknown`, never fabricated), not a Gate 2a defect. The fix
adds `HEALTH_AVAILABILITY_FROM_ATTR="available"` and
`HEALTH_HEALTH_FROM_ATTR="healthy"` to the fixture's `Event`, making it
genuinely represent the real `Available→Unavailable` transition it always
claimed to (its own doc comment: "a real health-transition event admitted
through the same `CorrelationIngress` mechanism") — the assertion is
unchanged and now correctly passes because the fixture's input is
honest, not because the assertion was weakened.

## Final validation (guardian-g9 VM, full workspace incl. `guardian-gui`)

```
cargo fmt --check
  -> clean, no diff

cargo clippy --workspace --all-targets --all-features -- -D warnings
  -> Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.14s
     (zero warnings/errors)

cargo test --workspace
  -> 409 passed; 0 failed; 3 ignored (unchanged ignore set from baseline)
```

The two previously-failing tests now pass:
`dbus_surface::tests::incidents_list_reflects_a_real_incident_the_engine_actually_opened`
and `p2_2b_dbus_contract_suite` (which now also covers the new R4 check).

## Files changed by this repair

- `crates/guardian-core/src/providers/health.rs` — R1 (modified,
  allowed scope).
- `crates/guardian-daemon/src/bin/guardian-daemon.rs` — R2 (modified,
  allowed scope).
- `crates/guardian-daemon/src/dbus_surface.rs` — stale-fixture correction
  (modified, allowed scope).
- `crates/guardian-daemon/tests/phase2_2b_contract.rs` — R3/R4 (modified,
  allowed scope).
- `docs/evidence/p2/GATE2B_HEALTH_LIFECYCLE_INTEGRATION_REPAIR_EVIDENCE.md`
  — this file (added, allowed scope).

Not touched (confirmed via `git diff --stat` showing no additional
change beyond what was already present before this repair started):

- `crates/guardian-core/src/correlation.rs`,
  `crates/guardian-core/tests/correlation_contract.rs` (Gate 2a
  transition-confidence repair candidate — forbidden scope).
- `docs/guardian/30_TDD/gates/phase2-2c-manifest.toml`,
  `docs/guardian/30_TDD/gates/phase2-2c-tdd.md` (parked Gate 2c —
  forbidden scope; content not read beyond confirming `git status`).
- `docs/guardian/30_TDD/gates/phase2-2a-health-lifecycle-repair-manifest.toml`/
  `-tdd.md`, `docs/guardian/30_TDD/gates/phase2-2b-manifest.toml`/
  `-tdd.md` (forbidden scope).
- `crates/guardian-daemon/src/dbus_surface.rs::IncidentWire` shape,
  `crates/guardian-client/src/lib.rs::IncidentWire` shape — unchanged
  (7-field tuple, locked).

## Git state

All work left uncommitted, per `commit_policy =
"leave-uncommitted-for-independent-review"`. No commit, tag, or push
performed.
