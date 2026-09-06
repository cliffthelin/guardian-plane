---
title: "Gate 2c TDD — VM evidence and restart/epoch proof"
kind: "gate-tdd"
status: "active"
last_reviewed: "2026-09-06"
---
# Gate 2c TDD

Governing manifest: `docs/guardian/30_TDD/gates/phase2-2c-manifest.toml`.
Full context: `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §9, §20, §21.

This gate produces evidence, not new production code, beyond whatever
minimal VM-only test harness the run itself needs. It proves on real
hardware/systemd what Gates 2a/2b only proved in isolation or against a
mock bus.

## Requirements

**R1 — Real unit stop/start produces a real `Incidents1` transition
(`P2-VM-001`).**
Requirement: on the disposable Ubuntu 26.04.1 VM, a real stop/start of an
already-evidenced unit (e.g. `cups.service`) produces a real, observable
`Incidents1` transition over the real system bus — not a mock, not
asserted from source reading alone.
Evidence: a captured transcript/log under `docs/evidence/p2/` showing the
unit transition and the corresponding `Incidents1.ListIncidents()`
change, reproducible from a fresh VM clone at this gate's `baseline_sha`.

**R2 — Daemon restart during an open incident loses it and resets the
ingress epoch (`P2-VM-002`).**
Requirement: a real `guardian-daemon` restart while an incident is open
loses that incident (per §9's accepted no-persistence semantics) and
begins a fresh `CorrelationIngress` epoch at sequence 0 — confirmed on
real hardware, not asserted from source reading alone.
Evidence: a captured transcript/log under `docs/evidence/p2/` showing an
open incident before restart, the restart itself, and its absence
(alongside a fresh ingress epoch) after restart.

## Out of scope for this gate

Any new correlation/daemon-wiring logic (Gates 2a/2b, both closed by the
time this gate runs); any change to `Incidents1`/`IncidentWire`; any
non-VM (Layer 1/2/3) test work.
