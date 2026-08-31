# Guardian G2 — Privilege Requirement Inventory

Classifies each known/planned Guardian capability area by the privilege it
actually requires, per `docs/guardian/30_TDD/GUARDIAN_G2_IMPLEMENTATION_HANDOFF.md`
§5. Built before either prototype's hardening manifest, as required.

Categories:

```text
no privilege                    — Guardian needs nothing beyond an ordinary process
provider-owned authorization    — Guardian calls an already-authoritative
                                    privileged service that performs its own
                                    authorization; Guardian itself needs no
                                    elevated privilege for this area
Guardian polkit authorization   — Guardian's own method is polkit-gated, but
                                    the action needs no further OS privilege
                                    once authorized
specific device/file access     — a narrow, named filesystem/device path
specific Linux capability       — a named, narrow capability
root/system privilege           — no narrower alternative found/researched
unknown — requires host research
```

| Capability area | Classification | Basis |
|---|---|---|
| systemd/service management (read unit state) | Guardian polkit authorization | `org.freedesktop.systemd1` read methods (`ListUnits`, `GetUnit`) require no special privilege beyond the system bus; systemd's own polkit actions gate *write* operations (start/stop/restart) at the provider — those specific writes are **provider-owned authorization**, not Guardian-held privilege, since systemd1 itself performs the polkit check. |
| systemd/service management (start/stop/restart units) | provider-owned authorization | `org.freedesktop.systemd1.Manager` methods are already polkit-gated by systemd itself (`org.freedesktop.systemd1.manage-units` etc.); a correctly-implemented Guardian adapter calls systemd's own D-Bus API and lets systemd perform its own authorization. Guardian needs no elevated privilege for this — only permission to call the D-Bus method, which is itself polkit-mediated by the provider. |
| cgroups (resource limits, transient scopes) | provider-owned authorization | Transient scope/slice creation goes through `systemd1.Manager.StartTransientUnit`, which is provider-owned per above. Direct cgroupfs writes (bypassing systemd) would require **specific device/file access** or **root/system privilege** — Guardian should not do this; TDD contract §17 requires using systemd's transient-unit API, not hand-written cgroup writes. |
| PSI (`/proc/pressure/*`) | no privilege | World-readable on stock Ubuntu 26.04.1 for the "some"/"full" pressure lines; the trigger-registration mechanism (`poll()` on an fd from `open()` with a written trigger spec) is also unprivileged for unprivileged monitors, per kernel PSI docs, though the kernel imposes an additional resource-use constraint on unprivileged triggers (TDD contract §16 research). No capability needed. |
| UDisks (drive/block topology, read) | no privilege | `org.freedesktop.UDisks2` read properties/methods require no privilege; UDisks exposes topology to any bus client. |
| UDisks (`PowerOff()`) | provider-owned authorization | UDisks' own `Drive.PowerOff()` requires and performs its own polkit check (`org.freedesktop.udisks2.power-off-drive` or the system-installed equivalent) before acting. Guardian calls it as an ordinary D-Bus client; Guardian itself needs no privilege escalation for this specific write. |
| BPF/eBPF | unknown — requires host research | Not exercised on this workstation or in either G2 VM session; modern kernels gate `bpf()` syscall use via `CAP_BPF`/`CAP_PERFMON`/`CAP_SYS_ADMIN` depending on program type and `kernel.unprivileged_bpf_disabled` sysctl state on Ubuntu 26.04.1, which was not empirically probed this pass. Deferred, marked unknown rather than guessed. |
| thermald (read policy) | no privilege | `org.freedesktop.thermald` read-only properties require no privilege. |
| thermald (write policy) | unknown — requires host research | Deferred per TDD contract §15; thermald's D-Bus write surface (if any beyond its own config file) was not inspected this pass. |
| power-profiles-daemon (read active profile) | no privilege | `net.hadess.PowerProfiles` properties are readable by any bus client. |
| power-profiles-daemon (`HoldProfile`) | Guardian polkit authorization | `HoldProfile`/`ReleaseProfile` are ordinary (non-polkit-gated per the D-Bus introspection reviewed in the TDD research) methods any client may call; if Guardian wants to gate *its own* use of this behind a Guardian-level policy decision (e.g. "should Guardian be allowed to hold a profile at all"), that gate is Guardian's own polkit action, not a privilege escalation — classified as `Guardian polkit authorization`, needing no OS-level privilege once authorized. |
| UPower (battery/UPS/power devices) | no privilege | `org.freedesktop.UPower` read surface is unprivileged. |
| NVML/NVIDIA | unknown — requires host research | Deferred; no NVIDIA hardware available in the disposable VM images used this pass. |
| fwupd | unknown — requires host research | Deferred per TDD contract §28; `org.freedesktop.fwupd`'s write methods were not inspected this pass. |
| NetworkManager | provider-owned authorization (for writes); no privilege (for reads) | NetworkManager's own D-Bus API performs its own polkit checks (`org.freedesktop.NetworkManager.*` actions) for configuration changes, including its checkpoint/rollback API (TDD contract §9 research). Reads are unprivileged. |
| journald (read) | no privilege | Reading the journal via `sd-journal`/`journalctl` as a member of `systemd-journal` group, or via unprivileged read where permitted, requires no elevated privilege; exact group membership needs is a packaging detail, not a process-privilege one. |
| journald (rotation/capacity policy) | unknown — requires host research | journald's own capacity limits are config-file-driven, not D-Bus-writable in the versions inspected during the original research pass; whether Guardian would need file-write privilege to `/etc/systemd/journald.conf.d/` or an equivalent was not re-verified this pass. Deferred. |
| AccountsService (read session list) | no privilege | `org.freedesktop.Accounts` read properties are unprivileged. |
| AccountsService (`SetSession`) | provider-owned authorization | AccountsService performs its own polkit check for session-affecting writes (TDD contract §12 research); Guardian calls the provider's own gated method. |
| apt/package state | unknown — requires host research | Deferred; no D-Bus package-management provider was inspected this pass (PackageKit was not probed). |
| generic hardware control | unknown — requires host research | Deliberately left unclassified rather than assuming any blanket device access; TDD contract explicitly forbids treating "system-management feature" as implying root without evidence. |
| I/O Guardian (storage power-off) | provider-owned authorization | UDisks-mediated per the `UDisks PowerOff()` row above; deferred as a real feature until the I/O Guardian module phase, per TDD contract §28. |
| USB Security / usbguard | unknown — requires host research | Deferred; usbguard's D-Bus interface and its own authorization model were not inspected this pass. |

