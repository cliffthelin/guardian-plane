---
title: "Guardian Hardening Backlog"
kind: "hardening-backlog"
status: "active"
last_reviewed: "2026-09-07"
tags:
  - tdd
  - backlog
---
# Guardian Hardening Backlog

Non-blocking findings that were real at the time they were found but did
not block the gate/candidate they were found in. This is the single home
for such items going forward — a genuinely non-blocking finding is
routed here, not left as a footnote in a gate's own handoff and not
silently dropped (`GUARDIAN_EXECUTION_PROTOCOL.md` step 10).

An entry needs only: source gate, finding, why non-blocking, revisit
trigger/phase, status. This is not a migration of every historical
finding — it is seeded with the items already known to remain genuinely
open, to establish the mechanism.

## Entries

### From Wave 1 (`docs/evidence/wave1/WAVE1_MILESTONE.md`)

- **Source gate**: Wave 1
  **Finding**: `JobRemoved` buffer correlates strictly by job path; it
  does not additionally filter by unit name, though the signal payload
  carries it and the governed unit (`cups.service`) is known statically
  before `RestartUnit` is called.
  **Why non-blocking**: the already-bounded eviction window this could
  narrow is already safe-failing — an eviction can only produce a
  conservative `ambiguous` outcome, never a false success.
  **Revisit trigger/phase**: any future gate that adds a second governed
  unit to the same buffer, or that revisits systemd job-signal handling.
  **Status**: open.

- **Source gate**: Wave 1
  **Finding**: no dedicated fake-service test exercises race scenarios
  A–F specifically against `rollback`'s own re-armed observation path
  (only `apply`/`observe` have dedicated fake-service race tests).
  **Why non-blocking**: closed for Wave 1 acceptance by fresh,
  independent real-VM rollback-failure reproduction instead.
  **Revisit trigger/phase**: any future gate that adds a second
  mutation capability with its own rollback path.
  **Status**: open.

- **Source gate**: Wave 1
  **Finding**: listener-thread cleanup is structurally bounded by the
  adapter's own connection lifetime (verified by reading the
  `Drop`/connection-teardown path and by a 5-cycle no-hang test), but no
  direct OS-thread-count instrumentation was added to measure this
  directly.
  **Why non-blocking**: the structural bound was independently verified
  by code reading and a repeated no-hang test; only direct
  instrumentation is missing, not the bound itself.
  **Revisit trigger/phase**: any future gate under sustained load/soak
  testing that could benefit from direct thread-count metrics.
  **Status**: open.

### From G9 (`docs/evidence/g9/G9_MILESTONE.md`, carried forward unrelated to Wave 1/Phase 2)

- **Source gate**: G9
  **Finding**: `guardian-testkit`'s `PrivateSessionBus::launch` performs
  an unbounded `BufReader::read_line()` waiting for `dbus-daemon`'s
  startup address line, with no timeout.
  **Why non-blocking**: confirmed test-only (no production binary
  depends on this pattern); plausibly explains one transient hang
  observed once, never reproduced since.
  **Revisit trigger/phase**: any future test-infrastructure cleanup
  pass, or a recurrence of the transient hang.
  **Status**: open.

- **Source gate**: G9
  **Finding**: `guardian-tui`'s text-polkit test action waits a fixed
  ~300ms after spawning `pkttyagent` before issuing the authorization
  check, with no synchronous "ready" signal from the agent.
  **Why non-blocking**: an inherently racy idiom, not observed to fail
  in any reproduction so far.
  **Revisit trigger/phase**: a future flaky-test investigation touching
  `guardian-tui`'s polkit test harness.
  **Status**: open.

- **Source gate**: G9
  **Finding**: `guardian-tui`'s `pkttyagent` cleanup is imperative
  (explicit `kill()`/`wait()` after the check returns) rather than
  RAII/Drop-guarded.
  **Why non-blocking**: no current code path skips it (no
  unwrap/expect/early-return exists between spawn and cleanup).
  **Revisit trigger/phase**: any future edit to that code path that
  introduces a panic or early return.
  **Status**: open.

- **Source gate**: G9 (inherited from G7)
  **Finding**: `StateDirectoryMode` is not set on either promoted
  systemd unit; `/var/lib/guardian/{daemon,helper}` end up at runtime
  mode 0755 rather than ADR-008 §4's stated 0750.
  **Why non-blocking**: no privilege-boundary issue — ownership is
  correct throughout, and cross-process data access is independently
  denied at the file level (0600/0700 modes) regardless of the parent
  directory's own listing permission.
  **Revisit trigger/phase**: any future packaging/unit-file revision
  pass.
  **Status**: open.

