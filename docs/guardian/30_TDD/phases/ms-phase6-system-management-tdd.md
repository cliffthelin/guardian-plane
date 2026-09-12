---
title: "Master-Spec Phase 6 — System Management TDD"
kind: "phase-tdd"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - tdd
  - system-management
---
# Guardian Plane — Master-Spec Phase 6: System Management TDD

Phase-level proof obligations for
[Master-Spec Phase 6 — System Management Execution Spec](ms-phase6-system-management-execution-spec.md).
Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md);
subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. Section labels are planning
labels, not normative IDs. The universal production-reachability rule is owned by
[Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) and applied — not
redefined — here.

## T6-1 — no generic broker

Reject generic command/argv/file/sysfs/D-Bus/property mutation.

## T6-2 — authorization ownership

For every write prove whether policy is Guardian-owned or provider-owned.

Provider-owned relay tests require:

- actual caller;
- real provider action;
- complete internally-derived authorization details;
- no raw caller-supplied action/detail surface;
- root helper provider check not treated as user authorization.

## T6-3 — session
Discovery, AccountsService/fallback, next-login, invalid/unavailable sessions,
external preference change, conditional preference restore.

## T6-4 — services
Preserve Wave 1 restart baseline; new start/stop separately typed; provider
restart; job partial/failure; helper crash; no blind replay; no auto-disable.

## T6-5 — fwupd
Inventory/update/preconditions/reboot/provider loss; irreversible/provider-
specific recovery classification.

## T6-6 — NetworkManager
Checkpoint owner, nonzero timeout, partial per-device outcomes, session
disconnect, provider restart, checkpoint loss, reboot/out-of-checkpoint changes.

## T6-7 — storage health
Reuse Phase 4 storage identity/temperature provider; removal/re-enumeration;
unsupported ≠ healthy.

## T6-8 — updates
Current provider contract, stale metadata, partial failure, reboot, no generic
rollback claim.

## T6-9 — hwctl
Informational only.

## T6-10 — Recovery target
ADR, boot dependency proof, no graphical desktop, safe exit, no silent default
target, revisit inherited-descriptor environment assumptions.

## T6-11 — reachability
Each write through real provider lifecycle with crash/restart/partial outcomes.
