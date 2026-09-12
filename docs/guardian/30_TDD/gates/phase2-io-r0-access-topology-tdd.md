---
title: "Master-Spec Phase 2 R0 Access Topology and Completeness Gate TDD"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-12"
---
# Master-Spec Phase 2 R0 Access Topology and Completeness Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-io-r0-access-topology-manifest.toml`.

This discovery/proof gate covers planning work `T2-R0-A` and `T2-R0-B` and
owns the already-minted requirements `P2-EVT-011` and `P2-VM-006`. It selects
no source merely from source inspection and authorizes no production provider
implementation or unit-file change.

## Authority chain and namespace

1. The [gate manifest](phase2-io-r0-access-topology-manifest.toml) owns only
   `P2-EVT-011` and `P2-VM-006` for this gate.
2. [TDD contract §53](../GUARDIAN_PHASE_0_1_TDD_CONTRACT.md) supplies their
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
acquire the R0-GOV IDs. Contract §53 and
[handoff §19](../GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md) are the ownership
fence.

## Normative requirements (contract §53, verbatim)

**P2-EVT-011:** Before production I/O evidence provider implementation, every mandatory G-A source class MUST have a governed route decision of Selected, Rejected, or ExplicitlyUnavailable based on its real upstream source exercised in a disposable/reference environment under a proven production-equivalent reproduction of all source-relevant access constraints of the accepted Guardian execution topology. Availability, evidence completeness, and route decision MUST remain distinct. Each decision MUST define applicable cadence, lifecycle, disappearance/re-enumeration, loss/gap, privilege, sandbox, and diagnostic-budget semantics. Missing production wiring alone MUST NOT establish Rejected or ExplicitlyUnavailable. Any route requiring relaxation of the accepted sandbox or introduction of a new privilege topology MUST trigger Contract Collision STOP/replan.

**P2-VM-006:** Real disposable/reference-environment evidence MUST exercise every mandatory G-A candidate route against its real upstream producer from a probe whose access profile is proven equivalent for every source-relevant restriction to the accepted packaged Guardian execution boundary applicable to that candidate. The proof MUST record equivalence as applicable for user/group, capabilities, NoNewPrivileges, address-family/network restrictions, device policy, process visibility, kernel-log visibility, control-group protections, filesystem visibility, namespaces, and every other source-relevant restriction. It MUST cover applicable cadence feasibility, startup/restart, disappearance/re-enumeration, and loss/gap behavior and distinguish selected, rejected, explicitly unavailable, partial, and unknown results. Host-shell-only evidence is insufficient. This candidate proof MUST NOT be represented as proof of a production constructor, packaged consumer, or production composition. Those remain required by P2-EVT-009 and P2-VM-004 under R1.

## Required outcome

