---
title: "Master-Spec Phase 2 R1 Production I/O Evidence Chain Gate TDD"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-12"
---
# Master-Spec Phase 2 R1 Production I/O Evidence Chain Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-io-r1-evidence-chain-manifest.toml`.

This gate owns the unchanged production-strength requirements `P2-EVT-009` and
`P2-VM-004`. Contract §52 minted their text; contract §53 transferred their
ownership atomically from G-A to this gate after the accepted G-A RED exposed
the original proof-order collision.

This definition establishes ownership and future RED/proof expectations only.
It authorizes no production implementation. Before execution, a separately
reviewed manifest amendment must name the exact test, evidence, ADR, and
production files derived from the accepted G-A source matrix and refresh the
execution baseline.

## Authority chain and dependencies

1. The [gate manifest](phase2-io-r1-evidence-chain-manifest.toml) owns only
   `P2-EVT-009` and `P2-VM-004`.
2. [TDD contract §52](../GUARDIAN_PHASE_0_1_TDD_CONTRACT.md) supplies their
   unchanged normative text; §53 supplies the repaired ownership.
3. The Master-Spec Phase 2 — I/O Guardian
   [Execution Spec](../phases/ms-phase2-io-guardian-execution-spec.md) and
   [Phase TDD](../phases/ms-phase2-io-guardian-tdd.md) supply planning context.
4. The [Guardian Execution Protocol](../GUARDIAN_EXECUTION_PROTOCOL.md) owns
   universal production reachability.
5. The [Phase Spec Doctrine](../GUARDIAN_PHASE_SPEC_DOCTRINE.md) supplies the
   planning/authority boundary.

R1 depends on accepted completion of all three R0 gates:

- `phase2-io-r0-access-topology` — candidate route and completeness decision;
- `phase2-io-r0-identity-contract` — evidence and mutation-grade identity; and
- `phase2-io-r0-recorder-architecture` — lifecycle/intake decision before R1
  interfaces freeze.

No R1 RED, scope amendment, or implementation may treat a partial R0 result as
satisfying these dependencies.

## Normative requirements (contract §52, verbatim)

**P2-EVT-009:** Guardian's production I/O evidence sources MUST expose truthful evidence availability and completeness under the actual packaged production topology. An observation MUST be interpreted as authoritative absence only when the selected provider contract proves authoritative completeness for the target. Partial, unavailable, and unknown visibility MUST remain explicit and MUST fail safely; incomplete visibility MUST NOT be treated as proof that a filesystem is unused or as sufficient evidence for a safety decision that requires authoritative completeness.

**P2-VM-004:** Real disposable/reference-environment evidence MUST prove the selected G-A I/O evidence routes from the actual packaged production daemon/helper topology with the accepted daemon sandbox active and unchanged. Host-shell-only reachability is insufficient. The proof MUST exercise the real upstream producer, production consumer, production cadence, provider topology, sandbox, and lifecycle applicable to each source, including its partial, unavailable, or unknown visibility behavior.

## Required production outcome

R1 consumes every governed G-A decision. A Rejected or
ExplicitlyUnavailable decision remains explicit and is never silently replaced
with another route or inferred merely from absent implementation. For every
route selected by G-A, R1 implements and proves:

1. the real upstream producer;
2. the actual production constructor or call site;
3. the actual packaged consumer;
4. the provider topology and accepted privilege boundary;
5. the unchanged accepted production sandbox;
6. real production cadence;
7. startup and restart lifecycle;
8. disappearance and re-enumeration where applicable;
9. degraded, loss, gap, partial, unavailable, and unknown behavior as
   applicable;
10. production composition; and
11. integration into the existing shared ingress and I/O correlation chain.

Production behavior keeps provider availability, epistemic completeness, and
the G-A route decision distinct. Aggregate counters never become request
latency or process ownership. Empty or missing evidence becomes authoritative
absence only when the production provider is usable, completeness is
authoritative for the target and interval, continuity is valid, and the actual
packaged proof supports the inference.

## G-A/R1 proof boundary

G-A's `P2-EVT-011`/`P2-VM-006` evidence establishes candidate feasibility and
completeness semantics under a proven production-equivalent access profile. It
does not establish an R1 constructor, packaged consumer, cadence, composition,
or shared-ingress path. Historical UDisks or PSI evidence may be reused only
where it directly proves an unchanged R1 obligation; capability or component-
only evidence is insufficient.

Host-shell observation, a manual `cargo run`, direct construction of a post-
transition model, or the G-A probe cannot satisfy `P2-VM-004`.

## Contract Collision preflight

| Requirement | Owner | Potential conflict | Required resolution |
|---|---|---|---|
| Production availability and completeness | R1, `P2-EVT-009` | Implementation could collapse unavailable, partial, or unknown into absence | Reuse the G-A contract and Guardian `Availability`/`Knowledge`; fail safely |
| Actual packaged composition | R1, `P2-VM-004` | Candidate or component evidence could be promoted to production proof | Exercise the real producer-to-consumer-to-ingress path under the installed unit |
| Exact production scope | Future reviewed R1 amendment | Broad crate globs could pre-authorize an unknown design | Keep production scope forbidden until the G-A ADR names selected routes and exact files |
| Sandbox and privilege topology | Existing G7/G9/ADR-002 authority | A selected route might require relaxed hardening or new helper authority | STOP/replan; this gate cannot silently change either boundary |
| R0 ownership | G-A/G-B/G-C | R1 could repair or reinterpret an incomplete R0 decision while implementing | Treat all three R0 gates as dependencies; return contract defects to their owner |

## R1 RED requirement

After the exact-scope amendment is accepted, create a distinct R1 RED before
production implementation. It must fail because selected routes lack actual
production construction, consumption, cadence, lifecycle, or composition—not
because the G-A candidate probe is missing or because a source is merely
unimplemented.

The RED must include negative production trajectories for source loss, cursor
or revision gaps, restart, disappearance/re-enumeration, unavailable or unknown
providers, incomplete visibility, and attempts to infer filesystem-unused,
latency, or process ownership from insufficient evidence.

## Layer-4 production proof

`P2-VM-004` is satisfied only by reproducible disposable/reference-environment
evidence from the actual installed production daemon/helper topology. Record
environment and provider versions, installed unit properties, real producer
stimulus, actual consumer observation, cadence, lifecycle, degraded paths,
composition into shared ingress/correlation, and the final unchanged sandbox.

Candidate G-A evidence remains linked provenance but is never the R1 acceptance
result.

## Scope and STOP conditions

The manifest intentionally withholds every production path. Do not begin RED or
implementation until a separately reviewed amendment names exact files and a
current baseline. Do not use broad `crates/**`, `debian/**`, service, D-Bus, or
polkit scope to bypass that decision.

STOP if implementation requires relaxed daemon hardening, new or expanded
helper authority, another privileged boundary, changed UDisks identity,
changed PSI descriptor ingress, a new public D-Bus surface, or a competing
availability/completeness taxonomy.

## Evidence by normative ID

- `P2-EVT-009`: R1 RED/GREEN tests and the production source matrix proving
  truthful runtime availability, completeness, continuity, and inference.
- `P2-VM-004`: raw reference-environment proof for the complete actual
  producer-to-packaged-consumer-to-shared-ingress/correlation trajectory.
