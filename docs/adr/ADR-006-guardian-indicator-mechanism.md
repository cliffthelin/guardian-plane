# ADR-006: Guardian desktop indicator mechanism

- Status: Accepted
- Date: 2026-09-01
- Governing gate: G6 — Desktop Indicator Decision

## Revision note

An independent audit of an earlier version of this ADR found its
`Status: Accepted — conditional selection` line functioned as a
substitute for a required PASS: `ksni`'s Xfce menu-open result was
UNRESOLVED (not PASS) at the time, and several other required tests
(Xfce reconnect, clean logout/login lifecycle, daemon-unavailable
degraded state) were untested. That version's status line and body text
were internally honest about these gaps in its own Caveats section, but
the top-line status overclaimed relative to what §38's "no gate advances
until all required tests pass" model actually requires. This revision
follows the evidence-closure work that resolved every one of those
items (see `docs/evidence/g6/G6_CANDIDATE_COMPARISON.md`'s revision
history and the closure documents it cites) and reflects the outcome:
`ksni` now passes every required test directly evidenced against it, on
both target environments, with one disclosed minor residual (noted
below). The status line is updated accordingly, from "Accepted —
conditional selection" to "Accepted."

## Context

G6 is an early decision/spike gate (not a production-implementation
gate — see the §30/§39 ordering resolution recorded in the G6 handoffs)
that must select, with real evidence rather than library-recency
preference, which of three candidate mechanisms Guardian's future
desktop tray indicator should be built on:

1. Legacy GTK3 Ayatana AppIndicator (`libayatana-appindicator3`)
2. GLib-only Ayatana AppIndicator 2.x (`libayatana-appindicator-glib`)
3. Direct Rust SNI + canonical DBusMenu (`ksni`)

All three were built as real, minimal, explicitly non-production
prototypes (`tests/vm/g6-candidate-*/`) and tested against real GNOME
50/Wayland and Xfce 4.20 sessions in disposable qemu VMs, per contract
§30's required-test list. No mocks, headless fixtures, or synthetic D-Bus
sessions were used for any pass/fail determination — see the ten
checkpoint evidence documents in `docs/evidence/g6/` for full raw
evidence, and `G6_CANDIDATE_COMPARISON.md` for the consolidated matrix
this ADR is built from.

## Decision

**`ksni` (direct Rust SNI + canonical DBusMenu) is selected** as
Guardian's desktop indicator mechanism.

This is the only candidate that passes every required test in §30's
list that was directly evidenced against it, on both GNOME 50/Wayland
and Xfce 4.20, and the only one not disqualified by a structural,
uncorrectable defect:

