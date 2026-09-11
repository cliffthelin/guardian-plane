# Guardian Phase 2 — Observability & Correlation Milestone Record

## Project identity

```text
Project display name:  Guardian Plane
Repository slug:       guardian-plane
GitHub repository:     github.com/cliffthelin/guardian-plane
Permanent D-Bus namespace: io.github.cliffthelin.Guardian1
```

## Decision

```text
Phase:              Phase 2 — Observability & Correlation
Governing:          docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md,
                     docs/guardian/30_TDD/GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md,
                     docs/guardian/30_TDD/GUARDIAN_EXECUTION_PROTOCOL.md
Normative IDs:       30 total — P2-API-001..003, P2-COR-001..007,
                     P2-EVT-001..008 (P2-EVT-006 demoted 2026-09-08 to
                     acceptance criteria under P2-EVT-005/007/008 — see
                     "Normative ID reconciliation" below), P2-INC-001..004,
                     P2-REC-001..005, P2-VM-001..003 — ALL PASS
Status:              Accepted — PASS WITH NON-BLOCKING FINDINGS, across
                     seven gates/repairs and their independent reviews
                     (Gate 2a; two Gate 2a repairs; Gate 2b; one Gate 2b
                     repair; Gate 2c; the PSI inherited-descriptor
                     production-ingress gate and its final acceptance
                     repair)
Final validation:    475 passed, 0 failed, 4 ignored, measured
                     end-to-end (including guardian-gui) on the reference
                     platform (guardian-g9, Ubuntu 26.04, systemd 259) at
                     commit 5ff2df0; cargo fmt --check clean; cargo
                     clippy --workspace --all-targets --all-features
                     -- -D warnings clean
Landing commit:      5ff2df005ba73de775aa93fd15ffe37510ce223a
                     "feat: implement Phase 2 PSI inherited-descriptor
                     production ingress" (20 files, 12 modified, 8 added,
                     +10316/-63)
```

This record is written at publication time, after acceptance — it
preserves the actual audit/repair history below rather than collapsing
it into a clean narrative that hides the real rejection and repair
cycles several Phase 2 gates went through, per the convention already
established in `docs/evidence/g9/G9_MILESTONE.md`.

## Process note on the landing commit

**Publication-process deviation.** The independently accepted PSI
candidate was committed and pushed to `origin/main` before a separate
explicit confirmation of the push step. Subsequent file-by-file
integrity verification established that the pushed commit exactly
matched the independently reviewed and accepted candidate; no unreviewed
code entered the repository. No Phase 2 milestone tag had been created
at that point. The accepted technical artifact therefore remains valid,
while the publication sequencing deviation is preserved here as process
history. Future commit, push, and tag publication boundaries require the
applicable explicit owner authorization.

## Normative ID reconciliation

All 30 Phase 2 normative IDs are owner-confirmed accepted as of
2026-09-08, with one governed status change: **`P2-EVT-006`** was
demoted from a standalone ID to acceptance criteria under
`P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`, per a dated owner governance act
recorded in three places — the PSI gate manifest's `RESOLVED 2026-09-08`
block, `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §19's fifth revision
note, and `GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §51's "Owner confirmation,
resolved" block. Its requirement text was preserved verbatim, not
deleted, per supersede-don't-erase — it now lives in the PSI gate TDD's
Phase B section and in ADR-009.

Shipped source, tests, and the service unit still cite `P2-EVT-006`
directly. Governed census, counted 2026-09-11 across `crates/**/*.rs`
and `debian/*.service`:

```text
total citation lines:                              19
  explicitly qualified as demoted/former:           4
  unqualified:                                     15

by file:  psi.rs                              8  (4 qualified)
          guardian-daemon.rs                  7  (0 qualified)
          phase2_psi_ingress_contract.rs      2  (0 qualified)
          psi_ingress.rs                      1  (0 qualified)
          debian/guardian-daemon.service      1  (0 qualified)
```

So a minority of citations carry an explicit "demoted 2026-09-08" /
"formerly the standalone `P2-EVT-006`" qualifier and the majority do
not. (An earlier draft of this record, and the independent review that
first raised the item, both stated "10" — that figure was derived from a
lossy per-file aggregation and is superseded by the census above.)

This is recorded as known citation-consistency debt (finding 4 below),
not corrected in this closure pass — doing so would be a source-code
edit outside a documentation closure reconciliation, and would need its
own review. Future cleanup should remap these citations to the surviving
owning IDs (`P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`) in a separately
reviewed, provenance-only commit.

## PSI Capability Registry consistency — closed

