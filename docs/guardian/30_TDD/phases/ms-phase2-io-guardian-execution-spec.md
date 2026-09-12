---
title: "Master-Spec Phase 2 — I/O Guardian Execution Spec"
kind: "phase-execution-spec"
status: "active"
last_reviewed: "2026-09-12"
tags:
  - planning
  - phases
  - io-guardian
---
# Guardian Plane — Master-Spec Phase 2: I/O Guardian Execution Spec

**Status:** v3 — residual execution required

Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md).
Subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md)
for the product destination. Planning labels in this document are planning
labels only and are not normative IDs.

## 1. Phase status

Master-Spec Phase 2 remains **PARTIAL and substantially incomplete**.

The historical `phase2-observability-correlation` milestone is accepted and is
not reopened. It supplies correlation/incident/PSI substrate, not the complete
I/O Guardian product phase.

## 2. Stable goal

```text
physical port/device
→ block/partition/filesystem/mount
→ process/workload relationship
→ latency/kernel evidence
→ PSI
→ I/O evidence chain + incident
→ least-disruptive eligible recovery
→ honest confirmation/recovery handling
→ bounded incident recording
```

## 3. Accepted foundation

Reuse:

- Event/Incident/correlation foundation;
- provider-health lifecycle;
- live PSI ingress;
- Capability Registry;
- Provider Arbitrator decision model;
- G4 transaction/recovery machinery;
- helper/core privilege topology;
- Wave 1 provider-policy relay-authorization precedent;
- bounded in-memory recorder.

Do not confuse these models with production enforcement that does not yet exist.

## 4. Reconciled gaps

| Requirement | State |
|---|---|
| udev/physical topology | GAP |
| UDisks identity | PARTIAL |
| UDisks → correlation | GAP |
| process/open-handle evidence | GAP |
| mutation-grade process/cgroup/unit identity | GAP |
| I/O aggregate stats | GAP |
| request-latency evidence | GAP/diagnostic capability to define |
| I/O kernel/journal error source | GAP |
| PSI | COMPLETE |
| incident refs/confidence substrate | COMPLETE |
| normal-tier I/O mutations | GAP |
| bounded ring | COMPLETE |
| quota-capped local persistence | GAP |
| recorder independent lifecycle/intake | UNRESOLVED |

## 5. Residual dependency graph

Planning labels only.

```text
T2-IO-GOV
   ↓
T2-IO-R0
   ├───────────────┐
   ↓               ↓
T2-IO-R1           T2-IO-R3
   └───────┬───────┘
           ↓
T2-IO-R2A / T2-IO-R2B
           ↓
T2-IO-R4
           ↓
Master-Spec Phase 3
```

R2 contract drafting and provider experiments may overlap R3 after R0, but a
mutation capability is not accepted/published while recorder lifecycle/intake
remains undecided or unusable.

## 6. T2-IO-GOV — governance preflight

Before new gate manifests:

- settle manifest status lifecycle/remove-field decision;
- adopt production-reachability in the Execution Protocol;
- establish future Master-Spec phase/gate naming;
- record the execution-route reconciliation for the Master Spec's literal
  "transient scopes" wording;
- preserve actual-caller/provider-policy authorization as a standing write
  boundary.

### Transient-scope route reconciliation

The Master Spec's product outcome is reversible runtime cgroup throttling via
systemd/cgroup v2 rather than permanent unit edits.

v3's current execution strategy is:

- if an eligible workload already belongs to an authoritative systemd
  unit/scope, prefer a bounded runtime property change on that existing target
  where systemd/provider semantics support it;
- create a new transient scope only when Guardian explicitly owns enrollment and
  can prove moving/enrolling the workload does not violate lifecycle,
  accounting, authorization or service ownership.

This is an explicit Phase-Spec route refinement, not a silent Master-Spec edit.

## 7. T2-IO-R0 — architecture, access topology, identity and intake

R0's **first deliverable is an accepted access/topology decision**, before
production provider implementation.

The G-A/R1 proof-ownership repair separates two evidence levels:

```text
R0 exact-production-equivalent candidate decision
    ↓
R1 implementation + actual packaged production proof
```

G-A exercises each real upstream producer from a disposable/reference probe
whose access profile is proven equivalent for every source-relevant constraint
of the intended packaged execution boundary. That proves route feasibility and
the candidate completeness contract; it is not evidence of an unimplemented
production constructor, packaged consumer, or composed evidence chain.

### 7.1 Sandbox/access matrix

For every required source, prove candidate access under a production-equivalent
reproduction of every source-relevant packaged-unit constraint:

