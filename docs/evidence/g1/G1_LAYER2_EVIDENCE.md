# G1 Layer 2 — Real-Host Evidence

Disposable Ubuntu 26.04.1 VM, provisioned via `multipass launch 26.04` and
destroyed after this evidence was captured. Setup script:
`docs/evidence/g1/g1-layer2-vm-setup.sh`. Full server-side log:
`docs/evidence/g1/g1_layer2_server_transcript.log`.

## 1. Host versions (matches the researched Resolute baseline)

```text
PRETTY_NAME="Ubuntu 26.04 LTS"
VERSION="26.04 LTS (Resolute Raccoon)"
dbus-daemon                1.16.2-2ubuntu4
polkitd                    127-2ubuntu1
libpolkit-agent-1-0        127-2ubuntu1
libpolkit-gobject-1-0      127-2ubuntu1
rustc                      1.93.1
cargo                      1.93.1
```

## 2. What ran

`tests/vm/g1-layer2/` is a standalone Cargo package (not a workspace member,
so it never touches the primary workstation's `cargo build/test/clippy
--workspace`) with two binaries:

- `g1-layer2-server` — registers `AuthProbe1` on the **real system D-Bus**,
  backed by the **real** `guardian_core::authorization::polkit::PolkitAuthorizer`
  (production code, not a mock — the same type shipped in `guardian-core`).
- `g1-layer2-client` — a thin CLI that calls one `AuthProbe1` method and
  prints the real resolved unique bus name, real OS uid, and the outcome.

Two real local users were created: `guardiang01` (uid 1001, granted
`guardian.test.read` and `guardian.test.low-risk-write` by a real
`/etc/polkit-1/rules.d` rule matched on `subject.user`) and `guardiang02`
(uid 1002, granted nothing — denied purely by polkit's implicit defaults).

## 3. P0-AUTH-001 — caller identity cannot be spoofed (real system bus)

| Caller | Real UID | Claimed identity in method args | Action | Real polkit outcome |
|---|---|---|---|---|
| guardiang01 | 1001 | uid=1000, user="nobody", is_admin=false | low-risk-write | **Authorized** |
| guardiang02 | 1002 | uid=0, user="root", is_admin=true | low-risk-write | **Denied** |

`guardiang01` succeeded despite claiming to be an unprivileged stranger;
`guardiang02` was denied despite claiming to be root with `is_admin=true`.
The real system-bus unique name and real OS uid (read from
`GetConnectionUnixUser`) are the only things that ever reached
`guardian_core::identity::CallerIdentity` — proven directly in the server log
(`unique_name=:1.34 uid=Some(1001)`, `unique_name=:1.35 uid=Some(1002)`).
**Strong form proven on the real system bus, not the private test bus.**

## 4. P0-AUTH-002 — denied action does not apply (real polkit)

`guardiang01` (granted `low-risk-write` only) attempting `high-risk-write`
was denied by real polkit (never granted by any rule, falls to the action's
implicit `no` default). The G1 handoff §7 ordering invariant (already proven
structurally at Layer 1) applies identically here since it is the same
production `attempt()` code path — a real denial from `PolkitAuthorizer`
still short-circuits before any mutation. **Real-host denial proven.**

Granting `low-risk-write` did not implicitly grant `high-risk-write` for the
same real user (TDD contract §9, last line) — confirmed by the same test.

## 5. P0-AUTH-003 — background (non-interactive) request fails closed (real polkit)

`guardiang01`, non-interactively (`interactive=false`), attempting
`moderate-write` (which requires `auth_self` — interactive authentication)
was denied by real polkit with no prompt of any kind. **Real-host proof
obtained: a non-interactive request for an action that requires interaction
gets a flat, immediate denial from the real polkit authority — it never
even attempts to reach an authentication agent.**

## 6. SSH policy

`guardiang01` connecting over a real SSH session (password auth, creating a
genuine `systemd-logind` session — confirmed via `loginctl`) and attempting
the identical granted `low-risk-write` call got the identical `Authorized`
result as a local (non-SSH) call. **Guardian's G1 authorization boundary
does not special-case SSH at all — behavior is identical because the
authorization decision is derived purely from the D-Bus system-bus identity
and real polkit policy, which do not distinguish transport.** This is the
intentional SSH policy for G1: *supported, and behaviorally identical to any
other local caller*, not a special code path.

## 7. P0-AUTH-004 / P0-AUTH-005 — interactive completion: NOT obtained

This is reported honestly rather than fabricated or downgraded.

**What was attempted:** `pkttyagent --process <client-pid>` was registered
(as `guardiang01`, inside a real SSH session with a real logind session and
a real allocated pty) before the client made an `interactive=true` request
for `moderate-write` (which requires `auth_self`). Client-process liveness
during registration was explicitly verified via `ps -p <pid>` before
registering the agent.

**What happened:** the real `CheckAuthorization` call still returned a flat
denial (`is_authorized=false`, `is_challenge=false`) — polkit never routed
the request to the registered agent, and no password prompt ever appeared.
Two distinct failure modes were observed across attempts, both narrowed down
by direct experimentation:

- When `pkttyagent` was launched without a real controlling terminal of its
  own (e.g. via a non-pty `ssh host command` invocation), it failed outright
  with `Error opening current controlling terminal for the process
  ('/dev/tty')`.
- With a real pty (`ssh -t`, or an `expect`-driven interactive session) and
  a verified-alive target PID, `pkttyagent` registered without error, but
  the subsequent `CheckAuthorization` call still came back as a flat denial
  with no challenge and no prompt — i.e., polkit did not consider the
  registered agent applicable to resolve this specific `system-bus-name`
  subject in this VM's minimal session configuration, for a reason not
  conclusively identified within the time budget available for this pass
  (candidates include how `system-bus-name` subjects are correlated to a
  registered `--process` agent versus a full desktop session/seat, which
  this disposable VM does not have).

**Consequence:** the D-Bus, real-caller-identity, and real-polkit-decision
halves of P0-AUTH-004/005 are proven (the interactive flag is correctly
threaded through to a real `CheckAuthorization` call with `AllowUserInteraction`
set, per the server log and `PolkitAuthorizer` source). The specific
requirement that a real authentication agent complete an interactive
challenge — the strongest-form host evidence the G1 handoff §5.2 and this
review's own audit checklist require for these two tests — was **not**
obtained. This is reported as unexecuted, not downgraded to a pass.

## 8. Result

```text
P0-AUTH-001: PASS (real system bus, real distinct OS users)
P0-AUTH-002: PASS (real polkit denial + granularity)
P0-AUTH-003: PASS (real polkit non-interactive fail-closed)
P0-AUTH-004: INCOMPLETE (interactive flag reaches real polkit; agent completion not obtained)
P0-AUTH-005: INCOMPLETE (pkttyagent registered but did not receive the challenge; no completion evidence)
SSH policy:  PASS (real SSH session, identical behavior confirmed intentional)
```