`Capabilities1.ListCapabilities()` and `Capabilities1.PsiSummary()`
previously disagreed structurally: `PsiSummary` read PSI state through
`PsiFileSource::real()`, a pathname probe against `/proc/pressure`,
which is unconditionally invisible to the sandboxed daemon
(`ProcSubset=pid`) — so PSI always reported unsupported there, while
`ListCapabilities` had already been repaired to classify from the shared
`PsiAvailability` state. The two public surfaces could never agree while
PSI was actually working.

The final acceptance repair makes both surfaces `Arc::clone`s of one
`PsiAvailability` instance, constructed once in `main()` from one
`PsiDescriptorPlan`, and forbids `PsiFileSource::real()` from all three
production PSI call sites (enforced by a dedicated source-text contract
test). Independently reproduced live, twice, against the real installed
daemon in the `guardian-g9` VM: the normal 3-resource state, and a
freshly-constructed `:graceful` partial-resource condition (one resource
deliberately omitted) — both surfaces agreed in both states, including
the discriminating case of a resource reporting `available=true` with
legitimately all-zero measurements versus one reporting
`available=false` with the same all-zero values. `PsiSummary`'s
measurements were confirmed to be real, moving kernel observations
(cross-checked against `/proc/pressure` read from outside the sandbox
under induced load), not synthetic placeholders.

## Inherited systemd activation descriptors — accepted, with one recorded gap

Descriptors inherited via `OpenFile=` now go through `libsystemd` 0.7.2
(pure Rust build, `unsafe` isolated inside the dependency, zero `unsafe`
in Guardian code — the workspace keeps `unsafe_code = "forbid"`), which
sets `FD_CLOEXEC` on the inherited range. Live-confirmed via
`/proc/<pid>/fdinfo` across the normal 3-resource case, a daemon
restart, the `:graceful`-omission case, and after FD renumbering (name-
based resolution correctly followed `psi-io` when it moved from fd 5 to
fd 4). Name-based FD resolution, malformed/duplicate-name handling, and
`LISTEN_FDS`/`LISTEN_FDNAMES` count-consistency checks were all found
equal to or stricter than the systemd 259 reference behavior.

**`LISTEN_PIDFDID` is not validated** (Guardian validates `LISTEN_PID`
only, via two independent implementations that must agree). This is a
genuine, exercised divergence from the systemd-259 native activation
contract, not an absent edge case — the field is live on the reference
platform. It was independently adjudicated:

```text
PIDFDID NON-BLOCKING — DIFFERENCE PROVEN IMMATERIAL UNDER GOVERNED
PRODUCTION MODEL
```

on the grounds that the one guarantee `LISTEN_PIDFDID` adds over
`LISTEN_PID` — rejecting a recycled-PID collision — describes a delivery
window that does not exist in Guardian's consumption model (the
environment is read exactly once, atomically, at startup, in a freshly-
`execve`'d child), and every other threat `LISTEN_PIDFDID` might guard
against (a forged `LISTEN_*` environment, an unrelated inherited fd) is
already available to exactly the actor who could forge `LISTEN_PID`
itself, so `LISTEN_PIDFDID` would add no defense against that actor
either. Closure path, if the production/threat model ever widens, is
recorded as forward guidance in `GUARDIAN_HARDENING_BACKLOG.md` (finding
2 below): a ~10-line safe `nix::fstat`-based pidfd-inode comparison,
using only an already-present transitive dependency, no new `unsafe`.

## Independent audit history (preserved, not collapsed)

