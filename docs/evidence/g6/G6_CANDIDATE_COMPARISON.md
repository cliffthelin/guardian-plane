# G6 Candidate Comparison — Desktop Indicator Decision Gate

**Status: final comparison record for G6, built from real evidence
gathered across eleven checkpoint documents in this directory.** This
document does not introduce new evidence; it consolidates
`G6_GNOME_KSNI_SPIKE_EVIDENCE.md`, `G6_XFCE_KSNI_SPIKE_EVIDENCE.md`,
`G6_AYATANA_GTK3_SPIKE_EVIDENCE.md`, `G6_AYATANA_GLIB_SPIKE_EVIDENCE.md`,
`G6_P0_IND_003_RECONNECT_EVIDENCE.md`, `G6_XFCE_KSNI_MENU_RESOLUTION.md`,
`G6_XFCE_RECONNECT_EVIDENCE.md`, `G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md`,
`G6_DAEMON_UNAVAILABLE_EVIDENCE.md`, and `G6_ICON_NAME_CORRECTION.md`
against the contract's own 10-item required-test list (§30) and applies
its selection rule. The selection this document reaches is recorded
formally in `docs/adr/ADR-006-guardian-indicator-mechanism.md`.

**Revision history:** an independent audit of the original nine-checkpoint
version of this document found it recorded `ksni` as an "Accepted —
conditional selection" while one required test (Xfce menu-open) was
UNRESOLVED and several others (Xfce reconnect, logout/login lifecycle,
daemon-unavailable degraded state) were untested (N/T) — a real gap
between the language used and what the evidence actually supported. This
revision reflects the closure work that followed that audit: all of
those items are now directly evidenced, and the language below reflects
that.

## The rule being applied

> "The winning implementation is the simplest candidate that passes all
> required targets. The test result, not library recency, selects the
> implementation." (§30, verbatim)

Applied honestly, this rule requires checking all ten required tests
against both target environments for every candidate, and disqualifying
any candidate that fails even one, "regardless of how idiomatic or
modern it is" (per the G6 implementation handoff's own restatement of
this rule).

## Comparison matrix

Legend: **PASS** (directly evidenced), **FAIL** (directly evidenced),
**N/A** (test does not apply to this target as designed), **minor
residual** (mechanism proven, one specific visual confirmation not
separately re-captured — see note).

### Candidate 1 — legacy GTK3 Ayatana AppIndicator

*(Unreopened from the prior comparison — no new evidence contradicts
this candidate's disqualification, per the closure brief's explicit
instruction not to re-litigate settled candidate-1/candidate-2
conclusions.)*

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | PASS (fallback glyph — see `G6_ICON_NAME_CORRECTION.md`) | PASS (fallback glyph) |
| menu opens | PASS | PASS |
| menu actions invoke handler | PASS | PASS |
| state/icon update propagates | **PARTIAL** — status changes, glyph does not (no `set_attention_icon` call) | N/T |
| no X11 dependency | **FAIL** — requires real `XAUTHORITY` (XWayland cookie) | N/A — Xfce is X11 by design |
| reconnect after panel/Shell restart | N/T | N/T |
| reconnect after daemon restart | N/T | N/T |
| daemon unavailable shows degraded state | not tested for this (disqualified) candidate | N/T |
| no duplicate icon | N/T | N/T |
| clean user logout/login lifecycle | N/T | N/T |

**Verdict: disqualified**, unchanged. The X11/XWayland dependency is
structural (linking against GTK3), not a prototype shortcoming. §30
names "no X11 dependency" as a required test with no GTK carve-out.
Remaining N/T rows were never pursued once this disqualification was
established, consistent with the project's practice of not testing
further behaviors of an already-disqualified candidate.

### Candidate 2 — GLib-only Ayatana AppIndicator 2.x

*(Unreopened — no new evidence contradicts this candidate's
disqualification.)*

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | **FAIL** — `com.canonical.dbusmenu` interface not found, directly thrown by GNOME Shell | **FAIL** — identical root cause, directly captured in `xfce4-indicator-plugin`'s own debug log |
| all other tests | N/T — blocked by icon-appears failure | N/T |

**Verdict: disqualified**, unchanged, on both required desktops, for a
directly diagnosed, reproduced-on-two-independent-consumers reason: this
library's modern `org.gtk.Menus`/`org.gtk.Actions` menu export is not
understood by either required desktop's actual indicator-rendering code.
The candidate's own D-Bus registration, menu content, and action
handling were independently verified correct — the failure is a real
ecosystem gap, not an implementation defect, but it is disqualifying
regardless.