| Source | Preferred mechanism | Sandbox boundaries to exercise | Governed fallback decision |
|---|---|---|---|
| udev/topology | native udev/netlink or existing provider | `AF_UNIX`, PrivateNetwork, namespace restrictions | narrow supplied channel/collector/provider reuse |
| UDisks | system D-Bus | bus/provider availability | truthful unavailable |
| process/open handles | `/proc` or stronger supported source | ProtectProc, ProcSubset, cross-user permissions | narrow read collector if justified |
| disk aggregate stats | `/sys/block/*/stat` or strongest reachable source | actual unit sysfs visibility | descriptor/collector only if required |
| request latency | bounded diagnostic provider | diagnostic budget + privilege | explicit unavailable/escalation state |
| kernel I/O events | journal/kernel source | ProtectKernelLogs, journal permissions | minimal gap-aware event source |
| PSI | inherited descriptors | already accepted | reuse unchanged |

Do not broaden the daemon/helper sandbox as the default answer.

Do not reject a route or call it unavailable merely because R1 has not yet
implemented its production constructor or consumer. A route that needs relaxed
hardening, new helper authority, or another privileged boundary triggers
Contract Collision STOP/replan.

An empty process/open-handle scan under incomplete visibility is **not proof that
a filesystem is unused**.

### 7.2 Shared evidence identity contract

Define stable identities and coverage/completeness semantics for:

- physical USB/device/block/mount;
- process lifetime;
- cgroup lifetime;
- systemd manager + unit/scope + invocation;
- current block-device mapping;
- observation source/cursor/revision.

### 7.3 Mutation-grade target contract

Before R2A, define the minimum safe target used to authorize throttling:

- boot/process lifetime identity;
- systemd manager, unit/scope, current invocation;
- cgroup lifetime/membership;
- current controlled block-device identity/mapping;
- controller availability + relevant ancestor constraints;
- affected workloads;
- protected-target exclusions;
- ownership/arbitration revision.

PID or cgroup pathname alone is insufficient.

### 7.4 Correlation-model extension remit

R0 explicitly owns the contract for extending the accepted correlation model to
new I/O evidence classes. R1 will implement:

- I/O evidence types;
- temporal joins;
- topology/resource associations;
- missing/contradictory evidence;
- incident representation/confidence.

Do not alter accepted PSI/provider-health rules merely to make new events fit.

### 7.5 Recorder lifecycle/intake architecture

G-C must be independently accepted before any R1 scope amendment, R1 RED, or
R1 implementation begins. This necessarily settles the recorder decision before
R1 interfaces freeze.

The Master Spec depicts `guardian-recorder` as a boot-onward component; current
code embeds recorder state in `guardian-daemon`, creates writable state first,
and feeds it from monitoring ticks.

Current architectural hypothesis:

> If "boot onward" and failure independence require collection to survive
> `guardian-daemon` or its serving D-Bus connection failing, an independent
> minimal recorder process is load-bearing.

R0 must produce an ADR/evidence decision covering:

- earliest supported collection point / unavoidable boot gap;
- independent intake path;
- bounded nonblocking producer queue;
- dropped-event reporting;
- daemon/D-Bus failure behavior;
- `/var` unavailable/full behavior;
- boot/session provenance;
- replay;
- storage identity safety;
- whether embedded architecture can satisfy the required failure behavior;
- reconciliation with the already-accepted daemon-restart boundary:
  `guardian-daemon` restart creates a fresh ingress epoch and loses its
  in-memory Incident state.

If R0 selects an independent recorder that survives `guardian-daemon`, that
creates an intentional asymmetry:

```text
recorder evidence/timeline may survive daemon restart
while correlation Incident state does not
```

That asymmetry is acceptable only when specified deliberately. Persisted
recorder events must retain boot/daemon/ingress-epoch provenance and must not be
silently replayed as though the pre-restart Incident remained open. Changing the
accepted incident-loss/fresh-epoch semantics is outside R0 unless separately
governed.

A separate thread does not satisfy process-failure independence.

## 8. T2-IO-R1 — production I/O evidence chain

After R0:

```text
physical/udev/UDisks identity
+ process/workload evidence
+ aggregate I/O stats
+ bounded request-latency evidence where available
+ kernel I/O error/reset events
+ PSI
→ new I/O association rules
→ shared ingress
→ correlated I/O incident/evidence chain
```

Aggregate disk counters and per-request latency are separate capabilities.
Do not infer outstanding request age/process ownership from aggregate counters.

Minimal Phase 2 kernel/journal ingestion owns cursor/provenance/gap semantics;
Phase 5 later generalizes it.

R1 owns the unchanged production-strength requirements `P2-EVT-009` and
`P2-VM-004`. For every G-A-selected route, R1 must implement and independently
prove the real upstream producer, actual production constructor/call site,
actual packaged consumer, provider topology, unchanged accepted production
sandbox, real cadence, startup/restart lifecycle, applicable disappearance and
re-enumeration, degraded/loss/gap behavior, production composition, and
integration into shared ingress and I/O correlation. G-A candidate-probe
evidence and host-shell visibility cannot satisfy this proof.

