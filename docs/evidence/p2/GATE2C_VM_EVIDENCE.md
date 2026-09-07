# Gate 2c VM evidence — `P2-VM-001` / `P2-VM-002`

Governing manifest: `docs/guardian/30_TDD/gates/phase2-2c-manifest.toml`.
Governing TDD: `docs/guardian/30_TDD/gates/phase2-2c-tdd.md`.

Owned normative IDs this artifact evidences: `P2-VM-001` (real provider
transition → real `Incidents1` transition), `P2-VM-002` (real daemon
restart during an open incident loses it).

Published baseline: `33e2a6af990118459cc0c1ede7f5c497bb95755a`
(`origin/main`).

**This artifact repairs a governance/packaging gap only.** An
independent reviewer found Gate 2c's technical behavior (both VM
scenarios) sound — reproduced live and matched the implementer's
reported results — but found no evidence artifact had ever been written
to the repository, which the manifest's own `required_evidence` field
requires. This file is that artifact. It is written after the fact, so
it explicitly separates what is a retained/reproducible direct
observation from what is a summary of an earlier run whose raw
transcript was never captured — see "Evidence provenance" below before
reading the scenario sections.

## Evidence provenance (read this first)

Two prior rounds of work on Gate 2c happened before this artifact was
written, neither of which left a retained raw transcript in the
repository (`git log` on this repository contains no commit that ever
added anything under `docs/evidence/p2/GATE2C*`, and no such file existed
on disk before this one):

1. **Original implementer run** (round 1). Per the task context handed
   to this repair, the implementer ran both scenarios live on
   `guardian-g9`: masked/unmasked `upower.service`, observed a real
   provider degradation → real `Event` → dwell advancement → incident
   open via live `Incidents1.ListIncidents()` calls → recovery →
   incident closed (referred to as `P2-VM-001` in that round), and
   separately restarted the real `guardian-daemon` process during an
   open incident and observed the in-memory incident store empty out
   afterward with a new PID and D-Bus unique bus name (`P2-VM-002` in
   that round). **No raw transcript, log capture, or artifact from this
   round was ever committed or otherwise retained in this repository or
   found on the `guardian-g9` VM's filesystem during this repair's own
   preflight search.** Everything stated about round 1 in this file is
   therefore labeled **"prior run summary, transcript not retained"** —
   it is drawn only from what was reported about that round, not from
   any raw log this author has independently seen, and no specific
   numeric values (PIDs, bus names, timestamps, incident IDs) from round
   1 are reproduced here, because none were retained to reproduce
   honestly.
2. **Independent reviewer's live reproduction** (round 2). A reviewer
   uninvolved in round 1, explicitly distrusting the absence of any
   retained artifact, re-ran both scenarios live on `guardian-g9` itself
   and reported reproducing the identical real results, plus a
   `cargo fmt --check`/`clippy`/`cargo test --workspace` run (409 passed,
   0 failed, 3 ignored) against a checksum-verified fresh sync of the
   exact baseline content. This round's raw output was also not
   committed to the repository as an artifact — the same defect this
   file exists to fix — but its verdict and counts are cited in the
   governing task context handed to this repair and are consistent with
   round 3 below.
3. **This repair's own confirmatory re-run** (round 3 — the primary
   evidentiary basis of this artifact). `guardian-g9` was still running
   and reachable when this repair was carried out. Per the governing
   task's explicit allowance for "a confirmatory re-run of the exact
   same already-proven sequence," this repair re-ran both scenarios
   live on `guardian-g9`, directly, itself — not a new/different trigger
   design, the same UPower mask/unmask sequence and the same daemon
   restart already proven twice before. Every command, timestamp, PID,
   bus name, and `ListIncidents()` output quoted in the two sections
   below was captured directly by this repair, live, during this run.
   This is the strongest and most detailed evidence in this artifact,
   and is presented as this repair's own direct observation, not a
   restatement of rounds 1 or 2.