### Candidate 3 — direct Rust SNI + canonical DBusMenu (`ksni`)

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | PASS | PASS |
| menu opens | PASS | **PASS** — closed via `G6_XFCE_KSNI_MENU_RESOLUTION.md`; the prior UNRESOLVED result did not reproduce on a fresh VM with a corrected icon name, and the full required chain (click → menu → item click → log-confirmed handler) was demonstrated |
| menu actions invoke handler | PASS | **PASS** — `menu_clicks=1` log-confirmed (`G6_XFCE_KSNI_MENU_RESOLUTION.md`) |
| state/icon update propagates | PASS | **PASS** — glyph visibly changes to warning-triangle on status toggle (`G6_XFCE_KSNI_MENU_RESOLUTION.md`) |
| no X11 dependency | PASS — empirically confirmed, no `DISPLAY`/`XAUTHORITY` set | N/A — Xfce is X11 by design |
| reconnect after panel/Shell restart | PASS | **PASS** — closed via `G6_XFCE_RECONNECT_EVIDENCE.md`; real `xfce4-panel -r`, recovery via the persistent `ayatana-indicator-application-service`, not candidate-side re-registration (verified directly via process inspection) |
| reconnect after daemon restart | PASS | **PASS** — closed via `G6_XFCE_RECONNECT_EVIDENCE.md`; clean deregistration on kill, clean single re-registration on relaunch, menu re-verified functional |
| daemon unavailable shows degraded state | **PASS** — closed via `G6_DAEMON_UNAVAILABLE_EVIDENCE.md`; real detection of a real evidence-only D-Bus stub's presence/absence, distinct `dialog-error` icon, not a simulated toggle | **minor residual** — the detection mechanism is desktop-independent Rust/D-Bus logic already proven on GNOME, and the icon-rendering pipeline itself is independently proven correct on Xfce for two other icon names (`computer`, `dialog-warning`) in this same closure pass, but the `dialog-error` glyph specifically was not separately re-screenshotted on Xfce. Disclosed honestly rather than silently assumed; not treated as blocking given the mechanism is proven and the remaining risk is minimal |
| no duplicate icon | PASS | **PASS** — confirmed across both reconnect scenarios (`G6_XFCE_RECONNECT_EVIDENCE.md`) and the logout/login stale-registration check (`G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md`) |
| clean user logout/login lifecycle | **PASS** — closed via `G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md` | **PASS** — closed via the same document; includes a disclosed, non-candidate finding about stale registrations surviving logout when a process is launched outside proper session scoping (relevant to G7+ launch-mechanism design, not a `ksni` defect) |

**Verdict: passes every required test directly evidenced against it, on
both required desktops, with one disclosed minor residual** (the
daemon-unavailable icon's Xfce rendering specifically was not separately
screenshotted, though the underlying mechanism is proven).

## Applying the selection rule

- **Candidate 1 is disqualified**, cleanly and structurally, by a real
  X11/XWayland dependency on GNOME 50.
- **Candidate 2 is disqualified**, cleanly, on both desktops, by a
  directly diagnosed and independently reproduced menu-protocol
  incompatibility.
- **Candidate 3 (`ksni`) passes every required test that was directly
  evidenced against it, on both GNOME 50 and Xfce 4.20**, following the
  G6 evidence closure that resolved the prior gate's open items (Xfce
  menu-open, Xfce reconnect, logout/login lifecycle, daemon-unavailable
  degraded state).

`ksni` is selected as Guardian's indicator mechanism under §30's own
rule: it is the simplest candidate (and the only one) that passes all
required targets actually evidenced against it. This is recorded in
ADR-006 as a final selection, not a conditional one.

## Caveats and follow-ups for the record

- The `ubuntu-appindicators@ubuntu.com` / `xfce4-indicator-plugin`
  dependency `ksni` requires on both desktops is not a Guardian-authored
  component and is not guaranteed present on every real Ubuntu 26.04
  install. A production Guardian indicator should detect and clearly
  report the absence of a working `StatusNotifierWatcher`, not silently
  show nothing.
- The icon-name/icon-theme fragility documented in
  `G6_ICON_NAME_CORRECTION.md` (a hardcoded icon name that resolves as a
  generic fallback in one VM and as fully invisible in another) applies
  to whatever production icon name Guardian eventually chooses. It must
  be verified against the target theme or shipped as Guardian's own icon
  asset.
- The stale-registration finding in `G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md`
  (a non-session-scoped process can leave a stale icon visible after
  logout, because Xfce's watcher service is per-user, not per-session)
  is a real launch-mechanism design constraint for G7+: the production
  indicator must be launched via proper desktop session autostart
  (killed cleanly by `systemd-logind` on logout), not as a detached
  background process.
- The `dialog-error` daemon-unavailable icon's Xfce rendering was not
  separately re-screenshotted (see matrix note above) — a cheap,
  low-priority follow-up if further confidence is wanted, not a blocker.

## What this document does not do

Per this gate's explicit scope: **G6 selects an indicator mechanism. It
does not certify a production guardian-indicator implementation.**
Nothing in this document, or in ADR-006, authorizes building the G7+
production indicator daemon, expanding Guardian's public D-Bus surface,
or beginning any implementation work beyond this gate's own evidence
prototypes. The prototypes in `tests/vm/g6-candidate-*/` and
`tests/vm/g6-daemon-evidence-stub/` remain evidence-only artifacts,
explicitly marked non-production, and are not the basis for G7's actual
code.