The R1 gate depends on all three R0 gates: access topology, identity contract,
and recorder architecture. Its production-file scope remains withheld until
the accepted G-A ADR/source matrix identifies the selected routes and a
separately reviewed manifest amendment names the exact files.

## 9. T2-IO-R3 — recorder intake + bounded persistence

Implement the R0 recorder decision.

Must provide:

- bounded intake;
- bounded ring;
- quota-capped local persistence;
- nonblocking/dropped-event semantics;
- persistence worker that cannot accumulate blocked writes;
- daemon/D-Bus/storage failure behavior;
- never depend on monitored removable media;
- transaction/incident evidence intake required by R4.

The recorder is not the transaction database.

## 10. T2-IO-R2A — shared cgroup I/O intervention

Phase 2 owns the first production capability; Phase 3 reuses/extends it.

### Apply owner

Default architecture:

```text
client
→ typed privileged helper operation
→ helper resolves actual caller
→ provider-policy authorization as required
→ helper-owned transaction
→ systemd D-Bus
→ observed effective control
```

Pure target/arbitration/validation model code may live in `guardian-core`.
That does not make the unprivileged daemon the mutation owner.

### Intervention ownership

R2A must define a durable resource/property intervention record sufficient to
prevent stale restoration and overlapping-write corruption.

At minimum:

- target + property;
- transaction/intervention identity;
- arbitration/target revision;
- prior effective value;
- Guardian-applied value;
- expiry/confirmation state;
- active overlapping Guardian intervention(s);
- external-change detection;
- crash recovery classification.

The existing transaction engine should remain the transaction authority; do not
create a second transaction database merely for R2A.

### Authorization

If systemd/provider policy owns authorization, the helper mediates the
provider's real action/details for the actual requester through the accepted
closed provider-authorization request shape.

No root-helper relay may bypass end-user policy.

### Overlap and external change

For the same resource/property, explicitly choose:

- composable;
- serialized/queued;
- rejected as conflict.

Rollback/restoration may occur only while target identity, intervention
ownership and relevant revision remain valid. A Phase 2 rollback must never
erase a later Phase 3/external intervention.

### Effective throttling

A successful property write is not acceptance. Real controller/device support,
ancestor constraints, and measured effect must be observed.

Include buffered-I/O/writeback behavior in supported acceptance coverage.

### Required product disposition

R2A is part of Master-Spec Phase 2 closure on governed supported targets unless
the owner explicitly changes the product requirement. It is not an optional
"if accepted" convenience.

## 11. T2-IO-R2B — moderate storage recovery

Typed capabilities, where provider semantics support them:

- clean unmount;
- UDisks safe power-off;
- read-only remount.

### Honest recovery classification

**Clean unmount**
- busy refusal is normal/safe;
- remount is conditional compensation, not restoration of application state.

**UDisks power-off**
- no general automatic inverse;
- sibling impact and explicit user intent/preconditions required;
- disappearance may be expected success;
- reconnect may require human action;
- `RollbackKind::None`/human recovery may be correct.

**Read-only remount**
- returning writable is conditional;
- never auto-restore writable merely because a timer expires after filesystem
  errors.

Per-operation provider-policy authorization ownership must be explicit.

High/Very-High recovery remains deferred.

## 12. T2-IO-R4 — capability-positive integrated acceptance

Must prove at least one supported real mutation path, not only observe-only
branches:

```text
real I/O condition
→ real evidence
→ R1 chain/incident
→ recorder capture
→ eligible R2 action
→ observed effective outcome
→ honest confirm/recovery handling
→ persisted bounded evidence
```

Also cover device disappearance/re-enumeration and refused/busy paths.

## 13. Phase 2 exit criteria

- all required providers have accepted access topology;
- evidence completeness is explicit;
- mutation-grade target identity exists;
- I/O association model is productionized;
- recorder lifecycle/intake/persistence is accepted;
- R2A exists on governed supported targets;
- eligible R2B capabilities are honestly classified;
- actual-caller/provider-policy authorization is preserved;
- overlapping interventions/external changes cannot be overwritten by stale
  rollback;
- R4 passes through a real supported mutation capability;
- Phase 3 consumes, not repairs, these contracts.

## 14. Decisions and superseded routes

Recorded per the doctrine's requirement to preserve rejected, superseded, and
current decisions with their rationale. States are explicit; an open hypothesis
is not a decision.

### SUPERSEDED — G-A proves actual production composition before R1

The original R0-GOV ownership assigned `P2-EVT-009` and `P2-VM-004` to G-A,
requiring actual production constructors and consumers even though G-A precedes
and forbids production provider implementation. The accepted G-A RED probe at
SHA-256
`d224a52544dde336604abbdcb246fcc9eed9a46762777bff37b6ab8dab345952`
exposed that impossible contract before evidence was falsified to close it.