Before the re-run, this repair verified `guardian-g9`'s checked-out
`Guardian` source (`/home/ubuntu/Guardian`) matched this repository's
tracked content: a file-by-file `sha256sum` comparison of every tracked
file outside `.git`/`target` between the local working tree and the VM
checkout showed identical hashes for every file under `crates/` and
`tests/` (the code Gate 2c's evidence depends on) with zero differences;
the only hash differences were in a handful of Phase 2 governance
documents (`phase2-2b-*`, `phase2-2c-manifest.toml`,
`GUARDIAN_HARDENING_BACKLOG.md`, one evidence file) that had been
edited locally after the VM's last sync — none of which affect
`crates/` or `tests/` behavior. The VM's `guardian-daemon` binary
(`/usr/bin/guardian-daemon`, rebuilt same-day) and its running
`guardian-daemon.service`/`upower.service` were confirmed active before
the run began.

## Environment

- VM: `guardian-g9`, disposable, `multipass`, `Ubuntu 26.04 LTS`
  (`VERSION_CODENAME=resolute`).
- Guardian checkout: `/home/ubuntu/Guardian`, content-verified identical
  (see above) to this repository's `crates/`/`tests/` at baseline
  `33e2a6af990118459cc0c1ede7f5c497bb95755a`.
- `guardian-daemon.service`: real systemd-managed production daemon,
  unprivileged, real system D-Bus (`io.github.cliffthelin.Guardian1`).
- D-Bus interfaces used: `io.github.cliffthelin.Guardian.Incidents1`
  (`ListIncidents`, at object path
  `/io/github/cliffthelin/Guardian1/Incidents`) and
  `io.github.cliffthelin.Guardian.Capabilities1` (`ListCapabilities`, at
  `/io/github/cliffthelin/Guardian1/Capabilities`) — real names,
  confirmed by live `busctl introspect` during this run (the interface
  name is `io.github.cliffthelin.Guardian.Incidents1`, not
  `io.github.cliffthelin.Guardian1.Incidents1` — noted here because it
  is easy to get wrong from the bus name alone).
- Configured dwell: `CorrelationPolicy::default().health_min_dwell =
  Duration::from_secs(1)` (`crates/guardian-core/src/correlation.rs`).
  The daemon's own monitoring/capability-registry tick interval observed
  live during this run was ~30 seconds, so in practice a debounced
  candidate is promoted on the tick immediately following the tick that
  first observed the bad/good state, once the 1-second dwell has
  necessarily already elapsed by then.

## `P2-VM-001` — real provider transition → real `Incidents1` transition

**Trigger used**: `sudo systemctl mask --now upower.service` /
`sudo systemctl unmask upower.service && sudo systemctl start
upower.service`, per the gate TDD's corrected R1 (see the Contract
Collision correction in `phase2-2c-tdd.md` for why this trigger replaces
the originally-proposed `cups.service`/`systemd-logind.service`
approach).

### Round 3 (this repair, directly observed)

Baseline capability state, confirmed via a live `ListCapabilities` call
before any action:

```
upower.display-device   available  healthy
upower.battery-presence available  healthy
```

Baseline incidents: `ListIncidents()` → `0` entries.

Action (masked at `2026-09-07T11:40:47-06:00`):

```
$ sudo systemctl mask --now upower.service
Created symlink '/etc/systemd/system/upower.service' → '/dev/null'.
```

Daemon's own tick log observed the degradation on its next real tick:

```
Sep 07 11:40:59 guardian-g9 guardian-daemon[644406]: [guardian-daemon] capability registry tick: 6/11 capabilities available
```

(down from `8/11` on every prior tick that session). A live
`ListCapabilities` call immediately after confirmed the real transition
at the registry level:

```
upower.display-device   degraded  error   (was available/healthy)
upower.battery-presence degraded  error   (was available/healthy)
```

`ListIncidents()` called at this same moment still returned `0` — the
debounce candidate had just been created on that tick, dwell (1s) had
not yet been re-checked on a later tick. One tick later
(`Sep 07 11:41:29`), `ListIncidents()` returned:

```
2 incidents:
  guardian.correlation.incident-000000  ingress-24  ""        open  "capability upower.display-device debounced transition to Unavailable (direct provider report)"  confidence=unknown  primary_resource=upower.display-device
  guardian.correlation.incident-000001  ingress-24  ""        open  "capability upower.battery-presence debounced transition to Unavailable (direct provider report)" confidence=unknown  primary_resource=upower.battery-presence
```

Both incidents `open`, both keyed to the correct real `capability_id`,
both produced with no mock/fake bus involved — this is the real
production `guardian-daemon` process's real `Incidents1` object on the
real system bus.

Recovery action (at `2026-09-07T11:42:20-06:00`):

```
$ sudo systemctl unmask upower.service
Removed '/etc/systemd/system/upower.service'.
$ sudo systemctl start upower.service
$ systemctl is-active upower.service
active
```

After the next tick, `ListCapabilities` confirmed both capabilities back
to `available`/`healthy`, and `ListIncidents()` returned both incidents
now `closed`:

```
2 incidents:
  guardian.correlation.incident-000000  ingress-24  ingress-24  closed  ... upower.display-device
  guardian.correlation.incident-000001  ingress-24  ingress-24  closed  ... upower.battery-presence
```

Full round-trip — real degradation, real incident open, real recovery,
real incident close — reproduced directly by this repair on
`guardian-g9`.

### Round 1 (prior run summary, transcript not retained)

Reported (not independently re-verified from a raw log by this author):
the same mask/unmask sequence against `upower.service`, observed to
produce a real provider-health degradation through
`HealthTransitionProducer`, admitted as a real production `Event`,
promoted via dwell advancement, opening a real incident observed live
through `Incidents1.ListIncidents()`, then closing on recovery. Assigned
the label `P2-VM-001` in that round. No PIDs, timestamps, or exact
incident text from round 1 are restated here because none were
retained.

