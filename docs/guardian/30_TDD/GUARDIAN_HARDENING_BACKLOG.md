---
title: "Guardian Hardening Backlog"
kind: "hardening-backlog"
status: "active"
last_reviewed: "2026-09-11"
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
  **Prior disposition (Phase 2 milestone publication, preserved)**: not
  closed at that pass. It (`docs/evidence/p2/PHASE2_MILESTONE.md`,
  governance item G-1) confirmed the finding, extended it to a sixth
  manifest (the PSI inherited-descriptor-ingress gate, same defect), and
  annotated all six in place: each now carries a dated `RESOLVED
  2026-09-11` comment naming its accepting commit and stating explicitly
  that the `status` field is stale, non-authoritative historical
  metadata, and that acceptance authority is the accepted commit + the
  independent verdict and evidence + the milestone record. The stale
  values themselves were left at `"not-started"` on purpose: this
  entry's own revisit trigger prescribes either wiring lifecycle status
  into gate-closure procedure or removing the field, and neither had
  happened at that point. The repository defines no status vocabulary at
  all (there is no schema behind `schema_version = 1`, and
  `"not-started"` is itself undocumented), so assigning a coined terminal
  value during a publication pass would have silently settled an open
  design question by inventing semantics. An explicitly-marked stale
  field was the more honest interim state.
  **Status**: closed — resolved by commit
  `4710ecd092db368bf91bb37b9eac4904d67baa66`. The revisit trigger's
  second branch was taken:
  `docs/guardian/30_TDD/GUARDIAN_PHASE_SPEC_DOCTRINE.md` now establishes
  the gate-manifest convention that new gate manifests omit a lifecycle
  `status` field unless a future governed schema explicitly reintroduces
  one, and that existing historical manifests retain their status fields
  as preserved, stale, non-authoritative history and are not
  retroactively edited for this convention. The mechanism question that
  kept this entry open is therefore settled going forward.

  Scope of the resolution, stated precisely: the closure is **omit the
  field going forward and preserve historical stale metadata**. It is not
  "update the old statuses." No historical manifest was edited by that
  commit, and the six Phase 2 manifests' existing `status =
  "not-started"` values did **not** become accurate — they remain stale
  metadata, now explicitly annotated as such and explicitly
  non-authoritative. Acceptance authority remains the accepted commit
  plus the independent verdict/evidence plus the milestone record.

### From the Phase 2 PSI inherited-descriptor ingress final acceptance repair (`docs/evidence/p2/PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md`, independent re-review)

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair (found by the independent focused re-review, not by
  the repair itself)
  **Finding**: the two tests added for the Part B (CLOEXEC) fix only
  prove that `acquire_canonical_listen_fds()` returns `Ok(0)` and does
  not panic in a non-activated test process; neither asserts
  `FD_CLOEXEC` on anything. Deleting the canonical-acquisition call from
  `psi_descriptor_plan_from_env` would leave all 475 workspace tests
  green while silently regressing CLOEXEC.
  **Why non-blocking**: the fix itself is proven — by live evidence
  (`/proc/<pid>/fdinfo` inspection against the real installed daemon
  across the normal case, a restart, the `:graceful` case, and FD
  renumbering), not by these two tests. Only the *regression guard* is
  missing, not the property itself.
  **Revisit trigger/phase**: a future pass over `psi_ingress.rs` tests
  that adds a source-text production guard analogous to
  `dbus_surface_never_opens_psi_by_pathname` — e.g. asserting the
  production region reachable from `psi_descriptor_plan_from_env`
  contains the `acquire_canonical_listen_fds()` call.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: Guardian validates `LISTEN_PID` (via two independent
  implementations that must agree) but not `LISTEN_PIDFDID`, a genuine
  divergence from the systemd-259 native activation contract that is
  live and exercised on the reference platform, not merely absent.
  **Why non-blocking**: independently adjudicated — "PIDFDID
  NON-BLOCKING — DIFFERENCE PROVEN IMMATERIAL UNDER GOVERNED PRODUCTION
  MODEL." The one guarantee `LISTEN_PIDFDID` adds over `LISTEN_PID`
  (rejecting a recycled-PID collision) describes a delivery window that
  does not exist in Guardian's consumption model (environment read once,
  atomically, at startup, in a freshly-`execve`'d child); every other
  threat it might guard against is already available to exactly the
  actor who could forge `LISTEN_PID` itself.
  **Revisit trigger/phase**: if the production/threat model ever widens
  beyond a single systemd-launched instance (e.g. descriptors consumed
  after a re-exec, or `LISTEN_*` propagated through an intermediary).
  Closure path if needed: a ~10-line safe `nix::fstat`-based pidfd-inode
  comparison, using the already-present transitive `nix` dependency, no
  new `unsafe`.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: adopting `libsystemd` 0.7.2 for one `fcntl`-based
  `FD_CLOEXEC` call pulled in 8 new crates (including an HMAC-SHA256
  primitive and a full `nom` parser-combinator stack, neither used by
  Guardian's actual call path). The candidate evaluation compared three
  systemd-protocol crates but never evaluated the minimal primitive: bare
  `nix::fcntl` (already a transitive dependency) would deliver
  byte-identical CLOEXEC semantics for 2 crates instead of 8, with zero
  new `unsafe`.
  **Why non-blocking**: defensible trade-off, not an oversight in the
  shipped decision — `libsystemd` also provides an independent,
  separately-maintained re-implementation of the `LISTEN_PID` check that
  Guardian cross-validates against and fails closed on disagreement,
  which bare `nix::fcntl` would forfeit. The evaluation as documented is
  incomplete (it should have named and rejected the `nix`-only
  alternative explicitly), but the chosen dependency is proportionate in
  outcome.
  **Revisit trigger/phase**: a future dependency-audit pass, or if
  `libsystemd`'s maintenance status changes.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: `LISTEN_PID`/`LISTEN_PIDFDID`/`LISTEN_FDS`/`LISTEN_FDNAMES`
  are deliberately never unset from the daemon's environment after
  activation is consumed (`unset_env: false`), unlike native
  `sd_listen_fds(1)`.
  **Why non-blocking**: equivalent to the native `sd_listen_fds(0)` mode,
  which is itself a supported native option. Stale reconsumption is
  independently closed two other ways: `psi_descriptor_plan_from_env()`
  is called exactly once, before the D-Bus server is built, and the
  daemon never `exec`s a child (verified by reading the call graph); even
  in a hypothetical future self-`exec`, `FD_CLOEXEC` has already closed
  the raw descriptors by then.
  **Revisit trigger/phase**: any future change that introduces a
  self-`exec` or child-process spawn from `guardian-daemon`.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: `ListCapabilities` (a `CAPABILITY_REGISTRY_TICK_INTERVAL
  = 30s`-refreshed snapshot) and `PsiSummary` (a live read) share one
  authoritative `PsiAvailability` state but not one sampling instant —
  after a runtime PSI degradation, `PsiSummary` can flip before
  `ListCapabilities` catches up, for up to ~30s.
  **Why non-blocking**: this is not the structural, permanent
  contradiction the repair fixed (that was closed and live-verified in
  both the normal and `:graceful` states); it is bounded staleness
  inherent to a periodically-refreshed registry snapshot shared by all
  six capability providers, not specific to PSI.
  **Revisit trigger/phase**: a future gate that needs sub-30s capability-
  registry freshness for any provider, or that unifies the registry-tick
  and live-read sampling models.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: Guardian does not verify inherited PSI descriptors are
  actually procfs pressure files (e.g. via `statfs`/`PROC_SUPER_MAGIC`)
  before trusting them — only their advertised `LISTEN_FDNAMES` name and
  `LISTEN_PID` match are checked.
  **Why non-blocking**: only exploitable by an actor who can already
  `execve` `guardian-daemon` with a controlled environment and controlled
  fds 3/4/5 — i.e. one who controls the root-owned unit file or is root,
  under the governed production model where the launcher is PID 1. Such
  an attacker has far more direct paths available than forging PSI
  readings.
  **Revisit trigger/phase**: a future hardening pass, or if Guardian's
  threat model is extended to include a launcher less trusted than
  systemd/root.
  **Status**: open.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: the implementation's own final-acceptance self-report
  enumerated 7 focused-repair files; the actual working tree at review
  time held 8 untracked files — the omitted one was
  `docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md`
  (the Contract Collision stop-and-report record, which predates the
  repair and contains no code).
  **Why non-blocking**: documentation-only omission from a self-report's
  file inventory; the file itself was reviewed, was legitimately in
  scope, and its absence from the count did not hide any unreviewed
  change. Execution-protocol step 7 already requires reviewers to derive
  the changed-file set from Git directly rather than trust an
  implementer's inventory, which is exactly what caught this.
  **Revisit trigger/phase**: none specific — recorded so the pattern
  (self-reported file counts should be cross-checked against `git
  status`, not trusted) stays visible.
  **Status**: closed — the file is included in the landing commit
  (`5ff2df0`) and accounted for in
  `docs/evidence/p2/PHASE2_MILESTONE.md`.

- **Source gate**: Phase 2 PSI inherited-descriptor ingress, final
  acceptance repair
  **Finding**: the independent re-review of the physical/production-
  reachability defect class (see below) observed the same underlying
  failure mode recur across five separate Phase 2 instances (Gate 2a
  health-lifecycle repair, Gate 2b integration repair, PSI production
  instantiation, the PSI threshold repair, and the Capabilities1/
  PsiSummary contradiction itself) — a fully green test suite repeatedly
  coexisted with a production code path that could never physically
  reach the behavior the tests exercised, because the tests drove
  synthetic/direct input rather than the real production trajectory at
  the real sampling cadence.
  **Why non-blocking**: each of the five instances was independently
  found and repaired at the gate where it occurred; there is no current
  unrepaired instance. This entry exists to make the recurring pattern
  itself visible, since no existing normative rule in
  `GUARDIAN_EXECUTION_PROTOCOL.md` currently names it.
  **Revisit trigger/phase**: a future Phase 3 contract/protocol pass
  should consider minting a normative rule requiring evidence to show a
  given input/measurement is physically producible at the production
  sampling cadence through the actual production call path — not merely
  constructible in a unit test — with a defined proof-obligation form
  (an EWMA/arithmetic argument, a call-graph argument, or a live
  sandboxed-production-surface argument, depending on what is being
  proven). The five instances above are the evidentiary basis if/when
  that rule is drafted.
  **Status**: closed — resolved by commit
  `4710ecd092db368bf91bb37b9eac4904d67baa66`, which added the mandatory
  universal **Production-reachability preflight** to
  `docs/guardian/30_TDD/GUARDIAN_EXECUTION_PROTOCOL.md`. That section
  requires, for every production state transition or composed capability
  relied upon by a capability, incident, recovery or safety decision, at
  least one acceptance test exercising a trajectory the real upstream
  system can physically produce under the actual production cadence,
  sandbox, provider topology and lifecycle. The same commit's
  `GUARDIAN_PHASE_SPEC_DOCTRINE.md` assigns ownership of that universal
  rule to the Execution Protocol rather than to any phase identifier, so
  phase documents apply it without redefining it. It carries no normative
  ID: every `P0`/`P1`/`P2` ID in this repository is phase-scoped and
  owned by a gate manifest, and an ID here would wrongly imply manifest
  ownership of a rule that binds every gate. The five instances above are
  preserved in that section as its historical justification.

  Scope of the resolution, stated precisely: this finding was a
  **forward doctrine/governance gap** — the recurring pattern had no
  named rule — and that forward gap is what the commit closed. No
  historical Phase 2 implementation was retroactively changed by it. Each
  of the five instances had already been found and repaired at the gate
  where it occurred, and those repairs and their evidence stand
  unmodified.

## Rule

Do not perform a giant historical backlog migration in a single pass.
Add an entry when a gate's own completion report identifies a real,
non-blocking finding; do not restate an entry already closed elsewhere
(e.g. an item a later gate's evidence shows fixed) — mark it closed here
instead, with a pointer to the closing evidence.