Produce an accepted, source-by-source access decision before production
provider implementation. Each candidate is exercised against its real upstream
producer under a proven production-equivalent reproduction of every source-
relevant access constraint. For every selected source the decision defines the
evidence that can be observed and exactly when a missing observation means:

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
| Candidate I/O evidence availability and completeness | G-A, `P2-EVT-011` | Candidate contracts; existing `guardian-provider-api::Availability` / `Knowledge` are read-only precedent | A new I/O-only taxonomy could contradict accepted provider semantics | Reuse the accepted types where sufficient; document any missing dimension and STOP for an exact scoped extension rather than create overlapping states in this gate |
| Production-equivalent candidate reachability | G-A, `P2-VM-006` | Disposable/reference probe plus read-only packaged unit | Host-shell visibility or an incomplete sandbox replica could be mistaken for acceptance | Exercise the real upstream producer and prove equivalence for every source-relevant access constraint; never claim a production constructor, consumer, or composition |
| Actual packaged production reachability | R1, `P2-EVT-009` / `P2-VM-004` | Future exact production scope, daemon construction sites, provider route | G-A candidate evidence could be silently promoted to production proof | R1 owns an independent RED and actual packaged proof after implementation; G-A evidence cannot satisfy it |
| Accepted daemon hardening | G7/G9 and accepted PSI gate; read-only here | `debian/guardian-daemon.service` | udev/netlink, cross-process procfs, diskstats, journal, or kernel logs may be blocked | The unit is an unchanged constraint. A required hardening-directive change is Contract Collision STOP/replan |
| Privilege topology | ADR-002; read-only here | daemon/helper boundary | A probe could silently move observation into the privileged helper or add a broker | No helper authority, generic execution, new privileged boundary, or silent fallback is authorized; such a need is STOP/replan |
| UDisks identity and provider authority | Accepted UDisks provider; read-only here | `crates/guardian-core/src/providers/udisks.rs` | A topology source could re-key devices using volatile `/dev` names or compete with UDisks | UDisks object/stable identity remains authoritative where its contract applies; observations may associate with it but do not redefine it |
| PSI access | Accepted PSI supplied-descriptor gate; regression only | PSI provider, packaged `OpenFile=` route | A generalized source collector could reopen or replace the accepted descriptor design | PSI is contextual evidence and its accepted route is reused unchanged |
| Access mechanisms | G-A acceptance criteria under `P2-EVT-011` | selected per source | Minting an ID per mechanism would duplicate normative ownership | Native API, kernel interface, supplied channel, D-Bus, or structured fallback remains subordinate acceptance detail; no new ID is created |

**Preflight verdict:** no conflict is resolved by changing production code or
the unit in this gate. Availability and completeness record partial,
unavailable, or unknown visibility as applicable; the separate route decision
records Selected, Rejected, or ExplicitlyUnavailable. Any necessary hardening/
privilege change stops the gate for governance review.

## Candidate production-equivalence preflight

G-A does not claim the universal production-reachability proof that R1 must
later apply under `P2-EVT-009` and `P2-VM-004`. Under `P2-VM-006`, each matrix
row must identify and prove:

1. the real upstream producer;
2. the candidate mechanism and probe consumer;
3. equivalence for every source-relevant restriction of the intended packaged
   execution boundary;
4. candidate cadence feasibility, startup, restart, disappearance, and re-
   enumeration where applicable;
5. degraded, unavailable, partial, unknown, and gap behavior where applicable;
6. privilege and diagnostic-budget implications; and
7. the explicit boundary between this candidate proof and R1 production
   construction, consumption, cadence, and composition.

The equivalence proof covers, as applicable, daemon user/group, capabilities,
`NoNewPrivileges`, address-family and network restrictions, device policy,
process visibility, kernel-log visibility, control-group protections,
filesystem visibility, namespaces, and any other source-relevant restriction.
It does not authorize helper observation, new helper authority, a new privileged
boundary, or relaxed hardening.

## Mandatory source matrix

The decision must cover all rows below. The entries in the second and third
columns are hypotheses to test, not accepted conclusions.

| Source | Candidate route / repository precedent | Required challenge |
|---|---|---|
| UDisks topology | system D-Bus and accepted UDisks provider | provider availability, object identity, event lifecycle, and a new governed G-A decision distinct from historical construction proof |
| udev/kernel topology events | native udev/netlink or provider reuse | `RestrictAddressFamilies=AF_UNIX`, `PrivateNetwork=yes`, namespace behavior, loss/gap behavior |
| process/open handles | `/proc` or a stronger supported source | `ProtectProc=invisible`, `ProcSubset=pid`, cross-UID access, PID lifetime, completeness boundary |
| aggregate disk statistics | `/sys/block/*/stat`, `/proc/diskstats`, or stronger reachable source | production-equivalent visibility, counter reset/wrap, device mapping, cadence feasibility |
| request latency/diagnostics | bounded diagnostic provider | normal-tier availability, diagnostic budget, privilege, aggregation versus per-request evidence |
| minimal kernel/journal I/O events | journal or other bounded kernel event route | `ProtectKernelLogs=yes`, journal permissions, cursor/provenance/gap semantics, Phase 5 non-duplication |
| PSI context | accepted inherited descriptors | unchanged descriptor acquisition, source availability, contextual rather than fabricated device/process attribution |

