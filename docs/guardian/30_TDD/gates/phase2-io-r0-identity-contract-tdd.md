---
title: "Master-Spec Phase 2 R0 I/O Evidence and Mutation-Grade Identity Gate TDD"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-11"
---
# Master-Spec Phase 2 R0 I/O Evidence and Mutation-Grade Identity Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-io-r0-identity-contract-manifest.toml`.

This gate covers planning work `T2-R0-C` and `T2-R0-D` and owns only the
already-minted `P2-EVT-010`. It begins only after
`phase2-io-r0-access-topology` is accepted, because identity and completeness
cannot claim guarantees stronger than the selected sources actually provide.

## Authority chain and namespace

1. The [gate manifest](phase2-io-r0-identity-contract-manifest.toml) owns only
   `P2-EVT-010` for this gate.
2. [TDD contract §52](../GUARDIAN_PHASE_0_1_TDD_CONTRACT.md) supplies its
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

## Normative requirement (contract §52, verbatim)

**P2-EVT-010:** I/O evidence identity MUST remain stable and reuse-safe across the relevant physical-device/block/partition/filesystem/mount lifecycle, process lifetime, cgroup lifetime, systemd manager/unit/scope/invocation, current block-device mapping, and observation source/cursor/revision lifecycle. Stale, reused, or changed identity MUST be detectable and MUST NOT silently refer to a different resource. Mutation-grade I/O target criteria from `T2-R0-D` MUST bind as explicit acceptance criteria under this requirement together with the existing fail-closed and transaction-precondition authority (GP-05, GP-06, and §14.2 of `GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`); they receive no standalone R0 normative ID.

## Required concepts

The contract must keep two types and two decisions visibly separate:

1. **Observation identity** answers what source observed which resource, at
   which source/cursor/revision and lifecycle point.
2. **Mutation-grade target identity** answers whether a future write target is
   still the same validated resource, has complete affected-workload identity,
   and satisfies all required ownership and controller preconditions.

Observation identity can be useful while incomplete. Mutation-grade identity
cannot be inferred from mere observation, a PID, `/dev/sdX`, a cgroup pathname,
or a systemd unit name.

`T2-R0-D` receives no standalone ID. Its acceptance criteria bind under
`P2-EVT-010`, GP-05, GP-06, and contract §14.2. This gate defines and tests the
contract but performs no system mutation.

## Contract Collision Table (mandatory preflight — completed)

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Stable I/O observation identity | G-B, `P2-EVT-010` | new `guardian-core::io_identity`; new provider-api evidence/completeness model | Volatile names could be promoted to identity or source completeness could be overstated | Identity carries lifecycle-qualified stable components and source provenance; unknown/incomplete remains explicit and cannot satisfy stronger predicates |
| UDisks device authority | Accepted UDisks provider; regression only | `crates/guardian-core/src/providers/udisks.rs` | A new model could replace UDisks stable/object identity with `/dev/sdX` | The UDisks provider is read-only; the new contract associates with its accepted identity rather than redefining or re-keying it |
| Accepted correlation identity | Closed Phase 2 correlation gates; regression only | `correlation.rs`, `incident.rs` | New I/O identity could reopen `resource_refs.first()` keying or public Incident shape | No correlation-key, `IncidentWire`, Incident, or D-Bus change is authorized; any required change is STOP/replan |
| Mutation-grade target validity | Future R2A, specialized here by `P2-EVT-010` + GP-05/GP-06/§14.2 | new identity contract/tests only | Defining target validity could drift into apply, helper, transaction, or provider authorization work | This gate exposes falsifiable validation only. All writes and mutation paths remain future R2A scope |
| Completeness authority | G-A decision consumed by G-B | new provider-api evidence model | G-B could invent completeness independent of G-A reachability proof | G-A's accepted source contract is a hard dependency; G-B may encode but not strengthen its claims |

**Preflight verdict:** the model can be added in the exact new modules without
changing accepted UDisks, correlation, Incident, transaction, helper, or public
API ownership. If implementation proves otherwise, STOP and request an exact
scope/governance amendment.

