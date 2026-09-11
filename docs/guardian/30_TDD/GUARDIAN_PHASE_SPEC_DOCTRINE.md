---
title: "Guardian Phase Spec Doctrine"
kind: "phase-spec-doctrine"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - planning
  - governance
  - phases
---
# Guardian Phase Spec Doctrine

## Authority boundary

The [Guardian Master Spec](../00_Project/GUARDIAN_MASTER_SPEC.md) defines the
durable product destination and phase outcomes. A Phase Execution Spec records
the current, mutable route toward that destination. A Phase TDD records the
current phase-level proof obligations for that route.

Phase Execution Specs and Phase TDDs are planning and coordination documents.
They do not become normative merely because they live in `30_TDD/`, and they do
not outrank the Master Spec, accepted contracts, gate manifests/TDDs, ADRs, or
other established normative authority.

A Phase Spec may revise architecture strategy, experiments, gate order,
provider route, implementation boundaries, or deferrals. It may not silently
revise a Master-Spec outcome.

## Mutable route; preserved decisions

Mutation is expected; silent mutation is forbidden. Each Phase Spec maintains:

- phase goal and outcomes;
- exit criteria and dependencies;
- current architecture and safety invariants;
- planned gates, experiments, opportunities, and deferrals; and
- rejected, superseded, and current decisions with their rationale.

When evidence changes the route:

```text
evidence
→ decide whether destination or route is affected
→ preserve the superseded decision and rationale
→ revise the Phase Spec and affected Phase TDD
→ create the revised RED proof
→ continue
```

Implementation difficulty or a failing first implementation is never sufficient
reason to weaken a requirement. Stop, preserve the evidence, distinguish a
mechanism failure from a requirement failure, obtain the governing decision,
and write the revised RED proof before new production implementation.

## Universal production reachability

The universal production-reachability rule is owned by
[Guardian Execution Protocol](GUARDIAN_EXECUTION_PROTOCOL.md), not by a phase
identifier. Phase documents must apply that rule when planning gates; they do
not duplicate it as a separate normative home.

## Write ownership, authorization, and recovery

The Provider Arbitrator is decision policy: it determines which provider is
authoritative and whether Guardian policy permits a write. It is not runtime
enforcement, caller-authorization proof, a mutex or lease, a transaction
ledger, serialization of overlapping interventions, or proof that later
rollback remains safe.

Every write requires a resource-scoped intervention-ownership contract that
states at least:

- stable target, resource, and property identity;
- active Guardian intervention owner and transaction;
- arbitration revision and validation preconditions;
- whether overlapping Guardian requests compose, queue, or are rejected;
- external-change detection;
- expiry and confirmation ownership;
- crash/restart recovery; and
- a restoration rule that cannot overwrite a later Guardian or external
  intervention.

Authorization follows the accepted helper/core topology. Guardian uses its own
typed polkit action only when it owns the policy decision. When a privileged
helper relays a provider-owned action, it resolves the actual requester at its
own inbound D-Bus boundary, checks the provider's real action and all
authorization-relevant details through a closed internally derived request, and
never accepts raw action IDs or detail maps from a caller. Provider
authorization of the root helper is not authorization of the end user.

Every write also states honest action-specific recovery semantics: the snapshot,
native rollback if any, Guardian compensation if any, irreversible effects,
confirmation/timeout behavior, crash-point classification, external-change
invalidators, and when human recovery is required. `Native`, `Emulated`,
`BestEffort`, and `None` classify recovery; they do not promise perfect
restoration.

## Cross-phase invariants

- Clients remain unprivileged.
- Public privileged operations remain typed and bounded; no generic privileged
  broker is introduced.
- Provider hierarchy is native API/library, kernel interface, structured CLI,
  then scraped CLI only when no stronger interface exists.
- Unknown ownership, capability, identity, authorization, or preconditions fail
  closed for writes.
- Diagnostics remain bounded and must not worsen the constrained resource.
- Raw evidence remains authoritative; normalized views do not justify deleting
  it.
- Recorder state is bounded and never requires monitored removable media as its
  live destination.
- `UNKNOWN` is never silently presented as healthy.
- Real privileged and system-level proof occurs in governed reference or
  disposable environments.

## Experiments and opportunities

An experiment proves or rejects a route; it does not silently change the
destination. A discovery that strengthens a phase goal may propose a Phase-Spec
revision. A useful adjacent capability belongs in a later phase or backlog.
When an approach is impossible, preserve the evidence and supersede the method
without weakening the required outcome.

## Phase closure and naming

Phase closure requires reconciled exit criteria, production composition and
reachability evidence, honest lifecycle/authorization/ownership/recovery
semantics, explicit deferrals, and independent acceptance. A library-only
capability is not production completion.

Historical **Phase 2 — Observability & Correlation** remains an accepted
repository milestone. It is distinct from Master-Spec Phase 2 — I/O Guardian;
future planning must always use the qualified Master-Spec phase names, including
Master-Spec Phase 3 — Observability, to avoid treating the historical milestone
as completion of the Master-Spec I/O Guardian outcome.

## Gate-manifest convention

New gate manifests omit a lifecycle `status` field unless a future governed
schema explicitly reintroduces one. Existing historical manifests retain their
status fields as preserved, stale, non-authoritative history and are not
retroactively edited for this convention.

## Planning provenance

This doctrine records the final planning route that followed Sonnet
reconciliation, Astra cross-phase challenge, v3/v3.1 verification, and final
architecture acceptance. Those reviews are provenance, not doctrine; their
handoff prompts are not integrated as normative requirements.
