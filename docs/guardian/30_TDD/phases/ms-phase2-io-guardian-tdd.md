---
title: "Master-Spec Phase 2 — I/O Guardian TDD"
kind: "phase-tdd"
status: "active"
last_reviewed: "2026-09-12"
tags:
  - planning
  - phases
  - tdd
  - io-guardian
---
# Guardian Plane — Master-Spec Phase 2: I/O Guardian TDD

**Status:** v3

Phase-level proof obligations for
[Master-Spec Phase 2 — I/O Guardian Execution Spec](ms-phase2-io-guardian-execution-spec.md).
Governed by [Guardian Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md);
subordinate to the [Guardian Master Spec](../../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. Section labels are planning
labels, not normative IDs. The universal production-reachability rule is owned by
[Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) and applied — not
redefined — here.

## T2-GOV — governance

Before new manifests, evidence the manifest-lifecycle decision and universal
production-reachability protocol rule.

## T2-R0-A — sandbox/access matrix

Run each candidate route against its real upstream producer from a
disposable/reference probe whose access profile is proven equivalent for every
source-relevant restriction to the accepted packaged Guardian execution
boundary. This is candidate/access proof before production implementation, not
proof of an actual constructor, packaged consumer, or production composition.

Required proof dimensions:

- address-family restrictions;
- PrivateNetwork;
- PrivateDevices;
- ProtectProc/ProcSubset;
- ProtectKernelLogs;
- ProtectControlGroups;
- system-bus availability;
- sysfs visibility;
- journal permission/cursor behavior.

For each source record:

```text
provider availability:
  Available / Degraded / Unavailable / Unsupported / Unknown

evidence completeness:
  Authoritative / Partial / Unknown

route decision:
  Selected / Rejected / ExplicitlyUnavailable

candidate mechanism
fallback/rejected alternatives
why accepted sandbox remains unchanged
```

This is a planning representation of the three separately governed dimensions,
not a new normative taxonomy.

A host-shell proof outside the unit is insufficient.

Missing production wiring alone is neither rejection nor unavailability. A
route requiring relaxed hardening, new helper authority, or another privileged
boundary stops for governance review. G-A owns `P2-EVT-011` and `P2-VM-006`.

## T2-R0-B — observation completeness

Required cases:

- process/open-handle coverage unavailable or partial;
- cross-user process inaccessible;
- journal gap/cursor discontinuity;
- disk-stat counter reset/re-enumeration;
- request-latency provider unavailable;
- diagnostic budget refuses escalation.

Absence of observed handles must never become "filesystem unused" unless the
provider contract proves authoritative completeness for the target.

## T2-R0-C — identity contract

Prove stale/reuse safety for:

- `/dev/sdX`;
- USB re-enumeration;
- PID reuse;
- cgroup path/lifetime;
- systemd invocation;
- block-device mapping;
- mount changes.

## T2-R0-D — mutation target

A throttle target is valid only when all required mutation-grade identity and
affected-workload constraints are known.

Test:

- cgroup contains multiple workloads;
- protected system/Guardian target;
- cgroup ancestor lacks/limits controller;
- target moves after snapshot;
- unit reinvoked;
- block mapping changes.

## T2-R0-E — recorder architecture/intake

G-C must be independently accepted before any R1 scope amendment, R1 RED, or
R1 implementation begins. This necessarily proves the selected recorder
lifecycle before R1 interfaces freeze.

Required failure tests:

- daemon fails/restarts;
- daemon D-Bus serving connection unavailable;
- recorder intake unavailable/backpressured;
- dropped event accounting;
- `/var` unavailable/read-only/full;
- spill worker blocks/fails;
- reboot/boot provenance;
- persistence/replay bounds;
- daemon restart with accepted fresh-ingress-epoch / in-memory-Incident-loss
  semantics;
- surviving recorder evidence across that restart, if the selected recorder
  architecture permits it.

If recorder evidence survives a daemon restart, tests must prove:

- boot/daemon/ingress-epoch provenance distinguishes pre- and post-restart
  records;
- persisted evidence is not replayed as an already-open live Incident;
- the accepted Incident-state reset is not silently changed by recorder replay;
- later correlation may reference historical recorder evidence only through an
  explicitly governed historical-evidence path.

If embedded architecture is selected, it must prove the required boot-onward and
failure-independence semantics. Otherwise the decision remains RED.

## T2-R1 — I/O correlation model

Extend the accepted model without regressing accepted PSI/provider-health rules.

R1 owns unchanged `P2-EVT-009` and `P2-VM-004` and depends on the access-
topology, identity-contract, and recorder-architecture R0 gates. Before R1
acceptance, create a distinct RED and then prove for every selected source:

- its real upstream producer;
- actual production constructor/call site and packaged consumer;
- actual provider topology and unchanged accepted production sandbox;
- real cadence and startup/restart lifecycle;
- disappearance/re-enumeration where applicable;
- degraded, loss, gap, partial, unavailable, and unknown behavior as applicable;
- production composition; and
- integration into shared ingress and I/O correlation.

The G-A candidate probe and host-shell evidence cannot satisfy this production
proof. Exact production-file scope must be added by a separately reviewed R1
manifest amendment after the G-A ADR/source matrix is accepted.

Cases:

- full aligned I/O evidence;
- missing optional evidence;
- contradictory evidence;
- temporal mismatch;
- re-enumeration;
- aggregate activity without per-request latency;
- per-request diagnostic unavailable;
- journal gap;
- multiple candidate processes/devices.

Every evidence-chain claim identifies direct observation versus inference.

## T2-R3 — recorder persistence

Prove:

- hard ring bound;
- hard persistence quota;
- deterministic retention;
- nonblocking producer;
- bounded spill worker;
- no monitored-removable dependency;
- transaction/incident evidence reaches intake;
- restart/replay semantics.

Recorder may not be used as transaction authority.

## T2-R2A-1 — actual caller/provider-policy authorization

For a provider-owned systemd mutation:

- helper resolves actual inbound caller;
- provider's real policy action is checked for that caller;
- authorization-relevant details are internally derived from validated target;
- raw action/details cannot be caller supplied;
- no Guardian-invented action silently replaces provider policy;
- provider's own check of root helper is not treated as end-user authorization.

## T2-R2A-2 — intervention ownership

Test:

- first intervention succeeds;
- overlapping compatible request;
- overlapping conflicting request;
- Phase 3-style later intervention exists during Phase 2 restore;
- external systemd property change after snapshot;
- daemon/helper crash after Apply;
- reboot;
- expiry while target unchanged;
- expiry after target revision changed.

Stale restore must fail closed rather than overwrite later authority.

## T2-R2A-3 — effective throttle

Prove through the actual provider/controller:

- controller enabled;
- device/controller combination supports requested property;
- ancestor constraints represented;
- property change is observed as effective;
- workload behavior measurably changes;
- buffered-I/O/writeback supported case included;
- unsupported target yields explicit Unsupported, not success.

A successful D-Bus property write alone is insufficient.

## T2-R2B — storage operations

### Unmount
- busy refusal;
- clean release then unmount;
- device/mount identity changes;
- conditional remount compensation only when safe.

### UDisks power-off
- provider preconditions;
- sibling impact;
- explicit initiation;
- expected disappearance;
- no fake automatic inverse;
- human recovery classification where appropriate.

### read-only remount
- successful transition;
- filesystem error state;
- no unconditional writable restoration.

Every operation has an authorization owner and recovery classification.

## T2-R4 — integrated acceptance

Required capability-positive trajectory:

```text
supported real I/O workload/device
→ real providers
→ I/O evidence chain
→ Incident
→ recorder intake
→ accepted R2A or R2B capability
→ real observed effect
→ conflict-safe confirm/recovery
→ bounded persistence
```

Also include at least one busy/refused path and one re-enumeration/disappearance
path.

Synthetic final Event/Incident injection is insufficient.

## T2-HANDOFF — Phase 3

Phase 3 receives:

- process/resource evidence;
- minimum mutation-grade process/cgroup/unit identity;
- shared cgroup provider/intervention owner;
- I/O association model;
- minimal journal cursor/provenance/gap contract;
- stable PSI/incidents/capability state;
- working recorder intake/persistence.
