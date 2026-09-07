# Gate 2a transition-confidence repair — evidence

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-2a-transition-confidence-repair-manifest.toml`.
Governing TDD:
`docs/guardian/30_TDD/gates/phase2-2a-transition-confidence-repair-tdd.md`.

Reopened normative ID: `P2-COR-003` (confidence derivation only — its own
"debounced transition into `Unavailable` opens exactly one incident"
behavior is unchanged). `P2-COR-004` is explicitly not reopened;
regression-only in this repair.

Baseline: `0d071e194c353ffb42afd566be958a2a40925cd0` (matches the
manifest's recorded `baseline_sha` and the task's stated published
baseline). Working tree confirmed at this commit before any change.

Produced on a fresh tar-transfer of the working tree (uncommitted changes
included) into the disposable `guardian-g9` Ubuntu 26.04 multipass VM
(full `libadwaita-1-dev` workspace build, `guardian-gui` included),
independent of any stale prior copies on that VM (a stale ~3.7GB `target/`
directory from an earlier session was removed to free disk space before
this transfer).

## Root cause

`crates/guardian-core/src/correlation.rs`'s `classify(event: &Event)`
derived a health-transition `Event`'s `Confidence` from the **destination**
`(availability_to, health_to)` pair alone, via `health_direction()`'s
`HealthDirection::Bad(Confidence)` payload — any `Unavailable` destination
unconditionally became `Confidence::Confirmed`; any `Available + Warning`
destination unconditionally became `Confidence::Probable`, regardless of
what the capability transitioned *from*. Binding §4.2 names specific
**transitions** (`Available→Unavailable` is `Confirmed`; `Healthy→Warning`
is `Probable`), not destination states, so `Degraded→Unavailable`,
`Unknown→Unavailable`, `Error→Warning`, and `Degraded→Warning` all
incorrectly inherited a confidence tier §4.2 never authorized for them.

## Transition-provenance implementation

Two new optional attribute constants
(`crates/guardian-core/src/correlation.rs`):

```rust
pub const HEALTH_AVAILABILITY_FROM_ATTR: &str = "availability_from";
pub const HEALTH_HEALTH_FROM_ATTR: &str = "health_from";
```

A new private function, called only once state classification has already
decided a transition is `Bad`:

```rust
fn transition_confidence(
    availability_from: Option<Availability>,
    availability_to: Availability,
    health_from: Option<Health>,
    health_to: Health,
) -> Confidence {
    if availability_from == Some(Availability::Available)
        && availability_to == Availability::Unavailable
    {
        return Confidence::Confirmed;
    }
    if health_from == Some(Health::Healthy) && health_to == Health::Warning {
        return Confidence::Probable;
    }
    Confidence::Unknown
}
```

`classify()`'s `HEALTH_TRANSITION_EVENT_TYPE` branch now: (1) computes
`state = health_direction(availability_to, health_to)` exactly as before
— **byte-for-byte unmodified** `health_direction()` body, so destination
classification (R2) is untouched; (2) for `HealthDirection::Good`, passes
it through unchanged; (3) for `HealthDirection::Bad(_)`, reads
`availability_from`/`health_from` from the event's attributes (`None` if
absent or unparseable — parsing is infallible per
`Availability`/`Health`'s `FromStr`, so a malformed token parses to their
own `Unknown` variant, which also fails both `transition_confidence` checks
and correctly falls through to `Confidence::Unknown`, never fabricating
`Available`/`Healthy`) and calls `transition_confidence` to produce the
real `Bad(Confidence)` payload.

`HealthDirection::Bad(Confidence)`'s enum shape is unchanged (per the
manifest's Contract Collision Table row 1) — only the source of the
`Confidence` value inside it changed.

## RED evidence

Six new tests were added to `crates/guardian-core/tests/
correlation_contract.rs` and run against the pre-fix candidate (attribute
constants added so the test file compiled, but `classify()`/
`transition_confidence` not yet wired) before any confidence-derivation
source change:

```
running 46 tests
...
test transition_confidence_degraded_to_unavailable_is_unknown_not_confirmed ... FAILED
test transition_confidence_unknown_to_unavailable_is_unknown_not_confirmed ... FAILED
test transition_confidence_error_to_warning_is_unknown_not_probable ... FAILED
test transition_confidence_degraded_to_warning_is_unknown_not_probable ... FAILED
test transition_confidence_missing_from_provenance_unavailable_destination_is_unknown ... FAILED
test transition_confidence_missing_from_provenance_warning_destination_is_unknown ... FAILED