### Round 2 (independent reviewer, cited)

Reported (cited from the task context governing this repair, not a raw
log this author has seen): the reviewer independently re-ran the
identical mask/unmask sequence live on `guardian-g9` and reproduced the
same real result, without trusting round 1's unretained artifacts.

## `P2-VM-002` — real daemon restart during an open incident loses it

Requirement (per the gate TDD's corrected R2): prove only the
externally-observable process-level fact that a `guardian-daemon`
restart during an open incident loses that incident. This artifact does
not claim, and the public `IncidentWire`/`Incidents1` surface does not
expose, any externally-observed ingress-epoch or ingress-sequence value
— the fresh-`IngressClock`-epoch *mechanism* is proven separately and
deterministically at Gate 2a
(`crates/guardian-core/tests/correlation_contract.rs`,
`p2_evt_004_fresh_ingress_clock_resets_epoch`), combined with the
already-confirmed fact that `guardian-daemon`'s `main()`
(`crates/guardian-daemon/src/bin/guardian-daemon.rs`) constructs a fresh
`IngressClock::new()`/`CorrelationEngine::new()` pair at process start.

### Round 3 (this repair, directly observed)

Pre-restart daemon identity, confirmed live:

```
$ systemctl show guardian-daemon -p MainPID
MainPID=644406
$ busctl --system call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s io.github.cliffthelin.Guardian1
":1.20209"
```

`upower.service` was masked again (`2026-09-07T11:43:32-06:00`) to
produce a fresh open-incident state before the restart. After the next
tick, `ListIncidents()` confirmed two real open incidents plus the two
already-closed ones from the `P2-VM-001` run above:

```
4 incidents:
  incident-000003  open    upower.battery-presence
  incident-000002  open    upower.display-device
  incident-000000  closed  upower.display-device
  incident-000001  closed  upower.battery-presence
```

Restart (`2026-09-07T11:44:41-06:00`):

```
$ sudo systemctl restart guardian-daemon
$ systemctl show guardian-daemon -p MainPID
MainPID=647630
$ busctl --system call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s io.github.cliffthelin.Guardian1
":1.20318"
$ busctl --system call io.github.cliffthelin.Guardian1 /io/github/cliffthelin/Guardian1/Incidents io.github.cliffthelin.Guardian.Incidents1 ListIncidents
a(sssssss) 0
```

Genuinely new PID (`644406` → `647630`), genuinely new D-Bus unique bus
name (`:1.20209` → `:1.20318`, allocated by the bus itself, not
guessable/reusable), and `ListIncidents()` returns `0` — all four prior
incidents (two open, two closed) are gone from the fresh process's
in-memory store, confirming the accepted no-persistence semantics
directly, on the real production binary, over the real system bus.

`upower.service` was restored afterward (`unmask` + `start`); the new
daemon process's next tick confirmed both capabilities back to
`available`/`healthy` and `ListIncidents()` returning `0` (correctly —
the fresh process never observed a bad state, so it never opens an
incident for one), leaving the VM in a clean, healthy, incident-free
state.

### Round 1 (prior run summary, transcript not retained)

Reported (not independently re-verified from a raw log by this author):
a real `guardian-daemon` process restart during an open incident,
observed to empty the in-memory incident store, with a genuinely new PID
and D-Bus unique bus name. Assigned the label `P2-VM-002` in that round.
No specific PID/bus-name values from round 1 are restated here because
none were retained.

### Round 2 (independent reviewer, cited)

Reported (cited from the task context governing this repair, not a raw
log this author has seen): the reviewer independently reproduced the
identical restart-during-open-incident result live on `guardian-g9`.

## Reproducibility

Both scenarios above (round 3) were performed on a fresh, running
instance of the disposable `guardian-g9` VM whose tracked source content
was checksum-verified identical to this repository's `crates/`/`tests/`
at `baseline_sha = 33e2a6af990118459cc0c1ede7f5c497bb95755a`. Either
scenario is reproducible from a fresh VM clone at this baseline by
repeating the exact commands quoted above.

## Validation (this repair, directly run on `guardian-g9`)

Run against the checksum-verified content described above:

```
$ cargo fmt --check
(clean, exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.43s
(exit 0, no warnings)

$ cargo test --workspace
(per-crate `test result: ok` lines sum to)
409 passed; 0 failed; 3 ignored
```

These totals match the manifest's `expected_baseline` exactly and match
the independent reviewer's previously reported totals (round 2, cited
above).

## Restoration

`upower.service` was left `unmask`ed and `active` at the end of this
run; `guardian-daemon.service` was left `active` on its new (post-
restart) PID; no open incidents remained; the VM was left in the same
clean state it was found in, modulo the intentional, reversed test
actions above.
