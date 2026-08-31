---
title: "TDD Gate Index"
kind: "tdd-index"
status: "active"
last_reviewed: "2026-08-30"
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
| G2 | Privilege topology — **implementation complete, independent audit PASS WITH NON-BLOCKING FINDINGS, milestone tag pending** | [Privilege & Authorization](../20_Control_Plane/Privilege_and_Authorization.md), [systemd](../10_Platform/systemd.md) | [Implementation](GUARDIAN_G2_IMPLEMENTATION_HANDOFF.md), [Independent review](GUARDIAN_G2_INDEPENDENT_REVIEW_HANDOFF.md), [ADR-002](../../adr/ADR-002-guardian-privilege-topology.md) |
| G3 | Core data models | [Capability Registry](../20_Control_Plane/Capability_Registry.md), [Provider Arbitrator](../20_Control_Plane/Provider_Arbitrator.md), [Events](../20_Control_Plane/Event_and_Incident_Model.md) | |
| G4 | Transaction engine | [Transaction Engine](../20_Control_Plane/Transaction_Engine.md), [NetworkManager](../10_Platform/NetworkManager.md) | |
| G5 | Diagnostic safety | [Diagnostic Budget](../20_Control_Plane/Diagnostic_Budget.md), [PSI](../10_Platform/PSI.md), [Flight Recorder](../20_Control_Plane/Flight_Recorder.md) | |
| G6 | Indicator decision | [StatusNotifierItem](../10_Platform/StatusNotifierItem.md), [Client Surfaces](../20_Control_Plane/Client_Surfaces.md) | |
| G7 | Production daemon | [Architecture](../20_Control_Plane/Architecture.md), [Privilege](../20_Control_Plane/Privilege_and_Authorization.md) | |
| G8 | Initial providers | [Platform Index](../10_Platform/INDEX.md) | |
| G9 | Clients & packaging | [Client Surfaces](../20_Control_Plane/Client_Surfaces.md) | |

## Rule

Do not duplicate detailed acceptance criteria here. The TDD contract is authoritative; this page exists so a coding agent can navigate from a gate to the relevant concepts and external provider sources.