test result: FAILED. 40 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

Each failed for the right reason — the pre-fix destination-only rule
produced the wrong tier:

| Test | left (pre-fix, wrong) | right (expected) |
|---|---|---|
| `transition_confidence_degraded_to_unavailable_is_unknown_not_confirmed` | `Confirmed` | `Unknown` |
| `transition_confidence_unknown_to_unavailable_is_unknown_not_confirmed` | `Confirmed` | `Unknown` |
| `transition_confidence_error_to_warning_is_unknown_not_probable` | `Probable` | `Unknown` |
| `transition_confidence_degraded_to_warning_is_unknown_not_probable` | `Probable` | `Unknown` |
| `transition_confidence_missing_from_provenance_unavailable_destination_is_unknown` | `Confirmed` | `Unknown` |
| `transition_confidence_missing_from_provenance_warning_destination_is_unknown` | `Probable` | `Unknown` |

**Honest note — tests that were already green pre-fix (regression guards,
not RED evidence):**

- `transition_confidence_degraded_availability_from_healthy_health_from_to_available_warning_is_probable_per_literal_rule`
  (the stakeholder-added edge case) — pre-fix destination-only code already
  returns `Probable` for any `Available+Warning` destination, so this test
  cannot distinguish pre-fix from post-fix behavior. It is retained as a
  literal-rule regression guard, not RED evidence.
- The two pre-existing tests updated with correct `from`-provenance
  (`confidence_available_to_unavailable_opens_with_confirmed_confidence`,
  `confidence_available_warning_opens_with_probable_confidence`) — adding
  attributes the pre-fix `classify()` never read did not change pre-fix
  behavior (still `Confirmed`/`Probable`, both already correct for their
  destinations), so these also stayed green throughout and are regression
  guards, not RED evidence.
- `confidence_sustained_degraded_opens_with_unknown_confidence` — sustained
  `Degraded` availability never satisfies either `transition_confidence`
  condition regardless of `from`-provenance, so it was `Unknown`
  pre-fix and post-fix alike (unaffected by this repair).

## GREEN evidence — governed confidence cases

All six required cases pass after the fix
(`crates/guardian-core/tests/correlation_contract.rs`):

| Case | Test | Result |
|---|---|---|
| `Available → Unavailable` | `confidence_available_to_unavailable_opens_with_confirmed_confidence` | `Confirmed` |
| `Degraded → Unavailable` | `transition_confidence_degraded_to_unavailable_is_unknown_not_confirmed` | `Unknown` |
| `Unknown → Unavailable` | `transition_confidence_unknown_to_unavailable_is_unknown_not_confirmed` | `Unknown` |
| `Healthy → Warning` | `confidence_available_warning_opens_with_probable_confidence` | `Probable` |
| `Error → Warning` | `transition_confidence_error_to_warning_is_unknown_not_probable` | `Unknown` |
| missing/incomplete `from`-provenance | `transition_confidence_missing_from_provenance_unavailable_destination_is_unknown`, `transition_confidence_missing_from_provenance_warning_destination_is_unknown` | `Unknown` (both) |

Bonus coverage of the defect table's fourth row (`Degraded → Warning`,
health dimension not satisfying `Healthy→Warning`):
`transition_confidence_degraded_to_warning_is_unknown_not_probable` →
`Unknown`.

## Edge-case result: `Degraded`(avail_from) + `Healthy`(health_from) → `Available`+`Warning`

Test:
`transition_confidence_degraded_availability_from_healthy_health_from_to_available_warning_is_probable_per_literal_rule`.

Asserts `Confidence::Probable`. This is the correct literal §4.2 outcome:
the governed Probable rule is `health_from == Healthy && health_to ==
Warning`, keyed purely on the health dimension. `availability_from ==
Degraded` does not appear anywhere in that rule, so it does not block the
match — `transition_confidence`'s second condition is satisfied on its own
terms. No additional `availability_from` check was added to "fix" this
case; doing so would have invented policy §4.2 does not state. The
requirement was genuinely unambiguous once read literally, so no STOP was
needed here.

