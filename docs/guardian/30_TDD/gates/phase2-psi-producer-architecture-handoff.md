---
title: "Phase 2 PSI Producer — Architecture & Implementation Handoff (SUPERSEDED)"
kind: "architecture-handoff"
status: "superseded-2026-09-07"
last_reviewed: "2026-09-07"
---
# Phase 2 PSI Producer — Architecture & Implementation Handoff

> # SUPERSEDED — 2026-09-07 (governance-repair pass)
>
> **This document describes an architecture that was reviewed and
> withdrawn. Do not implement it. It is preserved in full, unedited below
> this banner, as the historical record of a real, independently-reasoned
> design — per `AGENTS.md`'s "supersede, don't hide" discipline — and
> because the two architecture-review FAIL findings against it are part of
> why the current architecture is what it is.**
>
> **What was withdrawn.** Everything specific to a cross-process producer:
> the `guardian-psi` binary and crate, the `guardianpsi` system user, the
> `debian/guardian-psi.service` unit, the
> `io.github.cliffthelin.GuardianPsi1` well-known bus name and its
> `PressureCrossing` signal, the bus-policy file, the sender-authentication
> design (§3.4), the D-Bus-vs-Unix-socket comparison (§2), the proposed
> sandbox (§6), the producer/consumer library split (§3.5), and the Layer-2
> IPC test plan (§10) and VM plan (§11) that depended on them. None of it
> is built.
>
> **Why.** Two independent architecture reviews FAILED the design:
> (1) **PSI message authority too broad** — `correlation.rs`'s `classify()`
> gates incident admission solely on `Event.severity`, which this design
> populated from the producer's self-reported `to_severity` (§3.2/§3.3)
> with no daemon-side verification against the raw measurement, so a
> compromised or buggy `guardian-psi` could fabricate unlimited `Critical`
> incidents; and (2) **the proposed new D-Bus interface conflicts with
> higher-order Phase 2 §51 language unless §51 is explicitly amended** —
> the two reviewers disagreed on whether `P2-API-002`'s enumeration is
> closed to four named client interfaces or forbids any new bus name, a
> disagreement now **moot** and deliberately left unadjudicated. A
> subsequent disposable-VM experiment then proved a narrower mechanism —
> systemd `OpenFile=` — works.
>
> **What replaces it.** In-daemon PSI production via systemd-inherited
> `OpenFile=` descriptors with daemon-owned classification: no new process,
> no new UID, no new D-Bus interface, no new IPC protocol, no
> producer-supplied severity, and no weakening of `ProcSubset=pid`. See
> `docs/adr/ADR-009-guardian-psi-producer-topology.md` **as revised
> 2026-09-07** (read its current Decision, not the superseded one preserved
> inside it), and
> `docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md`
> §9–§10.
>
> **Where this document's still-valid content went.** The parts of this
> handoff that were about *Guardian* rather than about the withdrawn IPC
> boundary were carried forward into the implementation gate's contract,
> `docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`
> and `-tdd.md`: the event-authority split (§4 here → the gate TDD's
> "Daemon authority requirements", strengthened — the daemon now
> classifies from bytes it read itself rather than trusting a reported
> severity); the scope boundary (§12 here → the gate TDD's "Scope
> exclusions", carried over essentially verbatim in substance); the
> `resource_refs = ["/proc/pressure/{resource}"]` hard constraint (§3.3
> here → gate TDD R10); the degradation/failure discipline (§7 here → gate
> TDD R16/R18); and the "real production systemd units, never manual
> `cargo run`" VM rigor (§11 here → gate TDD Phase F). This document was
> superseded in place rather than folded away entirely, so the withdrawn
> design and the reasons it failed stay legible.
>
> **This document is not deleted, and its §13 independent-review
> requirement was satisfied** — by the two reviews that failed it. That is
> the review process working, not a gap.

**This document is architecture and governance only. No production Rust
changes, no gate manifest, and no Phase 2 milestone/tag are produced by
it.** It is the bounded implementation-ready package for the architecture
`docs/adr/ADR-009-guardian-psi-producer-topology.md` selects (option (a):
a dedicated, narrow, unprivileged `guardian-psi` process). Read that ADR
first — it carries the decision and the rejected alternatives; this
document carries the operational contract a future implementation gate
must build against, plus everything that gate's own RED tests and VM
evidence must prove before it can be accepted.

This document requires **independent architectural review before any
implementation gate is opened against it** (§9). It does not authorize
implementation by its own existence.

Full precedent context (not re-derived here): `docs/guardian/30_TDD/gates/
phase2-psi-production-ingress-preflight.md` (Contract Collision, ruled-out
options (b)/(d), real VM evidence); `docs/adr/ADR-002-guardian-privilege-
topology.md` (Model B precedent); `docs/adr/ADR-001-guardian-dbus-
namespace-and-versioning.md` (namespace/versioning discipline);
`crates/guardian-core/src/providers/health.rs` (the existing
snapshot-diff producer pattern this design mirrors);
`crates/guardian-daemon/src/bin/guardian-daemon.rs` (`admit_event`, the
single `CorrelationIngress` admission point every producer must feed
through).

---

## 1. Responsibility split

```
guardian-psi owns:    kernel PSI trigger registration and polling
                       (crate guardian-core::providers::psi, unmodified
                       library code, reused as-is)
guardian-daemon owns: Guardian Event construction/admission, Phase 2
                       ingress, correlation, incidents (unchanged
                       CorrelationIngress/admit_event single-admission
                       point, Gate 2b's own established pattern)
```

`guardian-psi` sends a typed **PSI observation**, never a Guardian
`Event`. It is not, and must never become, an event-injection authority:
its wire message has no `EventId`, no timestamp, no `normalized_key`, no
`resource_refs`, and no severity field it controls directly — only a
closed, three-valued resource enum, a closed, three-valued severity-pair
(`from`/`to`), a producer-local sequence, and a raw measured reading. Full
shape in §4.

`guardian-daemon` remains the sole constructor and sole admitter of
Guardian `Event`s. It receives the typed observation, converts it to an
`Event` using the same construction discipline `providers::health::
transition_event` already established (§3), and feeds the result through
its existing `admit_event(&ingress_clock, &engine, event)` — the same
call site `monitoring_tick` and `capability_registry_tick` already use.
No new admission point is created.

---

## 2. Selected IPC: narrow internal D-Bus interface

Full comparison and rationale live in ADR-009. Summary of the comparison
table:

| Property | A. Narrow D-Bus interface (selected) | B. Narrow Unix-domain socket |
|---|---|---|
| Typing strength | Strong — `zbus`-generated typed signal, closed enum fields validated at the type layer before a handler ever sees them | Would need a hand-rolled framed protocol; typing only as strong as new parsing code makes it |
| Sender authentication | Bus-daemon-stamped `sender` field, unforgeable by another connection; well-known-name ownership independently verifiable via `GetNameOwner` | `SO_PEERCRED` at accept-time — equally strong cryptographically, but zero code in this workspace implements it today |
| Existing precedent | Exclusive — every IPC boundary in this codebase (`Guardian1`, `GuardianHelper1`, `Capabilities1`, `Incidents1`, `Transactions1`, six provider proxies) is `zbus` | None — `grep -rn "UnixListener\|UnixStream" crates/` returns nothing |
| Bus/filesystem policy | `busconfig` XML, same mechanism already governing `Guardian1`/`GuardianHelper1` ownership | Directory/socket-file permissions; no existing pattern to extend |
| Restart behavior | Bus daemon resolves `sender='io.github.cliffthelin.GuardianPsi1'` match rules to whichever process currently owns the name — no manual reconnect-and-resubscribe logic needed on either side | Manual reconnect loop required (no existing pattern for one) |
| Spoofing resistance | Structural: the `sender` field cannot be set by the sending client, only by the bus daemon itself | Structural via `SO_PEERCRED`, but requires new code to check it correctly, an unaudited new attack surface |
| Framing/protocol complexity | None — `zbus` derive macros generate the signal signature and (de)serialization | New: message framing, length-prefixing or delimiter handling, partial-read/write handling |
| Packaging impact | One more `busconfig` file plus `BusName=` in the unit, following an existing pattern exactly | A new `RuntimeDirectory=`-owned socket path, new permission scheme to design and justify from scratch |
| Testing tooling | `guardian_testkit::PrivateSessionBus` already exists and is already used for Layer 2 D-Bus contract tests (`crates/guardian-daemon/tests/phase2_2b_contract.rs`) | No equivalent test harness exists for a Unix socket in this workspace |
| Public-API implications | New, differently-named well-known name (`GuardianPsi1`), sibling to `GuardianHelper1` — not one of `P2-API-002`'s four named interfaces (`Guardian1`/`Capabilities1`/`Incidents1`/`Transactions1`); see ADR-009's reasoning for why this reading is sound, anchored in `GuardianHelper1`'s own precedent | N/A — a socket has no D-Bus interface identity to collide with anything |

**Recommendation: A (D-Bus), selected.** Both mechanisms satisfy
`RestrictAddressFamilies=AF_UNIX` (the system D-Bus socket and a raw Unix
socket are both `AF_UNIX`) — sandboxability is not a differentiator.
Every other row favors D-Bus, most decisively existing precedent
(exclusive project-wide use), testing tooling (an existing harness vs.
none), and spoofing resistance being structural rather than
newly-implemented.

---

## 3. IPC contract, defined precisely

### 3.1 Identity

```
Well-known bus name:  io.github.cliffthelin.GuardianPsi1
Object path:          /io/github/cliffthelin/GuardianPsi1
Interface:             io.github.cliffthelin.GuardianPsi1
Bus:                   system bus (same bus as Guardian1/GuardianHelper1;
                        no private bus — guardian-daemon already
                        maintains multiple concurrent system-bus
                        connections, e.g. its Guardian1-serving
                        connection and its separate registry-tick
                        connection, so a third connection dedicated to
                        this subscription follows an existing pattern)
Owning service:        guardian-psi, running as the dedicated
                        `guardianpsi` system user (§6)
```

This is a **new, separate well-known name**, never an addition to
`Guardian1`'s own object tree — structurally identical in kind to how
`GuardianHelper1` is a separate name from `Guardian1` today. See ADR-009
for why this does not collide with `P2-API-002`'s freeze.

### 3.2 Signal shape

Exactly one signal, no methods, no properties — there is no ingress
surface to inject into because there is no method to call:

```
PressureCrossing(
    resource:          s   -- "cpu" | "memory" | "io"
    from_severity:      s   -- "nominal" | "elevated" | "critical"
    to_severity:        s   -- "nominal" | "elevated" | "critical"
    producer_sequence: t   -- u64, monotonic within one guardian-psi
                              process lifetime; resets to 0 on restart
    measured_avg10:     d   -- the raw `some` line avg10 (percent
                              stalled) that produced `to_severity`'s
                              classification
)
```

This is the accepted G8 model's own `ThresholdEvent{resource, from, to}`
(`crates/guardian-core/src/psi.rs`) plus exactly the two additions needed
for the properties this task requires: `producer_sequence` for
loss/duplicate/restart detection (§7), and `measured_avg10` for forensic/
attribute value only — it is not part of the correlation key and the
daemon must never treat it as authoritative for anything beyond an
attribute on the constructed `Event`.

**Deliberately omitted, and why:**

- **Pressure class (`some`/`full`)** — not included as a variable field.
  The accepted G8 classification model (`crate::psi::classify`, called
  from `present_severity` in `providers/psi.rs`) only ever classifies the
  `some` line; there is no accepted `full`-line semantics to preserve.
  If a future gate adds `full` support, that is a `GuardianPsi1` →
  `GuardianPsi2` interface-major change (ADR-001 discipline), not a
  silent field reinterpretation.
- **Configured threshold/trigger identity** — not included. Correlation
  never consumes the registered `threshold_us`/`window_us` values (only
  the resulting severity transition), and this data is producer-local
  configuration, not per-message state. Nothing in the accepted
  correlation contract needs it to travel per-observation.

### 3.3 EventId/timestamp/normalized_key — daemon-side construction

`guardian-daemon`, on receiving and authenticating (§3.4) a
`PressureCrossing` signal, constructs the `Event` using a new
`guardian-core` function living beside `providers::psi`'s existing code
(not a new binary-local module — see §3.5), mirroring `providers::
health::transition_event`'s exact shape:

```rust
Event {
    event_id: EventId::new(format!(
        "guardian.p2.psi.{resource}.event-{producer_sequence}"
    )),
    timestamp_monotonic: producer_sequence,
    timestamp_wall: format!("sequence-{producer_sequence}"),
    source_provider: ProviderId::new("guardian.p2.psi-producer"),
    event_type: "psi_threshold_crossing".to_owned(),
    resource_refs: vec![format!("/proc/pressure/{resource}")],
    severity: match to_severity {
        Critical => Risk::High,
        Elevated => Risk::Moderate,
        Nominal  => Risk::Observe,
    },
    normalized_key: normalize_key(&raw_description),
    raw_reference: raw_description,
    attributes: { "from" -> from_severity, "to" -> to_severity,
                  "measured_avg10" -> measured_avg10 },
}
```

**Hard constraint, non-negotiable by a future implementer:**
`resource_refs` MUST be `["/proc/pressure/{resource}"]`, exactly matching
the existing `event_from_crossing` convention. `GUARDIAN_PHASE_0_1_TDD_
CONTRACT.md`'s "PSI correlation identity, corrected" repair (§51,
2026-09-06) binds PSI correlation to `resource_refs.first()` specifically
*because* `normalized_key` embeds transition-text (`{from}->{to}`) that
would prevent two different-transition Critical crossings for the same
resource from correlating. This design must not regress that fix.

