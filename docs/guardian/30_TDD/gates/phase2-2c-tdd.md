---
title: "Gate 2c TDD — VM evidence and restart/epoch proof"
kind: "gate-tdd"
status: "active"
last_reviewed: "2026-09-07"
---
# Gate 2c TDD

Governing manifest: `docs/guardian/30_TDD/gates/phase2-2c-manifest.toml`.
Full context: `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §9, §20, §21.

This gate produces evidence, not new production code, beyond whatever
minimal VM-only test harness the run itself needs. It proves on real
hardware/systemd what Gates 2a/2b only proved in isolation or against a
mock bus.

## Requirements

**R1 — Real provider transition produces a real `Incidents1` transition
(`P2-VM-001`).**
Requirement: on the disposable Ubuntu 26.04.1 VM, a real, safe, reversible
VM action causes an actually registry-observed capability/provider (per
the Gate 2b health-diff producer, `guardian-core/src/providers/
health.rs`, which diffs successive `capability_registry_tick` snapshots)
to transition `Availability`/`Health`, producing a real, observable
`Incidents1` transition over the real system bus — not a mock, not
asserted from source reading alone.

This corrects two independent errors in the prior wording. First, an
arbitrary unit such as `cups.service` is never read by the registry at
all, so stopping/starting it cannot produce any transition. Second, the
originally-intended `systemd-logind.service` does not work either:
`systemd_capabilities()` (`guardian-core/src/providers/registry.rs`) only
asks whether `org.freedesktop.systemd1`'s `LoadUnit` + property read for
`systemd-logind.service` succeeds; `read_state()` maps any `Ok(_)` to
`Available`/`Healthy` regardless of that unit's own `ActiveState`, so an
ordinary stop/start of `systemd-logind.service` produces no observable
Availability/Health change through this code path (confirmed by reading
the accepted Gate 2b code directly, matching Gate 2b's own accepted test
`build_engine_with_one_real_open_incident` in
`crates/guardian-daemon/tests/phase2_2b_contract.rs`, which synthesizes —
never actually performs — "the shape `capability_registry_tick` would
observe if the real `systemd-logind.service` unit actually went
unavailable").

Confirmed candidate, validated live on `guardian-g9` during this preflight
correction: `sudo systemctl mask --now upower.service` causes
`guardian-core/src/providers/upower.rs`'s `display_device()` read to fail
with a real `org.freedesktop.systemd1.UnitMasked` D-Bus error (confirmed
via `busctl`/`dbus-send`), which `registry.rs`'s `read_state()` maps to a
real `Available`/`Healthy` -> `Degraded`/`Error` transition of the
`upower.display-device` capability; `sudo systemctl unmask upower.service
&& sudo systemctl start upower.service` reverses it back to
`Available`/`Healthy` (also confirmed live). The Gate 2c implementer may
use this candidate directly, or — per `AGENTS.md`'s required lookup
workflow — confirm an equivalent safe/reversible candidate against
another of the six registry providers (systemd, PSI, logind, UDisks2,
UPower, AccountsService).

Evidence: a captured transcript/log under `docs/evidence/p2/` showing the
real provider transition and the corresponding
`Incidents1.ListIncidents()` change, reproducible from a fresh VM clone
at this gate's `baseline_sha`.

**R2 — Daemon restart during an open incident loses it; fresh ingress
epoch is a process-level fact, not an externally re-derived value
(`P2-VM-002`).**
Requirement: a real `guardian-daemon` restart while an incident is open
loses that incident (per §9's accepted no-persistence semantics) —
confirmed on real hardware, not asserted from source reading alone.

This corrects the prior wording's additional claim that the VM run must
observe "a fresh `CorrelationIngress` epoch at sequence 0" as an external
fact. `IncidentWire` (`crates/guardian-daemon/src/dbus_surface.rs`,
`crates/guardian-client/src/lib.rs`) is confirmed to be exactly a 7-field
`String` tuple with no ingress-sequence field on any public surface, and
no production log line in `guardian-daemon` emits `ingress_sequence`
today — there is no channel through which a VM-level Layer 4 test could
externally observe the raw sequence value without changing production
code or the public D-Bus shape, both forbidden for this gate. Claiming
such an observation would not be a real VM proof.

The fresh-epoch *mechanism* itself does not need re-proving here: it is
already proven deterministically at Gate 2a
(`crates/guardian-core/tests/correlation_contract.rs`,
`p2_evt_004_fresh_ingress_clock_resets_epoch`), and `guardian-daemon`'s
`main()` is confirmed to construct a fresh `IngressClock::new()` /
`CorrelationEngine::new()` pair at process start
(`crates/guardian-daemon/src/bin/guardian-daemon.rs`), never reusing or
persisting a prior instance. Gate 2c's job is only to prove the
process-level fact that a genuinely fresh `guardian-daemon` process
starts with no cross-restart incident state — the epoch-reset
consequence follows from the already-accepted Gate 2a proof plus this
fresh-process fact, not from a new external observation channel.

Evidence: a captured transcript/log under `docs/evidence/p2/` showing an
open incident before restart, the real process restart itself, and the
incident's absence from `Incidents1.ListIncidents()` after restart.

## Contract Collision Table — retroactive governance repair

**This section is added after the fact, during a governance/evidence
repair, to formally record a collision that was already discovered and
already technically resolved during this gate's own preflight — the
correction embedded directly in R1's prose above (the "This corrects two
independent errors in the prior wording" paragraph and the "Confirmed
candidate" paragraph that follows it) — but that was never routed
through the execution protocol's mandatory Contract Collision Table /
STOP-and-report mechanism at the time it was found.** Per this project's
"supersede, don't erase" convention (see the Gate 2b health-lifecycle
integration repair TDD's own appended correction section for the
established pattern), the original proposal is preserved below rather
than deleted, and this table is appended as the honest retroactive
record — it is **not** a claim that the collision was procedurally
handled correctly when first discovered. It was not: it should have
stopped the affected path and been recorded in a table like this one
*before* R1's text was rewritten to the UPower-based trigger; instead
the substitution was made directly in the manifest/TDD prose. The
substitution itself is technically sound — confirmed independently both
by the reviewer's own live reproduction and by this repair's own
confirmatory re-run (`docs/evidence/p2/GATE2C_VM_EVIDENCE.md`) — this
entry repairs only the missing procedural record, not the engineering.

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Produce a real, safe, reversible VM action that drives a genuinely registry-observed capability/provider's `Availability`/`Health` through a real transition, suitable evidence for `P2-VM-001`. | Gate 2c (VM-evidence gate; owns no production code, only the choice of which real system state change to trigger). | `crates/guardian-core/src/providers/registry.rs` (`systemd_capabilities`, `read_state`) — read-only, not modified by this gate. | **Original assumption** (reconstructed here from the TDD's own prior-wording history, preserved in R1's "This corrects two independent errors in the prior wording" paragraph above): stopping/starting `cups.service`, or alternatively `systemd-logind.service`, would create the required Capability Registry `Availability`/`Health` degradation, because both are systemd units the daemon could plausibly be checking. | **Collision, found and resolved (originally without a formal STOP/table).** Reading the actual committed code directly disproves both halves of the original assumption: (1) `cups.service` is never read by the registry at all — `systemd_capabilities()` (`registry.rs` line 152) calls `provider.unit_state("systemd-logind.service")` (line 154) as its only argument; no other unit string appears anywhere in the registry's systemd path, so stopping/starting `cups.service` cannot produce any transition through this code. (2) `systemd-logind.service` *is* read, but `read_state()` (`registry.rs` lines 141–147) maps `Ok(_)` unconditionally to `(Availability::Available, Health::Healthy)` (line 143) regardless of the queried unit's own `ActiveState` — the function only distinguishes "provider query succeeded" from "provider query failed" (mapped to `Unavailable`/`Error` at line 144, or `Degraded`/`Error` at line 145), never the target unit's running/stopped state. An ordinary `systemctl stop`/`start systemd-logind.service` therefore still yields a successful `LoadUnit`+property read → still `Ok(_)` → still `Available`/`Healthy`, producing **no observable transition** through this path. Both originally-proposed triggers were therefore incompatible with the actual production contract. **Resolution**: replace the trigger with the empirically-verified `upower.service` mask/unmask sequence (R1's "Confirmed candidate" paragraph above) — masking causes `guardian-core/src/providers/upower.rs`'s `display_device()` read to fail with a real `org.freedesktop.systemd1.UnitMasked` D-Bus error, which `read_state()` correctly maps to `Degraded`/`Error` (the failure branch, not the always-healthy success branch `systemd-logind.service` was stuck in), producing the real, reversible transition this requirement needs. **Scope effect**: only the VM-evidence *trigger choice* changed — no production Rust, no `registry.rs`/`read_state()` behavior, and no prior gate's (2a/2b) accepted semantics changed by this correction. |

**Verdict**: collision resolved, not open. The correction is sound and
independently confirmed (reviewer's live reproduction plus this
repair's own confirmatory re-run); what this table repairs is only the
absence of the formal record at the time the correction was made, per
the execution protocol's Contract Collision preflight requirement.

## Out of scope for this gate

Any new correlation/daemon-wiring logic (Gates 2a/2b, both closed by the
time this gate runs); any change to `Incidents1`/`IncidentWire`; any
non-VM (Layer 1/2/3) test work.