## Regression validation

- **`P2-COR-004`**: both `p2_cor_004_flapping_faster_than_dwell_produces_no_thrashing`
  and `p2_cor_004_flapping_faster_than_dwell_via_fresh_observations_produces_no_thrashing`
  are byte-for-byte unmodified in this repair's diff and remain green.
- **State classification unchanged (R2)**: `health_direction()`'s body is
  byte-for-byte unmodified (confirmed via `git diff -- crates/guardian-core/src/correlation.rs`
  — the diff around `health_direction` shows zero changed lines inside the
  function; `transition_confidence` is inserted as a wholly new function
  after it). `r4_fresh_health_observation_classify_matches_state_table`
  and `r4_event_driven_available_error_never_misclassified_as_good` remain
  green unmodified.
- **Dwell/debounce/recovery mechanics unchanged**: `advance_health_dwell`
  and `FreshHealthObservation` are byte-for-byte unmodified in this
  repair's diff (confirmed via `git diff`). `r2_no_fresh_observation_means_no_promotion_from_wall_time_alone`,
  `r3_fresh_observation_unresolved_never_falsely_advances_or_corrupts`,
  `r5_symmetric_recovery_fresh_observation_closes_incident`, and
  `regression_second_real_bad_event_still_promotes_pending_candidate`
  remain green unmodified.
- **Full prior Gate 2a ID set green**: all 46 tests in
  `crates/guardian-core/tests/correlation_contract.rs` pass (39 baseline +
  7 net-new; see "Files changed" below for the +7 breakdown), covering the
  original 19 normative IDs plus the first repair's own tests.

## Discovered Contract Collision — NOT resolved unilaterally, reported here

Full-workspace `cargo test --workspace` (both on the host, excluding
`guardian-gui`, and on the `guardian-g9` VM including it) surfaces exactly
two failures, **both inside `crates/guardian-daemon/**`, forbidden scope
for this repair**:

```
dbus_surface::tests::incidents_list_reflects_a_real_incident_the_engine_actually_opened
  crates/guardian-daemon/src/dbus_surface.rs:481
  assertion `left == right` failed
    left: "unknown"
   right: "confirmed"

p2_2b_dbus_contract_suite (P2-API-001 sub-assertion)
  crates/guardian-daemon/tests/phase2_2b_contract.rs:202
  assertion `left == right` failed
    left: "unknown"
   right: "confirmed"
```