## RED proof to create during gate execution

No RED tests are created or run during gate-artifact preparation. Once assigned,
write the contract tests before implementation. They must fail because the
repository does not yet expose the required lifecycle-qualified I/O identities,
completeness proofs, or mutation-grade validation result.

The RED suite must make each failure case below falsifiable. A test passes only
when changed or incomplete identity yields a typed fail-closed result, never a
new resource silently accepted under an old reference.

## Observation-identity cases

- `/dev/sdX` reuse cannot preserve physical/block identity by name alone.
- USB/device re-enumeration changes the relevant lifecycle identity even when a
  display name is reused.
- partition, filesystem, or mount creation/removal/remount changes the
  applicable layer identity.
- PID reuse cannot preserve process identity; process lifetime must be bound to
  stronger birth/lifecycle evidence.
- cgroup pathname reuse cannot preserve cgroup identity; lifetime and membership
  revision remain explicit.
- systemd manager identity, unit/scope identity, and invocation identity are
  distinct; a manager, unit, scope, or invocation change invalidates an
  identity-qualified observation.
- current block-device mapping and mount association are revision-sensitive.
- observation source identity, cursor, revision, restart, loss, and gap are
  represented; discontinuity cannot become a continuous authoritative view.

Direct observations and inferred associations remain distinguishable. Partial
or unknown source coverage is retained and cannot be upgraded by a join.

## Mutation-grade acceptance cases

Each of the following must deny target validity before future authorization or
apply:

- the cgroup contains multiple workloads and the complete affected set is not
  explicitly accepted;
- the target is Guardian itself or another protected system target;
- the I/O controller is unavailable;
- an ancestor disables or constrains the required controller/property;
- the target moves after snapshot or validation;
- a systemd unit is reinvoked after validation;
- controlled block mapping or mount identity changes after validation;
- cgroup membership or lifetime changes after validation;
- provider ownership/arbitration revision changes after validation; or
- any required affected-workload identity is incomplete or unknown.

The valid fixture must contain, at minimum, boot/process lifetime identity,
systemd manager + unit/scope + current invocation, cgroup lifetime + membership
revision, current controlled block identity/mapping, controller availability and
ancestor constraints, complete affected workloads, protected-target exclusion,
and ownership/arbitration revision. Validation is repeatable immediately before
a future apply; this gate does not implement that apply.

## GREEN and invariants

GREEN requires typed models and deterministic tests for every case above. It
also proves:

- equality and freshness do not depend on display-only volatile names;
- stale and incomplete states are explicit, machine-readable, and fail closed;
- observation identity never implicitly certifies mutation safety;
- mutation-grade validation cannot be constructed when a required component is
  unknown;
- accepted UDisks identity remains unchanged;
- accepted correlation keying and the seven-field `IncidentWire` remain
  unchanged; and
- no public D-Bus, helper, transaction, authorization, or production provider
  path was introduced.

## Production reachability and later consumers

This contract consumes G-A's accepted real-source topology. The universal
Production-reachability preflight remains Protocol-owned. Unit tests use
controlled lifecycle/reuse fixtures, but later R1/R2A gates must prove their
real production constructors and current-source validation; direct construction
of a post-transition model cannot prove production composition.

The contract is deliberately strong enough that future R2A cannot reinterpret
"same PID/path/unit/device name" as "same target" or weaken incomplete identity
to a warning.

## Scope and stop conditions

Only the manifest's new evidence/completeness module, new I/O identity module,
their exact export lines, contract tests, and evidence file may change. No
mutation, apply/observe/confirm/commit/rollback, public API, or service work is
authorized.

STOP if satisfying a test requires changing UDisks identity, accepted
correlation keying, Incident/IncidentWire, provider authorization semantics,
helper authority, or any transaction/mutation path.

## Evidence by normative ID

- `P2-EVT-010`: the two contract-test suites and
  `PHASE2_IO_R0_IDENTITY_CONTRACT_EVIDENCE.md`, with explicit mapping from every
  lifecycle and mutation-grade case to its test.
