---
title: "Master-Spec Phase 4 — Thermal & Power Execution Spec"
kind: "phase-execution-spec"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - thermal
  - power
---
# Guardian Plane — Master-Spec Phase 4: Thermal & Power Execution Spec

**Status:** v3 architecture/TDD-ready

Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md)
for the product destination. Planning labels in this document are planning
labels only and are not normative IDs.

## 1. Goal

Unified thermal/power observation and bounded control without competing writers.

The universal Provider Arbitrator exists before Phase 4. Phase 4 owns the
resource-level thermal/power ownership signals and provider adapters needed to
make arbitration enforceable in this domain.

## 2. Provider/ownership model

For each controllable resource/channel represent:

- physical/logical resource identity;
- authoritative controller/provider;
- read-only vs writable;
- channel-level ownership evidence;
- intervention owner/revision;
- provider delegation semantics;
- safe limit provenance;
- rollback/recovery class.

Daemon presence is not by itself proof that a particular channel is owned.

Delegating a supported request through the authoritative controller is not a
single-writer violation. Competing with that controller is.

## 3. Proposed gates

### 4A — sensor/resource/ownership contracts
### 4B — hwmon/platform telemetry
### 4C — thermald/PPD ownership and delegation
### 4D — NVML
### 4E — storage-temperature adapter
Phase 4 owns the first temperature-specific storage adapter needed for thermal
views. Phase 6 later extends/reuses storage providers for broader health.
### 4F — per-machine Thermal Policy
### 4G — optional CoolerControl/LACT delegation
### 4H — real-system acceptance

## 4. Authorization

For provider-owned control, preserve provider-policy authorization through the
accepted actual-caller relay rule. Use Guardian-owned actions only for truly
Guardian-owned policy.

## 5. Restoration

Thermal/profile/cap restoration is conditional:

- target identity unchanged;
- provider still authoritative;
- safe limit still valid;
- no external/later intervention superseded it.

Temperature history/hardware effects cannot be rolled back.

## 6. Non-goals

No general fan overclocking, arbitrary sysfs writes, duplicate PWM owner, or
global hard-coded safety threshold.

## 7. Exit

Truthful telemetry, resource-level ownership, provider delegation,
per-machine policy, conditional restoration and real provider-state transitions.

## 8. Decisions and superseded routes

Recorded per the doctrine's requirement to preserve rejected, superseded, and
current decisions with their rationale. States are explicit; an open hypothesis
is not a decision.

### CURRENT — Phase 4 owns the domain realization, not the universal rule

Arbitration and the single-writer rule are established control-plane
architecture accepted in Phase 0 (`crates/guardian-core/src/arbitration.rs`,
G3). Phase 4 owns the thermal/power-specific realization: resource- and
channel-level ownership signals, provider adapters, and delegation semantics
that make arbitration enforceable for these resources.

Rationale: the Master Spec introduces the single-writer rule inside its Thermal
& Power discussion, which invites the misreading that Phase 4 originates it. An
independent review drew exactly that inference and proposed that Master-Spec
Phase 3 wait on Phase 4 gate `4C` for arbitration; that route was rejected on
repository evidence and is preserved in the Master-Spec Phase 3 Execution Spec.
Recording the counterpart here keeps both halves of the correction visible.

### CURRENT — delegation is not a single-writer violation

Dispatching a supported request through the authoritative controller
(`thermald`, power-profiles-daemon, or an optional provider such as
CoolerControl/LACT) preserves that controller's authority and is permitted.
Writing the same channel in competition with it is not.

Rationale: the Master Spec directs Guardian to surface these tools' state and
defer control to them rather than duplicate their low-level PWM/GPU logic. A
rule that forbade all delegated requests would make `4C` and `4G` impossible
while providing no additional safety.

### CURRENT — telemetry and ownership precede any control gate

`4A` (contracts) and the ownership model precede every write-capable gate, and
fan control plus other high-consequence actions remain outside the normal tier
per the Master Spec's explicit deferral.

Rationale: absent owner certainty must fail closed to read-only. Establishing
which channels are genuinely writable, and by whom, is a precondition for any
control decision rather than a byproduct of one.

### CURRENT — Phase 4 owns the first storage-temperature adapter

Gate `4E` produces the temperature-specific storage adapter. Master-Spec Phase 6
gate `6F` reuses that identity and adapter for broader storage health rather
than creating a second storage provider.

Rationale: the dependency runs forward (Phase 4 → Phase 6), so no later phase
becomes a prerequisite for an earlier one, and one storage identity avoids two
providers disagreeing about the same device.

### No rejected routes; no open hypotheses

Phase 4 carries no rejected route of its own and no unresolved architectural
hypothesis at this time. This is recorded factually rather than populated for
symmetry with other phases.
