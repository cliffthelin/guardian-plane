---
title: "Master-Spec Phase 5 — Logs & Incidents TDD"
kind: "phase-tdd"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - tdd
  - logs
  - incidents
---
# Guardian Plane — Master-Spec Phase 5: Logs & Incidents TDD

Phase-level proof obligations for
[Master-Spec Phase 5 — Logs & Incidents Execution Spec](ms-phase5-logs-incidents-execution-spec.md).
Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md);
subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. Section labels are planning
labels, not normative IDs. The universal production-reachability rule is owned by
[Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) and applied — not
redefined — here.

## T5-1 — Phase 2 cursor/intake reuse
No duplicate journal cursor/provenance/gap semantics; no second recorder intake
authority.

## T5-2 — raw evidence
Views never mutate raw journal evidence.

## T5-3 — normalization
Volatile fields collapse; meaningful device/error/service dimensions do not.

## T5-4 — bounded dedupe
Bounded state, deterministic flush/eviction, hostile unique-message flood.

## T5-5 — journald policy
Typed supported fields only; provider-policy authorization where applicable;
reload/external-change failure.

## T5-6 — disk defense
Distinguish:
- quota/retention;
- rotation;
- truncation.

Truncation tests must explicitly acknowledge irreversible deletion and never
pretend rollback exists.

## T5-7 — export/full disk
Low-space/full-disk export refusal/degradation while critical collection
continues; bounded producer/recorder behavior.

## T5-8 — production reachability
Real journal flood, real growing file with open writer descriptor, real
low/full-disk export failure.