### CURRENT — candidate proof in G-A; production proof in R1

G-A owns `P2-EVT-011` and `P2-VM-006`: a governed source-route decision and
real-upstream proof under a production-equivalent reproduction of every
source-relevant access constraint. R1 owns the unchanged `P2-EVT-009` and
`P2-VM-004`: implementation and actual packaged production composition. This
changes the execution route, not the Master-Spec outcome or R0-to-R1 dependency
direction, and preserves the stronger production proof rather than weakening
it.

### SUPERSEDED — Master-Spec Phase 2 as a reconciliation formality

An earlier planning draft treated Master-Spec Phase 2 as substantially satisfied
by the historical `phase2-observability-correlation` milestone, requiring only
reconciliation.

**Superseded by repository evidence.** An independent audit of production call
sites found 2 of 9 required evidence-chain links present (PSI ingress and the
incident refs/confidence substrate), 0 of the normal-tier I/O recovery rungs
implemented, and the recorder's quota-capped local spill explicitly open in
`guardian-daemon`'s own source and startup logging. UDisks topology events are
observed but reach a log line rather than the shared correlation ingress, and
UDisks power-off precondition validation is exercised only from example
binaries, never from production.

Rationale for preservation: the superseded reading is the exact error the
production-reachability rule now guards against — treating library capability as
production completion.

### CURRENT — Phase 2 owns its minimal kernel/journal I/O event source

An earlier review draft assigned the kernel UAS/reset/error evidence source to
Master-Spec Phase 5, which would have made a later phase a prerequisite for
completing an earlier one.

**Current decision:** Master-Spec Phase 2 owns a minimal, I/O-scoped
kernel/journal event source with cursor/provenance/gap semantics sufficient for
its own outcome. Phase 5 later generalizes journald policy, Log Lens and
incident/log views, and reuses these contracts rather than creating a second
cursor or recorder intake authority.

Rationale: no forward-phase dependency may gate an earlier phase's own required
production capability. The repository has no journald provider today, so nothing
is duplicated by Phase 2 owning the minimal source.

### CURRENT — one cgroup writer, established in R2A

The Master Spec assigns cgroup throttling to both the Phase 2 recovery ladder
(Low rung) and Phase 3 throttle-before-kill. An earlier draft left both phases
claiming it independently.

**Current decision:** `T2-IO-R2A` introduces the first and only shared typed
cgroup I/O-throttle capability, provider and durable intervention record.
Master-Spec Phase 3 reuses and extends that same writer for whole-machine
policy; it does not create competing ownership, a second writer, or a second
intervention ledger.

Rationale: the G2 privilege inventory classifies `cgroups.transient_scopes` as
provider-owned, so the apply route is systemd D-Bus through the privileged
helper regardless of which phase requests it. Two owners of one writer is the
conflicting-control failure class the Master Spec's single-writer rule exists to
prevent.

### CURRENT — route refinement on the Master Spec's literal "transient scopes"

Recorded in §6. The Master Spec's destination — reversible runtime throttling
via systemd/cgroup v2 rather than permanent unit edits — is preserved. The
execution route prefers a bounded runtime property change on an existing
authoritative unit/scope, creating a new transient scope only where Guardian
owns enrollment.

Rationale: re-parenting an already-managed service into a Guardian-created scope
would change its cgroup accounting and lifecycle ownership, colliding with the
Master Spec's own instruction not to fight existing managers for write
ownership. This refines the mechanism, not the destination, and is recorded
rather than applied silently.

### OPEN HYPOTHESIS — recorder independent vs embedded lifecycle

**Not a decision.** §7.5 states the current architectural hypothesis that an
independent minimal recorder process is load-bearing if "boot onward" and
failure independence require collection to survive `guardian-daemon` failing.

Repository facts informing it: the current `BoundedRecorder` is in-process
state created and fed by the daemon's monitoring tick, so it terminates with the
daemon; the Master Spec depicts `guardian-recorder` as a distinct boot-onward
component; the existing Flight Recorder page requires survival of persistence
failure but asserts nothing about process independence.

This remains RED until `T2-IO-R0` produces an ADR/evidence decision, including
reconciliation with the accepted daemon-restart fresh-ingress-epoch and
in-memory Incident-loss boundary. It must not be recorded as resolved, and a
separate thread does not satisfy process-failure independence.

### CURRENT — high and very-high recovery remain deferred

The Master Spec defers forced USB resets, driver/module intervention and the
other highest-consequence actions out of the normal-control tier. Phase 2 does
not pull them forward to mark the phase complete; only the Observe, Low and
Moderate rungs are in scope.
