---
title: "Master-Spec Phase 3 — Observability Execution Spec"
kind: "phase-execution-spec"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - observability
---
# Guardian Plane — Master-Spec Phase 3: Observability Execution Spec

**Status:** v3 — detailed plan; production implementation blocked on Master-Spec Phase 2 closure

Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md)
for the product destination. Planning labels in this document are planning
labels only and are not normative IDs.

## 1. Goal

Build explainable whole-machine observability from completed Phase 2 pressure,
identity, I/O evidence, intervention and recorder contracts.

## 2. Hard prerequisites

Phase 3 production work begins after Phase 2 provides:

- reachable provider topology;
- minimum process/cgroup/unit mutation-grade identity;
- production I/O association model;
- working recorder intake/persistence;
- shared cgroup provider + intervention ownership/ledger;
- integrated real-system acceptance.

## 3. Core architecture

Daemon/core owns observations and inference. Clients render.

Every bottleneck result distinguishes observation, association, hypothesis,
confidence, contradictions, missing evidence, provenance and temporal relation.

Always-on collection is bounded; expensive diagnostics are budget-governed.

## 4. Gate plan

### 3A — observability/history contracts

Bounded history, provenance, degradation and registry/live-sampling timing.

### 3B — PSI-first overview

Production-backed CPU/memory/I/O pressure and history.

### 3C-i — richer process/resource attribution provider

This **extends Phase 2's minimum mutation-grade target contract**. It does not
introduce process→cgroup→unit identity for the first time.

Adds richer:

```text
process ↔ cgroup ↔ systemd unit/scope ↔ session
CPU/RSS/I/O contribution
history/drill-down
```

### 3C-ii — attribution/correlation

Association is not causation. Preserve multiple candidates.

### 3D — boot-health baseline

Machine-relative baseline. Phase 2 udev/storage facts available; Phase 4 thermal
facts optional until produced.

### 3E — evidence-chain bottleneck engine

Consumes Phase 2 I/O evidence and later optional thermal/log evidence.

### 3F — whole-machine throttle-before-kill extension

Consumes the **same Phase 2 intervention owner, provider and durable
intervention record/ledger**.

Phase 3 owns policy/eligibility and additional bounded resource semantics; it
does not create a second writer or restoration mechanism.

A Phase 3 intervention must compose/reject deterministically with an active
Phase 2 intervention. Its rollback may not erase another active intervention.

Confirmation/session loss, external property changes and inherited controller
constraints are part of this gate.

### 3G — clients

GUI/TUI/CLI render daemon authority.

### 3H — real composition

Natural workload → pressure → attribution → evidence-chain → optional shared
intervention → observed effect/recovery.

## 5. Safety

- no stale target writes;
- no second intervention ledger/writer;
- no unsupported causal claims;
- no unbounded deep diagnostics;
- missing evidence is explicit;
- intervention restore requires valid identity/ownership/revision.

## 6. Exit

Phase 3 closes when observability and intervention policy operate over the
completed Phase 2 contracts without reimplementing their providers,
authorization or mutation ownership.

## 7. Decisions and superseded routes

Recorded per the doctrine's requirement to preserve rejected, superseded, and
current decisions with their rationale. States are explicit; an open hypothesis
is not a decision.

### REJECTED — "Phase 3 depends on Phase 4 for single-writer arbitration"

An independent review asserted that gate `3F` could not proceed until
Master-Spec Phase 4 gate `4C` supplied single-writer arbitration, on the reading
that the Master Spec introduces the single-writer rule within its Thermal &
Power section.

**Rejected on repository evidence.** `crates/guardian-core/src/arbitration.rs`
is the Provider Arbitrator, accepted at G3 in Phase 0 under the Phase 0/1 TDD
contract. It is capability-generic — it answers which provider is authoritative
for a capability and whether Guardian policy permits a write — and it predates
Master-Spec Phase 4 entirely. Master-Spec Phase 4 owns the thermal/power
*realization* of ownership signals and adapters, not the universal rule.

Consequence: `3F` uses the established arbitration contract and the accepted
helper/core privilege topology directly. It does not wait for, and must not
re-invent, arbitration.

Rationale for preservation: the rejected route was a reasonable inference from
Master-Spec prose and was disproven only by reading accepted source. Recording
it prevents the same inference being drawn again from the same prose.

### SUPERSEDED — Phase 3 readiness as a reconciliation judgement

An earlier draft made Phase 3 implementation contingent on "Master-Spec Phase 2
reconciliation sufficient for Phase 3 data contracts" — a judgement call with no
named artifacts.

**Superseded by** the hard prerequisites in §2, which name the specific Phase 2
residual work that must complete first. The repository audit that forced this
change found the Phase 3 evidence-chain engine would otherwise have had roughly
two of nine required evidence links available, unable to produce the Master
Spec's own worked bottleneck example.

### SUPERSEDED — single combined process/cgroup attribution gate

An earlier draft carried one gate `3C` that bundled building a process/cgroup
provider together with attribution logic.

**Superseded by** the `3C-i` / `3C-ii` split. The two are different risk
classes: one is new kernel-surface ingestion under the accepted daemon sandbox,
the other is pure correlation over data already obtained. `3C-i` is further
scoped as an *extension* of Phase 2's mutation-grade target contract rather than
first introduction of process→cgroup→unit identity.

### CURRENT — Phase 3 extends the Phase 2 cgroup writer

Master-Spec Phase 2 establishes the single shared typed cgroup intervention
capability, provider and durable intervention record. Phase 3 reuses that exact
writer and ledger, owning only eligibility policy, additional bounded resource
semantics, and whole-machine decision behavior.

Rationale: one writer per controlled resource. A second Phase 3 writer or
restoration mechanism would reintroduce the conflicting-control failure class
the single-writer rule exists to prevent, and would make stale-rollback
corruption possible across phases.

### CURRENT — evidence-chain ordered after Phase 2 I/O assembly

Gate `3E` is sequenced after Phase 2 `R1` because the Master Spec's evidence
chain is predominantly I/O-chain data. Phase 4 thermal facts remain explicitly
optional until that capability exists, so `3D` and `3E` degrade truthfully
rather than blocking on a later phase.

### No open hypotheses

Phase 3 currently carries no unresolved architectural hypothesis of its own. The
recorder lifecycle question is owned by Master-Spec Phase 2 `R0` and is recorded
there, not duplicated here.
