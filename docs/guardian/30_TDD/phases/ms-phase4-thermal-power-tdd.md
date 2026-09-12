---
title: "Master-Spec Phase 4 — Thermal & Power TDD"
kind: "phase-tdd"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - tdd
  - thermal
  - power
---
# Guardian Plane — Master-Spec Phase 4: Thermal & Power TDD

Phase-level proof obligations for
[Master-Spec Phase 4 — Thermal & Power Execution Spec](ms-phase4-thermal-power-execution-spec.md).
Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md);
subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. Section labels are planning
labels, not normative IDs. The universal production-reachability rule is owned by
[Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) and applied — not
redefined — here.

## T4-1 — sensor truth
Finite values, units, disappearance/reappearance, identity, unavailable ≠ zero.

## T4-2 — resource-level ownership
Test daemon/provider present but channel unowned; channel owned; ownership
changes; ambiguous/conflicting owner; provider restart.

Presence alone cannot decide write permission.

## T4-3 — provider delegation
PPD/thermald/optional provider supported request is delegated through the
authoritative controller rather than treated as competing low-level write.

Provider-policy authorization uses actual caller/details.

## T4-4 — NVML
Throttle reason fidelity, multi-GPU identity, unsupported metrics, provider
loss, no write surface until governed.

## T4-5 — storage temperature
Phase-4 adapter handles SATA/NVMe/USB/removal and is reusable by Phase 6 storage
health without a second identity/provider.

## T4-6 — Thermal Policy
Provider/manufacturer limits preferred; override provenance; machine-specific;
unknown safe range fails closed.

## T4-7 — intervention lifecycle
Snapshot, ownership, authorization, Apply, observed effective state,
confirmation, conditional restore.

Inject provider/owner change before restore. Stale restore must fail closed.

## T4-8 — production reachability
Real sensor/throttle change; supported profile/cap request; owner/provider change
during intervention.