- The legacy GTK3 Ayatana AppIndicator candidate is **disqualified** by a
  real X11/XWayland dependency on GNOME 50/Wayland (`app_indicator_new`
  requires `gtk::init()`, which fails without a valid `XAUTHORITY`
  pointing at the session's real XWayland auth cookie) — "no X11
  dependency" is one of §30's ten named required tests, with no
  carve-out for GTK-linked candidates, and this is a property of linking
  against GTK3 itself, not a fixable prototype shortcoming.
- The GLib-only Ayatana AppIndicator 2.x candidate is **disqualified** on
  both required desktops by a directly diagnosed, independently
  reproduced protocol incompatibility: it exports its menu via the
  modern `org.gtk.Menus`/`org.gtk.Actions` D-Bus interfaces, which
  neither GNOME's `ubuntu-appindicators@ubuntu.com` extension nor Xfce's
  `xfce4-indicator-plugin` understand — both still expect the legacy
  `com.canonical.dbusmenu` protocol. The candidate's own D-Bus
  registration, menu content, and action wiring were independently
  verified correct via direct D-Bus calls; the failure is a real
  ecosystem gap between this library and both required desktop
  consumers, not an implementation defect, but it is disqualifying
  regardless.
- `ksni` passes every required test directly evidenced against it, on
  both GNOME 50 and Xfce 4.20: icon appears, menu opens, menu actions
  invoke the handler, state/icon update propagates (favorably, with no
  extra API call needed — a real, disclosed contrast with the GTK3
  candidate's status-without-icon-change finding), no X11 dependency
  (directly confirmed, GNOME only — moot on Xfce per §30's own X11-by-
  design framing), reconnect after panel/Shell restart (on GNOME, via an
  extension disable/enable cycle — the correct Wayland analog, since no
  in-place shell restart exists there; on Xfce, via a real
  `xfce4-panel -r`, with recovery independently traced to the
  persistent `ayatana-indicator-application-service`, not candidate-side
  logic), reconnect after daemon restart (both desktops, real kill +
  relaunch, clean deregistration confirmed via D-Bus introspection, no
  duplicate), daemon unavailable shows degraded state (real detection of
  a real evidence-only D-Bus stub's presence/absence, a distinct
  `dialog-error` icon, not a simulated toggle), no duplicate icon
  (confirmed across every reconnect and logout/login scenario), and
  clean user logout/login lifecycle (both desktops, including a real
  logout, a fresh relaunch, and confirmation of exactly one indicator
  afterward).

## Minor disclosed residual (not treated as blocking)

The `dialog-error` daemon-unavailable icon's rendering was directly
confirmed on GNOME (`G6_DAEMON_UNAVAILABLE_EVIDENCE.md`) but not
separately re-screenshotted on Xfce. The detection mechanism itself is
desktop-independent Rust/D-Bus polling logic already proven correct on
GNOME, and the Xfce icon-rendering pipeline is independently proven
correct for two *other* icon names (`computer`, `dialog-warning`) in
this same closure pass — so the residual risk is low, but it is
disclosed rather than silently assumed. A cheap follow-up, not a
blocker to this selection.

## Why not the others, restated plainly

- Legacy GTK3 Ayatana AppIndicator: works, but requires X11/XWayland even
  in a pure-Wayland GNOME session — disqualifying per §30's explicit
  required-test list, and not something the candidate's own code could
  fix (it is a property of the C library it links against).
- GLib-only Ayatana AppIndicator 2.x: despite being the library its own
  predecessor's runtime deprecation warning recommends switching to, it
  is the only candidate that fails to render on either required desktop
  out of the box, because neither desktop's actual indicator-consuming
  code understands its modern menu-export protocol. Real irony, real
  disqualification.

`ksni` was not favored for being pure Rust, or for any recency/idiom
preference — both alternatives were given full, fair evaluation attempts
(including, for candidate 2, direct D-Bus verification that its own
implementation is correct, independent of the desktops that fail to
render it) and were disqualified strictly on §30's own required-test
results.

## Real findings carried forward for G7+ (not this ADR's scope to act on)

- The `ubuntu-appindicators@ubuntu.com` / `xfce4-indicator-plugin`
  dependency `ksni` requires on both desktops is not a Guardian-authored
  component and is not guaranteed present on every real Ubuntu 26.04
  install. A production Guardian indicator should detect and clearly
  report the absence of a working `StatusNotifierWatcher`, not silently
  show nothing.
- Icon names must be verified present in the target icon theme before
  use, or Guardian should ship its own icon asset — see
  `G6_ICON_NAME_CORRECTION.md` for the concrete failure mode this
  guards against (a name that resolves as a generic fallback in one
  environment and renders with zero visible content, with zero errors,
  in another).
- The production indicator must be launched via proper desktop session
  autostart (cleaned up by `systemd-logind` on logout), not as a
  detached background process — see
  `G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md`'s stale-registration finding
  for the concrete failure mode this guards against on Xfce.
- `ksni`'s reconnect-after-panel-restart behavior depends on the host
  desktop's own recovery mechanism (GNOME extension fallback bus-scan;
  Xfce's persistent indicator-application service), not on `ksni`
  proactively re-announcing itself. Both were independently confirmed
  in this gate's evidence, but this dependency should be understood by
  whoever builds G7's production indicator on top of `ksni`.

## What this ADR does not do

Consistent with this gate's explicit scope: **G6 selects an indicator
mechanism. It does not certify a production guardian-indicator
implementation.** This ADR does not authorize building the G7+ production
indicator daemon, expanding Guardian's public D-Bus surface, or beginning
any implementation work beyond this gate's own evidence prototypes. The
`ksni`-based prototype in `tests/vm/g6-candidate-ksni/` and the minimal
`tests/vm/g6-daemon-evidence-stub/` used to evidence the
daemon-unavailable required test are evidence-only artifacts (each
explicitly marked as such in its own module doc comment) and are not the
basis for G7's actual code — G7 must build its own production indicator
daemon using `ksni` as a library dependency, subject to normal G7 TDD
discipline, not by promoting either spike artifact directly.

## Evidence

Full raw evidence, including screenshots with explicit provenance
labeling, VM setup/teardown records, and D-Bus introspection captures:

- `docs/evidence/g6/G6_GNOME_KSNI_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_XFCE_KSNI_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_AYATANA_GTK3_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_AYATANA_GLIB_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_P0_IND_003_RECONNECT_EVIDENCE.md` (GNOME)
- `docs/evidence/g6/G6_XFCE_KSNI_MENU_RESOLUTION.md` (closes Xfce menu-open)
- `docs/evidence/g6/G6_XFCE_RECONNECT_EVIDENCE.md` (closes Xfce reconnect)
- `docs/evidence/g6/G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md` (both desktops)
- `docs/evidence/g6/G6_DAEMON_UNAVAILABLE_EVIDENCE.md` (real stub-based detection)
- `docs/evidence/g6/G6_ICON_NAME_CORRECTION.md` (evidentiary-precision correction)
- `docs/evidence/g6/G6_CANDIDATE_COMPARISON.md` (consolidated matrix this
  ADR's decision is drawn from)