- **Source gate**: G9
  **Finding**: ADR-006 was never given the short addendum the G9
  implementation handoff's own §8 called for, noting a gate-ownership
  correction (contract §38 assigns the production indicator to G9, not
  G7).
  **Why non-blocking**: documentation-only — the underlying
  gate-ownership question is correctly resolved in practice.
  **Revisit trigger/phase**: any future ADR-006 revision for an
  unrelated reason.
  **Status**: open.

- **Source gate**: G9
  **Finding**: no legible screenshot of the indicator's icon glyph
  exists on either required desktop, independently attributed to the
  desktop's own systray-plugin SNI-icon compositing in this VM.
  **Why non-blocking**: protocol-level SNI state (registration, live
  `IconName`/`Title`) is correct and live on both required desktops;
  contract §30 requires a functioning indicator test suite, not
  pixel-perfect rendering proof.
  **Revisit trigger/phase**: a future desktop-environment upgrade in the
  VM image, or a client-surface gate that needs a legible glyph
  screenshot for its own evidence.
  **Status**: open.

### From the Phase 2 Gate 2a/2b health-lifecycle dependency repair (`docs/evidence/p2/GATE2A_TRANSITION_CONFIDENCE_REPAIR_EVIDENCE.md`, `docs/evidence/p2/GATE2B_HEALTH_LIFECYCLE_INTEGRATION_REPAIR_EVIDENCE.md`)

- **Source gate**: Phase 2 Gate 2a transition-confidence repair
  **Finding**: `classify()` in `crates/guardian-core/src/correlation.rs`
  still defaults a *missing* (not malformed) `health_to` attribute to
  `Health::Healthy` for hand-built/malformed `Event`s — a pre-existing
  backward-compatibility default. Real, repaired producer `Event`s (post
  Gate 2a transition-confidence repair + Gate 2b health-lifecycle
  integration repair) always emit `health_to` now, so this is moot for
  genuine production `Event`s, but the defensive default itself was not
  hardened and remains a latent risk surface for hand-crafted/malformed
  external `Event`s.
  **Why non-blocking**: no real production code path can produce an
  `Event` missing `health_to` any more; the default only matters for
  synthetic/malformed input, which is not a path any owned normative ID
  exercises.
  **Revisit trigger/phase**: any future gate that accepts externally- or
  adversarially-constructed `Event`s into the correlation engine, or a
  dedicated hardening pass over `classify()`'s defaulting behavior.
  **Status**: open.

- **Source gate**: Phase 2 Gate 2b health-lifecycle integration repair
  **Finding**: `crates/guardian-daemon/src/dbus_surface.rs`'s
  `incidents_list_reflects_a_real_incident_the_engine_actually_opened`
  test genuinely earns its `Confirmed` result from real provenance
  attributes now (verified causally by independent review, not
  cosmetic), but it still uses hand-built `Event`s rather than exercising
  the complete `HealthTransitionProducer`/fresh-observation-advancement
  path the way `phase2_2b_contract.rs`'s repaired fixtures now do.
  **Why non-blocking**: the test's assertions are causally earned (real
  attribute values drive the real `classify()`/`transition_confidence()`
  path), not a stale fixture papering over a gap; aligning it to the full
  production path is a consistency improvement, not a correctness fix,
  and no owned normative ID requires it.
  **Revisit trigger/phase**: a future pass that consolidates
  `dbus_surface.rs`'s test fixtures onto the same real-producer/
  fresh-snapshot pattern `phase2_2b_contract.rs` now uses.
  **Status**: open.

### From Phase 2 Gate 2c evidence/governance repair

- **Source gate**: Phase 2 Gate 2c (evidence/governance repair; noted
  during this repair, not fixed by it — out of this repair's scope)
  **Finding**: every Phase 2 gate manifest's `status` field
  (`phase2-2a-*`, `phase2-2b-*`, `phase2-2c-manifest.toml`) still reads
  `"not-started"` even though each of those gates is actually closed/
  accepted or, in Gate 2c's case, in active repair — the field is never
  updated on closure anywhere in the Phase 2 manifest set.
  **Why non-blocking**: purely a stale metadata field; no gate's actual
  scope, evidence, or validation gating reads or depends on this value,
  so it does not affect what any gate proves or enforces.
  **Revisit trigger/phase**: a future pass that either wires `status`
  updates into gate-closure procedure or removes the field if it is not
  meant to be load-bearing.
  **Status**: open.

## Rule

Do not perform a giant historical backlog migration in a single pass.
Add an entry when a gate's own completion report identifies a real,
non-blocking finding; do not restate an entry already closed elsewhere
(e.g. an item a later gate's evidence shows fixed) — mark it closed here
instead, with a pointer to the closing evidence.