```text
Gate 2a — correlation engine (10a20ab)
  Implemented TDD-contract Phase 2 correlation engine per contract.
  Verdict: ACCEPTED. No dedicated original evidence document exists for
  this gate specifically (acceptance recorded narratively in the Phase 2
  handoff); no manifest/TDD file exists for it either — both predate
  5a39ac9's "establish governed gate execution workflow" commit, which
  introduced the manifest/TDD convention used by every later gate.

Gate 2a health-lifecycle repair (0d071e1)
  Reopened Gate 2a to repair a health-lifecycle edge-trigger/dwell gap:
  the original test suite drove `classify()`/`transition_confidence()`
  with synthetic hand-built health-event-shaped input rather than the
  real production dwell-advancement trajectory
  (`advance_health_dwell`), which could not physically reach some
  tested transitions at the real sampling cadence.
  Evidence: docs/evidence/p2/GATE2A_HEALTH_LIFECYCLE_REPAIR_EVIDENCE.md
  Verdict: PASS WITH NON-BLOCKING FINDINGS.

Gate 2a transition-confidence repair + Gate 2b health-lifecycle
integration repair (33e2a6a)
  Landed together. Repaired a second Gate 2a defect (transition-
  confidence attribute provenance) and a Gate 2b integration gap: the
  daemon's own main() called admit_event but never advanced provider
  health through the real dwell path, so provider-health incidents could
  never open in production despite passing tests that never exercised
  main()'s actual wiring. Also updated
  GUARDIAN_HARDENING_BACKLOG.md with two related findings.
  Evidence: docs/evidence/p2/GATE2A_TRANSITION_CONFIDENCE_REPAIR_EVIDENCE.md,
            docs/evidence/p2/GATE2B_HEALTH_LIFECYCLE_INTEGRATION_REPAIR_EVIDENCE.md
  Verdict: PASS WITH NON-BLOCKING FINDINGS (each).

Gate 2b — daemon observability wiring (89ff50f)
  Implemented Gate 2b: daemon-side correlation/incident observability
  wiring and the read-only Incidents1 D-Bus surface.
  Evidence: docs/evidence/p2/GATE2B_EVIDENCE.md
  Verdict: ACCEPTED.

Gate 2c — VM evidence, retroactive collision governance (4d5df97)
  Real-VM evidence pass plus retroactive Contract Collision governance
  cleanup.
  Evidence: docs/evidence/p2/GATE2C_VM_EVIDENCE.md
  Verdict: ACCEPTED AND PUBLISHED.

PSI inherited-descriptor production-ingress gate + final acceptance
repair (5ff2df0)
  Closed the ownership gap left by TDD contract §51's false factual
  predicate ("PSI events already produced by G8's providers::psi
  wiring"): G8 had built a complete, correct, fully-tested PSI library
  capability that no production call site ever instantiated, so no live
  PSI Event had ever reached the shared Phase 2 correlation ingress.
  Newly minted P2-EVT-005/007/008 (P2-EVT-006 minted then demoted, see
  "Normative ID reconciliation" above) and P2-VM-003, owner-confirmed
  2026-09-08. A Contract Collision was correctly surfaced and stopped on
  before implementation (an OpenFile= descriptor-visibility experiment,
  recorded in phase2-psi-production-ingress-preflight.md §9, resolved
  §10) rather than resolved unilaterally.

  The implementation's own final-acceptance self-report claimed two
  fixes over an internally-identified prior contradiction (Capabilities1/
  PsiSummary disagreement; inherited descriptors lacking libsystemd/
  CLOEXEC). An independently-dispatched, read-only re-review — with no
  prior involvement in the repair — mechanically re-derived the original
  contradiction, reproduced both the normal and a freshly-constructed
  `:graceful` partial-resource condition live against the real installed
  daemon (not the implementer's transcript), adjudicated the
  LISTEN_PIDFDID divergence on its merits, reconciled test history back
  to the true immediate baseline (467/0/4, not the two-passes-earlier
  456/0/3), and ran fmt/clippy/full-suite validation independently.
  Evidence: docs/evidence/p2/PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md
  Verdict: PASS WITH NON-BLOCKING FINDINGS.
  PIDFDID verdict: PIDFDID NON-BLOCKING — DIFFERENCE PROVEN IMMATERIAL
  UNDER GOVERNED PRODUCTION MODEL.
  Exact next action returned: "Commit the accepted PSI repair and
  perform final Phase 2 closure reconciliation." (See "Process note on
  the landing commit" above for how that commit actually happened.)
```

## Test-count progression (reconciled)

```text
409/0/3  (baseline_sha 4d5df97, before any PSI-gate change)
  -> 454/0/3   (+45)  PSI library implementation
  -> 456/0/3   (+2)   physical-reachability repair
  -> 463/0/4   (+7, +1 ignored)  Phase F/H real-kernel EBUSY evidence
  -> 467/0/4   (+4)   Blockers 3 & 4 (shared PsiAvailability)
  -> 475/0/4   (+8)   final acceptance repair (this milestone's landing
                       commit)
```

The four ignored tests (unchanged in count since 463/0/4, none added by
the final repair): `providers::psi::tests::
real_kernel_psi_triggers_are_scoped_per_open_file_description_not_per_inode`
(needs a writable real `/proc/pressure/cpu`),
`restart_capability::tests::restart_unit_apply_is_idempotent_for_the_same_key_against_real_systemd`
and `restart_capability::tests::repeated_real_restarts_are_all_correlated_and_never_ambiguous`
(need a real system bus + `cups.service`), and
`restart_capability::race_tests::no_matching_signal_is_bounded_and_reports_ambiguous`
(exercises the full 20s `JOB_WAIT_TIMEOUT`) — all pre-existing,
environment/runtime-gated, not capability gaps.

## Governance items resolved at publication

The independent re-review surfaced six record-keeping inconsistencies
that did not affect any technical acceptance. Resolved here:

