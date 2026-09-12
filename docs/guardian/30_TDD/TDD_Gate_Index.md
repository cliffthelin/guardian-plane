---
title: "TDD Gate Index"
kind: "tdd-index"
status: "active"
last_reviewed: "2026-09-11"
tags:
  - tdd
  - gates
---
# TDD Gate Index

This page is a navigation layer over the governing [Phase 0/1 TDD Contract](GUARDIAN_PHASE_0_1_TDD_CONTRACT.md).

| Gate | Purpose | Primary wiki pointers | Handoffs |
|---|---|---|---|
| G0 | Public contracts — **PASS, tagged `phase0-g0-public-contracts`** | [D-Bus API](../20_Control_Plane/D-Bus_API_Contract.md), [Source drift](../20_Control_Plane/Source_Contract_Drift.md) | [Implementation](GUARDIAN_PHASE_0_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G0_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g0/G0_MILESTONE.md) |
| G1 | Identity & authorization — **PASS, tagged `phase0-g1-identity-authorization`** | [Privilege & Authorization](../20_Control_Plane/Privilege_and_Authorization.md), [polkit](../10_Platform/Polkit.md) | [Implementation](GUARDIAN_G1_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G1_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g1/G1_MILESTONE.md) |
| G2 | Privilege topology — **PASS, tagged `phase0-g2-privilege-topology`** | [Privilege & Authorization](../20_Control_Plane/Privilege_and_Authorization.md), [systemd](../10_Platform/systemd.md) | [Implementation](GUARDIAN_G2_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G2_INDEPENDENT_REVIEW_HANDOFF.md), [ADR-002](../../adr/ADR-002-guardian-privilege-topology.md), [Milestone](../../evidence/g2/G2_MILESTONE.md) |
| G3 | Core data models — **PASS, tagged `phase0-g3-core-data-models`** | [Capability Registry](../20_Control_Plane/Capability_Registry.md), [Provider Arbitrator](../20_Control_Plane/Provider_Arbitrator.md), [Events](../20_Control_Plane/Event_and_Incident_Model.md) | [Implementation](GUARDIAN_G3_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G3_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g3/G3_MILESTONE.md) |
| G4 | Transaction engine — **PASS, tagged `phase0-g4-transaction-engine`** | [Transaction Engine](../20_Control_Plane/Transaction_Engine.md), [NetworkManager](../10_Platform/NetworkManager.md) | [Implementation](GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G4_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g4/G4_MILESTONE.md) |
| G5 | Diagnostic safety — **PASS, tagged `phase0-g5-diagnostic-safety`** | [Diagnostic Budget](../20_Control_Plane/Diagnostic_Budget.md), [PSI](../10_Platform/PSI.md), [Flight Recorder](../20_Control_Plane/Flight_Recorder.md) | [Implementation](GUARDIAN_G5_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G5_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g5/G5_MILESTONE.md) |
| G6 | Indicator decision — **PASS, tagged `phase0-g6-indicator-decision`** | [StatusNotifierItem](../10_Platform/StatusNotifierItem.md), [Client Surfaces](../20_Control_Plane/Client_Surfaces.md) | [Implementation](GUARDIAN_G6_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G6_INDEPENDENT_REVIEW_HANDOFF.md), [ADR-006](../../adr/ADR-006-guardian-indicator-mechanism.md), [Milestone](../../evidence/g6/G6_MILESTONE.md) |
| G7 | Production daemon — **PASS, tagged `phase0-g7-production-daemon`** | [Architecture](../20_Control_Plane/Architecture.md), [Privilege](../20_Control_Plane/Privilege_and_Authorization.md) | [Implementation](GUARDIAN_G7_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G7_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g7/G7_MILESTONE.md) |
| G8 | Initial providers — **PASS, tagged `phase0-g8-initial-providers`** | [Platform Index](../10_Platform/INDEX.md) | [Implementation](GUARDIAN_G8_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G8_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g8/G8_MILESTONE.md) |
| G9 | Clients & packaging — **PASS, tagged `phase0-g9-clients-packaging`** | [Client Surfaces](../20_Control_Plane/Client_Surfaces.md), [ADR-006](../../adr/ADR-006-guardian-indicator-mechanism.md), [ADR-007](../../adr/ADR-007-guardian-gui-tui-client-separation.md), [ADR-008](../../adr/ADR-008-guardian-package-filesystem-layout.md) | [Implementation](GUARDIAN_G9_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G9_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/g9/G9_MILESTONE.md) |

