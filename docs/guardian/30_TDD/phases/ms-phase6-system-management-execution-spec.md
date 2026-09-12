---
title: "Master-Spec Phase 6 — System Management Execution Spec"
kind: "phase-execution-spec"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - system-management
---
# Guardian Plane — Master-Spec Phase 6: System Management Execution Spec

**Status:** v3 architecture/TDD-ready

Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md)
for the product destination. Planning labels in this document are planning
labels only and are not normative IDs.

## 1. Goal

Broader system management through bounded provider-specific operations, without
turning Guardian into a generic admin broker.

## 2. Existing substrate

Wave 1 service restart is accepted. Phase 6 extends it.

## 3. Authorization ownership — no blanket "Guardian action per domain"

Every write declares one of two authorization models:

### Guardian-owned policy
A closed Guardian polkit action is valid only when Guardian genuinely owns the
policy decision.

### Provider-owned policy mediated by Guardian
The privileged helper:

- resolves the actual inbound caller;
- checks the provider's real action and complete authorization-relevant details
  through a closed internally-derived request;
- then calls the provider;
- never trusts provider authorization of root helper as end-user authorization;
- never substitutes a Guardian action that changes policy ownership silently.

The Master Spec's example Guardian action names do not override the later,
higher-authority relay-authorization contract for provider-owned operations.

## 4. Gates

### 6A — common mutation/recovery contract
### 6B — AccountsService session selection
### 6C — service-management extension from Wave 1
### 6D — fwupd
### 6E — NetworkManager
### 6F — storage health, reusing Phase 4 storage identity/temperature adapter
### 6G — updates
### 6H — hwctl
### 6R — later Recovery target

## 5. Recovery honesty

Each domain declares native rollback, compensation, best effort, none and
human-recovery cases.

NetworkManager preserves native checkpoint ownership/results. Service operations
do not claim process/application state restoration. Firmware/packages do not
claim generic downgrade rollback. Session preference restoration does not undo
a session already launched.

## 6. Safety

No RunCommand/RunShell/arbitrary property broker; actual caller; correct policy
owner; stable target; intervention ownership; provider restart/reboot semantics;
partial outcomes visible; no automatic service disable.

## 7. Exit

Provider-specific domains accepted without weakening Wave 1 or provider policy.

## 8. Decisions and superseded routes

Recorded per the doctrine's requirement to preserve rejected, superseded, and
current decisions with their rationale. States are explicit; an open hypothesis
is not a decision.

### SUPERSEDED — gate 6C as new service management

An earlier draft described gate `6C` as introducing bounded service management
over a systemd D-Bus provider, with no reference to existing work.

**Superseded on repository evidence.** Wave 1 already shipped a production
service-restart mutation — `crates/guardian-helper/src/restart_capability.rs`,
accepted and tagged `wave1-first-production-mutation` — with polkit
authorization, live-read preconditions, transaction and correlation semantics
already in place. Gate `6C` therefore *extends* that accepted capability rather
than re-owning the domain, and its baseline acceptance tests must first prove
the Wave 1 behavior remains unchanged.

Rationale for preservation: the omission was found only by auditing the
repository rather than the plan. Recording it keeps the reason `6C` is an
extension visible to anyone reading the gate later.

### CURRENT — per-domain authorization ownership, declared explicitly

Each write declares whether Guardian genuinely owns the policy decision (closed
Guardian polkit action) or the provider owns it (helper-mediated relay resolving
the actual caller with internally derived details).

The Master Spec's illustrative polkit action names — such as the
`org.guardian.*` examples in its architecture section — are examples of
namespacing, not an instruction to substitute a Guardian-owned action wherever a
provider already owns the policy decision. The later, higher-authority
relay-authorization contract governs provider-owned operations.

Rationale: substituting a Guardian action for provider policy would silently
move the policy owner and let provider authorization of the root helper stand in
for end-user authorization. Declaring the model per write makes that
substitution impossible to perform accidentally.

### CURRENT — no generic administrative broker

The public surface never becomes `RunCommand`, `RunShell`,
`SetArbitrarySystemdProperty`, or `InvokeProvider(name, opaque_payload)`. Each
domain receives its own provider, typed contract, polkit scope, risk
classification, preconditions and rollback semantics.

Rationale: a generic broker collapses every domain's distinct authorization and
recovery semantics into one unbounded action, which is the failure mode the
whole privilege topology exists to prevent. Phase 6 does not close by providing
an escape hatch for unimplemented domains.

### CURRENT — storage health reuses the Phase 4 adapter

Gate `6F` consumes the storage identity and temperature adapter produced by
Master-Spec Phase 4 gate `4E` rather than creating a second storage provider.

Rationale: the dependency runs forward (Phase 4 → Phase 6). Two storage
providers could disagree about the same device's identity or health.

### CURRENT — high-consequence actions stay deferred

General fan overclocking, kernel parameter tuning, automatic driver changes,
forced USB resets and automatic service disabling remain deferred out of the
normal-control tier. Phase 6 does not absorb them merely because it is named
System Management.

### CURRENT — Recovery target is a later, separately governed subphase

Gate `6R` is treated as a later Phase 6 subphase requiring its own boot/recovery
ADR and exceptionally strong VM proof, not a prerequisite for ordinary system
management. It must also revisit inherited-descriptor environment assumptions,
since it changes boot and process topology.

### No open hypotheses

Phase 6 carries no unresolved architectural hypothesis at this time.