Aggregate counters must never be presented as request age, request latency, or
process ownership. A fallback is accepted only after the preferred route is
tested against the exact production-equivalent access profile and the fallback
itself passes the same proof.

## RED proof lifecycle

The pre-repair G-A RED probe exists and is frozen at SHA-256
`d224a52544dde336604abbdcb246fcc9eed9a46762777bff37b6ab8dab345952`.
It correctly exposed the G-A/R1 ownership collision and remains governance
provenance. Its selected-route predicate encoded the former `P2-EVT-009` /
`P2-VM-004` production-constructor and packaged-consumer burden, so it must be
revised—not recreated from scratch—under G-A's newly owned `P2-EVT-011` and
`P2-VM-006` before GREEN.

The revised RED must fail because governed candidate-access decisions,
completeness dispositions, and production-equivalent candidate evidence do not
yet exist. It must not fail merely because R1 production constructors or
consumers have not been implemented. Missing production wiring cannot establish
rejection or unavailability.

RED must include negative fixtures proving that:

- an empty but incomplete handle scan cannot yield `unused`;
- host visibility cannot satisfy `P2-VM-006` candidate production-equivalence/
  reachability when the equivalent probe cannot obtain the source;
- aggregate disk counters cannot claim request latency or process ownership;
- source loss or a cursor gap cannot become authoritative absence;
- an unavailable diagnostic tier cannot be reported as available; and
- a changed sandbox is rejected as evidence for the accepted topology.

## GREEN and acceptance

GREEN is the accepted access ADR and evidence matrix, not production provider
code. Every source has a selected, rejected, or explicitly unavailable route;
the availability/completeness contract is unambiguous; the real-upstream,
production-equivalent candidate proof is reproducible; and no accepted
hardening, UDisks identity, PSI route, or privilege boundary changed.

`P2-VM-006` requires disposable/reference-environment evidence against each
real producer from a probe whose access profile is proven equivalent for every
source-relevant restriction. Evidence records commands, versions, packaged
unit properties, probe-profile equivalence, producer stimulus, candidate
observation, cadence feasibility/lifecycle, and negative paths. Host-shell
reachability cannot satisfy it. This is not `P2-VM-004` production proof.

## Scope and stop conditions

This gate may add only its named probe, ADR, and evidence files. It must not
implement R1 providers, persistently change host/system configuration, change a
service unit, relax the sandbox, add helper authority, introduce a privileged
boundary, or edit existing contracts. Before later implementation, a reviewed
scope amendment must name the exact selected production files.

Controlled producer and lifecycle stimuli inside the disposable/reference
environment are allowed when required by `P2-VM-006`, including applicable
start/restart, process open/close, source disappearance/reappearance, device
re-enumeration, and gap/loss trajectories. Such stimuli must not alter the
accepted access or privilege topology being tested.

STOP if proof requires a sandbox relaxation, privileged observation not already
governed, a change to accepted UDisks identity or PSI ingress, a new public API,
or a duplicate completeness taxonomy.

## Evidence by normative ID

- `P2-EVT-011`: source matrix, negative completeness proof, and ADR-010.
- `P2-VM-006`: raw real-producer transcript plus proof that the probe reproduces
  every source-relevant access constraint of the intended packaged execution
  boundary.

The collision-discovering RED probe at SHA-256
`d224a52544dde336604abbdcb246fcc9eed9a46762777bff37b6ab8dab345952`
is preserved as governance provenance. After this ownership repair is accepted,
the probe must be revised in its own G-A execution pass; it is not modified by
this governance pass.