## Wave 1 — separately governed interstitial stage (not a numbered gate, not TDD-contract Phase 2)

Wave 1 was introduced by §50 of the TDD contract after G0–G9 closed, as a
distinct, unnumbered interstitial stage — it does not consume the `G10`
slot and is not TDD-contract Phase 2 (§47's own read-only observability/
correlation expansion, which remains separate, distinct, and not started
by Wave 1). See §50's own disambiguation rule for the full governing
terminology.

| Stage | Purpose | Governing amendment | Handoffs |
|---|---|---|---|
| Wave 1 | First production mutation (systemd `cups-restart`) — **PASS, tagged `wave1-first-production-mutation`** | [TDD contract §50](GUARDIAN_PHASE_0_1_TDD_CONTRACT.md) | [Implementation](GUARDIAN_WAVE1_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_WAVE1_INDEPENDENT_REVIEW_HANDOFF.md), [Milestone](../../evidence/wave1/WAVE1_MILESTONE.md) |

## Master-Spec Phase 2–6 execution planning (not numbered gates)

Separate from everything above. The rows below are **Phase Execution Specs
and Phase TDDs**: mutable planning and proof-route documents governed by
[Guardian Phase Spec Doctrine](GUARDIAN_PHASE_SPEC_DOCTRINE.md) and
subordinate to the [Guardian Master Spec](../00_Project/GUARDIAN_MASTER_SPEC.md),
accepted contracts, gate manifests/TDDs and ADRs. They do not become
normative merely by living under `30_TDD/`.

They are **not** numbered legacy gates, they do not occupy a `G<n>` slot,
and their presence here is **not** evidence that a Master-Spec phase has
passed or closed. Universal per-task procedure — including the mandatory
Production-reachability preflight the phase documents apply but do not
own — lives in
[Guardian Execution Protocol](GUARDIAN_EXECUTION_PROTOCOL.md).

Master-Spec phase numbering is independent of the historical
implementation milestone **Phase 2 — Observability & Correlation** (tag
`phase2-observability-correlation`). In particular, Master-Spec Phase 2 is
I/O Guardian and Master-Spec Phase 3 is Observability; neither is that
milestone, and the milestone is not completion of either.

| Master-Spec phase | Execution Spec | Phase TDD |
|---|---|---|
| Phase 2 — I/O Guardian | [Execution Spec](phases/ms-phase2-io-guardian-execution-spec.md) | [Phase TDD](phases/ms-phase2-io-guardian-tdd.md) |
| Phase 3 — Observability | [Execution Spec](phases/ms-phase3-observability-execution-spec.md) | [Phase TDD](phases/ms-phase3-observability-tdd.md) |
| Phase 4 — Thermal & Power | [Execution Spec](phases/ms-phase4-thermal-power-execution-spec.md) | [Phase TDD](phases/ms-phase4-thermal-power-tdd.md) |
| Phase 5 — Logs & Incidents | [Execution Spec](phases/ms-phase5-logs-incidents-execution-spec.md) | [Phase TDD](phases/ms-phase5-logs-incidents-tdd.md) |
| Phase 6 — System Management | [Execution Spec](phases/ms-phase6-system-management-execution-spec.md) | [Phase TDD](phases/ms-phase6-system-management-tdd.md) |

Each phase document carries its own gate plan, decision history, exit
criteria and proof obligations. Read them there; they are deliberately not
duplicated into this index.

## Rule

Do not duplicate detailed acceptance criteria here. The TDD contract is authoritative; this page exists so a coding agent can navigate from a gate to the relevant concepts and external provider sources.

The same rule governs the Master-Spec phase table above: it is navigation
only. Phase gate plans, decision histories and acceptance obligations are
authoritative in the linked Phase Execution Spec and Phase TDD, never here.
