---
title: "Master-Spec Phase 5 — Logs & Incidents Execution Spec"
kind: "phase-execution-spec"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - phases
  - logs
  - incidents
---
# Guardian Plane — Master-Spec Phase 5: Logs & Incidents Execution Spec

**Status:** v3 architecture/TDD-ready

Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md)
for the product destination. Planning labels in this document are planning
labels only and are not normative IDs.

## 1. Goal

Raw logs remain authoritative; Guardian provides bounded policy, Log Lens,
disk-space defense, incident correlation and export.

## 2. Cross-phase reuse

Phase 5 reuses:

- Phase 2's minimal journal/kernel I/O cursor/provenance/gap contract;
- Phase 2 recorder intake/persistence;
- incident/evidence references.

It generalizes these capabilities rather than creating a second event cursor or
recorder path.

## 3. Gates

### 5A — log evidence contracts
### 5B — journald policy/provider generalization
### 5C — Log Lens
### 5D — four-layer disk defense
### 5E — incident/log correlation
### 5F — incident export
### 5G — clients
### 5H — flood/disk-pressure acceptance

## 4. Destructive-action distinction

Quota, retention and rotation are not the same as truncation.

- dedupe is a view;
- rotation may preserve evidence;
- quota/retention expires evidence under explicit policy;
- truncation deletes bytes irreversibly.

Automatic truncation remains explicit opt-in and must be classified as
irreversible (`RollbackKind::None` or equivalent truthful recovery posture).

## 5. Safety

Raw evidence links remain available subject to source retention; bounded
cardinality; export cannot starve recorder; monitored removable media never
becomes required recorder storage.

## 6. Exit

Real journal/file flood trajectory, bounded grouping, raw-reference retrieval,
disk defense and export under low/full-disk conditions.

## 7. Decisions and superseded routes

Recorded per the doctrine's requirement to preserve rejected, superseded, and
current decisions with their rationale. States are explicit; an open hypothesis
is not a decision.

### SUPERSEDED — Phase 5 as owner of the kernel-event source Phase 2 requires

An earlier review draft placed the kernel UAS/reset/error evidence source — a
link Master-Spec Phase 2's own correlation chain requires — inside Master-Spec
Phase 5. That created a backward dependency in which a later phase gated
completion of an earlier one.

**Superseded.** Master-Spec Phase 2 owns a minimal, I/O-scoped kernel/journal
event source with its own cursor/provenance/gap semantics. Phase 5 generalizes
journald policy, Log Lens and incident/log views on top of those contracts.

Rationale: the repository currently has no journald provider at all, so Phase 2
owning the minimal source duplicates nothing. The inversion, had it stood, would
have made Master-Spec Phase 2 unclosable until Phase 5 shipped.

### CURRENT — no second cursor, no second recorder authority

Phase 5 reuses Phase 2's journal cursor/provenance/gap contract and the Phase 2
recorder intake/persistence path rather than introducing parallel
implementations.

Rationale: two cursors over the same journal would disagree about gaps and
positions, and a second recorder intake would create a competing authority over
bounded evidence. Generalization extends the accepted contract; it does not fork
it.

### CURRENT — truncation is classified irreversible

Quota, retention, rotation and truncation are distinguished explicitly.
Automatic truncation stays opt-in per path and is classified as irreversible
rather than being presented with a rollback that does not exist.

Rationale: the recovery classification must be honest about what deleting bytes
means. Raw evidence remains authoritative, and a normalized or deduplicated view
never justifies destroying its source.

### CURRENT — export must not starve the recorder

Incident export is bounded and may degrade or refuse under low- or full-disk
conditions while critical collection continues. Monitored removable media is
never a required live recorder destination.

Rationale: the Master Spec's founding constraint for this area is that the
incident recorder must never depend on the thing it is diagnosing. Export is a
consumer of recorded evidence, not a competitor for its storage budget.

### No rejected routes beyond the inversion; no open hypotheses

Phase 5 carries no further rejected route and no unresolved architectural
hypothesis at this time.
