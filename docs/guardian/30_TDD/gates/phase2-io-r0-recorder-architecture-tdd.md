---
title: "Master-Spec Phase 2 R0 Recorder Lifecycle and Intake Architecture Gate TDD"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-11"
---
# Master-Spec Phase 2 R0 Recorder Lifecycle and Intake Architecture Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-io-r0-recorder-architecture-manifest.toml`.

This independent R0 root covers planning work `T2-R0-E` and owns the
already-minted `P2-REC-006` and `P2-VM-005`. Recorder architecture remains
open. This TDD defines the behavior that selects an architecture; it does not
prescribe an independent process.

## Authority chain and namespace

1. The [gate manifest](phase2-io-r0-recorder-architecture-manifest.toml) owns
   only `P2-REC-006` and `P2-VM-005` for this gate.
2. [TDD contract §52](../GUARDIAN_PHASE_0_1_TDD_CONTRACT.md) supplies their
   normative requirement text; the manifest and this TDD do not mint it.
3. The Master-Spec Phase 2 — I/O Guardian
   [Execution Spec](../phases/ms-phase2-io-guardian-execution-spec.md) and
   [Phase TDD](../phases/ms-phase2-io-guardian-tdd.md) supply mutable planning
   context.
4. The [Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md)
   supplies universal procedure, including production reachability.
5. The [Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md) supplies the
   authority boundary between the
   [Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md), phase planning, and
   gate contracts.

`T2-*` names are planning labels. Production reachability is Protocol-owned and
ID-less. Defining this gate does not accept or pass it.

This is **Master-Spec Phase 2 — I/O Guardian**, not the historical **Phase 2 —
Observability & Correlation** milestone tagged
`phase2-observability-correlation`. This gate does not reopen that milestone,
and historical wildcard references such as `P2-EVT-*` or `P2-REC-*` do not
acquire the R0-GOV IDs. Contract §52 and
[handoff §19](../GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md) are the ownership
fence.

## Normative requirements (contract §52, verbatim)

**P2-REC-006:** Recorder lifecycle and intake semantics MUST satisfy the published `T2-R0-E` failure contract, including the required boot-onward collection and failure behavior, without weakening or redefining accepted bounded-ring semantics, fresh-ingress-epoch semantics, or the loss of in-memory Incident state on `guardian-daemon` restart. Architecture selection MUST follow behavior-first evidence; this requirement does not prescribe an independent recorder process. If recorder evidence survives a daemon restart, it MUST preserve boot/daemon/ingress-epoch provenance distinguishing pre- and post-restart records and MUST NOT silently replay persisted evidence as an already-open live Incident or resurrect pre-restart Incident state. Later correlation may reference historical recorder evidence only through an explicitly governed historical-evidence path.

**P2-VM-005:** Real reference-environment evidence MUST prove the selected recorder lifecycle under daemon failure/restart, D-Bus-serving failure, intake failure/backpressure, dropped-event accounting, `/var` unavailable/full/read-only behavior, persistence-worker blocking/failure, reboot/boot provenance, and bounded persistence/replay. The proof MUST exercise interaction with the accepted fresh-ingress-epoch and in-memory-Incident-loss behavior. If recorder evidence survives daemon restart, the proof MUST demonstrate preserved boot/daemon/ingress-epoch provenance, no replay as an already-open live Incident, no resurrection of pre-restart Incident state, and historical-evidence use only through an explicitly governed path.

## Preserved invariants

No candidate may weaken or redefine:

- the accepted P0/P2 hard bound on the in-memory recorder ring;
- the accepted fresh `guardian-daemon` ingress epoch after restart;
- the accepted loss of in-memory live Incident state on daemon restart;
- the rule that critical recorder storage is local, quota-capped, and never
  requires a monitored removable device; or
- the rule that the recorder is evidence storage, not transaction authority.

Evidence surviving daemon restart creates no permission to resurrect the lost
Incident state. It is historical evidence, not a live Incident.

## Contract Collision Table (mandatory preflight — completed)

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Boot-onward recorder lifecycle and required failure behavior | G-C, `P2-REC-006` | architecture tests/experiments, then ADR | The Master Spec depicts a separate component while current code embeds recorder state in the daemon | Architecture stays open; behavior evidence selects among candidates and records any unavoidable boot gap honestly |
| Accepted bounded ring | P0/P2 recorder contract; regression only | `crates/guardian-core/src/recorder.rs` | A new intake/persistence design could weaken bounds or block producers | Existing semantics are immutable acceptance constraints; candidates must prove bounded nonblocking behavior around them |
| Fresh ingress epoch and Incident loss | Closed Phase 2 correlation/VM gates; regression only | correlation/incident code | Persisted records could be replayed as a still-open pre-restart Incident | Surviving evidence carries boot/daemon/epoch provenance and remains historical; live Incident state is never reconstructed in this gate |
| Privilege topology and hardening | ADR-002 and accepted service units; read-only here | daemon/helper/unit boundaries | A candidate could add privileged IPC, helper authority, a broker, or relax the sandbox | Any such need is Contract Collision STOP/governance review, not a scoring advantage |
| Production recorder implementation | Future post-ADR gate | production Rust, units, IPC, storage/package files | Preauthorizing all plausible paths would make the open decision fictitious | No production path is initially authorized; ADR acceptance must be followed by an exact manifest scope amendment |
| Historical evidence in later correlation | Future separately governed work | correlation ingress/model | Architecture could silently feed replayed evidence into current correlation | This gate defines the prohibition only; any historical-evidence path needs explicit future ownership and scope |

