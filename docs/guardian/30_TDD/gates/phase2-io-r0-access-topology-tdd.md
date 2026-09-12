---
title: "Master-Spec Phase 2 R0 Access Topology and Completeness Gate TDD"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-11"
---
# Master-Spec Phase 2 R0 Access Topology and Completeness Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-io-r0-access-topology-manifest.toml`.

This discovery/proof gate covers planning work `T2-R0-A` and `T2-R0-B` and
owns the already-minted requirements `P2-EVT-009` and `P2-VM-004`. It selects
no source merely from source inspection and authorizes no production provider
implementation or unit-file change.

## Authority chain and namespace

1. The [gate manifest](phase2-io-r0-access-topology-manifest.toml) owns only
   `P2-EVT-009` and `P2-VM-004` for this gate.
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

**P2-EVT-009:** Guardian's production I/O evidence sources MUST expose truthful evidence availability and completeness under the actual packaged production topology. An observation MUST be interpreted as authoritative absence only when the selected provider contract proves authoritative completeness for the target. Partial, unavailable, and unknown visibility MUST remain explicit and MUST fail safely; incomplete visibility MUST NOT be treated as proof that a filesystem is unused or as sufficient evidence for a safety decision that requires authoritative completeness.

**P2-VM-004:** Real disposable/reference-environment evidence MUST prove the selected G-A I/O evidence routes from the actual packaged production daemon/helper topology with the accepted daemon sandbox active and unchanged. Host-shell-only reachability is insufficient. The proof MUST exercise the real upstream producer, production consumer, production cadence, provider topology, sandbox, and lifecycle applicable to each source, including its partial, unavailable, or unknown visibility behavior.

## Required outcome

Produce an accepted, source-by-source access decision for the actual packaged
Guardian topology. For every selected source the decision defines the evidence
that can be observed and exactly when a missing observation means:

- authoritative absence;
- partial visibility;
- unavailable visibility; or
- unknown.

Existing `Availability` and `Knowledge` concepts are reused or extended when
they can express the required facts. Any extension must preserve the difference
between provider availability and epistemic completeness. A second vocabulary
with overlapping meanings is a collision, not an implementation convenience.

**Binding invariant:** absence of observed handles or processes is never
reported as "filesystem unused" unless the selected provider contract proves
authoritative completeness for that target and observation interval.

## Contract Collision Table (mandatory preflight — completed)

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Truthful production I/O evidence availability and completeness | G-A, `P2-EVT-009` | Future source contracts; existing `guardian-provider-api::Availability` / `Knowledge` are read-only precedent | A new I/O-only taxonomy could contradict accepted provider semantics | Reuse the accepted types where sufficient; document any missing dimension and STOP for an exact scoped extension rather than create overlapping states in this gate |
| Actual packaged reachability | G-A, `P2-EVT-009` / `P2-VM-004` | `debian/guardian-daemon.service`, daemon construction sites, provider route | A host shell may see `/proc`, `/sys`, netlink, or journal data that the hardened service cannot see | Apply the Protocol production-reachability preflight and prove the route from the real unit; host-shell evidence is non-acceptance evidence |
| Accepted daemon hardening | G7/G9 and accepted PSI gate; read-only here | `debian/guardian-daemon.service` | udev/netlink, cross-process procfs, diskstats, journal, or kernel logs may be blocked | The unit is an unchanged constraint. A required hardening-directive change is Contract Collision STOP/replan |
| Privilege topology | ADR-002; read-only here | daemon/helper boundary | A probe could silently move observation into the privileged helper or add a broker | No helper authority, generic execution, new privileged boundary, or silent fallback is authorized; such a need is STOP/replan |
| UDisks identity and provider authority | Accepted UDisks provider; read-only here | `crates/guardian-core/src/providers/udisks.rs` | A topology source could re-key devices using volatile `/dev` names or compete with UDisks | UDisks object/stable identity remains authoritative where its contract applies; observations may associate with it but do not redefine it |
| PSI access | Accepted PSI supplied-descriptor gate; regression only | PSI provider, packaged `OpenFile=` route | A generalized source collector could reopen or replace the accepted descriptor design | PSI is contextual evidence and its accepted route is reused unchanged |
| Access mechanisms | G-A acceptance criteria under `P2-EVT-009` | selected per source | Minting an ID per mechanism would duplicate normative ownership | Native API, kernel interface, supplied channel, D-Bus, or structured fallback remains subordinate acceptance detail; no new ID is created |

