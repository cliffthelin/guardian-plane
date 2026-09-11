# ADR-009: Guardian PSI production topology — in-daemon, via systemd-inherited descriptors

- Status: Accepted (**revised 2026-09-07, governance-repair pass — the
  Decision changed; the original decision is preserved below as labelled
  superseded history, per AGENTS.md's "do not rewrite historical accepted
  ADR rationale to hide an earlier decision; supersede it"**). Architecture
  selection only — no production implementation is authorized by this ADR;
  see "Scope" below.
- Date: 2026-09-07 (original decision), revised 2026-09-07 (this pass)
- **Filename note:** this document's original title was "Guardian PSI
  producer process and internal IPC boundary," and the filename
  `ADR-009-guardian-psi-producer-topology.md` reflects that. The title
  above is updated to describe what this ADR now decides. **The filename
  and ADR number are deliberately NOT changed** — this document is already
  referenced by `docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md`
  and `docs/guardian/30_TDD/gates/phase2-psi-producer-architecture-handoff.md`,
  and renaming it would break those references and obscure the decision
  history this revision exists to preserve.
- Governing task: Phase 2 PSI production-ingress architecture pass,
  following `docs/guardian/30_TDD/gates/phase2-psi-production-ingress-preflight.md`
  (Contract Collision confirmed; options (b)/(d) ruled out on real
  evidence; §9 records the `OpenFile=` experiment and §10 records the
  collision's formal resolution). The implementation gate for this ADR's
  consequences is
  `docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-manifest.toml`
  and its linked `-tdd.md`; this ADR does not implement anything.

## Context

`GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §51 Decision item 3 puts PSI-event
correlation in scope for TDD-contract Phase 2, on the false predicate
that G8 already produces live PSI events in production. It does not:
`guardian-daemon`'s `main()` never instantiates `guardian_core::
providers::psi::PsiEventSource` (confirmed by call-graph trace, preflight
§4). Gate 2b correctly declined to add this wiring itself (no owned
`P2-API-*` ID required it) and explicitly flagged the higher-order
question as unresolved. The preflight document records this as a formal
Contract Collision: a real requirement (live PSI events must reach the
shared `CorrelationIngress`) with no owning gate, and a second, entangled
collision — any process that reads `/proc/pressure/*` must not weaken
`guardian-daemon`'s or `guardian-helper`'s accepted `ProcSubset=pid`
hardening (ADR-002/G7/G9, confirmed still in force by G9 evidence).

Four architecture options were evaluated in the preflight document (§5)
and two independent VM reproductions (§8, Claude-run `guardian-g9` and a
Codex-run dedicated `guardian-psi-option-d` VM) closed one of them
empirically:

- **(b) narrower `ProcSubset` value** — ruled out. `man systemd.exec` on
  this host (systemd 259) documents only `ProcSubset=all|pid`; no
  path-scoped or PSI-specific value exists on this platform.
- **(d) `BindReadOnlyPaths=/proc/pressure`** — ruled out by two
  independent VM reproductions. The literal form cannot even start the
  unit under `ProcSubset=pid` (`226/NAMESPACE`: a `subset=pid` procfs
  mount contains no `pressure` node to bind onto). The nearest working
  relaxation (a relocated bind destination) exposes read-only PSI content
  without widening the sandbox anywhere else, but the real, unmodified
  `PsiEventSource::register` still fails against it: the kernel PSI
  trigger-registration ABI's mandatory `O_RDWR` open is rejected with
  `EROFS` by the read-only bind, before any write or `poll()` is reached.
- **(c) weaken `guardian-daemon`'s own `ProcSubset=pid`** — not selected.
  Explicitly available as a fallback only if (a) were independently shown
  infeasible; it is not. Rejected because it is exactly the kind of
  accepted-hardening exception AGENTS.md and the execution protocol
  require independent sign-off for, not a unilateral choice inside an
  implementation pass, and because it re-exposes every non-PID top-level
  `/proc` entry (not just `/proc/pressure`) to the primary daemon's mount
  namespace — `ProcSubset` has no finer grain (confirmed, preflight §4).
- **(a) a dedicated, narrow, unprivileged, separate production process**
  — **originally selected by this ADR; that selection is now superseded.**
  See "Superseded decision (original, 2026-09-07)" and "Why the original
  decision was withdrawn" below.

**All four of those dispositions still stand on their own evidence.** What
changed is that a **fifth mechanism the preflight's §5 never identified**
was subsequently found and proven: systemd's `OpenFile=` directive,
documented in `systemd.service(5)` (not `systemd.exec(5)`, which is where
the preflight's `ProcSubset=` research looked — the direct reason §5
missed it). `OpenFile=` has systemd (PID 1) open the named paths **in PID
1's own namespace** and pass the resulting descriptors into the service
via the standard `LISTEN_FDS`/`LISTEN_FDNAMES`/`LISTEN_PID` protocol. Its
documented purpose is verbatim this use case: to let services access files
they cannot access themselves because they run in a separate mount
namespace. Preflight §9 records the full experiment on `guardian-g9`
(Ubuntu 26.04, systemd 259, kernel 7.0.0-30-generic); the load-bearing
results are summarized under "Evidence and tests" below.

## Decision

**Selected: in-daemon PSI production via systemd-inherited `OpenFile=`
descriptors, with daemon-owned classification.**

```
systemd (PID 1) → OpenFile= PSI FDs → guardian-daemon
  → providers::psi (unmodified classification logic)
  → daemon-owned classification → Guardian Event
  → existing admit_event ingress → correlation
```

Concretely, this ADR decides:

1. **`guardian-daemon` itself is the PSI producer.** No new process. The
   live PSI event path is instantiated inside `guardian-daemon`'s own
   `main()`, feeding the single, already-existing `admit_event(&ingress_
   clock, &engine, event)` admission point that `monitoring_tick` and
   `capability_registry_tick` already use — the exact single-admission-point
   design Gate 2b built and Gate 2b's own R2 required any future PSI
   producer to feed through. No new ingress point is created.
2. **No new service identity.** No `guardianpsi` user, no new UID. The
   daemon continues to run as `guardiand`.
3. **No new D-Bus interface, and no new IPC protocol of any kind.** No
   `io.github.cliffthelin.GuardianPsi1`, no `PressureCrossing` signal, no
   Unix-domain socket, no framed wire protocol. There is no cross-process
   boundary in this design, so there is nothing to authenticate.
4. **Classification is daemon-owned, from bytes the daemon read itself.**
   No component outside `guardian-daemon` supplies, influences, or
   self-reports a severity. `Event.severity` — the field
   `crates/guardian-core/src/correlation.rs`'s `classify()` gates incident
   admission on — is derived by the daemon from the raw PSI text it read
   through its own inherited descriptor, via the accepted, unmodified G5
   classifier (`crate::psi::classify`, reached through
   `providers::psi`'s `present_severity`).
5. **No weakening of the accepted sandbox.** `ProcSubset=pid`,
   `ProtectProc=invisible`, and every other accepted directive on
   `debian/guardian-daemon.service` stay exactly as G7/G9 left them. The
   sole unit-file delta is **up to three additive `OpenFile=` lines** (one
   per monitored resource). `/proc/pressure` remains invisible **by
   pathname** inside the daemon's mount namespace even while the daemon
   holds working descriptors to it, and `systemd-analyze security` scores
   identically to the live unit (`0.6 SAFE`, tables diff clean apart from
   the unit name) with those lines present.
6. **`guardian-helper` is not involved at all**, and no privileged write
   path, polkit rule, or Linux capability is added anywhere.
7. **One descriptor per monitored resource**, resolved **by
   `LISTEN_FDNAMES` name, never by index**, with `LISTEN_PID` validated
   against the daemon's own PID. One descriptor serves both roles —
   trigger registration *and* the pressure-text re-read
   `PsiEventDispatcher::dispatch_wake` performs. Descriptors are declared
   `:graceful` so a PSI-less kernel or container degrades PSI
   observability only, rather than aborting the unit before `ExecStart`.
   A missing descriptor is a truthful `PsiReading::Unavailable`, never
   "no pressure" — the behavior `providers/psi.rs` already implements per
   `P1-PSI-005`.
8. **One small, safe library extension**, and no more: `PsiFileSource`
   gains explicit per-resource path resolution alongside its existing
   `base_dir` behaviour, leaving `real()`, `at()`, `read()`, `path()`,
   `PsiTrigger::register`, `PsiEventSource`, `PsiEventDispatcher`, and all
   seven existing unit tests working untouched. No `unsafe` (the workspace
   sets `unsafe_code = "forbid"`, `Cargo.toml:30`, which rules out
   `OwnedFd::from_raw_fd`) and no new dependency: `/proc/self/fd/N` stays
   visible under `ProcSubset=pid` because it is process-associated, so an
   inherited fd number becomes a `File` through plain, safe
   `File::options().read(true).write(true).open(...)` — verified working
   inside the sandbox.

**This decision dissolves both architecture-review FAIL findings against
the superseded decision structurally, rather than mitigating them.** With
no cross-process producer, there is no producer-supplied severity reaching
`classify()` at all; and with no new D-Bus interface, there is nothing to
reconcile with §51's `P2-API-002` language — which is precisely why the two
reviewers' disagreement about `P2-API-002`'s scope (recorded below) is now
moot and is deliberately left unadjudicated.

## Superseded decision (original, 2026-09-07) — preserved, not erased

**The following was this ADR's Decision as originally accepted. It is no
longer the decision.** It is preserved verbatim because it was a genuine,
independently-reasoned selection made on the evidence available at the
time, and because the honest narrative matters: option (a) was selected,
was independently reviewed, failed on two grounds, and only *afterwards*
was `OpenFile=` identified and proven. Nothing in this ADR should be read
as implying `OpenFile=` was always the selected solution — it was not
identified at all until after the reviews below.

> **Selected: option (a).** A new, narrow, unprivileged production process,
> `guardian-psi`, owns kernel PSI trigger registration and polling. It
> reuses `crates/guardian-core/src/providers/psi.rs`'s existing, accepted
> `PsiFileSource`/`PsiTrigger`/`PsiEventSource` code unmodified. It runs
> under its own dedicated systemd unit and its own dedicated unprivileged
> service user (`guardianpsi`), sandboxed identically to `guardian-daemon`'s
> own accepted unit except for the one directive that is the entire reason
> this unit exists (`ProcSubset=` omitted/`all` instead of `pid`) and the
> removal of write-path directives `guardian-psi` does not need (no
> `ReadWritePaths=`, no `StateDirectory=` — it is a pure reader/reporter,
> holds no state, per the no-PSI-persistence scope boundary below).
>
> `guardian-daemon`'s and `guardian-helper`'s existing accepted hardening
> (`debian/guardian-daemon.service`, `debian/guardian-helper.service`) is
> **completely unchanged** by this decision. Neither `ProcSubset=pid` nor
> any other directive on either unit is touched.
>
> **IPC mechanism: a narrow internal D-Bus interface, not a Unix domain
> socket.** `guardian-psi` owns a new, dedicated well-known system-bus
> name, `io.github.cliffthelin.GuardianPsi1`, sibling to (never nested
> under) the existing `io.github.cliffthelin.Guardian1` and
> `io.github.cliffthelin.GuardianHelper1` names — the same "separate
> process, separate bus name" pattern ADR-002/G7/G9 already established for
> `guardian-helper`. It exposes exactly one signal, `PressureCrossing`, at
> `/io/github/cliffthelin/GuardianPsi1` on interface
> `io.github.cliffthelin.GuardianPsi1` — no method, no property, no
> generic ingress surface. `guardian-daemon` is an ordinary D-Bus client of
> this interface, exactly as it already is for six read-only provider
> interfaces (`systemd1`, `login1`, `UDisks2`, `UPower`, `AccountsService`,
> and now `GuardianPsi1`). Full wire contract, sandbox unit, failure
> semantics, and testing/VM-evidence plan are specified in the companion
> handoff document, not repeated here.
>
> This is a new interface name, not an addition to `Guardian1`,
> `Capabilities1`, `Incidents1`, or `Transactions1` — `P2-API-002`'s freeze
> ("no new `Guardian1`/`Capabilities1`/`Incidents1`/`Transactions1`
> method/interface/object path... unless a future §51 revision explicitly
> approves one") is a closed enumeration of four specific, already-shipped
> interface names, all served by `guardian-daemon`'s own `Guardian1`
> connection. `GuardianPsi1` is neither: it is a distinct well-known name
> owned by a distinct process, structurally identical in kind to
> `GuardianHelper1`, which the freeze's own enumeration does not name
> either despite predating it. Reading `P2-API-002` as reaching a
> differently-named interface owned by a different service would make the
> already-accepted `GuardianHelper1` retroactively non-compliant with a
> rule written after it shipped — an inconsistent reading. This ADR treats
> the freeze as scoped to the four named surfaces, consistent with its own
> literal text.

**Why option (a) was chosen at the time (preserved rationale).** On the
evidence then available, (b) and (d) were empirically dead and (c) was the
only remaining in-daemon path — and (c) required deliberately widening
`guardian-daemon`'s already-accepted `ProcSubset=pid` sandbox to `all`,
re-exposing every non-PID top-level `/proc` entry, not just
`/proc/pressure`. Option (a) cost more to build (new binary, unit, service
user, IPC surface, and possibly a §51 revision) but touched **zero**
accepted hardening, and matched this project's own established
privilege-separation precedent (ADR-002's Model B: keep narrow,
single-purpose components separate from the two existing hardened
processes rather than widening either one's exposure). That reasoning was
sound for the option set that was known. It is superseded only because the
option set turned out to be incomplete.

## Why the original decision was withdrawn

### The two independent architecture-review FAIL findings

Both reviews FAILED ADR-009 as originally written. Both findings are
recorded here in substance, as the reason the decision changed.

1. **PSI message authority too broad.**
   `crates/guardian-core/src/correlation.rs`'s `classify()` gates incident
   admission solely on `Event.severity`. The superseded design populated
   that field from the producer's own self-reported `to_severity` (the
   `PressureCrossing` signal's `to_severity` field), with **no daemon-side
   verification against the raw measurement**. A compromised or buggy
   `guardian-psi` could therefore fabricate unlimited `Critical`
   incidents. The companion handoff's security argument ("guardian-psi
   cannot submit arbitrary Guardian events") correctly bounded the
   *vocabulary* the producer could speak, but not the *authority* of the
   single field that actually gates incident admission.

2. **The proposed new D-Bus interface conflicts with higher-order Phase 2
   §51 language unless §51 is explicitly amended.** The superseded
   Decision's `P2-API-002` argument (preserved verbatim above) read the
   freeze as a closed enumeration of four named client interfaces. **The
   two reviewers disagreed** on whether `P2-API-002` is such a closed
   enumeration or instead forbids any new bus name absent an explicit §51
   revision. **That disagreement is now moot** and is recorded here as
   history only: the selected architecture introduces no new D-Bus
   interface, so nothing in it depends on which reading is correct. **This
   ADR deliberately does not adjudicate it.** A future gate that genuinely
   proposes a fifth bus name must resolve the question then, on its own
   merits — it is not resolved, implicitly or otherwise, by this
   revision.

### The narrower mechanism, found only afterwards

Only after those reviews was `OpenFile=` identified (in
`systemd.service(5)`) and then proven on `guardian-g9`. It satisfies the
requirement the collision named while leaving the hardening boundary
literally unchanged, which neither (a) nor (c) could do. See "Evidence and
tests."

## Alternatives considered

**Option (a): a dedicated `guardian-psi` process + `GuardianPsi1` D-Bus
interface** — originally selected, now **withdrawn**. See "Superseded
decision" and "Why the original decision was withdrawn" above. Beyond the
two FAIL findings, the selected architecture also strictly dominates it on
cost: no new binary, crate, unit, service user, bus-policy file, IPC
contract, or `debian/postinst` change is needed at all.

**Option (c): weaken `guardian-daemon`'s own `ProcSubset=pid`** — still
not selected, and now clearly unnecessary. `OpenFile=` obtains exactly the
capability (c) would have bought, without the sandbox widening that made
(c) require independent hardening sign-off. Recorded as a fallback only if
a future independent review finds the selected mechanism infeasible for a
reason this pass did not anticipate.

**Options (b) and (d)** — remain ruled out on the preflight's own real,
independently reproduced VM evidence (§4, §8.1–§8.11). Unchanged by this
revision.

**`BindPaths=` (writable, relocated bind)** — **not evaluated**, because
the fallback branch was never entered: `OpenFile=` succeeded. Recorded as
not needed. It would in any case be strictly worse than `OpenFile=`: it
adds a real mount and exposes PSI at a non-standard path, where
`OpenFile=` adds neither.

**IPC alternatives (narrow Unix-domain socket vs. D-Bus)** — this
comparison was material only to the superseded design and is **moot**
under the selected architecture, which has no IPC boundary at all. The
original comparison is preserved in the companion handoff document
(superseded in place, not deleted).

**Reusing `guardiand`'s existing service identity for a separate
`guardian-psi`** — moot for the same reason: there is no separate process.
The original rejection rationale (a shared UID would collapse the bus-policy
ownership boundary the superseded design's sender-authentication argument
relied on, and would blur attribution for no operational benefit) is
preserved in the revision history below as part of the superseded design's
record.

## Evidence and tests

This ADR is an architecture selection, not an implementation — no
`owned_normative_ids` are closed, no RED/GREEN test evidence is produced,
and no VM run is performed by this ADR itself. The full experimental
record lives in the preflight document and is not re-derived here:
preflight §4 and §8.1–§8.11 for the options ruled out, and preflight §9
for the `OpenFile=` experiment. The load-bearing results this decision
rests on, all empirically established on `guardian-g9` (Ubuntu 26.04,
systemd 259, kernel 7.0.0-30-generic):

- **Descriptors are opened in PID 1's namespace.** Confirmed
  mechanistically: the inherited descriptor's `/proc/self/fdinfo/N`
  reports an `mnt_id` matching PID 1's host `proc rw` mount, while the
  service's own `/proc/self/mountinfo` shows
  `proc rw,hidepid=invisible,subset=pid`.
- **Default open mode is `rw`**, verified against the `systemd.service(5)`
  man page rather than assumed — which matters because the kernel PSI
  trigger ABI requires `O_RDWR` (the exact requirement that killed option
  (d) with `EROFS`).
- **One descriptor per monitored resource**, derived from real code, not
  assumed: `PsiTrigger::register` (`providers/psi.rs:114-133`) performs
  exactly one `open` and writes one trigger line; `PsiEventSource`
  (`providers/psi.rs:160-163`) holds one `PsiTrigger` and one
  `PsiEventDispatcher`; the dispatcher (`providers/psi.rs:168-174`) is
  bound to a single `PsiResourceKind`; and only the `some` class is ever
  classified (`present_severity`, `providers/psi.rs:294-303`, calls
  `classify(resource.some, …)`), so no `(resource, full)` descriptor is
  needed. 1 for the accepted G8 shape (CPU only), at most 3 for
  `cpu`+`memory`+`io`.
- **PSI triggers are per-open-file-description, not per-inode** (proven
  twice): a second trigger write to the same description returns `EBUSY`;
  a trigger on a separate description of the same file succeeds.
- **One descriptor covers both roles.** `dispatch_wake` re-reads pressure
  text *by pathname* (`providers/psi.rs:68-72`), which returns
  `Unavailable` under `ProcSubset=pid` — so a trigger-only descriptor
  would have produced a registered trigger that could never classify its
  wake. Tested: `lseek(0)+read` and `pread` both return live PSI text on
  the same inherited descriptor, before and after trigger registration.
- **The real, unmodified `PsiEventSource` ran the complete path inside the
  accepted sandbox**: `PsiFileSource::read` → `PsiEventSource::register` →
  `poll(POLLPRI)` real wake under bounded `stress-ng` → a real Guardian
  `Event` (`event_type: "psi_threshold_crossing"`, `resource_refs:
  ["/proc/pressure/cpu"]`, `severity: Moderate`, `normalized_key: "psi cpu
  threshold crossing nominal->elevated"`). **Honest qualification:**
  because the current API accepts only a path, that Rust run *reopened*
  the inherited descriptor via `/proc/self/fd/N` rather than polling the
  inherited descriptor directly; writing the trigger to the inherited
  descriptor itself and polling it was proven separately with a direct
  syscall harness. This ADR does not overstate what the Rust run proved.
- **Sandbox genuinely unweakened.** Inside the `OpenFile=`-equipped unit,
  `/proc/pressure` and all three files, `/proc/sys/kernel/hostname`,
  `/proc/meminfo`, `/proc/uptime`, `/proc/net`, `/proc/1/status`, and
  `/proc/1/cmdline` all remain `ENOENT`; `/proc/self/status` and
  `/proc/self/fd` remain visible exactly as in the control.
  `systemd-analyze security` on the accepted directive set **plus** three
  `OpenFile=` lines vs. the live `guardian-daemon.service`: tables diff
  clean apart from the unit name, both **`0.6 SAFE`**.
- **Restart semantics:** each restart receives a fresh open file
  description — proven, because trigger re-registration succeeds every
  time, where a stale description would return `EBUSY`. The previous
  registration is destroyed when the old description closes.
  `guardian-daemon.service` never sets `FileDescriptorStoreMax`.

**Six concrete defects were found, all mitigable, none disqualifying** —
recorded in full in preflight §9.9 and binding on the implementation gate:
(1) hard startup coupling (`202/FDS` before `ExecStart` on a missing path;
mitigated by `:graceful` plus truthful `Unavailable`); (2) `graceful`
reorders the FD list (resolve by `LISTEN_FDNAMES` name, never by index);
(3) descriptors arrive without `FD_CLOEXEC` (latent today; a hand-rolled
parser must set it); (4) FD-store staleness (never enable
`FileDescriptorStoreMax` on this unit; treat `EBUSY` at registration as a
hard error); (5) no in-place re-arm (recovery requires reopening for a
fresh description); (6) avoid a shared-directory symlink farm
(`/proc/self/fd/N` resolves against the *reader's* fd table — build such
paths in-process).

The RED test requirements and the real-VM acceptance plan a future
implementation gate must satisfy are specified in
`docs/guardian/30_TDD/gates/phase2-psi-inherited-descriptor-ingress-tdd.md`
— defined there, not executed by this ADR.

## Consequences

- **No new crate, binary, systemd unit, system user, or D-Bus bus-policy
  file is created.** Every packaging consequence the superseded decision
  carried (`crates/guardian-psi`, `debian/guardian-psi.service`, a
  `guardianpsi` user in `debian/postinst`, and
  `debian/io.github.cliffthelin.GuardianPsi1.conf`) is withdrawn.
- **`debian/guardian-daemon.service` gains up to three additive
  `OpenFile=` lines and nothing else.** No directive is removed, relaxed,
  or reordered. This is the only unit-file delta in the whole design.
- **`guardian-daemon` gains a live PSI event path in `main()`**, feeding
  the existing `admit_event` point. It gains no new public method, D-Bus
  interface, object path, or client-visible surface — `P2-API-002` is
  untouched under either of the two contested readings, which is the point
  of the reselection.
- **`crates/guardian-core/src/providers/psi.rs` needs one small,
  additive library extension** (explicit per-resource path resolution on
  `PsiFileSource`), not the producer/consumer split the superseded design
  required. Specifically withdrawn: the superseded consequence that
  `dispatch_wake`/`event_from_crossing` must be split so a separate
  producer process could link the classifier without linking `Event`
  construction. In the selected architecture the daemon does both, so no
  such split is needed — `Event` construction stays exactly where it is
  and the daemon calls it directly, the same way it already calls
  `providers::health::transition_event`.
- **`Event.severity` provenance is strictly better than in the superseded
  design**, and better than in any cross-process design: the daemon
  classifies from raw PSI bytes it read itself through its own descriptor.
  No component can self-report a severity into `classify()`.
- **`guardian-helper`, polkit, and every privileged write path are
  entirely uninvolved**, exactly as in the superseded design.
- **PSI failure degrades PSI observability only.** A missing or
  unregisterable descriptor is a truthful `Unavailable`; provider-health
  correlation, the monitoring-tick recorder, `Capabilities1`, and
  `Incidents1` all continue functioning.
- **Future `full`-line pressure-class support** (currently parsed but never
  classified) would need one additional descriptor per resource only if a
  future gate decides `full` warrants a separate open file description; it
  is out of scope here, and — unlike in the superseded design — it needs
  no interface-major migration, because there is no wire interface.

## Rollback and migration

There is no external compatibility surface to migrate: no new D-Bus name,
no new IPC protocol, no new client-visible field. Rollback of the selected
architecture is to remove the `OpenFile=` lines from
`debian/guardian-daemon.service` and stop instantiating the PSI path in
`main()` — after which `guardian-daemon` behaves exactly as it does today
at baseline `4d5df97`, with PSI reported truthfully as `Unavailable`
rather than silently as "no pressure." The library extension
(`PsiFileSource` per-resource path resolution) is additive and can remain
harmlessly, since `real()`/`at()` behaviour is unchanged.

Rollback of *this revision* (i.e. reverting to the superseded option-(a)
design) would require re-answering both architecture-review FAIL findings
in "Why the original decision was withdrawn," including the `P2-API-002`
scope question the two reviewers disagreed on and this ADR deliberately
left unadjudicated.

## Revision history

- **2026-09-07, original:** selected option (a) — a dedicated, narrow,
  unprivileged `guardian-psi` process with its own `guardianpsi` service
  user and a new `io.github.cliffthelin.GuardianPsi1` D-Bus interface
  exposing one `PressureCrossing` signal; rejected a Unix-domain socket in
  favour of D-Bus (no `UnixListener`/`UnixStream` precedent anywhere in
  the workspace; `zbus` is the exclusive IPC investment; bus-daemon
  `sender` stamping gives spoofing resistance structurally; the existing
  `guardian_testkit::PrivateSessionBus` harness supports contract
  testing); and rejected reusing `guardiand`'s identity for the new
  process (a shared UID would collapse the bus-policy ownership boundary
  the sender-authentication argument relied on and would blur attribution
  for no operational benefit, contrary to ADR-002's Model B least-privilege
  precedent). The full original Decision text is preserved verbatim above.
- **2026-09-07, governance-repair pass (this revision):** the Decision is
  **changed**, not merely clarified. Two independent architecture reviews
  FAILED the original decision — (1) PSI message authority too broad
  (`classify()` gates incident admission solely on `Event.severity`, which
  the original design populated from the producer's own self-reported
  `to_severity` with no daemon-side verification against the raw
  measurement, so a compromised or buggy `guardian-psi` could fabricate
  unlimited `Critical` incidents); and (2) the proposed new D-Bus
  interface conflicts with higher-order Phase 2 §51 language unless §51 is
  explicitly amended, with the two reviewers disagreeing on whether
  `P2-API-002`'s enumeration is closed to four named client interfaces or
  forbids any new bus name — a disagreement now **moot** under the
  reselected architecture and deliberately left unadjudicated here.
  A subsequent disposable-VM experiment on `guardian-g9` then proved a
  narrower mechanism, systemd `OpenFile=` (documented in
  `systemd.service(5)`, which is why the preflight's `systemd.exec(5)`
  research never identified it), and the decision was reselected to
  in-daemon PSI production via inherited descriptors with daemon-owned
  classification: no new process, no new UID, no new interface, no new IPC
  protocol, and no weakening of `ProcSubset=pid`. The title is updated to
  describe what this ADR now decides; **the filename and ADR number are
  deliberately unchanged** so existing references from the preflight and
  the companion handoff keep resolving. No prior gate's acceptance, tag,
  or evidence is altered by this revision; the original decision and its
  rationale are superseded in place, never erased, per AGENTS.md's
  ADR discipline.
- **2026-09-08, owner governance act + acceptance repair (this
  revision):** two independent whole-repair audits of the resulting
  candidate both returned `PASS WITH NON-BLOCKING FINDINGS`; the project
  owner adjudicated five specific acceptance blockers on top of that
  PASS and confirmed the pending mint from the entry above, with one
  adjustment. `P2-EVT-005`, `P2-EVT-007`, `P2-EVT-008`, and `P2-VM-003`
  are **ACCEPTED**. `P2-EVT-006` is **DEMOTED**: it is no longer a
  standalone normative ID; its descriptor-acquisition requirements
  (`OpenFile=`/`LISTEN_FDNAMES` name-based resolution never positional,
  the `:graceful` partial-set case, the never-add-`FileDescriptorStoreMax`
  rule, `EBUSY` as a hard, observable, never-swallowed error) are
  preserved **verbatim and unweakened** and now bind as **acceptance
  criteria** under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008` — the IDs
  descriptor acquisition was always in service of — rather than as a
  free-standing ID. The identical demotion is recorded in
  `TDD_CONTRACT.md` §51's revision history,
  `GUARDIAN_PHASE2_IMPLEMENTATION_HANDOFF.md` §19's fifth revision note,
  and the gate TDD's "Normative ID decision" section. This same repair
  pass additionally: (1) closed a real FD-lifetime defect — inherited
  descriptors were reopened independently on every trigger-register and
  every subsequent dispatch, producing multiple distinct open file
  descriptions rather than the one this ADR's Decision (below) always
  intended, and leaving the raw systemd-supplied descriptor un-marked
  `FD_CLOEXEC`; the fix opens `/proc/self/fd/N` exactly once per resource
  at startup into one owned `std::fs::File`, explicitly sets
  `FD_CLOEXEC` on it via a safe `rustix` `fcntl` call over a
  `BorrowedFd`, and reuses that single File for trigger registration,
  `poll()`, and every subsequent pressure-text reread — still no
  `unsafe`, `unsafe_code = "forbid"` unchanged; (2) made per-resource PSI
  availability (cpu/memory/io) individually truthful at startup instead
  of a single aggregate "N/N monitored" line; (3) made the Capability
  Registry's PSI capability reporting observe this same live descriptor
  state (shared via a narrow `Arc<Mutex<_>>` wired in `main()`) instead
  of re-probing `/proc/pressure` by pathname, which the sandbox always
  denied and which made a live PSI capability report `Unsupported`; (4)
  replaced §F.14 item 2's logs-plus-correlation-outcome methodology for
  `P2-VM-003` with direct observation of a live PSI `Event`'s fields on
  the real VM, via bounded, temporary, env-var-gated instrumentation
  removed again before this pass's completion. No prior gate's
  acceptance, tag, or evidence is altered; the Decision below (in-daemon
  production via inherited `OpenFile=` descriptors, daemon-owned
  classification, no new process/UID/interface) is unchanged by this
  revision — only the FD-ownership/lifecycle mechanics beneath it, the
  observability of PSI availability, and the VM evidence methodology for
  `P2-VM-003` are repaired.
