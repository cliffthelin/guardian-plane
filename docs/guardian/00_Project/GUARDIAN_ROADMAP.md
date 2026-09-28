---
title: "Guardian Roadmap"
kind: "roadmap"
status: "active"
last_reviewed: "2026-09-27"
tags:
  - guardian
  - wiki
  - roadmap
  - status
---
# Guardian Roadmap

**This is the single authoritative entry point for project memory: what
is decided, what is in progress, what is complete, and what remains
planned.** It exists so that no agent — including a fresh session with no
prior context — has to reconstruct project state from raw git history,
scattered handoffs, or an external search.

This page routes; it is not a second normative home. Authority for any
specific claim stays in the document it points to. If this page and a
linked document disagree, the linked document wins and this page is
stale — fix it (see [Maintenance](#maintenance)).

## How to use this page

Read this before consulting any other file in the repository, and before
consulting any external source, for a question about:

- a prior decision or its current status;
- work in progress;
- completed work;
- remaining planned work.

This requirement is not optional and is stated as a hard rule in
`AGENTS.md` under "Required project-memory routing." If this page answers
the question, stop here or follow its pointer. Only fall through to other
repository files, and then to external sources, when it does not.

## Completed work

All accepted, tagged gates and milestones are tracked in one place — the
[TDD Gate Index](../30_TDD/TDD_Gate_Index.md). Do not restate gate status
here; it drifts out of sync. As of this page's `last_reviewed` date:

- Phase 0/1 gates G0–G9: all **PASS**, each tagged `phase0-g0-...` through
  `phase0-g9-...`.
- Wave 1 (first production mutation): **PASS**, tagged
  `wave1-first-production-mutation`.
- Historical milestone **Phase 2 — Observability & Correlation**: accepted,
  tagged `phase2-observability-correlation`. This is distinct from
  Master-Spec Phase 2 (I/O Guardian) below — see the Gate Index's own
  disambiguation.

## In-progress work / open decisions

The active frontier is **Master-Spec Phase 2 — I/O Guardian**, governed by
its [Execution Spec](../30_TDD/phases/ms-phase2-io-guardian-execution-spec.md)
and [Phase TDD](../30_TDD/phases/ms-phase2-io-guardian-tdd.md) under the
[Guardian Phase Spec Doctrine](../30_TDD/GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Read the Execution Spec's own "Phase status" and "Reconciled gaps"
sections for the exact current state; do not treat the summary below as a
substitute.

- R0 (architecture/access-topology/identity/intake decision) is open.
  Three gates: **G-A** (access topology), **G-B** (evidence/mutation-grade
  identity), **G-C** (recorder lifecycle/intake architecture). Normative
  IDs were minted by owner confirmation; no R0 manifest, TDD, or
  implementation is authorized yet.
- The G-A/R1 proof-ownership split is the current accepted reading:
  G-A owns a governed candidate source-route decision; production
  provider implementation and proof belong to R1, which depends on all
  three R0 gates.
- Recorder independent-vs-embedded lifecycle is an explicit **open
  hypothesis**, owned by R0/G-C — not yet promoted to a decision.
- Non-blocking findings carried forward from prior gates, including any
  still open from the Phase 2 PSI final-acceptance repair, live in the
  [Hardening Backlog](../30_TDD/GUARDIAN_HARDENING_BACKLOG.md). Check its
  `Status: open` entries before assuming a finding is unaddressed or
  re-discovering one.

Do not begin R1 scope, R0 manifests, or implementation from this page
alone — confirm against the Execution Spec and this repository's gate
discipline in `AGENTS.md` first.

## Remaining planned work

Master-Spec phases beyond Phase 2 are planning/proof-route documents, not
started, reached through the
[TDD Gate Index](../30_TDD/TDD_Gate_Index.md#master-spec-phase-26-execution-planning-not-numbered-gates):

- Phase 3 — Observability
- Phase 4 — Thermal & Power
- Phase 5 — Logs & Incidents
- Phase 6 — System Management

Each has its own Execution Spec and Phase TDD under
`docs/guardian/30_TDD/phases/`. None is normative merely by existing; none
is evidence that planning work has started implementation.

## Maintenance

This page must be updated, as part of the same change, whenever:

- a gate, wave, or Master-Spec phase changes status (opened, passed,
  repaired, superseded);
- an R0/R1/R2… decision is accepted or reopened;
- a Hardening Backlog entry is opened or closed at a scope large enough
  to change the "In-progress work" summary above.

Bump `last_reviewed` on every such update. See the
[Wiki Update Workflow](../50_Operations/Wiki_Update_Workflow.md) for the
full per-change checklist. A change that alters project status without
updating this page is incomplete.