```text
G-1  All six Phase 2 gate manifests read status = "not-started" despite
     being accepted/closed. Already a tracked, open backlog item from
     the Gate 2c pass. ANNOTATED, NOT RENAMED, by this publication: the
     repository defines no status vocabulary at all — there is no schema
     behind schema_version = 1, and "not-started" is itself undocumented
     — so assigning a coined terminal value here would have silently
     settled an open design question by inventing semantics. Instead all
     six manifests keep their historical value and each gained a dated
     RESOLVED 2026-09-11 comment naming its accepting commit and stating
     that the status field is stale, non-authoritative historical
     metadata, and that acceptance authority is the accepted commit +
     the independent verdict and evidence + this milestone record. The
     backlog entry stays OPEN: its own revisit trigger prescribes either
     wiring lifecycle status into gate-closure procedure or removing the
     field, and neither has happened.

G-2  Gate 2a has no manifest/TDD file. Confirmed historical artifact:
     Gate 2a (10a20ab) predates 5a39ac9, which introduced the manifest/
     TDD convention every later gate uses. Not a gap to backfill.

G-3  Gate 2a has no dedicated original evidence document, only its two
     repair evidence docs. Its acceptance is recorded narratively in the
     Phase 2 implementation handoff. Historical artifact, same cause as
     G-2. Not backfilled.

G-4  P2-EVT-006 is cited on 19 lines across shipped source, tests, and
     the service unit; 4 carry an explicit demotion qualifier and 15 do
     not. Recorded as known citation-consistency debt (see "Normative ID
     reconciliation" above); not corrected in this documentation-only
     closure pass.

G-5  The PSI gate manifest's allowed_scope does not enumerate several
     files the accepted candidate actually changed: registry.rs,
     g8_real_evidence.rs, Cargo.lock, ADR-009, and several governance
     docs. Reviewed individually during the focused re-review and found
     each in-scope in substance (registry.rs/g8_real_evidence.rs are the
     Blocker-4 shared-PsiAvailability fallout already accepted in a
     prior pass; Cargo.lock is mechanical lockfile fallout from the
     allowed libsystemd addition; ADR-009 and the governance docs are
     explicitly the kind of documentation the manifest's own
     docs/evidence/p2/** and gate-doc allowances contemplate). The
     manifest's allowed_scope text was not amended retroactively to list
     them individually — recorded here as the authoritative reconciliation
     instead.

G-6  The manifest's psi.rs scope note says "no new dependency"; the
     accepted repair adds libsystemd 0.7.2. Under the strict reading
     (the clause is attached specifically to the providers/psi.rs
     library-extension entry, which gained no dependency, while
     crates/guardian-daemon/** — where libsystemd actually landed — is
     allowed wholesale) the repair was in-scope as written. The gate's
     own final-acceptance-repair task additionally and explicitly
     contemplated adopting a library for descriptor acquisition. The
     manifest was not separately amended to record the addition
     in-line — recorded here instead.
```

None of G-1 through G-6 invalidates any technical acceptance above; all
are documentation/record-keeping items that this publication act itself
resolves, per `GUARDIAN_EXECUTION_PROTOCOL.md`'s own placement rule.

## Non-blocking findings carried forward

See `GUARDIAN_HARDENING_BACKLOG.md`, section "From the Phase 2 PSI
inherited-descriptor ingress final acceptance repair", for the full,
individually-dispositioned list (regression-skew, dependency weight,
missing CLOEXEC regression guard, environment-cleanup posture,
descriptor-type validation, file-accounting, and the forward physical/
production-reachability doctrine item this repair's review surfaced
across five separate Phase 2 instances).

## Evidence index (referenced, not duplicated here)

```text
docs/guardian/30_TDD/GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md
docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md (§51)
docs/adr/ADR-009-guardian-psi-producer-topology.md (revised 2026-09-07)
docs/evidence/p2/GATE2A_HEALTH_LIFECYCLE_REPAIR_EVIDENCE.md
docs/evidence/p2/GATE2A_TRANSITION_CONFIDENCE_REPAIR_EVIDENCE.md
docs/evidence/p2/GATE2B_EVIDENCE.md
docs/evidence/p2/GATE2B_HEALTH_LIFECYCLE_INTEGRATION_REPAIR_EVIDENCE.md
docs/evidence/p2/GATE2C_VM_EVIDENCE.md
docs/evidence/p2/PHASE2_PSI_INHERITED_DESCRIPTOR_INGRESS_EVIDENCE.md
docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md
docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml
docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-tdd.md
crates/guardian-daemon/src/psi_ingress.rs (new)
crates/guardian-daemon/src/dbus_surface.rs, src/bin/guardian-daemon.rs
crates/guardian-core/src/providers/psi.rs, src/providers/registry.rs
debian/guardian-daemon.service (three additive OpenFile=:graceful lines)
```
