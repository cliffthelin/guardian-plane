---
title: "Phase 2 PSI Production-Ingress Preflight — Contract Collision, architecture decision required"
kind: "contract-collision-preflight"
status: "contract-collision-resolved-architecture-reselected-pending-implementation-gate"
last_reviewed: "2026-09-07"
---
# Phase 2 PSI Production-Ingress Preflight

**This is a preflight-only document, not an executable gate.** No
`*-manifest.toml` pairs with this file and none should be created yet:
per `GUARDIAN_EXECUTION_PROTOCOL.md`'s binding rule, a Contract Collision
that surfaces "more than one materially different trust-boundary design"
must stop before production implementation and be adjudicated, not
resolved unilaterally inside a gate. This file is that stop-and-report
record for one specific, real, confirmed gap: **`guardian-daemon` never
instantiates a live PSI production event loop**, despite Phase 2's own
governing text including PSI-event correlation in scope. It does not
reopen Gate 2a, Gate 2b, or Gate 2c — none of their accepted code,
evidence, or acceptance is touched or re-litigated by this file. It does
not implement anything.

Full context (not to be re-derived here): `GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`
§51 (Decision item 3, "PSI correlation identity, corrected");
`GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §1/§3/§19/§20;
`docs/guardian/30_TDD/gates/phase2-2b-tdd.md` R2 (the local deferral this
file follows up on); `docs/adr/ADR-002-guardian-privilege-topology.md`;
`docs/evidence/g2/PRIVILEGE_REQUIREMENT_INVENTORY.md`.

---

## 1. The requirement, quoted, with precedence resolved

§51's Decision (`GUARDIAN_PHASE_0_1_TDD_CONTRACT.md`, item 3) states
TDD-contract Phase 2 is exactly:

> "3. Provider-health-transition correlation and PSI-event correlation
> (already produced by G8's `providers::psi` wiring) into that same
> Incident model — 'correlated with,' never 'caused by.'"

This clause has two parts: a **scope inclusion** ("PSI-event correlation
... into that same Incident model" — affirmatively in scope, not among
§51's explicit exclusions list) and a **factual predicate**
("already produced by G8's `providers::psi` wiring"). The predicate is
false — confirmed directly against production source (§3 below) — but
the scope inclusion does not depend on the predicate being true; it is a
freestanding requirement. §51's repair passes corrected the identical
kind of false-predicate defect for provider-health ("already produced" →
corrected to "capability_registry_tick returns snapshots only... this
phase's scope therefore includes producing a provider-health transition
`Event`") but never applied the same correction to the PSI predicate,
even though the PSI predicate is symmetrically false. This is
precedence-resolved as follows: **PSI-event correlation remains
in-scope for Phase 2** (the inclusion clause is independent of the
predicate), and **the false predicate is an uncorrected defect of the
same shape §51 already fixed once for provider-health**, not a
deliberate deferral. Nothing in §51's "explicitly excludes" list
(Transactions1, severity, Flight Recorder persistence, journald,
diagnostic-escalation, master-spec Phase 2, Wave 1 backlog items)
names PSI production wiring — PSI is absent from every deferral list in
the governing text.

The implementation handoff repeats the same uncorrected predicate twice:

> "PSI events | G5/G8 ... | Correlation input, in scope | Yes | Real
> `/proc/pressure` reads, `ThresholdMonitor`, produces real `Event`s via
> `poll()` — **production-wired since G8**" (§1, capability table)

> "PSI telemetry/events | Yes (G5 model, G8 wiring) | **Yes** — real
> `/proc/pressure` via `poll()` | No | ... | Needs correlation, not
> re-implementation" (§3, primitive inventory)

Both rows are corrected in `docs/guardian/30_TDD/gates/phase2-2b-tdd.md`
R2 (quoted in full, §2 below) — but that correction was made as a local,
in-gate scoping note, not as an amendment to §51/the implementation
handoff, and not as a formal Contract Collision Table entry per the
execution protocol. **No `P2-*` normative ID, and no gate manifest
(2a/2b/2c), ever assigns ownership of building a live PSI production
event loop** — confirmed by reading `owned_normative_ids` in all three
gate manifests (`phase2-2b-manifest.toml`: `P2-API-001..003` only;
`phase2-2c-manifest.toml`: `P2-VM-001..002` only, explicitly "produces
evidence, not new code"). The requirement is real; its ownership fell
through a crack between the planning-pass predicate error and three
gates' worth of local, individually-reasonable scoping decisions.

---

## 2. Historical text preserved (supersede, don't erase)

`docs/guardian/30_TDD/gates/phase2-2b-tdd.md` R2, verbatim, unmodified by
this file:

> "PSI is not a live event producer in `guardian-daemon` as of this gate
> (confirmed: no production PSI event-generation loop exists outside a
> standalone example) — this gate does not add one, since no owned
> `P2-API-*` ID requires it and doing so would be scope creep beyond R1's
> 'required foundation only' framing. The single-admission-point design
> is PSI-compatible: a future gate that wires a live PSI producer must
> feed it through this same `IngressClock`/`CorrelationIngress` point,
> not a separate one, but standing that producer up is not this gate's
> work."
>
> "*Process note, recorded per this gate's own review:* when a Contract
> Collision Table ... surfaces a real conflict like this one (TDD text
> presupposing a producer that doesn't exist in production), the
> protocol's existing stop-and-report rule governs — resolve it by
> stopping and reporting, not by adjudicating it unilaterally inside the
> implementation. This TDD correction is the result of that conflict
> being raised after the fact; future gates should raise it before
> editing."

**This file is that "future gate" raising it before editing**, per that
process note's own instruction. Gate 2b's local disposition was a
defensible in-gate scoping call at the time (no owned `P2-API-*` ID
named it, and R1's "required foundation only" framing did legitimately
bound R1 to provider-health) — but it was not, and could not be, a
resolution of the higher-order §51 scope-inclusion question, which Gate
2b's own manifest had no authority to narrow. That question is resolved
in §1 above by reading §51 directly, not by accepting either Gate 2b's
silence or this task's own framing at face value.

`crates/guardian-daemon/src/bin/guardian-daemon.rs`'s module doc comment
independently confirms the same fact from the production source itself
(quoted in full, §3 below) and was already, correctly, not modified by
Gate 2b — it states the gap honestly rather than hiding it.

---

## 3. Contract Collision Table

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Live PSI events must reach the shared production `CorrelationIngress` so `P2-COR-001`/`P2-COR-002` (PSI `Critical`-crossing correlation) have real, non-test data to operate on in production, per §51 Decision item 3's scope-inclusion clause. | No gate owns this. Gate 2a owns the *correlation logic* consuming PSI events (`P2-COR-001/002`, closed, tested against synthetic events). Gate 2b owns *daemon wiring* but explicitly declined PSI production (R2, quoted above). Gate 2c owns *VM evidence only*, no new code. | `crates/guardian-daemon/src/bin/guardian-daemon.rs` (`main()`, `monitoring_tick`, `capability_registry_tick` — the only three producers actually wired to `admit_event`); `crates/guardian-core/src/providers/psi.rs` (`PsiEventSource`, complete and production-ready as a *library* capability, confirmed unmodified and correct). | §51/the implementation handoff assert PSI is "production-wired since G8" and require no new provider read for it; Gate 2b's own TDD (written later, verified against real source) found this false and treated it as out-of-scope-for-this-gate rather than escalating; no subsequent gate ever picked it up. The result: a real, binding scope-inclusion (§1) with zero owning gate. | **Collision confirmed real, not resolved here.** Per the execution protocol's binding STOP rule and per AGENTS.md/this task's explicit instruction not to unilaterally amend §51 to declare PSI deferred: this table records the collision and the precedence resolution (§1) but does **not** pick an implementation. Resolution requires an architecture decision (§5 below) before any gate can be assigned `owned_normative_ids` for a live PSI producer. |
| Any production mechanism that lets `guardian-daemon` (or a new component) read `/proc/pressure/*` must not silently weaken the accepted G2/G7 `ProcSubset=pid`/`ProtectProc=invisible` hardening on `guardian-daemon` or `guardian-helper` (`debian/guardian-daemon.service`, `debian/guardian-helper.service`, both accepted at G7/G9, confirmed still in force by `docs/evidence/g9/g9-daemon-security.txt`/`g9-helper-security.txt`). | G2 (ADR-002), G7/G9 hardening — accepted, not owned by any Phase 2 gate. | `debian/guardian-daemon.service`, `debian/guardian-helper.service`. | A live PSI producer inside either existing hardened unit needs `/proc/pressure/*` visibility; `ProcSubset=pid` on both units currently makes it invisible (verified, §4 below), independent of DAC permissions. | **Not resolved here** — see §5's evaluated options. AGENTS.md's privilege rules and this task's step 5 forbid resolving this collision by unilaterally weakening either unit's hardening inside this preflight pass. |

**Verdict: collision open, not resolved.** This table exists to satisfy
the execution protocol's mandatory stop-and-report rule, not to declare
a winner among the options in §5.

> **Verdict superseded (governance-repair pass, 2026-09-07) — see §10.**
> The "collision open, not resolved" verdict above is preserved exactly
> as the accurate record of what this preflight could honestly conclude
> at the time it was written; it is **not** erased and was **not** wrong.
> It is superseded by §10, which records the collision as **RESOLVED** on
> evidence obtained later: the two horns of this table (Phase 2 requires
> live PSI events in the shared production ingress / the obvious in-daemon
> repair conflicts with the accepted `ProcSubset=pid` boundary) turned out
> not to be mutually exclusive, because systemd `OpenFile=` supplies the
> descriptor from PID 1's namespace without making `/proc/pressure`
> visible by pathname inside the sandbox at all (§9).

---

### 3a. Resolution (architecture-selection pass, 2026-09-07)

> **SUPERSEDED IN PLACE (governance-repair pass, 2026-09-07) — see §9 and
> §10.** Everything in this subsection is preserved verbatim as the
> historical record of the first architecture-selection pass. Its
> selection of **option (a)** (a separate, unprivileged `guardian-psi`
> production process with a new dedicated service identity and a new
> `io.github.cliffthelin.GuardianPsi1` D-Bus interface) is **no longer the
> selected architecture**: two independent architecture reviews returned
> FAIL findings against it (§9.1), and a subsequent disposable-VM
> experiment then proved a narrower mechanism — systemd `OpenFile=` —
> works (§9.2–§9.9). §9 records that experiment and the reselected
> architecture; §10 records the formal resolution of the §3 Contract
> Collision. The four-option disposition block below (options B and D
> REJECTED on real evidence, option C NOT SELECTED) is **not** disturbed
> by that reselection — all three of those dispositions still stand, on
> the same evidence; only option (a)'s "SELECTED FOR ARCHITECTURAL
> DESIGN" line is superseded, by a fifth mechanism (`OpenFile=`) that §5
> did not identify at all.

**This supersedes nothing above — the table and its "collision open, not
resolved" verdict stand exactly as recorded, preserved for the historical
record per "supersede, don't erase."** This subsection records that the
collision is now **resolved at the architecture-selection level** by a
separate, later pass, restated here so a reader of this file alone sees
the current disposition without needing to cross-reference other files
first.

Restated plainly, for the record, the shape of the collision this table
captured:

> Phase 2 requires live PSI events to enter the permanent shared
> correlation ingress. §51 incorrectly assumed this was already provided
> by G8. Production `guardian-daemon` does not instantiate
> `PsiEventSource`. The obvious in-daemon repair conflicts with the
> accepted `ProcSubset=pid` service-hardening boundary.

That collision is real, was correctly stopped-and-reported (not
unilaterally adjudicated) by this file at the time it was written, and is
**not** resolved by Gate 2b's earlier local disposition (§2 above,
preserved verbatim) — Gate 2b's R2 text was a defensible in-gate scoping
call ("no owned `P2-API-*` ID names it") but had no authority to resolve
§51's higher-order scope-inclusion question, and its own process note
says so explicitly. Both points stand exactly as §1/§2 already state them
and are not restated redundantly here beyond this one summary sentence.

**Disposition of the four §5 options, formalized:**

```
Option B — narrow ProcSubset carve-out: REJECTED — platform mechanism
  does not exist (§4/§5(b): systemd 259 on this host documents only
  ProcSubset=all|pid, no path-scoped or PSI-specific value).
Option D — BindReadOnlyPaths=/proc/pressure: REJECTED — two independent
  VM reproductions (Claude + Codex, §8.1-8.11).
  Reasons: bind destination absent under subset=pid (unit fails to start,
  226/NAMESPACE, §8.3); read-only mount incompatible with the kernel PSI
  trigger-registration ABI's mandatory O_RDWR open (EROFS, §8.6-8.7),
  even in the nearest working relaxed variant.
Option C — broaden guardian-daemon to ProcSubset=all: NOT SELECTED —
  unnecessarily broadens the primary daemon's already-accepted sandbox.
  Remains the fallback only if option A were independently shown
  infeasible; it has not been.
Option A — dedicated unprivileged PSI producer: SELECTED FOR
  ARCHITECTURAL DESIGN.
```

**Resolution record:** `docs/adr/ADR-009-guardian-psi-producer-topology.md`
formally selects option (a) — a new, narrow, unprivileged `guardian-psi`
production process, reusing `providers::psi`'s existing library code
unmodified, with a new dedicated `guardianpsi` service identity and a new
internal D-Bus interface (`io.github.cliffthelin.GuardianPsi1`, sibling
to `GuardianHelper1`, not an addition to the frozen `Guardian1`/
`Capabilities1`/`Incidents1`/`Transactions1` family). The full IPC
contract, sandbox proposal, event-authority split, failure/restart
semantics, RED test requirements, and real-VM acceptance plan are
specified in the companion document, `docs/guardian/30_TDD/gates/
phase2-psi-producer-architecture-handoff.md`.

**What is, and is not, closed by this resolution:** the *architectural
question* — which trust-boundary shape a live PSI producer should take —
is now decided and recorded per this project's governance order (an
ADR, per AGENTS.md's "when a gate requires an architectural decision, add
the required ADR"). The *implementation* is not: no gate manifest exists
yet, no `owned_normative_ids` are assigned, no production Rust changes,
and the companion handoff document's own §13 requires independent
architectural review of this selection before any implementation gate is
opened against it. This file's own §7 disposition line ("A future,
separately-assigned implementation pass may proceed only after a human
decision selects among §5's options... and that decision is recorded as
a... ADR") is satisfied by ADR-009's existence, not bypassed by it.

---

## 4. Call-graph and hardening verification (independently reproduced, not assumed)

**Production call graph.** `grep -rn "PsiEventSource" --include="*.rs" .`
across the whole workspace returns exactly four non-declaration hits:
one construction site (`crates/guardian-core/examples/
g8_psi_trigger_evidence.rs:17`, a standalone example binary, not part of
any systemd-managed production process) and the type's own definition/
doc comments (`crates/guardian-core/src/providers/psi.rs`) plus one
doc-comment mention inside `guardian-daemon.rs` itself (line 64,
explaining why it is *not* called). `crates/guardian-daemon/src/bin/
guardian-daemon.rs`'s `fn main()` (line 298) calls only `monitoring_tick`
(daemon-tick producer) and `capability_registry_tick`
(provider-health producer, feeding `HealthTransitionProducer`) — neither
references PSI in any form. No `#[cfg(test)]`/`tests/`/`examples/`
construction counts as production instantiation per this task's framing,
and none of the four hits above is reachable from `main()`. **Confirmed:
zero production PSI event admission exists today.**

**Systemd hardening.** `debian/guardian-daemon.service` and
`debian/guardian-helper.service` both carry `ProtectProc=invisible` and
`ProcSubset=pid`, confirmed still in force by the G9 evidence
(`docs/evidence/g9/g9-daemon-security.txt:35`, `g9-helper-security.txt:35`:
"`ProcSubset= ... Service has no access to non-process /proc files (/proc
subset=)`" — `systemd-analyze security`'s own description, not this
task's paraphrase). `man systemd.exec` on this host (systemd 259,
259.5-0ubuntu3.4) confirms: `ProcSubset=` "Takes one of 'all' (the
default) and 'pid'. If 'pid', all files and directories not directly
associated with process management and introspection are made invisible
in the `/proc/` file system" — a binary, mount-namespace-level
restriction with **no finer granularity**; there is no
PSI-specific/path-scoped `ProcSubset` value in this systemd version.
**Confirmed: both existing hardened units currently cannot see
`/proc/pressure/*`, and no narrower carve-out of `ProcSubset` itself
exists on this platform (§5's option (b) is ruled out on this evidence,
not assumed).**

**Privilege requirement.** `docs/evidence/g2/PRIVILEGE_REQUIREMENT_INVENTORY.md`
line 41 (accepted G2 evidence, VM-verified): "PSI (`/proc/pressure/*`) |
no privilege | World-readable on stock Ubuntu 26.04.1 for the
'some'/'full' pressure lines; the trigger-registration mechanism
(`poll()` on an fd from `open()` with a written trigger spec) is also
unprivileged for unprivileged monitors, per kernel PSI docs... No
capability needed." **This means the blocker is exclusively the
`ProcSubset=pid` sandboxing directive, not DAC permissions or Linux
capabilities** — a live PSI reader needs no root, no capability, and no
polkit authorization; it only needs to run in a process/mount namespace
that has not opted into `ProcSubset=pid`.

---

## 5. Bounded architecture options evaluated

Per this task's constraints: no generic privileged broker, no arbitrary
file reader, no shell execution, no broad daemon `/proc` access, no
client-side PSI producer. All four options below satisfy those
constraints; they differ in trust-boundary shape.

### (a) New, narrow, unprivileged, separate systemd unit dedicated to PSI

- **Mechanism**: a new small binary (e.g. `guardian-psi-monitor`) reusing
  the existing, complete, already-accepted `PsiEventSource`/`PsiTrigger`
  library code unmodified (`crates/guardian-core/src/providers/psi.rs`),
  running under its own systemd unit and its own dedicated unprivileged
  service user.
- **Process that reads PSI**: the new unit — neither `guardian-daemon`
  nor `guardian-helper`.
- **Privilege required**: none (§4). Full hardening directive set
  identical to `guardian-daemon`'s current unit **except** `ProcSubset=`
  must be `all` (or omitted) for this one new unit only; `ProtectProc=
  invisible` can remain (it governs visibility of other processes'
  `/proc/<pid>`, unrelated to `/proc/pressure`).
- **D-Bus/IPC boundary**: none exists today. This option requires
  inventing one — either (i) a new D-Bus interface/method for the new
  unit to report crossings to `guardian-daemon`, which raises whether
  `P2-API-002`'s freeze ("no new `Guardian1`/`Capabilities1`/
  `Incidents1`/`Transactions1` method/interface/object path... unless a
  future §51 revision explicitly approves one") extends to a
  *fifth*, new-named interface — an ambiguous reading this preflight
  does not resolve unilaterally — or (ii) a local Unix domain socket,
  which has zero precedent anywhere in this codebase (`grep -rn
  "UnixListener\|UnixStream" crates/` returns nothing).
- **Trust implications**: genuinely new. Whatever reports crossings into
  `guardian-daemon`'s `CorrelationIngress` must be authenticated (at
  minimum, peer-UID-pinned to the new unit's dedicated service user) or
  any local unprivileged process could forge PSI `Critical` events and
  open false incidents — a real event-injection attack surface never
  analyzed in G2's Privilege Requirement Inventory (which covered
  read/write capability needs, not event-provenance trust).
- **Sandbox implications**: `guardian-daemon`'s and `guardian-helper`'s
  existing accepted hardening is completely untouched (satisfies step 5
  cleanly). The new unit's own hardening is new surface, but narrow and
  unprivileged.
- **Failure behavior**: if the new unit crashes, PSI correlation stops
  silently; `guardian-daemon`/`guardian-helper` are unaffected. Needs an
  explicit decision on whether a producer-down condition should itself
  be observable (out of scope to decide here).
- **Restart behavior**: `Restart=on-failure` like the existing units;
  fresh trigger registration and a fresh IPC connection on every
  restart; no persistence, consistent with Phase 2's memory-only model.
- **Testability**: Layer 1 (existing `psi.rs`/`PsiEventSource` tests)
  unaffected. Needs a new Layer 2 test technique for the IPC boundary
  (none exists yet) and Layer 4 VM evidence for real cross-process
  delivery.
- **Scope impact**: **large**. New binary, new packaging unit, new
  service user, new IPC/D-Bus surface, and — if a new D-Bus interface is
  the chosen IPC — a §51 revision to explicitly approve it. This is
  materially more than "wire an existing library capability into the
  existing process"; it is closer to a new component design warranting
  its own gate sequence.

### (b) Narrower `ProcSubset`/`ProtectProc` carve-out specific to `/proc/pressure`

**Ruled out**, on real evidence, not assumption (§4): `man systemd.exec`
on this host (systemd 259) documents only `ProcSubset=all|pid` — no
path-scoped or PSI-specific value exists in this systemd version. Not a
viable mechanism on the current platform.

### (c) Weaken `guardian-daemon`'s own `ProcSubset=pid` to read PSI in-process

- **Mechanism**: reuse the existing, complete `PsiEventSource` directly
  inside `guardian-daemon`'s own `main()`, exactly the way
  `monitoring_tick`/`capability_registry_tick` already feed `admit_event`
  — no new process, no new IPC, no new D-Bus surface.
- **Process that reads PSI**: `guardian-daemon` itself.
- **Privilege required**: none beyond what `guardian-daemon` already has
  (§4).
- **D-Bus/IPC boundary**: none new — the smallest possible change,
  reusing the exact `admit_event` single-admission-point pattern Gate 2b
  already built for the other two producers.
- **Trust implications**: none new — no new external caller.
- **Sandbox implications**: **requires explicitly dropping
  `guardian-daemon`'s accepted `ProcSubset=pid` to `all` (or omitting
  it)** — this is precisely the change step 5 forbids making
  unilaterally in this task. It re-exposes every non-PID top-level
  `/proc` entry to `guardian-daemon`'s mount namespace (e.g. `/proc/sys`,
  `/proc/net`, `/proc/config.gz`), not just `/proc/pressure` — `ProcSubset`
  has no finer grain (§4/(b)). The other `Protect*` directives
  (`ProtectKernelTunables=`, `ProtectKernelModules=`, `ProtectKernelLogs=`,
  `ProtectControlGroups=`) remain in force and continue blocking *writes*
  to their respective trees regardless of `ProcSubset`, which bounds the
  practical severity of the widening somewhat — but the read-visibility
  widening itself is real and measurable, not cosmetic.
- **Failure behavior**: identical to `guardian-daemon` today (no new
  process).
- **Restart behavior**: identical to today.
- **Testability**: reuses G8's accepted `PsiEventSource` tests directly;
  daemon-wiring shape mirrors Gate 2b's already-accepted provider-health
  producer pattern closely.
- **Scope impact**: **small** relative to (a) — no new binary, unit,
  service user, or IPC surface. The cost is concentrated entirely in the
  one explicit, reviewable hardening exception step 5 requires be
  decided outside this task, not distributed across new infrastructure.

### (d) Bind-mount-specific carve-out (`BindReadOnlyPaths=/proc/pressure`) — unverified, not recommended without VM testing

Raised for completeness, not evaluated as confirmed viable: systemd's
`BindReadOnlyPaths=` can, in some configurations, bind a host path into a
unit's mount namespace even under other confinement directives. Whether
this can resurrect visibility of a `/proc` entry hidden by that same
unit's own `ProcSubset=pid`-restricted procfs instance is **not
confirmed** by the `man systemd.exec` text read for this preflight (which
describes `ProcSubset` as controlling "the procfs instance for the unit,"
suggesting a bind-mount sourced from the host's real `/proc` might layer
on top of — or might conflict with — the unit's own restricted instance,
depending on mount-namespace ordering this preflight did not empirically
test). If a real architecture decision leans toward preserving
`guardian-daemon`'s current `ProcSubset=pid` unchanged while still
reading PSI in-process, this option is worth a real VM test before being
ruled in or out — it is listed here so it is not silently lost, not
because it is recommended.

---

## 6. Recommendation (not a decision)

Option (b) is ruled out on platform evidence. Between (a) and (c), this
preflight does not select a winner — both are genuinely viable and
differ in kind, not merely in effort:

- **(a)** costs more to build (new binary/unit/IPC/§51 revision) but
  touches zero accepted hardening and is more consistent with this
  project's own established privilege-separation precedent (ADR-002's
  Model B: keep narrow, single-purpose components separate from the two
  existing hardened processes rather than widening either one's
  exposure).
- **(c)** costs less to build and reuses Gate 2b's exact wiring pattern,
  but requires a deliberate, reviewed exception to `guardian-daemon`'s
  own accepted `ProcSubset=pid` — smaller in scope, but the kind of
  privilege-boundary change AGENTS.md and this task both say must not be
  made inside an implementation pass without independent sign-off.

(d) is worth a real, cheap VM test before the decision is finalized,
since a positive result would let (c)'s simplicity coexist with (a)'s
"touch nothing accepted" property.

**This preflight's own recommendation, offered for the deciding
reviewer's consideration, not as a resolution**: test (d) first (cheap,
falsifiable, on `guardian-g9`); if it works, it dominates both (a) and
(c). If it does not work, prefer (a) over (c) — PSI needing zero
privilege (§4) makes (a)'s "new unprivileged unit" cost mostly
packaging/IPC-design effort rather than new privilege exposure, whereas
(c)'s cost is a direct, if bounded, widening of an already-accepted
security boundary that this project has treated as a first-class,
independently-reviewed decision at every prior gate (G2, G7, G9) that
touched it.

---

## 7. Disposition

**PHASE 2 PSI REPAIR — ARCHITECTURAL DECISION REQUIRED.** No production
code is changed by this file. No gate manifest is created. Gate 2a, Gate
2b, and Gate 2c remain exactly as accepted; none is reopened. Phase 2 is
not closed by this file and this file does not close it. A future,
separately-assigned implementation pass may proceed only after a human
decision selects among §5's options (or a genuinely different one this
preflight did not identify) and that decision is recorded as a §51
revision or an ADR, per this project's established governance order.

---

## 8. Addendum (2026-09-07) — Option (d) empirically tested, resolved NOT VIABLE

**Provenance note (reconciliation pass, 2026-09-07):** §8.1–§8.10 below
and §8.11 below are two independently-run disposable-VM reproductions of
the same option-(d) experiment, produced concurrently by two separate
agents against this same file — §8.1–§8.10 by a Claude-run session
(`guardian-g9`, the VM used throughout this project's prior gates), and
§8.11 by a Codex-run session (a freshly-launched, dedicated VM,
`guardian-psi-option-d`, deliberately not reusing `guardian-g9` or any
prior VM state). Both are preserved in full below as separate,
independently-sourced evidence — neither supersedes or is collapsed into
the other. **Shared conclusion, reached independently by both:** the
literal option (d) (`BindReadOnlyPaths=/proc/pressure`, same source and
destination) fails to start the unit at all under `ProcSubset=pid`
(`226/NAMESPACE`, no `pressure` node exists in a `subset=pid` procfs
mount for systemd to bind onto); and even the nearest working relaxation
that preserves `ProcSubset=pid` and successfully exposes read-only PSI
content cannot support the real kernel PSI trigger-registration ABI,
which requires an `O_RDWR` open that a read-only bind mount rejects with
`EROFS` before any write or `poll()` is reached. Two independent VMs,
two independent harnesses (a disposable Cargo project vs. the existing
`g8_psi_trigger_evidence` example binary), one identical failure
boundary. This section supersedes nothing above — §5's option (d) entry, §6's
recommendation to "test (d) first," and §7's disposition all stand
unmodified. This addendum reports the disposable-VM experiment §6
recommended, closing out (d) as an open question. No gate is opened by
this addendum, no `owned_normative_ids` are assigned, and no production
code (repo or VM-installed unit) was modified — every mount/bind/build
described below happened in a disposable, VM-local copy of the sandbox
context, using `guardian-g9` (the same disposable multipass VM used
throughout this project's prior gates). The real repo's
`debian/guardian-daemon.service` and Gate 2c's files were not touched.

### 8.1 Accepted hardening under test (quoted, unmodified)

`debian/guardian-daemon.service` (host repo, confirmed byte-identical to
the unit installed and running in `guardian-g9` via
`systemctl cat guardian-daemon.service`):

```
User=guardiand
Group=guardiand
NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ReadWritePaths=/var/lib/guardian/daemon
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
ProcSubset=pid
PrivateNetwork=yes
PrivateUsers=yes
UMask=0077
```

Every experiment below reproduces this exact directive set via
`systemd-run --pipe --wait --collect -p <directive>=<value> ...`, adding
only the one directive under test, never touching `debian/*.service` in
either the host repo or the VM's installed copy.

### 8.2 Experiment 1 — control (baseline reproduced)

`systemd-run` with the directive set above, unmodified, executing
`cat /proc/pressure/{cpu,memory,io}`:

```
cat: /proc/pressure/cpu: No such file or directory      (exit=1)
cat: /proc/pressure/memory: No such file or directory   (exit=1)
cat: /proc/pressure/io: No such file or directory        (exit=1)
```

All three paths are **absent** (`ENOENT`), not merely permission-denied
— confirms the control baseline exactly as predicted.

### 8.3 Experiment 2 — narrow bind, literal form (`BindReadOnlyPaths=/proc/pressure`, same source/destination)

Adding exactly `--property=BindReadOnlyPaths=/proc/pressure` (source and
destination both `/proc/pressure`, per the task's literal specification)
on top of the unmodified directive set above: the unit **never starts**.

```
Running as unit: run-p683826-i678931.service
Finished with result: exit-code
Main processes terminated with: code=exited, status=226/NAMESPACE
```

`journalctl` for that unit:

```
Failed to create destination mount point node '/run/systemd/mount-rootfs/proc/pressure', ignoring: No such file or directory
Failed to mount /proc/pressure to /run/systemd/mount-rootfs/proc/pressure: No such file or directory
Failed to set up mount namespacing: /proc/pressure: No such file or directory
Failed at step NAMESPACE spawning /usr/bin/bash: No such file or directory
```

**Root cause, independently confirmed** (not inferred from the error
text alone): `ProcSubset=pid` makes systemd mount the unit's private
`/proc` as `mount -t proc -o subset=pid`. Reproduced directly outside any
unit, as root in the VM:

```
$ mount -t proc -o subset=pid proc /mnt/test-proc-subset
$ ls /mnt/test-proc-subset/pressure
ls: cannot access '/mnt/test-proc-subset/pressure': No such file or directory
```

A `subset=pid` procfs mount contains **no `pressure` directory node at
all** — this is a kernel property of the `subset=pid` mount option
itself, not a systemd permission layer. `BindReadOnlyPaths=/proc/pressure`
(same-path form) therefore cannot succeed under `ProcSubset=pid`: there
is no mount point in the target namespace's own `/proc` for systemd to
bind onto. This is a hard, mechanistic incompatibility, not a
configuration-ordering bug that a different flag combination could route
around.

**Literal option (d), as specified in §5, is confirmed NOT VIABLE by
direct empirical test.**

### 8.4 Secondary probe — relocated-destination bind (not option (d) as specified; investigated for completeness)

To characterize the failure precisely (and check for a nearby variant
worth recording, per §6's spirit of testing (d) before ruling it out),
two isolating variants were run:

- **Variant A** — same-path bind (`/proc/pressure` → `/proc/pressure`)
  with `ProcSubset=pid` **removed** (all other directives unchanged):
  succeeds, `cat /proc/pressure/cpu` returns real PSI data. Confirms the
  failure in §8.3 is specifically attributable to `ProcSubset=pid`, not
  to `BindReadOnlyPaths` in general or to any other directive.
- **Variant B** — full directive set **including `ProcSubset=pid`**
  unchanged, but the bind targets a destination outside `/proc`
  (`BindReadOnlyPaths=/proc/pressure:/run/psi-pressure`): the unit
  starts successfully, and `cat /run/psi-pressure/cpu` returns real PSI
  data (`some avg10=0.00 avg60=0.00 avg300=0.00 total=...`).

Variant B is **not** option (d) as §5 specified it (a different
mountpoint, not `/proc/pressure` itself) and is not proposed here as a
substitute — it is reported because it isolates the failure mode and
because Experiments 3–5 needed *some* reachable PSI surface under intact
`ProcSubset=pid` to test the write-protocol question meaningfully rather
than stopping at "the literal form doesn't mount."

### 8.5 Experiment 3 — sandbox-broadening check (run against Variant B, since literal (d) never boots)

Inside the Variant B sandbox (`ProcSubset=pid` intact, relocated bind
present), each probed path:

| Path | Result |
|---|---|
| `/proc/pressure/cpu` (literal, unbound in this variant) | `No such file or directory` (still invisible) |
| `/proc/sys/kernel/hostname` | `No such file or directory` (still invisible) |
| `/proc/1/status` (unrelated real PID) | `No such file or directory` (still invisible) |
| `/proc/meminfo` (non-PID top-level) | `No such file or directory` (still invisible) |
| `/proc/self/status` (own PID-associated) | Visible, e.g. `Name: cat`, `State: R (running)` — unchanged from control |
| `/run/psi-pressure/cpu` (the relocated bind) | Real PSI data, readable |

`findmnt -o TARGET,SOURCE,FSTYPE,OPTIONS /run/psi-pressure`:

```
TARGET            SOURCE          FSTYPE OPTIONS
/run/psi-pressure proc[/pressure] proc   ro,nosuid,nodev,noexec,relatime
```

**Confirmed: the relocated bind does not broaden the sandbox anywhere.**
`ProcSubset=pid`'s restriction remains in force everywhere except the
one new, deliberately narrow, read-only mountpoint. (Literal option (d)
has no analogous "after" state to compare, since its unit never reaches
running — §8.3.)

### 8.6 Experiment 4 — write denial / read-vs-write-trigger distinction

Against the same Variant B mount:

```
$ echo "some 10000 1000000" > /run/psi-pressure/cpu
bash: /run/psi-pressure/cpu: Read-only file system

$ python3 -c 'import os; os.open("/run/psi-pressure/cpu", os.O_RDWR)'
OSError: [Errno 30] Read-only file system: '/run/psi-pressure/cpu'
```

The `open(O_RDWR)` call itself — not just a subsequent `write()` —
fails with `EROFS`. This is the exact call `PsiTrigger::register` makes
(`crates/guardian-core/src/providers/psi.rs`: `open(path, OFlags::RDWR |
OFlags::CLOEXEC, Mode::empty())`). **Flagged critical finding, exactly
the concern this task's Experiment 4 anticipated**: `BindReadOnlyPaths`
makes the *mount* read-only at the VFS/open-mode level, which is a
strictly stronger restriction than "no writes succeed" — it blocks the
kernel PSI monitor-registration protocol's required `O_RDWR` open
entirely, before any write is even attempted. Plain `cat` reads (`O_RDONLY`)
are unaffected; the real PSI trigger ABI's `O_RDWR` open is not.

### 8.7 Experiment 5 — real `PsiEventSource` code, run inside the sandbox

Per the task's requirement to exercise the actual production code (not
an approximation): a disposable, VM-local Cargo project
(`/home/ubuntu/psi-harness`, never added to the Guardian repo, never
committed) was created with a single path dependency on
`crates/guardian-core` **read, not modified** — calling
`PsiEventSource::register(...)` then `wait_for_event(...)` exactly as
`psi.rs` defines them, with the same VM-checked-out source used to build
the real `guardian-daemon` binary already installed in `guardian-g9`.
Built with `cargo build --release` (network + cached registry available
in the VM), copied to `/opt/psi-harness/psi-harness` (outside `/home`,
since `ProtectHome=yes` hides `/home` from the sandboxed context — this
is why the binary could not be run in place; nothing about the hardening
itself was changed to work around this, only the binary's location).

Run inside the full Variant B sandbox (`ProcSubset=pid` intact, all
other directives from §8.1 unchanged):

```
[harness] using PSI base dir: /run/psi-pressure
[harness] calling PsiEventSource::register (open O_RDWR + write kernel ABI trigger line)...
[harness] REGISTER FAILED: PSI trigger registration failed: Read-only file system (os error 30)
[harness] debug: Trigger(TriggerError(Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }))
```

Repeated under a bounded 6-second `stress-ng --cpu 2` load running
concurrently on the host (to rule out "no pressure event was even
possible" as a confound) — identical result, same `EROFS` failure at
registration, before any `poll()` is reached. Also run against the
literal `/proc/pressure` path (unbound, `ProcSubset=pid` intact):

```
[harness] using PSI base dir: /proc/pressure
[harness] REGISTER FAILED: PSI source unavailable
```

— i.e. `PsiFileSource::read` correctly reports `Unavailable` (`ENOENT`),
exactly as `psi.rs`'s own contract states (`P1-PSI-005`), confirming the
harness and the real code behave exactly as documented in both failure
modes.

**Result: the real, unmodified `PsiEventSource::register` fails under
every sandbox configuration tested that keeps `ProcSubset=pid` intact** —
either because the PSI path is invisible at all (literal form, §8.3), or
because it is visible but the mandatory `O_RDWR` trigger-registration
open is rejected by the read-only bind (`EROFS`, this section). No
configuration was found, under `ProcSubset=pid`, in which the real
production PSI event path completes. `poll(POLLPRI)` and a real wake were
never reached in any sandboxed run — registration failed first in every
case.

### 8.8 Experiment 6 — ingress feasibility judgment (unaffected by this addendum)

Not re-evaluated here: Experiment 6 in the task's framing was
conditioned on Experiment 5 succeeding, which it did not. The
daemon-wiring shape §5(c) already describes (reuse `PsiEventSource`
directly inside `guardian-daemon::main()`, feeding `admit_event` exactly
as `monitoring_tick`/`capability_registry_tick` already do) remains the
only concrete wiring path evaluated anywhere in this file, and it
requires `ProcSubset=pid` to be relaxed on `guardian-daemon` itself — the
outcome this addendum's results make unavoidable if a live PSI producer
must run inside an existing hardened unit under `ProcSubset=pid`. This
addendum does not select that outcome; it only closes off (d) as an
alternative that avoids it.

### 8.9 Security comparison table (observed values)

| Property / capability | Current sandbox | Option (d), literal (same-path bind) | Option (d), best-case variant tested (relocated bind) |
|---|---|---|---|
| `ProtectProc` | `invisible` | `invisible` (unit never starts) | `invisible` (unchanged) |
| `ProcSubset` | `pid` | `pid` (unit never starts — incompatible with the bind as specified) | `pid` (unchanged) |
| `/proc/pressure` visibility | invisible (`ENOENT`) | unit fails to start (`226/NAMESPACE`) — never resolves | still invisible at `/proc/pressure` itself; PSI content readable only at a relocated, non-standard path (`/run/psi-pressure`) |
| unrelated `/proc` visibility | hidden except `/proc/self/*` | n/a (unit never starts) | unchanged — confirmed still hidden (§8.5) |
| service UID | `guardiand` | `guardiand` (unchanged in every variant tested) | `guardiand` |
| capabilities | `CapabilityBoundingSet=`, `AmbientCapabilities=` (empty) | unchanged (empty) | unchanged (empty) |
| `NoNewPrivileges` | `yes` | unchanged | unchanged |
| new IPC boundary | none | none (moot — never starts) | none |
| privileged process | no | no (moot — never starts) | no, but cannot complete the real PSI trigger-registration protocol (`EROFS` on the mandatory `O_RDWR` open, §8.6–8.7) |

### 8.10 Verdict

```
OPTION D NOT VIABLE — ProcSubset=pid PREVENTS NARROW BIND
```

This is a compound, doubly-confirmed result, not a single data point:

1. Option (d) exactly as specified (`BindReadOnlyPaths=/proc/pressure`,
   same source/destination) fails to even start the unit under
   `ProcSubset=pid` — root-caused to the kernel's `subset=pid` procfs
   mount containing no `pressure` node to bind onto (§8.3).
2. The nearest working variant that preserves `ProcSubset=pid` (a
   relocated bind destination) does expose read-only PSI content without
   broadening the sandbox anywhere else (§8.5) — but the real,
   unmodified `PsiEventSource::register` code still fails against it,
   because the kernel PSI monitor-registration ABI's required `O_RDWR`
   open is rejected by the read-only bind mount (`EROFS`) before any
   write or `poll()` is reached (§8.6–8.7).

Both findings independently rule out (d) (in its literal form, and in
the closest viable-looking relaxation of it) as a way to give
`guardian-daemon` a live, poll-driven PSI producer while leaving
`ProcSubset=pid` untouched. Between the two remaining §5 options — (a)
new narrow unit, or (c) relax `guardian-daemon`'s own `ProcSubset=pid` —
this addendum expresses no preference beyond what §6 already recorded;
it only removes (d) from consideration. The architectural decision
called for in §7 remains open and remains a human decision, not one this
task or this addendum makes.

No production code, no `debian/*.service` file (host repo or VM), and no
Gate 2c file were modified in the course of this addendum. All
mount/bind experiments ran against disposable `systemd-run` transient
units in `guardian-g9`; the one disposable Cargo project used for
Experiment 5 (`/home/ubuntu/psi-harness`) lives outside the Guardian
repo checkout and was never committed or merged into it.

### 8.11 Independent reproduction on a separate clean VM

The literal option-(d) result was independently reproduced on a newly
launched, dedicated multipass VM named `guardian-psi-option-d`, rather
than reusing `guardian-g9` or any Gate 2c state:

```text
VM image:       Ubuntu 26.04 LTS
kernel:         7.0.0-30-generic
systemd:        259 (259.5-0ubuntu3.4)
repository:     4d5df972a4fde6ae788f13e0aac68c38e295a411
Guardian binary SHA-256:
eb561efeeb3494a4c48efd8564cbb95c78ea2000e6d0239992e7451b017c0955
```

The control transient unit used the complete accepted daemon sandbox.
It ran as `uid=999(guardiand)`, reported zero permitted/effective/
bounding/ambient capabilities and `NoNewPrivs: 1`, and its procfs
mount showed `hidepid=invisible,subset=pid`. `/proc/pressure`,
`/proc/sys`, `/proc/net`, `/proc/meminfo`, `/proc/uptime`, and the other
sampled non-PID procfs surfaces were all absent; `/proc/self/status`
remained readable and `/proc/1/status` remained absent.

Adding only `BindReadOnlyPaths=/proc/pressure` reproduced the exact
pre-`ExecStart` failure:

```text
status=226/NAMESPACE
Failed to create destination mount point node
'/run/systemd/mount-rootfs/proc/pressure': No such file or directory
Failed to set up mount namespacing: /proc/pressure: No such file or directory
```

As an isolating diagnostic only, changing `ProcSubset` to `all` while
leaving the other directives and the read-only bind intact made all
three PSI files readable and showed `/proc/pressure` as a distinct `ro`
mount. This confirms the literal failure is the interaction with
`subset=pid`, not a missing source path or malformed bind directive.
The real, unmodified `g8_psi_trigger_evidence` executable was then run
inside that diagnostic sandbox and failed at
`PsiEventSource::register` with:

```text
TriggerError: Read-only file system (os error 30)
```

Thus the independent run confirmed both failure boundaries recorded
above: the literal bind cannot be installed over the PID-only procfs,
and a successfully installed read-only bind cannot support the kernel
PSI ABI's mandatory `O_RDWR` trigger registration. Per the experiment's
stop condition, no stress workload, `poll(POLLPRI)` wake, or ingress
claim was attempted after registration failed. The dedicated VM did not
read or modify Gate 2c state, and no production Rust or service unit was
changed.

---

## 9. Addendum (governance-repair pass, 2026-09-07) — `OpenFile=` proven viable; architecture reselected

**This section supersedes nothing above.** §1–§7 stand exactly as
written; §8's two independent option-(d) VM reproductions (§8.1–§8.10,
Claude-run on `guardian-g9`; §8.11, Codex-run on the dedicated
`guardian-psi-option-d` VM) stand exactly as written, including the
§8 provenance note distinguishing them. Option (b) and option (d) remain
REJECTED on precisely the evidence §8 records, and option (c) remains
NOT SELECTED. What this section adds is (i) two independent architecture
reviews that FAILED §3a's option-(a) selection, and (ii) a fifth
mechanism — systemd `OpenFile=` — that §5 never identified, empirically
proven to work, which reselects the architecture and dissolves both FAIL
findings structurally.

**Honest narrative, recorded so it is not rewritten later:** option (a)
was genuinely selected, on the evidence available at the time (§3a); it
was then independently reviewed and failed on two grounds; and only
*afterwards* was `OpenFile=` identified and proven. `OpenFile=` was not
"always the answer" and this file must never be read as implying it was.

### 9.1 Two independent architecture reviews FAILED option (a) as selected

Both findings are recorded here in substance, as the reason the ADR-009
decision changed — not as defects to be mitigated inside option (a).

1. **PSI message authority too broad.**
   `crates/guardian-core/src/correlation.rs`'s `classify()` gates
   incident admission solely on `Event.severity`. ADR-009's option-(a)
   design populated that field from the producer's own self-reported
   `to_severity` (the `PressureCrossing` signal's `to_severity` field,
   architecture handoff §3.2/§3.3), with **no daemon-side verification
   against the raw measurement**. A compromised or buggy `guardian-psi`
   could therefore fabricate unlimited `Critical` incidents. The
   architecture handoff's own security argument (§9, "guardian-psi cannot
   submit arbitrary Guardian events") bounded the *vocabulary* the
   producer could speak, but not the *authority* of the one field that
   actually gates incident admission.

2. **The proposed new D-Bus interface conflicts with higher-order Phase 2
   §51 language unless §51 is explicitly amended.** ADR-009's Decision
   argued `P2-API-002`'s freeze is a closed enumeration of four named
   client interfaces and therefore does not reach a fifth, differently
   named, differently owned bus name. The two reviewers **disagreed** on
   whether `P2-API-002` is such a closed enumeration or instead forbids
   any new bus name absent a §51 revision. *This disagreement is now
   moot* under the reselected architecture — no new D-Bus interface is
   introduced at all — and is recorded here as history only. **It is
   deliberately not adjudicated by this file**, and nothing below depends
   on which reading is correct.

### 9.2 The mechanism: systemd `OpenFile=`

All findings in §9.2–§9.9 were established empirically on `guardian-g9`
(Ubuntu 26.04, systemd 259, kernel 7.0.0-30-generic) — the same
disposable multipass VM used throughout this project's prior gates.

- systemd opens `OpenFile=` paths **in PID 1's own namespace** and passes
  the resulting descriptors into the service. This is confirmed
  mechanistically, not inferred: the inherited descriptor's
  `/proc/self/fdinfo/N` reports an `mnt_id` matching PID 1's host `proc
  rw` mount, while the service's own `/proc/self/mountinfo` shows
  `proc rw,hidepid=invisible,subset=pid`. The descriptor therefore refers
  to a file the service's own procfs instance does not contain.
- `OpenFile=path[:fd-name:options]` is documented in **`systemd.service(5)`**
  — not `systemd.exec(5)`, which is where this preflight's earlier
  `ProcSubset=` research looked and is why §5 never identified it. Its
  **default open mode is `rw`** (verified against the man page text, not
  assumed). Its documented purpose is verbatim this use case: "to allow
  services to access files/sockets that they cannot access themselves
  (due to running in a separate mount namespace…)".
- Descriptors arrive through the standard listen-fd protocol:
  `LISTEN_FDS` / `LISTEN_FDNAMES` / `LISTEN_PID`, starting at **fd 3**, in
  declaration order, opened `O_RDWR`, and **without `FD_CLOEXEC`**.

### 9.3 Descriptor count, derived from real code (not assumed)

Reading production source rather than reasoning from the file layout:

- `PsiTrigger::register` (`crates/guardian-core/src/providers/psi.rs`,
  lines 114–133) performs exactly **one** `open(O_RDWR|CLOEXEC)` and
  writes exactly one trigger line. One `PsiTrigger` = one descriptor.
- `PsiEventSource` (`providers/psi.rs`, lines 160–163) holds exactly one
  `PsiTrigger` and one `PsiEventDispatcher`; the dispatcher
  (`providers/psi.rs`, lines 168–174) is bound to a single
  `PsiResourceKind`.
- Only the `some` class is ever classified: `present_severity`
  (`providers/psi.rs`, lines 294–303) calls `classify(resource.some, …)`.
  The `full` line is parsed by the G5 model but **never classified**, so
  **no `(resource, full)` descriptor is needed**. The naive
  per-`(resource, class)` reading that yields six descriptors is
  therefore **wrong**.

**Binding count: one descriptor per monitored resource — 1 for the
accepted G8 shape (CPU only), at most 3 for `cpu`+`memory`+`io`.**

### 9.4 Kernel ABI: PSI triggers are per-open-file-description

Proven empirically, twice: a second trigger write to the *same* open file
description returns `EBUSY`; a trigger written to a *separate* description
of the same file succeeds. This is a property of the open file
description, not of the inode.

### 9.5 A non-obvious requirement, found in the code and then confirmed

`PsiEventDispatcher::dispatch_wake` re-reads the pressure text through
`PsiFileSource::read` → `fs::read_to_string` **by pathname**
(`providers/psi.rs`, lines 68–72), which returns `Unavailable` under
`ProcSubset=pid`. Inheriting only a *trigger* descriptor would therefore
have produced a registered trigger that could never classify its own
wake — a silent, complete failure of the production path that no
trigger-only test would have caught.

Tested and confirmed: **the same descriptor serves both roles.**
`lseek(0)+read` and `pread` both return live PSI text on the inherited
descriptor, before and after trigger registration. One descriptor per
resource covers trigger registration *and* content re-read.

### 9.6 Real, unmodified `PsiEventSource` ran the complete path inside the accepted sandbox

Inside the `OpenFile=`-equipped unit carrying the full accepted daemon
directive set (§8.1), the real, unmodified library code ran end to end:
`PsiFileSource::read` → `PsiEventSource::register` → `poll(POLLPRI)` real
wake under a bounded `stress-ng` load → a real Guardian `Event`:

```text
event_type:     "psi_threshold_crossing"
resource_refs:  ["/proc/pressure/cpu"]
severity:       Moderate
normalized_key: "psi cpu threshold crossing nominal->elevated"
```

**Honest qualification, preserved deliberately:** because the current
`PsiFileSource`/`PsiTrigger` API accepts only a *path*, that Rust run
**reopened** the inherited descriptor via `/proc/self/fd/N` rather than
polling the inherited descriptor directly. Writing the trigger to the
inherited descriptor *itself* and polling that descriptor was proven
separately, with a direct syscall harness. This distinction is recorded
faithfully and must not be overstated: the Rust run proves the full
classification/`Event` path works inside the sandbox; the syscall harness
proves the inherited descriptor itself is trigger-capable and pollable.

### 9.7 The sandbox is genuinely unweakened

Inside the `OpenFile=`-equipped unit:

| Path | Result |
|---|---|
| `/proc/pressure` and all three files | `ENOENT` (still invisible **by pathname**) |
| `/proc/sys/kernel/hostname` | `ENOENT` |
| `/proc/meminfo` | `ENOENT` |
| `/proc/uptime` | `ENOENT` |
| `/proc/net` | `ENOENT` |
| `/proc/1/status` | `ENOENT` |
| `/proc/1/cmdline` | `ENOENT` |
| `/proc/self/status` | visible, unchanged from control |
| `/proc/self/fd` | visible, unchanged from control |

`systemd-analyze security` on the accepted directive set **plus three
`OpenFile=` lines**, compared against the live `guardian-daemon.service`:
the tables **diff clean apart from the unit name**, and both score
**`0.6 SAFE`**.

No mount is added, no path is relocated, and no directive is relaxed.
Unlike option (d)'s best case (§8.4's relocated bind, which exposed PSI
content at a non-standard path), `/proc/pressure` stays invisible **by
pathname** even to the very process holding working descriptors to it.

### 9.8 Restart semantics

Each restart receives a **fresh open file description**. This is proven,
not assumed: trigger re-registration succeeds on every restart, where a
stale reused description would return `EBUSY` (§9.4). The previous
registration is destroyed when the old description closes.
`debian/guardian-daemon.service` never sets `FileDescriptorStoreMax`.

### 9.9 Six concrete defects found — all mitigable, none disqualifying

All six are recorded; none blocks the mechanism, and each carries a
mitigation an implementation gate must honor.

1. **Hard startup coupling.** A missing `OpenFile=` path aborts the unit
   with `status=202/FDS` **before `ExecStart` runs** — so on a PSI-less
   kernel or inside a container the whole daemon would fail to start.
   *Mitigation:* the `:graceful` option (verified to start cleanly with
   only the descriptors that are present), with the daemon treating a
   missing descriptor as a truthful `PsiReading::Unavailable` and never
   as "no pressure" — which `providers/psi.rs` already does today, per
   `P1-PSI-005`.
2. **`graceful` reorders the FD list.** Descriptors must therefore be
   resolved by their `LISTEN_FDNAMES` **name**, never by a fixed index.
3. **Descriptors arrive without `FD_CLOEXEC`.** Latent today (the daemon
   spawns no subprocesses), but any hand-rolled listen-fd parser must set
   it.
4. **FD-store staleness.** With `FileDescriptorStoreMax` enabled and PSI
   descriptors pushed into the store, a stale, already-triggered
   descriptor is returned *first* under a duplicated name, so a naive
   name lookup picks it and registration fails `EBUSY`. *Recommendation:*
   never enable the FD store on this unit, and treat `EBUSY` at
   registration as a hard error, never as a benign retry condition.
5. **No in-place re-arm.** Because triggers are per-description (§9.4)
   and re-registration on the same description is `EBUSY`, recovery
   requires reopening to obtain a fresh description.
6. **Avoid a shared-directory symlink farm.** `/proc/self/fd/N` symlinks
   resolve against the **reader's** fd table — a real footgun if such
   paths were materialized into a shared directory. Production must build
   those paths in-process.

### 9.10 API extension needed — small, no `unsafe`, no new dependency

The workspace sets `unsafe_code = "forbid"` (`Cargo.toml:30`), which
rules out `OwnedFd::from_raw_fd`. But `/proc/self/fd/N` is
process-associated and therefore stays visible under `ProcSubset=pid`, so
an inherited fd number becomes a `File` via plain, safe
`File::options().read(true).write(true).open(...)` — **verified working
inside the sandbox**.

Minimal library change: give `PsiFileSource` explicit per-resource path
resolution alongside its existing `base_dir` behaviour (e.g. a
`from_paths(...)` constructor, or a `BTreeMap<PsiResourceKind, PathBuf>`
consulted by `path_for`), leaving `real()`, `at()`, `read()`, `path()`,
`PsiTrigger::register`, `PsiEventSource`, `PsiEventDispatcher`, and all
seven existing unit tests working untouched. `guardian-daemon` then parses
`LISTEN_FDS`/`LISTEN_FDNAMES` (roughly ten lines of `std::env`, checking
`LISTEN_PID == std::process::id()`) and passes `/proc/self/fd/N` paths.

### 9.11 `BindPaths=` (writable, relocated) was not evaluated

The fallback branch was never entered, because `OpenFile=` succeeded.
Recorded as **not needed**. It would in any case be strictly worse than
`OpenFile=`: it adds a real mount and exposes PSI at a non-standard path,
where `OpenFile=` adds neither.

### 9.12 Reselected architecture

```
systemd (PID 1) → OpenFile= PSI FDs → guardian-daemon → providers::psi
  (unmodified logic) → daemon-owned classification → Guardian Event
  → existing admit_event ingress → correlation
```

No `guardian-psi` process. No new UID. No new D-Bus interface. No new IPC
protocol. No producer-controlled severity. No helper involvement. No
weakening of `ProcSubset=pid`. The only unit-file delta is up to three
additive `OpenFile=` lines on `debian/guardian-daemon.service`.

**This dissolves both §9.1 FAIL findings structurally rather than
mitigating them:** with no cross-process producer there is no
producer-supplied severity reaching `classify()` at all — the daemon
classifies from bytes it read itself — and there is no new D-Bus
interface to reconcile with §51, which is exactly why the two reviewers'
`P2-API-002` disagreement became moot.

The formal ADR record of this reselection is
`docs/adr/ADR-009-guardian-psi-producer-topology.md` (revised in place,
its original option-(a) decision and both FAIL findings preserved as
labelled superseded history). The implementation contract is
`docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`
and its linked `-tdd.md`.

---

## 10. Contract Collision — formally RESOLVED (governance-repair pass, 2026-09-07)

The collision recorded in §3's table is, restated exactly as §3a already
restates it:

> Phase 2 requires live PSI events to enter the permanent shared
> correlation ingress. §51 incorrectly assumed this was already provided
> by G8. Production `guardian-daemon` does not instantiate
> `PsiEventSource`. The obvious in-daemon repair conflicts with the
> accepted `ProcSubset=pid` service-hardening boundary.

**Status: RESOLVED.**

**Resolution mechanism:** systemd `OpenFile=` (§9). The requirement is
satisfied **without weakening the hardening boundary** — the two horns of
the collision turned out not to be mutually exclusive after all, on
evidence:

- *First horn satisfied.* `guardian-daemon` itself instantiates the live
  PSI event path and admits real PSI `Event`s through the existing shared
  `admit_event`/`CorrelationIngress` admission point — the single
  admission point Gate 2b built and Gate 2b's own R2 required every future
  PSI producer to feed through. No new ingress point is created.
- *Second horn satisfied.* `ProcSubset=pid`, `ProtectProc=invisible`, and
  every other accepted directive on `debian/guardian-daemon.service`
  remain in force, unchanged. `/proc/pressure` remains invisible **by
  pathname** inside the daemon's mount namespace (§9.7), verified against
  the same probe set §8.5/§8.11 used. `systemd-analyze security` scores
  identically (`0.6 SAFE`) with the three additive `OpenFile=` lines
  present. The descriptors are opened by PID 1 in PID 1's namespace and
  handed in; the daemon never gains the ability to *name* the path.

**What was superseded, and what was not.** §3's "Verdict: collision open,
not resolved" is preserved exactly and was correct for what was knowable
when it was written — the mechanism that resolves the collision
(`OpenFile=`, documented in `systemd.service(5)`) was not among §5's four
evaluated options and was not identified until after two architecture
reviews failed §3a's option-(a) selection (§9.1). §3a's own
"resolved at the architecture-selection level by option (a)" disposition
is superseded by §9.12's reselection; §3a's rejections of options (b) and
(d), and its non-selection of option (c), are **not** superseded and
stand on §4/§8's unchanged evidence. Gate 2b's earlier local disposition
(§2, preserved verbatim) is likewise untouched: it remains a defensible
in-gate scoping call that had no authority to resolve §51's higher-order
scope-inclusion question, exactly as §2 and §3a already record.

**Consequential governance changes made by the same pass that wrote this
section**, listed here so this file alone shows the full disposition:

- `GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §51 Decision item 3's false
  predicate ("already produced by G8's `providers::psi` wiring") is
  corrected in place, the same way §51 already corrected the structurally
  identical provider-health predicate — **corrected, never narrowed to a
  deferral**. PSI production wiring is recorded as *in scope for this
  phase* and *not in fact already provided*.
- `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §1's capability-table row
  and §3's primitive-inventory row (both claiming PSI is
  "production-wired since G8") are corrected in place using that
  document's existing "Corrected (repair pass)" convention.
- `docs/adr/ADR-009-guardian-psi-producer-topology.md`'s Decision is
  revised; its original option-(a) decision, that decision's rationale,
  and both architecture-review FAIL findings are preserved as clearly
  labelled superseded history.
- A gate manifest/TDD pair now exists for the implementation pass:
  `docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`
  / `-tdd.md`.

---

## 11. Disposition (updated, governance-repair pass, 2026-09-07)

> §7 above is preserved verbatim and remains the accurate record of this
> file's original disposition. This section updates it; it does not
> replace or erase it.

**PHASE 2 PSI REPAIR — CONTRACT COLLISION RESOLVED, IMPLEMENTATION GATE
DEFINED, NOT YET IMPLEMENTED.** No production code is changed by this
file or by the governance-repair pass that appended §9–§11: no `.rs`
file, no `debian/*.service` file, and no Gate 2c file was modified.
Gate 2a, Gate 2b, and Gate 2c remain exactly as accepted; none is
reopened. Phase 2 is not closed by this file.

§7's own precondition — "a future, separately-assigned implementation
pass may proceed only after a human decision selects among §5's options
(or a genuinely different one this preflight did not identify) and that
decision is recorded as a §51 revision or an ADR" — is met in both of its
branches: `OpenFile=` **is** "a genuinely different one this preflight
did not identify," and the decision is recorded in **both** a revised
ADR-009 **and** a §51 revision. The remaining human act is confirmation
of the normative-ID minting decision the implementation gate's manifest
records (see that manifest's `owned_normative_ids` and the §51 revision
history entry that mints them), which is a governance act reserved to the
project owner.
