---
title: "PSI Inherited-Descriptor Ingress Gate TDD — P2-EVT-005..008, P2-VM-003"
kind: "implementation-gate-tdd"
status: "active"
last_reviewed: "2026-09-07"
---
# PSI Inherited-Descriptor Ingress Gate TDD

Governing manifest:
`docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`.

Full context (not to be re-derived here): TDD contract §51 (decision item
3 and its "PSI production wiring, corrected" subsection — the correction
that mints this gate's IDs); `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md`
§4.1 (PSI → incident correlation and its corrected `resource_refs.first()`
key), §6 (ingress order), §19 (ID inventory);
`docs/adr/ADR-009-guardian-psi-producer-topology.md` **as revised
2026-09-07** — read its current Decision, not the superseded option-(a)
decision preserved inside it;
`docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md`
§9 (the `OpenFile=` experiment and all six recorded defects) and §10
(Contract Collision RESOLVED); `AGENTS.md` "Safety rules", "No
placeholders", "Privilege rules". This file states required behavior and
acceptance evidence only — it is not an implementation tutorial.

**This gate is not a repair of an existing ID.** It closes an ownership
gap: no `P2-*` ID and no gate manifest ever owned building a live PSI
production event path.

---

## The defect (why this gate exists)

G8 delivered a complete, correct, tested PSI **library** capability —
`PsiFileSource`, `PsiTrigger`, `PsiEventSource`, `PsiEventDispatcher` in
`crates/guardian-core/src/providers/psi.rs`: real `/proc/pressure` reads
through the accepted G5 model, a real kernel trigger written per the PSI
ABI, and a real `poll(POLLPRI)` wait. Seven unit tests cover it and all
pass.

**Production `guardian-daemon` never instantiates any of it.** `main()`
(`crates/guardian-daemon/src/bin/guardian-daemon.rs`) wires exactly two
producers to `admit_event`: `monitoring_tick` and
`capability_registry_tick`. Neither references PSI. The only
`PsiEventSource` construction site anywhere in the workspace is a
standalone example binary (`crates/guardian-core/examples/
g8_psi_trigger_evidence.rs`) that no systemd unit runs. No live PSI
`Event` has ever reached the shared production correlation ingress.

The defect survived three gates because TDD contract §51's decision item 3
asserted PSI events were "already produced by G8's `providers::psi`
wiring" — a false factual predicate of exactly the same shape as the
provider-health predicate §51's 2026-09-05 repair pass corrected. Because
the predicate read as already-satisfied, **no ID was ever minted for it**.

**The lesson this gate must encode in its own tests:** a capable library
is not a production instantiation. Every requirement below that says
"proven" means proven against the production wiring, not against the
library in isolation.

---

## Contract Collision Table (mandatory preflight — completed before any requirement below was written)

| Requirement | Owner | Module/path | Potential conflict | Contract resolution |
|---|---|---|---|---|
| Live PSI events must reach the shared production `CorrelationIngress` (§51 decision item 3's scope-inclusion clause). | **This gate** (`P2-EVT-005`), newly minted. Previously unowned — the collision the preflight recorded. | `crates/guardian-daemon/src/bin/guardian-daemon.rs` (`main`, `admit_event`); `crates/guardian-core/src/providers/psi.rs`. | Gate 2a owns the correlation logic that consumes PSI events; Gate 2b owns the single admission point; neither owns producing them. A producer built anywhere but the existing `admit_event` call site would create a second admission point, contradicting Gate 2b's R2. | **No collision — resolved.** The producer feeds the **existing** `admit_event(&ingress_clock, &engine, event)` call site, exactly as `monitoring_tick` and `capability_registry_tick` do. Gate 2b's R2 explicitly anticipated this ("a future gate that wires a live PSI producer must feed it through this same `IngressClock`/`CorrelationIngress` point"). No new admission point, no change to Gate 2a's engine. |
| Reading `/proc/pressure/*` from `guardian-daemon` must not weaken the accepted `ProcSubset=pid`/`ProtectProc=invisible` hardening (ADR-002/G7/G9). | G2/G7/G9 hardening — accepted, not owned by any Phase 2 gate. This gate must satisfy it, not amend it. | `debian/guardian-daemon.service`. | The obvious in-daemon repair (option (c)) requires relaxing `ProcSubset=` to `all`, which the execution protocol and `AGENTS.md` forbid deciding inside an implementation pass. | **No collision — resolved on real VM evidence.** systemd `OpenFile=` (`systemd.service(5)`) opens the paths in **PID 1's** namespace and passes descriptors in; `/proc/pressure` stays invisible **by pathname** inside the daemon's own namespace, every probed unrelated procfs path stays `ENOENT` as in the control, and `systemd-analyze security` scores identically (`0.6 SAFE`). Preflight §9.7. **No directive is relaxed**; the only delta is additive `OpenFile=` lines. |
| PSI severity must not be attackable — `CorrelationEngine::classify()` gates incident admission solely on `Event.severity`. | **This gate** (`P2-EVT-007`). | `crates/guardian-core/src/correlation.rs` (`classify`, forbidden scope — consumed as-is); `crates/guardian-daemon/**`. | A cross-process producer that self-reports `to_severity` would let a compromised or buggy producer fabricate unlimited `Critical` incidents. This was one of two independent architecture-review FAIL findings against the superseded ADR-009 design. | **Dissolved structurally, not mitigated.** There is no cross-process producer: the daemon classifies from raw PSI bytes it read itself through its own descriptor, via the accepted, unmodified G5 classifier. No component outside `guardian-daemon` can supply, influence, or self-report a severity. `classify()` is untouched. |
| `P2-API-002` — no new D-Bus method/interface/object path beyond G9, absent an explicit §51 revision. | Gate 2b (closed, regression-only here). | `crates/guardian-daemon/src/dbus_surface.rs`. | The superseded ADR-009 design proposed a new `GuardianPsi1` bus name, and **two independent reviewers disagreed** on whether `P2-API-002`'s enumeration is closed to four named client interfaces or forbids any new bus name. | **Moot — deliberately not adjudicated.** This gate introduces no D-Bus method, interface, object path, or bus name of any kind, so neither reading is engaged. No `P2-API-002` exception is requested, granted, or implied. A future gate that genuinely proposes a new bus name must resolve the question then, on its own merits. |
| `providers/psi.rs`'s dispatch path re-reads pressure text **by pathname** (`fs::read_to_string`, lines 68–72), which returns `Unavailable` under `ProcSubset=pid`. | **This gate** (`P2-EVT-006`), library-extension scope. | `crates/guardian-core/src/providers/psi.rs`. | Inheriting only a *trigger* descriptor would produce a registered trigger that can never classify its own wake — a silent, total failure of the production path that no trigger-only test would catch. | **No collision — resolved on evidence.** The **same** inherited descriptor serves both roles: `lseek(0)+read` and `pread` both return live PSI text before and after trigger registration (preflight §9.5). One descriptor per resource covers trigger *and* content re-read. The library extension gives `PsiFileSource` explicit per-resource path resolution so the daemon can hand it `/proc/self/fd/N` paths — process-associated and therefore visible under `ProcSubset=pid`, reachable with plain safe `File::options()`, no `unsafe` (workspace `unsafe_code = "forbid"`, `Cargo.toml:30`) and no new dependency. |

**Verdict: no STOP triggered.** Every requirement below is active, not
blocked. The one genuinely open item is a **governance** question, not a
contract collision — see "Normative ID decision" immediately below.

---

## Normative ID decision — REQUIRES PROJECT-OWNER CONFIRMATION

This gate's `owned_normative_ids` (`P2-EVT-005`, `P2-EVT-006`,
`P2-EVT-007`, `P2-EVT-008`, `P2-VM-003`) are **newly minted** by the TDD
contract §51 PSI production-wiring governance-repair pass (2026-09-07).
Minting a normative ID is a governance act, so the reasoning is recorded
here in full and flagged for explicit owner confirmation.

**The question.** No existing `P2-*` ID owns live PSI production wiring
(established by the preflight, §1: `phase2-2b-manifest.toml` owns
`P2-API-001..003` only; `phase2-2c-manifest.toml` owns `P2-VM-001..002`
only and explicitly "produces evidence, not new code"; Gate 2a's IDs cover
the correlation engine that *consumes* PSI events). So this work must
either mint new IDs or attach to an existing one.

**Why not attach to an existing ID.** The obvious candidates all fail:

- `P2-COR-001`/`P2-COR-002` own the PSI **correlation rules** — how
  already-admitted PSI events group into incidents. They are Gate 2a's,
  closed, and tested against synthetic events. Attaching production wiring
  to them would reopen a closed gate to add scope it never had, and would
  conflate "how PSI events correlate" with "whether any PSI event exists."
- `P2-API-001` owns `Incidents1.ListIncidents()` returning real data. PSI
  production is not a D-Bus surface concern at all.
- `P2-EVT-001..004` own **ingress ordering** semantics, not production.
  None of them says anything about a producer existing.
- `P2-VM-001`/`P2-VM-002` are Gate 2c's, accepted and committed.

**Why minting is the right call here, despite the precedent.** Prior
Phase 2 repairs (`phase2-2a-health-lifecycle-repair`,
`phase2-2a-transition-confidence-repair`,
`phase2-2b-health-lifecycle-integration-repair`) deliberately minted no
new IDs — but every one of those was a **reopening of already-owned
work**: an existing ID's implementation or evidence was wrong, and the ID
already named the requirement. That precedent does not reach this case.
This is **genuinely unowned scope** — a requirement that is real and
binding (§51's scope-inclusion clause) but that no ID has ever named,
precisely because a false predicate made it look already-satisfied. The
repository's own rule for this case is explicit, in
`GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §19: "any future addition gets
the next unused number in its family." And §51 has minted IDs by revision
before, exactly this way — `P2-EVT-003`/`P2-EVT-004` (first repair pass),
`P2-REC-003`/`P2-REC-004`/`P2-REC-005` and `P2-API-003` (second repair
pass). Minting via a §51 revision is the established mechanism, not a
novel one.

**Family and numbering.** Existing families only — §51's own Consequences
text already establishes that PSI does not mint a separate family
("provider-health and PSI correlation rules both live under `P2-COR-*`
rather than each minting a separate family"). Production/ingress
requirements belong to `P2-EVT-*`; real-VM evidence belongs to `P2-VM-*`.
Next unused numbers, zero-padded, no suffix letters (the `002b` defect
§50's history records). Hence `P2-EVT-005..008` and `P2-VM-003`.

**What the owner is being asked to confirm:** that these five IDs are
minted, in these families, with the requirement text §51 and handoff §19
now carry. If the owner prefers a different disposition, this gate's
manifest, this TDD, §51's "PSI production wiring, corrected" subsection,
and handoff §19 must all change together — they are written consistently
against this decision and nowhere else records it.

**RESOLVED — dated owner governance act, 2026-09-08.** The paragraph
above requested confirmation; it is left unrewritten per this project's
supersede-don't-erase discipline. Having read two independent
whole-repair audits (both `PASS WITH NON-BLOCKING FINDINGS`) and
adjudicated five specific acceptance blockers, the project owner
confirms the mint with one adjustment, recorded identically in
`TDD_CONTRACT.md` §51's revision history and
`GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §19's fifth revision note:

- `P2-EVT-005`, `P2-EVT-007`, `P2-EVT-008`, `P2-VM-003` — **ACCEPTED**
  exactly as minted above.
- `P2-EVT-006` — **DEMOTED.** It is no longer a standalone normative ID.
  Its full requirement text (this file's own R1–R8, "Phase A" and
  "Phase B" below) is preserved **verbatim and unweakened** and now binds
  as **acceptance criteria** under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`
  — the IDs its descriptor-acquisition work was always in service of:
  there is no production instantiation (`P2-EVT-005`) and no
  daemon-owned classification (`P2-EVT-007`/`P2-EVT-008`) without a
  correctly-resolved inherited descriptor underneath them. This is a
  governance-status change only. R1–R8 remain mandatory; every test this
  file already requires for them remains required; the gate manifest's
  `owned_normative_ids` array is updated to `[P2-EVT-005, P2-EVT-007,
  P2-EVT-008, P2-VM-003]` to match, and `required_evidence` entries that
  named `P2-EVT-006` are relabelled (not deleted) accordingly. ADR-009
  carries the same demotion note.

---

## Requirements

Requirements are grouped by phase. **Every test named below is a RED
test**: it must be written and observed failing against the unmodified
baseline (`4d5df97`) before the corresponding production code is written,
per `AGENTS.md`'s TDD rule and the execution protocol's step 3.

### Phase A — inherited descriptor acquisition (acceptance criteria under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`, formerly `P2-EVT-006`, demoted 2026-09-08 — see "Normative ID decision" above)

**R1 — `LISTEN_PID` validation.**
Requirement: the daemon consumes inherited descriptors only when
`LISTEN_PID` parses and equals its own `std::process::id()`. A missing,
unparseable, or mismatched `LISTEN_PID` yields **zero** PSI descriptors
and no PSI event path — never a descriptor taken from another process's
environment, and never a panic.
Evidence: tests covering absent, non-numeric, and mismatched `LISTEN_PID`,
each asserting zero descriptors acquired and the daemon path still
constructible and running.

**R2 — Name-based resolution, never index.**
Requirement: descriptors are resolved by matching entries in
`LISTEN_FDNAMES` to the expected per-resource fd names, never by position.
`OpenFile=`'s `:graceful` option **reorders the FD list** when a path is
absent (preflight §9.9 defect 2), so index-based resolution is a real,
reproducible bug, not a style concern.
Evidence: a test supplying a permuted `LISTEN_FDNAMES` order asserts each
resource resolves to the correct descriptor; a test supplying a partial
set (e.g. `cpu` and `io` present, `memory` absent — the `:graceful` case)
asserts the present resources resolve correctly and the absent one is
reported absent, not mis-bound to a neighbour's descriptor. A test asserts
that a duplicated name does not silently resolve to an arbitrary entry
(preflight §9.9 defect 4).

**R3 — Descriptor count and shape.**
Requirement: exactly **one descriptor per monitored resource** — 1 for the
accepted G8 shape (CPU only), at most 3 for `cpu`+`memory`+`io`. Derived
from real code, not assumed: `PsiTrigger::register` performs exactly one
`open` and writes one trigger line
(`providers/psi.rs:114-133`); `PsiEventSource` holds one `PsiTrigger` and
one `PsiEventDispatcher` (`providers/psi.rs:160-163`); the dispatcher is
bound to a single `PsiResourceKind` (`providers/psi.rs:168-174`); and only
the `some` class is ever classified (`present_severity`,
`providers/psi.rs:294-303`, calls `classify(resource.some, …)`), so **no
`(resource, full)` descriptor exists**. Any implementation requesting six
descriptors is wrong.
Evidence: a test asserting the daemon requests/consumes exactly one
descriptor name per configured resource, and that the `full` class is
never given a descriptor of its own.

**R4 — `FD_CLOEXEC`.**
Requirement: inherited descriptors arrive **without** `FD_CLOEXEC`
(preflight §9.9 defect 3). If the implementation hand-rolls listen-fd
parsing, it must set `FD_CLOEXEC` on each descriptor it takes ownership
of. This is latent today (the daemon spawns no subprocesses) but must not
be left to chance.
Evidence: a test asserting `FD_CLOEXEC` is set on every acquired
descriptor after acquisition. If the implementation reaches descriptors
only through `/proc/self/fd/N` reopens (which are `CLOEXEC` by
construction under `File::options()`), the test must assert that property
instead, and the implementation must document that the raw inherited
descriptor is never retained.

**R5 — No descriptor store, and no symlink farm.**
Requirement: `FileDescriptorStoreMax` is not set on
`debian/guardian-daemon.service` and must not be added — with the store
enabled, a stale already-triggered descriptor is returned *first* under a
duplicated name, so naive name lookup picks it and registration fails
`EBUSY` (preflight §9.9 defect 4). Separately, `/proc/self/fd/N` paths
must be constructed **in-process**; they must never be materialized into a
shared directory as symlinks, because those symlinks resolve against the
**reader's** fd table (defect 6).
Evidence: a unit-file assertion that `FileDescriptorStoreMax` is absent;
code review plus a test asserting no `/proc/self/fd/N` path is written to
any filesystem location.

### Phase B — trigger registration through an inherited FD (acceptance criteria under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`, formerly `P2-EVT-006`, demoted 2026-09-08 — see "Normative ID decision" above)

**R6 — Registration through an inherited descriptor.**
Requirement: a kernel PSI trigger registers successfully against a
descriptor supplied by systemd, and the **same** descriptor is
subsequently pollable for `POLLPRI`.
Evidence: a Layer 2 test against a real inherited descriptor (or a
faithful local equivalent that supplies a descriptor the process did not
open by path), plus the Phase F VM evidence for the real kernel case.

**R7 — One descriptor, both roles.**
Requirement: the same descriptor that carries the trigger also serves
`PsiEventDispatcher::dispatch_wake`'s pressure-text re-read. This is the
non-obvious requirement that a trigger-only design would silently fail:
`dispatch_wake` re-reads **by pathname** through `PsiFileSource::read` →
`fs::read_to_string` (`providers/psi.rs:68-72`), which returns
`Unavailable` under `ProcSubset=pid`.
Evidence: a test proving a wake dispatched against an inherited-descriptor
source classifies successfully (i.e. the re-read returns live text, not
`Unavailable`), before and after trigger registration.

**R8 — `EBUSY` is a hard error.**
Requirement: PSI triggers are per-**open-file-description**, not
per-inode: a second trigger write to the same description returns `EBUSY`;
a trigger on a separate description of the same file succeeds (proven
empirically, twice — preflight §9.4). `EBUSY` at registration must
therefore surface as a hard, observable error for that resource — never
swallowed, never treated as "already fine," never retried in place. There
is **no in-place re-arm** (defect 5): recovery requires reopening for a
fresh description.
Evidence: a test asserting a duplicate registration attempt against one
description yields a typed error that reaches the daemon's operational
logging, and that the affected resource is reported unavailable rather
than silently monitored.

### Phase C — daemon-owned classification and `Event` construction (`P2-EVT-007`)

**R9 — Classification is daemon-owned, from bytes the daemon read.**
Requirement: the `severity` value that reaches
`CorrelationEngine::classify()` is derived by `guardian-daemon` from raw
PSI text it read itself through its own descriptor, via the accepted,
unmodified G5 classifier. **No component outside `guardian-daemon`
supplies, influences, or self-reports a severity.** This is the structural
dissolution of the first architecture-review FAIL finding, and it must be
provable by construction, not by convention.
Evidence: a test asserting there is no path by which an externally
supplied severity value can reach `classify()`; a code-level assertion
(test or documented review item) that no IPC, environment variable, file,
or D-Bus message carries a severity into the daemon.

**R10 — Daemon owns every authority field.**
Requirement: `guardian-daemon` owns and constructs, for every PSI
`Event`: the `EventId`; the ingress timestamp and ingress order (via the
existing shared `IngressClock`, never a producer-supplied value); the
severity/classification; the resource identity; the Guardian `Event`
itself; and the incident semantics that follow. `resource_refs` MUST be
`["/proc/pressure/{resource}"]` — the stable identity §51's "PSI
correlation identity, corrected" repair binds PSI correlation to. This
gate must not regress that fix.
Evidence: tests asserting each field's provenance, with an explicit
assertion on `resource_refs` for every resource kind.

**R11 — Real PSI `Event` construction.**
Requirement: a real crossing produces a Guardian `Event` with
`event_type == "psi_threshold_crossing"`, the correct
`resource_refs`, a severity derived from the accepted classifier's
mapping, and a `normalized_key` produced by the existing `normalize_key`
path. The reference shape observed in the VM experiment (preflight §9.6)
is `event_type: "psi_threshold_crossing"`, `resource_refs:
["/proc/pressure/cpu"]`, `severity: Moderate`, `normalized_key: "psi cpu
threshold crossing nominal->elevated"`.
Evidence: a test asserting the full constructed `Event` shape.

### Phase D — shared ingress, correlation, restart (`P2-EVT-005`)

**R12 — Production instantiation, not library capability.**
Requirement: `guardian-daemon`'s **production wiring** constructs the PSI
event path and admits real PSI `Event`s. **This requirement is stated
explicitly because the original defect was precisely that a capable
library was never instantiated in production.** A test that constructs
`PsiEventSource` directly and asserts it works does **not** satisfy this
ID, no matter how thorough — the test must exercise the daemon's own
production path (the same function `main()` calls, or `main()`'s wiring
factored into a testable function that `main()` then calls, with a test
asserting `main()` calls it).
Evidence: a test driving the daemon's production PSI wiring end-to-end;
plus an explicit assertion that no `#[cfg(test)]`-only construction is
what is being measured.

**R13 — Shared Phase 2 ingress admission.**
Requirement: PSI `Event`s are admitted through the **existing**
`admit_event(&ingress_clock, &engine, event)` call site — the single
daemon-wide admission point Gate 2b built. No second `IngressClock` or
`CorrelationEngine` is constructed anywhere.
Evidence: a test asserting PSI events and provider-health events share one
strictly-increasing ingress sequence (the technique
`monitoring_tick_and_a_second_source_share_one_monotonically_increasing_ingress_sequence`
already establishes in `guardian-daemon.rs`'s own tests).

**R14 — Correlation behaviour (regression against Gate 2a, unmodified).**
Requirement: a PSI `Critical` crossing for a previously-nominal resource
opens an incident (`P2-COR-001`); a second `Critical` crossing for the
same `resource_refs.first()` within the debounce window links to the
existing incident rather than opening a new one (`P2-COR-002`), **even
when its transition-description text differs**. Gate 2a's engine is
consumed as-is; `correlation.rs` is forbidden scope.
Evidence: tests driving real PSI events through the production ingress and
asserting both behaviours, with no modification to `correlation.rs` or
`correlation_contract.rs` anywhere in the diff.

**R15 — Restart and reinitialization.**
Requirement: on daemon restart, PSI descriptors are re-acquired and each
restart receives a **fresh open file description** — proven by the fact
that trigger re-registration succeeds every time, where a stale reused
description would return `EBUSY` (preflight §9.8). The previous
registration is destroyed when the old description closes. No PSI state
persists across restart, consistent with `P2-EVT-004`'s fresh-ingress-epoch
semantics and §9's accepted memory-only model.
Evidence: a test (and the Phase F VM item) asserting re-registration
succeeds after a restart cycle, and that no PSI state is carried over.

### Phase E — degradation, invalid input, non-interference (`P2-EVT-008`)

**R16 — Missing/partial descriptors degrade truthfully.**
Requirement: a missing `OpenFile=` path aborts the unit with
`status=202/FDS` **before `ExecStart`** unless `:graceful` is used
(preflight §9.9 defect 1) — so a PSI-less kernel or container would
otherwise take down the whole daemon. `:graceful` is therefore required.
The daemon must treat a missing descriptor as a truthful
`PsiReading::Unavailable`, **never** as "no pressure" — the behaviour
`providers/psi.rs` already implements per `P1-PSI-005`. The daemon starts
and runs normally with zero PSI descriptors present.
Evidence: a test asserting zero-descriptor and partial-descriptor startup
both succeed with PSI reported `Unavailable` for the absent resources; a
unit-file assertion that every `OpenFile=` line carries `:graceful`.

**R17 — Malformed/invalid raw PSI values are rejected.**
Requirement: malformed, non-finite (`NaN`, `inf`), and out-of-range raw
PSI values never cross any internal boundary as a valid measurement and
never become an admitted `Event`. A present-but-malformed file is a typed
parse error (existing accepted `PsiParseError` behaviour), never silently
treated as "no pressure" — the fail-open defect the G5 model exists to
prevent.
Evidence: tests covering malformed text, non-finite floats, and
out-of-range values, each asserting zero `Event`s admitted and a typed
error surfaced.

**R18 — Provider-health behaviour is unaffected.**
Requirement: adding the PSI producer changes nothing about the
provider-health path. Gate 2b and Gate 2b-repair provider-health tests
pass **unmodified**. A PSI failure of any kind (missing descriptor,
`EBUSY`, malformed content, trigger-wait error) degrades **PSI
observability only**: `capability_registry_tick`, `monitoring_tick`,
`Capabilities1`, and provider-health-driven `Incidents1` entries continue
functioning.
Evidence: existing provider-health tests green and untouched in the diff;
a test asserting a forced PSI failure leaves provider-health incident
production working.

### Phase F — real-VM evidence (`P2-VM-003`)

Produced from the **real production systemd unit**, never a manual
`cargo run`, following Gate 2c's `P2-VM-001`/`P2-VM-002` methodology and
rigor. Raw command output must be recorded, sufficient for **independent
reproduction** — not a narrative summary.

1. The accepted daemon sandbox is **still active and unchanged**: diff
   `debian/guardian-daemon.service` against the accepted G9/Gate-2c
   baseline and show the only delta is the additive `OpenFile=` lines;
   re-run `systemd-analyze security` on the installed unit and show no
   regression from its accepted score.
2. The exact PSI descriptors systemd supplied: `LISTEN_FDS`,
   `LISTEN_FDNAMES`, `LISTEN_PID` as the daemon actually received them,
   plus `/proc/self/fdinfo/N` for each.
3. Real kernel trigger registration succeeds inside the real unit.
4. `poll(POLLPRI)` wakes on a real kernel event under a bounded
   `stress-ng` load (confirming a real crossing occurred, not merely that
   registration did not error).
5. The daemon constructs a PSI Guardian `Event` — captured with its real
   field values.
6. That `Event` reaches the **shared Phase 2 ingress** (the same
   `admit_event` point), demonstrably not a separate path.
7. Correlation executes — a real PSI-driven incident is observable via
   `Incidents1.ListIncidents()` over the real system bus, and repeated
   same-resource crossings correlate via `resource_refs`, not
   `normalized_key`.
8. `/proc/pressure` remains **unavailable to the daemon by pathname** —
   `ENOENT` for `/proc/pressure` and all three files, from inside the
   running unit, while the daemon holds working descriptors.
9. Unrelated procfs remains hidden exactly as before:
   `/proc/sys/kernel/hostname`, `/proc/meminfo`, `/proc/uptime`,
   `/proc/net`, `/proc/1/status`, `/proc/1/cmdline` all `ENOENT`;
   `/proc/self/status` and `/proc/self/fd` visible exactly as in the
   control.
10. The daemon remains unprivileged with **no new capabilities** —
    `/proc/<pid>/status` shows the `guardiand` UID and
    `CapPrm`/`CapEff`/`CapBnd`/`CapAmb` all `0000000000000000`.
11. Restart obtains **fresh descriptors** — trigger re-registration
    succeeds after a real `systemctl restart`, which a stale description
    would not permit (`EBUSY`).
12. **PSI failure degrades PSI observability only** — with PSI made
    unavailable (e.g. descriptors withheld), the daemon starts, runs, and
    continues producing provider-health incidents.
13. **The provider-health/UPower path remains functional** —
    `Capabilities1` and provider-health-driven `Incidents1` entries behave
    as Gate 2c evidenced.

---

## Daemon authority requirements (restated, binding)

| Field / concern | Owned by |
|---|---|
| `EventId` | `guardian-daemon` |
| Ingress timestamp and ingress order | `guardian-daemon`'s existing shared `IngressClock`, at `admit_event` time |
| Severity / classification | `guardian-daemon`, derived from raw PSI text it read itself via the accepted, unmodified G5 classifier |
| Resource identity (`resource_refs`) | `guardian-daemon`, fixed to `/proc/pressure/{resource}` |
| Guardian `Event` construction | `guardian-daemon` |
| Incident semantics | `guardian-daemon` + Gate 2a's unmodified `CorrelationEngine` |

Nothing outside `guardian-daemon` holds any of these authorities, because
nothing outside `guardian-daemon` participates in this design.

---

## Scope exclusions (binding on this gate)

This gate must not introduce, and must not drift into:

- **PSI persistence of any kind** — no state directory, no disk-backed
  queue, no event or incident persistence across restart. §51's
  memory-only model stands.
- **Any new user-facing or public API** — no D-Bus method, interface,
  object path, bus name, property, or signal; no CLI/TUI/GUI/indicator
  surface. `P2-API-002` is not engaged and no exception is requested.
- **Any mutation capability** — no writable path beyond what the unit
  already has, no Linux capability, no privileged D-Bus call, no polkit
  interaction.
- **A generalized event-submission mechanism** — no `SubmitEvent`,
  `InjectEvent`, or equivalent, on any surface, internal or external.
- **Incident redesign** — no new `Incident` field, no `IncidentWire` shape
  change, no `Incident::link_event` signature change, no incident severity
  (deferred in full per §51's "Severity/wire disposition").
- **Helper coupling** — `guardian-helper` is not referenced, modified, or
  involved in any way.
- **Redesign of Phase 2 correlation or Gate 2c behaviour** —
  `correlation.rs` and all Gate 2c files are forbidden scope; their
  behaviour is consumed and regressed against, never reinterpreted.
- **Phase 2 feature growth** — no `full`-line pressure-class support (it
  is parsed but never classified today, and that stays true), no new
  correlation rule, no diagnostic escalation, no `Transactions1` work, no
  G9-successor or next-phase work.

---

## Validation and commit state

Run every command in the manifest's `[validation_commands]` and record
actual output. The pre-change baseline at `4d5df97` is **409 passed, 0
failed, 3 ignored**; confirm it before editing anything (execution
protocol step 2). The authoring host lacks `libadwaita-1-dev`, so
`guardian-gui` does not build there — run the full workspace on
`guardian-g9`, as prior gates did.

`commit_policy = "leave-uncommitted-for-independent-review"`. Do not
commit, tag, or push beyond what the manifest and the assigning task
explicitly authorize.