**Preflight verdict:** no conflict is resolved by changing production code or
the unit in this gate. A source that cannot satisfy the accepted topology is
recorded as partial, unavailable, unknown, or rejected. Any necessary
hardening/privilege change stops the gate for governance review.

## Production-reachability preflight

Apply the universal preflight in `GUARDIAN_EXECUTION_PROTOCOL.md`; this section
adds no new normative rule. Each matrix row must identify and prove:

1. the real upstream producer;
2. the production constructor or call site;
3. the packaged consumer and provider route;
4. the actual sandbox and privilege boundary;
5. production cadence, startup, restart, disappearance, and re-enumeration;
6. degraded, unavailable, partial, unknown, and gap behavior where applicable;
7. composition into the later evidence chain, rather than component-only
   availability.

## Mandatory source matrix

The decision must cover all rows below. The entries in the second and third
columns are hypotheses to test, not accepted conclusions.

| Source | Candidate route / repository precedent | Required challenge |
|---|---|---|
| UDisks topology | system D-Bus and accepted UDisks provider | provider availability, object identity, event lifecycle, and real daemon construction |
| udev/kernel topology events | native udev/netlink or provider reuse | `RestrictAddressFamilies=AF_UNIX`, `PrivateNetwork=yes`, namespace behavior, loss/gap behavior |
| process/open handles | `/proc` or a stronger supported source | `ProtectProc=invisible`, `ProcSubset=pid`, cross-UID access, PID lifetime, completeness boundary |
| aggregate disk statistics | `/sys/block/*/stat`, `/proc/diskstats`, or stronger reachable source | actual in-unit visibility, counter reset/wrap, device mapping, cadence |
| request latency/diagnostics | bounded diagnostic provider | normal-tier availability, diagnostic budget, privilege, aggregation versus per-request evidence |
| minimal kernel/journal I/O events | journal or other bounded kernel event route | `ProtectKernelLogs=yes`, journal permissions, cursor/provenance/gap semantics, Phase 5 non-duplication |
| PSI context | accepted inherited descriptors | unchanged descriptor acquisition, source availability, contextual rather than fabricated device/process attribution |

Aggregate counters must never be presented as request age, request latency, or
process ownership. A fallback is accepted only after the preferred route is
tested in the packaged topology and the fallback itself passes the same proof.

## RED proof to create during gate execution

No RED test or probe is created or run by this artifact-authoring pass. When
the gate is assigned, RED must first establish that the repository has no
governed production-topology decision covering every mandatory source and no
contract that can truthfully distinguish authoritative absence from incomplete
visibility for each target. The initial probe must fail until every source row
has a reproducible packaged-unit result and a completeness disposition.

RED must include negative fixtures proving that:

- an empty but incomplete handle scan cannot yield `unused`;
- a host-visible source cannot satisfy production reachability when the unit
  cannot obtain it;
- aggregate disk counters cannot claim request latency or process ownership;
- source loss or a cursor gap cannot become authoritative absence;
- an unavailable diagnostic tier cannot be reported as available; and
- a changed sandbox is rejected as evidence for the accepted topology.

## GREEN and acceptance

GREEN is the accepted access ADR and evidence matrix, not production provider
code. Every source has a selected, rejected, or explicitly unavailable route;
the availability/completeness contract is unambiguous; the production proof is
reproducible; and no accepted hardening, UDisks identity, PSI route, or
privilege boundary changed.

`P2-VM-004` requires disposable/reference-environment proof from the real
packaged production daemon/helper topology with the service sandbox active and
unchanged. Evidence records commands, versions, unit properties, producer
stimulus, consumer observation, cadence/lifecycle, and negative paths. Manual
`cargo run` and host-shell reachability cannot satisfy it.

## Scope and stop conditions

This gate may add only its named probe, ADR, and evidence files. It must not
implement R1 providers, change the service, add helper authority, mutate the
system, or edit existing contracts. Before later implementation, a reviewed
scope amendment must name the exact selected production files.

STOP if proof requires a sandbox relaxation, privileged observation not already
governed, a change to accepted UDisks identity or PSI ingress, a new public API,
or a duplicate completeness taxonomy.

## Evidence by normative ID

- `P2-EVT-009`: source matrix, negative completeness proof, and ADR-010.
- `P2-VM-004`: raw packaged-unit reference-environment transcript with the
  accepted sandbox unchanged.