**Root cause of the collision**: both tests construct a synthetic `Event`
shaped exactly like today's real (pre-Gate-2b-repair)
`HealthTransitionProducer` output — only `capability_id` and
`availability_to` attributes are set, no `availability_from`/`health_from`
— then assert the incident that opens from it carries
`Confidence::Confirmed`. Under the OLD, defective destination-only rule
this repair corrects, any `Unavailable` destination unconditionally became
`Confirmed`, so these tests passed. Under the corrected, §4.2-literal
transition rule (this repair's entire mandate), an `Unavailable`
destination with no `availability_from` provenance correctly yields
`Confidence::Unknown` (R3: never fabricate `Available` from absence) —
which is exactly what these tests now observe, and exactly why they now
fail.

**Why this is reported, not fixed here**: both files are explicitly listed
in this repair's `forbidden_scope`
(`crates/guardian-daemon/**`, `crates/guardian-daemon/tests/
phase2_2b_contract.rs`), and the manifest's own "Downstream / not
implemented here: Gate 2b forward requirement" section already assigns
`HealthTransitionProducer`'s provenance-emission fix — the change that
would let these two tests construct realistic events with genuine
`availability_from` and update their own expectations accordingly — to
Gate 2b's own separate, already-parked repair, not to this one. This
repair's execution protocol's binding STOP rule directs reporting a
genuine incompatibility between two authoritative requirements rather than
inventing a reconciliation; reaching into `guardian-daemon/**` to "fix" or
soften either test would violate `forbidden_scope`, and weakening this
repair's own correct confidence derivation to keep them passing would
defeat the repair's entire purpose (and violate `AGENTS.md`'s "do not
convert UNKNOWN into HEALTHY," applied here to provenance completeness).
This is therefore reported as a discovered, real, but out-of-this-repair's-scope
regression for independent review and for Gate 2b's own forward repair to
resolve (by emitting real `availability_from`/`health_from` provenance and
updating these two tests' own now-outdated `Confirmed` expectations
alongside it) — not silently fixed and not silently left unreported.

## Final validation

Run on `guardian-g9` (fresh tar-transfer, independent of stale prior VM
copies; a stale ~3.7GB `target/` directory from a prior session was
removed first):

```
cargo fmt --check                                                        # exit 0, no diff
cargo clippy --workspace --all-targets --all-features -- -D warnings     # exit 0, 0 errors/warnings (guardian-gui built clean)
cargo test --workspace --no-fail-fast
```

```
passed 405 failed 2 ignored 3   (407 total)
```

407 = 400 (manifest's recorded pre-repair baseline) + 7 net-new tests, all
in `crates/guardian-core/tests/correlation_contract.rs` (39 → 46
`#[test]` functions). 3 ignored, unchanged from baseline. The 2 failures
are exactly the two `guardian-daemon`-owned, forbidden-scope tests
documented above — both were part of the 400 passing baseline tests
before this repair (the baseline's `cargo test --workspace` was 400
passed / 0 failed / 3 ignored) and are the sole regression this repair's
otherwise-correct fix surfaces outside its own scope. No other test in the
407-test workspace total regressed.

Without `--no-fail-fast`, `cargo test --workspace` exits non-zero at these
two targets (`-p guardian-daemon --lib`, `-p guardian-daemon --test
phase2_2b_contract`) — reported here rather than suppressed.

## Files changed by this repair

- `crates/guardian-core/src/correlation.rs` (modified) —
  `HEALTH_AVAILABILITY_FROM_ATTR`/`HEALTH_HEALTH_FROM_ATTR` constants; new
  `transition_confidence()` function; `classify()`'s `HEALTH_TRANSITION_EVENT_TYPE`
  branch now separates destination-only state classification
  (`health_direction`, unchanged) from transition-derived confidence
  (`transition_confidence`, new) for the `Bad` case. `health_direction()`,
  `advance_health_dwell()`, and `FreshHealthObservation` are byte-for-byte
  unmodified.
- `crates/guardian-core/tests/correlation_contract.rs` (modified) — new
  `health_event_with_transition()` test fixture; 2 pre-existing tests
  (`confidence_available_to_unavailable_opens_with_confirmed_confidence`,
  `confidence_available_warning_opens_with_probable_confidence`) and one
  further pre-existing test (`p2_cor_005_cross_source_correlation_capped_and_worded_correctly`,
  its `evt-h1` construction) updated to supply correct real `from`-provenance
  so their assertions remain accurate under the corrected rule; 7 net-new
  `#[test]` functions (listed under "RED evidence"/"GREEN evidence" above).
- `docs/guardian/30_TDD/gates/phase2-2a-transition-confidence-repair-manifest.toml`,
  `docs/guardian/30_TDD/gates/phase2-2a-transition-confidence-repair-tdd.md`
  (pre-existing, authored before this execution turn as this repair's own
  governance files — read, not modified, by this execution).
- `docs/evidence/p2/GATE2A_TRANSITION_CONFIDENCE_REPAIR_EVIDENCE.md` (this
  file, new).

Explicitly NOT part of this repair, pre-existing in the working tree
before this task began, and left untouched (confirmed via `git status`/
`git diff --stat` at both the start and end of this task — identical
diffs):

- `docs/guardian/30_TDD/gates/phase2-2c-manifest.toml`
- `docs/guardian/30_TDD/gates/phase2-2c-tdd.md`
- `docs/guardian/30_TDD/gates/phase2-2b-health-lifecycle-integration-repair-manifest.toml`
  (untracked, pre-existing)
- `docs/guardian/30_TDD/gates/phase2-2b-health-lifecycle-integration-repair-tdd.md`
  (untracked, pre-existing)

Not touched, per `forbidden_scope`: `crates/guardian-core/src/providers/health.rs`,
`crates/guardian-daemon/**` (including `crates/guardian-daemon/tests/phase2_2b_contract.rs`),
`docs/guardian/30_TDD/gates/phase2-2a-health-lifecycle-repair-manifest.toml`/`-tdd.md`.

## Git state

Left uncommitted per the manifest's `commit_policy`
("leave-uncommitted-for-independent-review"). No commit, push, or tag was
made.