`source_provider` is proposed as `"guardian.p2.psi-producer"` (renamed
from the standalone example's `"guardian.g8.psi"`), matching `providers::
health`'s own `"guardian.p2.capability-health"` naming convention for
Phase 2 producers. This is a naming proposal for the implementing gate to
confirm, not a load-bearing architectural decision.

### 3.4 Sender authentication

`guardian-daemon` MUST NOT trust any field inside the signal payload as
identity evidence — there is none to trust (§3.2 has no identity field).
Authentication is structural, from the bus itself:

1. `guardian-daemon` registers a match rule scoped to
   `sender='io.github.cliffthelin.GuardianPsi1', interface=
   'io.github.cliffthelin.GuardianPsi1', member='PressureCrossing'`.
   The D-Bus daemon resolves `sender=<well-known-name>` to whichever
   unique connection **currently owns** that name at delivery time — a
   guarantee provided by the bus daemon itself, not by the sending
   client, and not spoofable by a different connection that does not own
   the name.
2. System bus policy (`debian/io.github.cliffthelin.GuardianPsi1.conf`)
   restricts **ownership** of the name to the dedicated `guardianpsi`
   user, mirroring `GuardianHelper1`'s existing single-owner-policy
   pattern exactly (no explicit `<deny>` needed beyond the one `<allow
   own=...>` rule, matching the same minimal shape the existing
   `Guardian1`/`GuardianHelper1` policy files already use, given the
   system bus's own baseline deny-by-default for name ownership).
3. Together, (1) and (2) establish: any `PressureCrossing` signal
   `guardian-daemon` accepts was genuinely emitted by the process running
   as `guardianpsi` that currently owns `io.github.cliffthelin.
   GuardianPsi1` — never a claim made by the message content itself.

This directly satisfies the requirement that the daemon establish the
signal sender is the *current legitimate owner* of the PSI service's
well-known name, not merely that some connection claims to be it.

### 3.5 Library-boundary work required before implementation

`crates/guardian-core/src/providers/psi.rs` currently conflates two
things inside `PsiEventDispatcher::dispatch_wake`/the private
`event_from_crossing` function: (a) classifying a wake into a severity
transition, and (b) constructing a full `Event` (with a placeholder
`"guardian.g8.psi"` provider id and example-oriented `EventId` scheme).
Before implementation, this must be split so `guardian-psi` can depend on
(a) only and never link (b):

- `guardian-psi` calls `PsiEventSource::register`/`wait_for_event`
  exactly as they exist today, but the dispatch path needs a variant
  (or `dispatch_wake` itself needs to be changed) that yields the typed
  crossing (`ThresholdEvent` + a producer-owned sequence + the measured
  `avg10`) instead of a full `Event`. This is a small, mechanical
  refactor, not new PSI semantics.
- Full `Event` construction (§3.3) becomes a new `guardian-core` function
  — living in `providers/psi.rs` beside the existing code, not copied
  into a `guardian-psi`-local module — called only from `guardian-daemon`
  after receiving and authenticating a signal. This mirrors `providers::
  health::transition_event` exactly: a pure-Rust, zero-I/O library
  function that the daemon binary calls, never a binary-local
  duplication of PSI semantics.
- `guardian-psi` itself never depends on `guardian_core::event::Event`,
  `EventId`, or `normalize_key` at all — architecturally, not just by
  convention, it cannot construct a Guardian `Event` because the type
  vocabulary to do so is not part of what it imports.

This refactor is implementation scope for a future gate, explicitly not
performed by this document.

---

## 4. Event authority (restated precisely)

| Field | Constructed by |
|---|---|
| `EventId` | `guardian-daemon` (via the new `guardian-core` conversion fn, §3.3) |
| Guardian timestamp / ingress order | `guardian-daemon`'s `IngressClock`, at `admit_event` time — unaffected by `producer_sequence`, which only feeds `timestamp_monotonic`/`timestamp_wall` display fields, matching `providers::health`'s existing convention |
| `normalized_key` | `guardian-daemon`, from the same `raw_reference` construction `event_from_crossing` already uses |
| `resource_refs` | `guardian-daemon`, fixed to `/proc/pressure/{resource}` (§3.3's hard constraint) |
| Confidence/source metadata | `guardian-daemon`'s existing `CorrelationEngine::classify`/`transition_confidence` path — unmodified by this design; PSI crossings feed the same confidence machinery Gate 2a already built |

No deviation from the task's default principle is warranted or proposed:
`guardian-psi` reports; `guardian-daemon` constructs and admits.

---

## 5. Service identity

**New dedicated unprivileged system user: `guardianpsi`.** Created in
`debian/postinst` following the exact pattern already used for
`guardiand` (`adduser --system --group --no-create-home`), never reusing
`guardiand`'s identity (ADR-009 §"Alternatives considered" states the
reasoning: shared UID would collapse the bus-policy ownership boundary
this design's sender-authentication argument relies on, and would blur
attribution for zero operational benefit). `guardian-helper`'s `root`
identity is obviously not reused — PSI needs zero privilege (confirmed,
preflight §4), so granting it root would be a pure regression.

No Linux capability is assumed necessary. `CapabilityBoundingSet=` and
`AmbientCapabilities=` are both empty, identical to `guardian-daemon`'s
and `guardian-helper`'s existing units. The preflight's own VM research
(§4, Privilege Requirement Inventory line 41) already confirms PSI read
and trigger-registration are both unprivileged for an unprivileged
monitor; nothing in this design changes that.

---

## 6. `guardian-psi` sandbox — PROPOSED, not proven

**This sandbox is proposed for independent review. It is not proven by
VM evidence — that is explicitly a future implementation-phase
requirement (§8), not something claimed here.**

Modeled directly on `debian/guardian-daemon.service`'s full accepted
directive set (quoted in full in the preflight document §8.1), varying
only what PSI access genuinely requires:

```ini
[Unit]
Description=Guardian production PSI producer (unprivileged, narrow)
After=dbus.socket
Wants=dbus.socket

[Service]
Type=simple
User=guardianpsi
Group=guardianpsi
BusName=io.github.cliffthelin.GuardianPsi1
ExecStart=/usr/bin/guardian-psi
Restart=on-failure

NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_UNIX
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallFilter=@system-service
DevicePolicy=closed
ProtectClock=yes
SystemCallArchitectures=native
ProtectHostname=yes
ProtectProc=invisible
# ProcSubset intentionally OMITTED (defaults to "all") -- the one
# deliberate, narrow relaxation this entire design exists to make,
# scoped to this one unit only. guardian-daemon and guardian-helper keep
# ProcSubset=pid unchanged (preflight doc explicitly notes this
# visibility is acceptable specifically because it is no longer granted
# to the primary daemon).
PrivateNetwork=yes
PrivateUsers=yes
UMask=0077

[Install]
WantedBy=multi-user.target
```

**Deliberate differences from `guardian-daemon.service`, each justified:**

- **`ProcSubset=` omitted** (defaults to `all`) — the sole reason this
  unit exists; `ProtectProc=invisible` is kept unchanged (it governs
  visibility of *other processes'* `/proc/<pid>` entries via `hidepid`
  semantics, which is unrelated to and independent of `/proc/pressure`
  visibility — confirmed in the preflight document's own reading of `man
  systemd.exec` and consistent with its VM evidence, §8.11: setting
  `ProcSubset=all` alone was sufficient to restore `/proc/pressure`
  visibility without touching `ProtectProc`). Keeping `ProtectProc=
  invisible` means that even a compromised `guardian-psi` process still
  cannot introspect other users' running processes via `/proc/<pid>` —
  strictly narrower than granting full `/proc` visibility would be.
- **No `ReadWritePaths=`, no `StateDirectory=`, no
  `Environment=..._STATE_DIR`** — `guardian-psi` holds no state of any
  kind (§9's scope boundary forbids PSI persistence); it needs zero
  writable filesystem access. This is *stricter* than `guardian-daemon`'s
  own unit, which needs one writable path for its recorder state.
  `ProtectSystem=strict` therefore applies with no carve-out at all.
- **`BusName=io.github.cliffthelin.GuardianPsi1`** replaces
  `io.github.cliffthelin.Guardian1` — its own identity, per §3.1.

Bus policy (`debian/io.github.cliffthelin.GuardianPsi1.conf`), mirroring
the existing `GuardianHelper1.conf` shape exactly:

```xml
<busconfig>
  <policy user="guardianpsi">
    <allow own="io.github.cliffthelin.GuardianPsi1"/>
  </policy>
  <policy context="default">
    <allow send_destination="io.github.cliffthelin.GuardianPsi1"/>
  </policy>
</busconfig>
```

(`send_destination` policy is included for parity with the existing
pattern even though this interface has no methods to call; it has no
practical effect since there is nothing to send *to* — signal receipt is
governed by the bus's own default broadcast-receive posture, unchanged
by this file.)

---

## 7. Availability and failure semantics

| Scenario | Behavior |
|---|---|
| `guardian-psi` absent at daemon startup | `guardian-daemon` starts normally; registers its `sender=`-scoped match rule regardless of whether the name currently has an owner. No PSI events are admitted until `guardian-psi` starts and emits one. No error, no retry loop, no crash. |
| `guardian-daemon` starts before `guardian-psi` | Same as above — match rules are resolved dynamically by the bus daemon to whichever process owns the name *at signal delivery time*, so no explicit reconnect/resubscribe logic is required once the match rule is registered once, at daemon startup. |
| `guardian-psi` starts before `guardian-daemon` | No effect — `guardian-psi` does not need a listener to emit signals; any crossings before the daemon subscribes are simply never observed (§8 boundedness — this is a gap, never backpressure). |
| systemd unit ordering | `Wants=` only, no `Requires=`, no `Before=`/`After=` between the two units — a fragile convenience ordering is explicitly not needed given the dynamic sender-resolution property above, matching this task's explicit instruction that a PSI outage must degrade observability, not correctness. |
| `guardian-psi` restart | Fresh process, `producer_sequence` resets to 0 — an expected "fresh producer epoch," symmetric with the already-accepted "fresh `IngressClock` epoch on `guardian-daemon` restart" semantics (Gate 2c, `P2-VM-002`). The daemon must treat a sequence value lower than the last-seen one as evidence of a producer restart (log once), never as corruption or a rejected message. |
| `guardian-daemon` restart | Unaffected by this design — existing accepted behavior (fresh `IngressClock`, in-memory incident state lost) stands unchanged. `guardian-psi` keeps running; it does not need to restart or resubscribe to anything (it has no subscription of its own — it only emits). |
| IPC/D-Bus connection loss | `guardian-daemon`'s subscribing connection reconnects on its own bounded retry cycle, following the existing pattern its registry-tick worker thread already uses (`match async_io::block_on(zbus::Connection::system()) { ... }`, logged, retried on a fixed interval) — never a panic, never an unbounded retry storm. |
| Malformed PSI message | Rejected at the typed-signal deserialization layer (an unrecognized `resource`/`from_severity`/`to_severity` string, or a signature mismatch) — dropped, logged once via the existing `eprintln!("[guardian-daemon] ...")` convention, never admitted as an `Event`, never a panic. |
| Unauthorized sender | Structurally cannot occur for the *sender field* itself (§3.4) — the bus daemon guarantees it. The adjacent risk (a rogue process trying to *own* the well-known name before the real service starts) is closed by bus policy restricting ownership to the `guardianpsi` user (§6); `guardian-daemon`'s match rule additionally only ever resolves to whoever *currently* owns the name, so a rogue owner would need to also pass bus-policy ownership restriction to be delivered at all. |
| Dropped observation | Expected and harmless — a missed crossing is a missed observability window, never a stuck/blocked state (§8). |
| Duplicate observation | Not expected under normal D-Bus signal delivery (no redelivery mechanism exists), but if it occurred, `CorrelationEngine`'s own already-accepted debounce/dedup machinery (Gate 2a) handles it identically to any other repeated crossing — no new duplicate-suppression logic is introduced at the IPC layer. |
| Pressure trigger registration failure | `guardian-psi` treats each of `cpu`/`memory`/`io` independently (mirroring `providers::registry::populate_registry`'s existing "one provider failing does not stop the others" discipline) — one resource's registration failure is logged and that resource's monitoring is simply absent, never silently reported as available. Only if all three resources fail does the process exit non-zero, deferring to `Restart=on-failure` rather than looping tightly on a per-resource retry. |

---

## 8. Boundedness

PSI crossings are inherently rate-limited by the kernel's own trigger
mechanism: `ThresholdMonitor::observe` only yields `Some(ThresholdEvent)`
on an actual severity-class crossing (not on every poll wake), and the
kernel PSI trigger ABI's own minimum averaging window
(`window_us >= 500_000`, enforced by `PsiTrigger::register`) bounds how
often a real crossing can even occur. Three independently-monitored
resources (`cpu`/`memory`/`io`) at this rate produce a message volume
several orders of magnitude below anything requiring an application-level
queue.

No unbounded structure is introduced by this design:

- D-Bus signals have **no delivery guarantee to a not-yet-subscribed
  listener** — a signal emitted while nothing is listening is simply not
  received, never buffered and redelivered later. This means a
  `guardian-daemon` outage cannot cause `guardian-psi`-side accumulation:
  there is no queue for messages to pile up in on either side.
  (This is a structural advantage over a hand-rolled socket protocol,
  which would need its own explicit bounded-buffer-and-drop policy to get
  the same property — see ADR-009's IPC comparison.)
- `guardian-daemon`'s own downstream structures (the debounce/dwell ring,
  the open-incident cap, the closed-incident ring) are already bounded
  with documented eviction/rejection policies from Gate 2a — this design
  adds no new consumer-side structure; PSI-derived `Event`s flow through
  exactly the same bounded machinery every other producer's events do.
- Restart/disconnect semantics (§7) never retain or replay missed
  messages — every gap is a silent, bounded loss of observability, never
  a growing backlog.

---

## 9. Security properties — argued, not merely asserted

| Property | Why it holds |
|---|---|
| Clients cannot inject PSI observations | Only a connection that both (a) runs as the `guardianpsi` UID (bus-policy-enforced ownership restriction, §6) and (b) currently owns `io.github.cliffthelin.GuardianPsi1` can have its `PressureCrossing` signals matched by `guardian-daemon`'s `sender=`-scoped rule (§3.4) — both are bus-daemon-enforced, not client-asserted. |
| Arbitrary local processes cannot impersonate `guardian-psi` | The `sender` field on any D-Bus message is stamped by the bus daemon itself from the real kernel socket connection, never settable by the sending client; a match rule scoped to the well-known name's current owner is therefore unforgeable by a different connection. |
| `guardian-psi` cannot submit arbitrary Guardian events | Its wire vocabulary (§3.2) has no `EventId`, timestamp, `normalized_key`, `resource_refs`, or free-form severity field — only a closed three-valued resource enum and a closed three-valued severity pair. `Event` construction code lives only in `guardian-core`/`guardian-daemon` (§3.5); `guardian-psi` does not link the types needed to build one. |
| The PSI process has no Guardian privileged-write authority | `guardian-psi` never constructs a D-Bus client proxy to `GuardianHelper1` or any write-capable interface; its sandbox (§6) grants it no writable filesystem path and no capability; it is architecturally a pure reader-and-broadcaster. |
| `guardian-helper` is not involved | Nothing in this design references `GuardianHelper1`, `root`, or any privileged-write code path. `guardian-helper`'s own unit, user, and code are untouched. |
| No generic broker appears | `GuardianPsi1` exposes exactly one signal with a closed, typed shape — never a method, never a path/argv/opaque-payload parameter, following the same "no claimed-identity field, no generic input" discipline ADR-002 required of `GuardianHelper1`'s own bounded methods. |
| `guardian-daemon`'s existing sandbox is completely unchanged | `debian/guardian-daemon.service` is not modified by this design (validated by `git diff` in §"Validation" of the completion report below) — `ProcSubset=pid` and every other accepted directive stand exactly as G7/G9 left them. |

---

## 10. Testing plan (define, do not write yet)

RED tests needed before implementation, for the full production chain
(PSI observation → authenticated internal IPC → daemon conversion →
shared Phase 2 ingress → correlation):

**Layer 1 (`guardian-core`, pure Rust):**
- The new observation→`Event` conversion function preserves
  `resource_refs = ["/proc/pressure/{resource}"]` (the §51 correlation-
  identity fix) for every `(resource, from, to)` combination.
- Severity mapping (`Critical→Risk::High`, `Elevated→Risk::Moderate`,
  `Nominal→Risk::Observe`) is exhaustively covered.
- The refactored dispatch path yields a typed observation (not an
  `Event`) and is unit-tested exactly like the existing
  `PsiEventDispatcher`/`ThresholdMonitor` tests, unmodified where they
  already pass.

**Layer 2 (mocked/private D-Bus, using the existing `guardian_testkit::
PrivateSessionBus` harness, following `phase2_2b_contract.rs`'s own
technique):**
- **Valid sender**: a connection that owns `io.github.cliffthelin.
  GuardianPsi1` on the private bus emits `PressureCrossing`;
  `guardian-daemon`'s subscriber admits a correctly-shaped `Event`
  through `admit_event`, observable via `Incidents1.ListIncidents()`.
- **Wrong sender**: a second connection, not owning the well-known name,
  attempts to emit a matching-shaped signal (or to claim the sender
  identity); the daemon's `sender=`-scoped match rule does not deliver it
  — zero `Event`s admitted.
- **Malformed observation**: an out-of-enum `resource`/`from_severity`/
  `to_severity` string, or a wrong-arity signature — rejected, logged,
  not admitted, no panic.
- **Producer absent**: daemon starts, subscribes, receives nothing —
  `Incidents1.ListIncidents()` stays empty for PSI-sourced incidents; no
  error state.
- **Producer reconnect**: original owner disconnects (name loses an
  owner), a second connection takes ownership and emits — subsequent
  signals resume being admitted with no daemon-side restart needed.
- **Duplicate/out-of-order `producer_sequence`**: two observations with a
  sequence that goes backward (simulating a restart) are both still
  individually valid observations and both flow through the existing
  correlation dedup machinery unchanged — the IPC layer itself performs
  no special-case suppression.
- **Bounded-queue behavior (negative)**: a burst of many rapid valid
  signals does not grow any daemon-side structure beyond the existing
  Gate 2a bounded rings' documented capacities.
- **No generic injection path (negative)**: an introspection-diff test,
  following `assert_p2_api_002_no_new_dbus_surface`'s exact technique,
  proves `GuardianPsi1`'s member list is exactly one signal
  (`PressureCrossing`) and nothing else — no method exists anywhere on
  this interface for any client to call.

---

## 11. Real VM acceptance plan (define, do not execute yet)

Using the real production systemd units (never manual `cargo run`),
following Gate 2c's own `P2-VM-001`/`P2-VM-002` methodology and rigor:

1. `guardian-daemon` retains its accepted `ProcSubset=pid` sandbox
   **unchanged** — diff `debian/guardian-daemon.service` against the
   accepted G9/Gate-2c baseline (byte-identical) and re-run
   `systemd-analyze security` against the installed unit, confirming no
   regression from its accepted score.
2. `guardian-psi` runs unprivileged — `/proc/<pid>/status` shows a
   non-root, non-`guardiand`, dedicated `guardianpsi` UID.
3. `guardian-psi` has zero capabilities — `CapPrm`/`CapEff`/`CapBnd`/
   `CapAmb` all `0000000000000000`, matching every other Guardian unit's
   evidenced baseline; explicitly confirm no capability was added "just
   in case."
4. Real PSI trigger registration succeeds inside the real
   `guardian-psi.service` unit — reproducing the preflight document's own
   Experiment 5 methodology (a bounded `stress-ng` load, confirming a
   real crossing occurs, not just that registration doesn't error).
5. `poll(POLLPRI)` wakes on a real kernel event inside the real unit.
6. A typed `PressureCrossing` signal crosses the real system-bus IPC
   boundary — captured via `busctl monitor` or equivalent, showing the
   real `sender=` unique name resolving to `guardian-psi`'s process.
7. The daemon authenticates the producer — confirmed by the adversarial
   case in item 11 below, not merely by the happy path succeeding.
8. A Guardian `Event` enters the real production Phase 2 ingress —
   `Incidents1.ListIncidents()` reflects a real PSI-driven incident over
   the real system bus, mirroring `P2-VM-001`'s own evidentiary shape but
   PSI-triggered rather than provider-health-triggered.
9. Correlation executes — the admitted incident's `Confidence`/grouping
   follows the existing, accepted correlation-engine semantics; repeated
   same-resource crossings are observed to correlate via `resource_refs`,
   not `normalized_key` (directly verifying the §51 fix is preserved in
   the real, not just the unit-tested, path).
10. Stopping `guardian-psi` does not break other daemon observability —
    `Capabilities1`, provider-health-driven `Incidents1` entries, and the
    monitoring-tick recorder continue functioning normally with
    `guardian-psi` stopped.
11. Restarting `guardian-psi` reconnects correctly — a fresh
    `producer_sequence` epoch is observed, and subsequent crossings
    resume being admitted with no `guardian-daemon` restart required.
12. **Unauthorized injection attempt is rejected** — a real, ordinary
    unprivileged local process (not `guardianpsi`) attempts to (a) own
    `io.github.cliffthelin.GuardianPsi1` itself (must fail: bus policy
    denies ownership to any user but `guardianpsi`) and (b), as a
    separate probe, send a well-shaped `PressureCrossing`-mimicking
    message while *not* owning the name (must never be delivered to
    `guardian-daemon`'s `sender=`-scoped subscription, confirmed by zero
    resulting `Event` admission).

---

## 12. Scope boundary (restated, binding on any future implementation gate)

This repair must not introduce: PSI persistence of any kind (no state
directory, no disk-backed queue); a generic event-submission mechanism
(no `SubmitEvent`/`InjectEvent`/`SendArbitraryEvent` or equivalent, on
any interface); a new user-facing/public API (`GuardianPsi1` is an
internal trust-boundary interface between two Guardian-owned processes,
not a documented client surface, and discloses nothing beyond what
`/proc/pressure` already exposes to any local reader); a new mutation
capability (no writable path, no capability, no privileged D-Bus call
anywhere in `guardian-psi`); Phase 2 feature growth beyond closing this
one gap (no `full`-line pressure support, no severity field, no new
correlation rule); diagnostic escalation; any G9-successor/next-phase
work.

---

## 13. Independent review requirement

Per this task's own governing instruction and this project's established
practice (ADR-002's Model A/B comparison, ADR-006's indicator-mechanism
spike, ADR-007/008's toolkit/packaging decisions — all independently
reviewed before their consequences were built): **this package (ADR-009
plus this document) requires independent architectural review before any
implementation gate is opened against it.** A future implementation gate
must be assigned its own `owned_normative_ids`, its own manifest, and its
own baseline — none of that is created by this document. In particular,
a reviewer should confirm independently: the `P2-API-002` freeze reading
in ADR-009 (§ "Decision"); the §3.5 library-boundary refactor's scope is
genuinely small and does not touch any accepted correlation semantics;
and the §6 sandbox proposal, once implemented, actually achieves the
`ProcSubset=all`-while-`ProtectProc=invisible` behavior this document
predicts from the preflight's diagnostic (not literal) VM evidence — real
VM confirmation of the *exact* proposed unit (§11 item 1) is still
required, since §8.11's `ProcSubset=all` test was run as an isolating
diagnostic, not as a full run of this proposed unit's complete directive
set.
