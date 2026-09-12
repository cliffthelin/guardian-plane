---
title: "Master-Spec Phase 3 — Observability TDD"
kind: "phase-tdd"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - tdd
  - observability
---
# Guardian Plane — Master-Spec Phase 3: Observability TDD

Phase-level proof obligations for
[Master-Spec Phase 3 — Observability Execution Spec](ms-phase3-observability-execution-spec.md).
Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md);
subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. Section labels are planning
labels, not normative IDs. The universal production-reachability rule is owned by
[Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) and applied — not
redefined — here.

## T3-0 — universal reachability

Every gate carries real producer/cadence/lifecycle/sandbox proof.

## T3-1 — observation/history

Provenance, finite values, bounded history, UNKNOWN ≠ zero, registry/live
sampling timestamps explicit.

## T3-2 — PSI overview

Natural pressure/recovery, independent resource degradation, no client severity
recompute.

## T3-3A — Phase-2-target extension

Starting from Phase 2's mutation-grade identity:

- PID reuse;
- cgroup movement/lifetime;
- unit invocation changes;
- session association;
- process exit;
- bounded top-N/history.

## T3-3B — attribution

Two competing workloads, temporal mismatch, innocent busy process, unavailable
dimension, contradictory association.

## T3-4 — diagnostic budget

Refused escalation visible; no unbounded trace; no "no problem" inference.

## T3-5 — boot baseline

Comparable samples, first-run state, outliers, real controlled regression,
qualified explanation.

## T3-6 — evidence-chain

Aligned, missing, contradictory, temporal mismatch, multiple hypothesis cases.

## T3-7 — shared intervention extension

Prove use of Phase 2's provider/intervention ownership.

Required cases:

- active Phase 2 intervention + Phase 3 request;
- composable overlap;
- rejected conflict;
- external systemd property change;
- target revision change;
- controller/ancestor constraint change;
- confirmation session disappears;
- daemon/helper restart;
- buffered-I/O/writeback attribution;
- rollback while another intervention remains active.

No stale restoration may clobber later state.

## T3-8 — real throttle effect

Property write plus measured effective control under real controller/device
support.

## T3-9 — client consistency

Equivalent daemon-owned truth across GUI/TUI/CLI.

## T3-10 — composition

```text
two workloads
→ pressure
→ ranked evidence retaining innocent alternative
→ shared intervention ownership
→ measured effect
→ safe restore or explicit non-restoration
```