**Preflight verdict:** behavior tests, bounded experiments, and an ADR can be
completed without altering production or accepted semantics. A candidate that
requires privilege expansion, hardening relaxation, provider-authorization
change, or Incident resurrection fails or triggers governance STOP.

## Behavior-first sequence

Execution order is binding:

```text
behavior-first RED contract
→ bounded architecture experiments
→ compare candidates against identical behavior
→ governed ADR
→ independently accept and amend exact production scope
→ implementation in a later authorized pass
→ reference-environment proof
```

This artifact-authoring pass creates or runs none of those RED tests,
experiments, implementation, or VM scenarios.

## RED proof to create during gate execution

RED is architecture-neutral. It must fail because no selected, governed
recorder lifecycle currently proves every required behavior—not because the
test demands a process name, IPC mechanism, unit, or storage implementation.

The same harness/scenario contract evaluates:

1. current embedded `BoundedRecorder` in `guardian-daemon`;
2. an independent minimal unprivileged recorder process; and
3. a serious intermediate design based on repository-supported boundaries,
   specifically the existing `RecorderSink`/spill-worker seam: retain
   daemon-owned bounded intake/ring while isolating bounded persistence behind
   a supervised worker boundary.

The intermediate must be implemented far enough as a bounded experiment to
measure its lifecycle and failure behavior. It may lose the comparison, but it
cannot be dismissed because it is not the preferred architecture. A thread is
not credited as process-failure independence.

## Mandatory behavior matrix

For each candidate, record supported, unsupported, failed, and unknown—not an
optimistic narrative—for these scenarios in order:

| Scenario | Required observable proof |
|---|---|
| Daemon SIGKILL/unexpected death | collection continuity or explicit bounded loss/gap; no fabricated graceful-drain guarantee |
| Daemon restart | fresh daemon epoch; pre-restart live Incident absent; surviving evidence remains historical |
| D-Bus-serving failure | intake/collection behavior independent of claims about the bus connection |
| Intake unavailable/backpressured | producer does not block indefinitely; bounded rejection/drop result |
| Dropped events | bounded/saturating accounting with provenance; no silent loss |
| `/var` unavailable | in-memory behavior remains bounded and truthful; no removable fallback |
| `/var` read-only | persistence failure is observable and does not masquerade as success |
| `/var` full/quota reached | deterministic bounded retention/rejection; no unbounded retry or memory growth |
| Persistence worker blocks/fails | producer and queue remain bounded; worker failure is observable |
| Reboot/boot | earliest supported collection point and unavoidable gap are measured; boot identity is explicit |
| Persistence bound | hard quota and deterministic retention are demonstrated |
| Replay boundary | bounded replay cannot become live state implicitly or duplicate current intake |
| Fresh ingress epoch | pre/post daemon records carry distinct daemon/epoch provenance |
| Incident loss | accepted in-memory Incident reset remains unchanged regardless of evidence survival |

Time, queue, ring, retry, record-size, spill, and persistence limits must be
finite and test-controlled. Failure injection must not depend on filling an
unbounded real filesystem or endangering the host.

## Evidence-survival contract

If any candidate preserves evidence across daemon restart, every persisted
record must retain explicit:

- boot provenance;
- producing daemon/process provenance;
- ingress epoch or an explicit pre-ingress source epoch; and
- ordering/gap information sufficient to distinguish pre- and post-restart.

On restart, persisted evidence must not reopen an already-open live Incident,
reconstruct pre-restart Incident state, or enter current correlation as though
newly observed. Later correlation may reference it only through a separately
governed historical-evidence path. If provenance is missing or ambiguous, the
record remains historical/unknown and fails closed for any safety decision.

## Candidate comparison and ADR

The comparison uses the same behavior matrix and records at least: lifecycle,
failure isolation, producer blocking risk, loss accounting, bounds, boot gap,
storage identity, replay semantics, privilege/hardening impact, packaging
complexity, and migration/rollback consequences.

ADR-011 must state the selected architecture, why each alternative lost,
evidence and tests, earliest collection point, unavoidable gaps, exact intake
and storage ownership, restart/reboot behavior, security boundary, and the exact
production files that a post-decision manifest amendment should authorize. The
ADR itself does not authorize those files.

## GREEN and reference-environment proof

Architecture-decision GREEN requires all behavior tests and experiments, a fair
candidate comparison, and an accepted ADR without production implementation.
`P2-VM-005` is not complete until a later authorized implementation is exercised
in the real reference environment against every matrix row, including actual
daemon/D-Bus/process lifecycle and controlled `/var` failure injection.

Reference evidence must show bounds and provenance from raw outputs, preserve
the fresh-ingress-epoch and Incident-loss boundary, and prove any surviving
evidence is not replayed as an open Incident. Component-only construction or a
manually launched substitute for the selected packaged topology is
insufficient.

## Scope and stop conditions

The initial gate may add only its exact behavior-test files, bounded probe, ADR,
and evidence. It may not change production recorder code, daemon code, units,
IPC, storage, packages, Cargo files, helper authority, or public APIs. After the
ADR is accepted, production implementation requires an independently reviewed
scope amendment naming only the selected files.

STOP if a candidate requires a new privileged boundary, expanded helper
authority, generic privileged broker, daemon hardening relaxation, provider
authorization semantics to change, silently widened storage authority,
unbounded buffering/retry, monitored-removable storage, or weakened
Incident/epoch semantics.

## Evidence by normative ID

- `P2-REC-006`: behavior contracts, bounded experiments, candidate comparison,
  and ADR-011.
- `P2-VM-005`: raw reference-environment evidence for the selected implemented
  lifecycle after the required post-decision scope amendment.