## Summary counts

```text
Total capability areas classified:        20
no privilege:                              8
provider-owned authorization:              6
Guardian polkit authorization:             1
specific device/file access:               0
specific Linux capability:                 0
root/system privilege:                     0
unknown — requires host research:          8 (BPF/eBPF, thermald-write, NVML,
                                              fwupd, journald-rotation,
                                              apt/package state, generic
                                              hardware control, usbguard)
```

## What this inventory means for the topology comparison

**No capability area inventoried this pass requires Guardian to hold
`root`/system privilege or a broad Linux capability directly.** Every write
path identified either delegates to an already-authoritative provider
(`provider-owned authorization`) or is Guardian's own polkit-gated bounded
action requiring no further OS privilege (`Guardian polkit authorization`,
matching G1's `guardian.test.*` pattern exactly). This is a materially
important finding: it means **neither** Model A nor Model B currently has a
demonstrated need for broad root or `CAP_SYS_ADMIN` for the capability
areas actually researched. The eight `unknown` areas are the ones that
could change this conclusion — they are honestly marked unknown rather than
assumed safe, and any future gate that implements one of them must redo
this classification with real research before assuming either topology's
current capability set remains sufficient.

This inventory directly bounds the G2 bounded test operation's own privilege
requirement (§6/§7 below): since it stands in for a `Guardian polkit
authorization`-class action (no further OS privilege once authorized), Model
A's and Model B's evidence should — if this inventory is accurate — show
that an *empty* `CapabilityBoundingSet=` is sufficient for the bounded
operation itself, and any capability need discovered during testing
localizes the disagreement precisely.
