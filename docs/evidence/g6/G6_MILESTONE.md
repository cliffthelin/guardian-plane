# Guardian Phase 0 — G6 Milestone Record

## Project identity

```text
Project display name:  Guardian Plane
Repository slug:       guardian-plane
GitHub repository:     github.com/cliffthelin/guardian-plane
Permanent D-Bus namespace: io.github.cliffthelin.Guardian1
```

## Gate shape — read before comparing to G0-G5's milestone records

G6 is a **decision/spike gate** (§30, "Desktop indicator decision gate"),
not an implementation gate. It has no `guardian-core` module, no
normative test IDs implemented as `cargo test` assertions, and — per
explicit instruction, consistent with the gate's own nature — **is not
tagged**. Its deliverable is a real-evidence-backed decision (recorded in
`docs/adr/ADR-006-guardian-indicator-mechanism.md`), not a merged
production code change. The evidence prototypes in
`tests/vm/g6-candidate-*/` are explicitly marked non-production and are
excluded from the Cargo workspace; they do not affect `cargo test
--workspace`, which remains unchanged at 189 passed throughout this gate.

## Decision

```text
Gate:               G6 — Desktop Indicator Decision
Selected mechanism: ksni (direct Rust SNI + canonical DBusMenu)
Decision record:    docs/adr/ADR-006-guardian-indicator-mechanism.md
Status:             Accepted
```

Candidate 1 (legacy GTK3 Ayatana AppIndicator) and candidate 2
(GLib-only Ayatana AppIndicator 2.x) were both disqualified by real,
directly-evidenced failures against §30's required-test list — an
X11/XWayland dependency on GNOME 50/Wayland for candidate 1, and a
menu-protocol incompatibility with both required desktops' actual
indicator-rendering code for candidate 2. `ksni` passes every required
test directly evidenced against it, on both GNOME 50 and Xfce 4.20, and
is selected on that basis.

**Revision note:** an independent audit of an earlier version of this
record found it, and the accompanying ADR-006, used "Accepted —
conditional selection" language in a way that functioned as a substitute
for a required PASS, while the Xfce menu-open result was genuinely
UNRESOLVED and several other required tests were untested. A closure
pass (documented in `G6_CANDIDATE_COMPARISON.md`'s revision history and
the four closure documents it cites) resolved every one of those items
with real evidence; this record now reflects that outcome. Full
reasoning: `ADR-006`.

## Evidence-gathering method

All evidence was gathered against real, disposable GNOME 50 and Xfce
4.20 sessions in qemu VMs (bare `qemu-system-x86_64` + QMP
`screendump`/`input-send-event`, no VNC client, no mocks, no headless
fixtures) — never against the primary workstation, and never inferred
from library documentation alone. Every VM was torn down and its overlay
disk destroyed after evidence capture; base cloud images were never
written to. Eleven checkpoint documents record this work in full, each
with explicit environment details, real captured process output, and
provenance-labeled screenshots:

```text
docs/evidence/g6/G6_GNOME_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_XFCE_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GTK3_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GLIB_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_P0_IND_003_RECONNECT_EVIDENCE.md
docs/evidence/g6/G6_XFCE_KSNI_MENU_RESOLUTION.md
docs/evidence/g6/G6_XFCE_RECONNECT_EVIDENCE.md
docs/evidence/g6/G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md
docs/evidence/g6/G6_DAEMON_UNAVAILABLE_EVIDENCE.md
docs/evidence/g6/G6_ICON_NAME_CORRECTION.md
docs/evidence/g6/G6_CANDIDATE_COMPARISON.md   (consolidated matrix)
```

## Required-test status (§30), consolidated

See `G6_CANDIDATE_COMPARISON.md` for the full per-candidate,
per-desktop matrix. Summary for the selected mechanism (`ksni`):

```text
icon appears                          GNOME PASS / Xfce PASS
menu opens                            GNOME PASS / Xfce PASS
menu actions invoke handler           GNOME PASS / Xfce PASS
state/icon update propagates          GNOME PASS / Xfce PASS
no X11 dependency                     GNOME PASS / Xfce N/A (X11 by design)
reconnect after panel/Shell restart   GNOME PASS / Xfce PASS
reconnect after daemon restart        GNOME PASS / Xfce PASS
daemon unavailable shows degraded     GNOME PASS (real stub-based detection) /
                                       Xfce minor residual (mechanism proven,
                                       icon not separately re-screenshotted)
no duplicate icon                     GNOME PASS / Xfce PASS
clean user logout/login lifecycle     GNOME PASS / Xfce PASS
```

## Real findings this gate surfaced, worth preserving for G7+

Findings that materially affect how the eventual production indicator
must be built, not just this gate's own scoring:

- **candidate 1's X11/XWayland dependency is structural** — a property
  of GTK3 itself, not a code defect. Confirms `ksni`'s selection is not
  merely "simplest," but the only candidate actually free of this
  dependency, empirically confirmed (launched with no `DISPLAY`/
  `XAUTHORITY` set, fully functional).
- **candidate 2's ecosystem gap is real and ironic** — the library its
  own predecessor's runtime deprecation warning recommends is the one
  candidate that fails to render on either required desktop, because
  neither desktop's actual indicator-consuming code understands its
  modern `org.gtk.Menus`/`org.gtk.Actions` menu export. Both GNOME's
  extension and Xfce's plugin independently fail at the identical
  `com.canonical.dbusmenu`-interface boundary — verified via GNOME
  Shell's own exception log and Xfce's indicator-plugin debug log
  respectively.
- **icon-name/icon-theme fragility is a real production hazard**,
  discovered while establishing a reconnect-testing baseline: a
  hardcoded icon name (`"emblem-default"`) that renders correctly in one
  provisioned VM can silently fail to render — zero errors, full
  successful D-Bus registration, just an invisible icon — in another,
  because current Adwaita dropped that specific icon. The eventual
  production indicator must verify its icon name against the actual
  target theme (or ship its own icon asset), not assume a
  freedesktop-icon-naming-spec name is universally present.
- **GNOME Wayland has no in-place shell/panel restart** — the "panel
  restart" reconnect scenario required substituting an
  `ubuntu-appindicators` extension disable/enable cycle, since GNOME's
  historical X11-only "Restart Shell" mechanism does not exist under
  Wayland. This is a real platform difference the eventual production
  reconnect-handling logic (and its own test suite, whenever G7+ builds
  one) must account for.
- **`ksni`'s reconnect-after-panel-restart success depends on
  GNOME-extension-side recovery logic**, not proactive re-announcement
  by `ksni` itself — the extension's own `_seekStatusNotifierItems`
  fallback bus-scan (explicitly written for indicators that "do not
  re-register... when the plugin is enabled/disabled") is what recovers
  the icon. A production indicator relying on this should not assume
  every StatusNotifierWatcher implementation provides an equivalent
  fallback.

## Open items closed during evidence closure

All four items originally carried forward here (Xfce menu-open, Xfce
reconnect, logout/login lifecycle, daemon-unavailable degraded state)
were closed with real evidence — see `G6_XFCE_KSNI_MENU_RESOLUTION.md`,
`G6_XFCE_RECONNECT_EVIDENCE.md`, `G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md`,
and `G6_DAEMON_UNAVAILABLE_EVIDENCE.md` respectively.

## Residual items for G7+ (not blocking this gate's acceptance)

```text
1. daemon-unavailable icon rendering on Xfce specifically: the detection
   mechanism and icon-rendering pipeline are both independently proven,
   but the dialog-error glyph itself was not separately re-screenshotted
   on Xfce. Low-priority follow-up, not a blocker.
2. "Daemon unavailable shows degraded state" was evidenced against a
   minimal, explicitly non-production evidence-only D-Bus stub (per the
   G6 handoff's own explicit permission for this) -- wiring the same
   mechanism against the real Guardian daemon's own eventual health
   signal is G7+ protocol-design work.
3. A stale-registration hazard was found on Xfce (see
   G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md): a non-session-scoped
   indicator process can leave a stale icon visible after logout,
   because Xfce's watcher service is per-user, not per-session. G7's
   production indicator must be launched via proper desktop session
   autostart, not as a detached process.
```

## Scope boundary (restated)

Per this gate's explicit instructions: **G6 selects an indicator
mechanism. It does not certify a production guardian-indicator
implementation.** No G7 production daemon work, no G8 provider work, no
G9 client/packaging work, no public D-Bus expansion, and no privileged-
helper changes were performed or authorized as part of this gate. The
evidence prototypes in `tests/vm/g6-candidate-*/` remain non-production
artifacts; G7 must build its own production indicator using `ksni` as a
library dependency, under normal G7 TDD discipline.

## Evidence index (referenced, not duplicated here)

```text
docs/guardian/30_TDD/GUARDIAN_G6_IMPLEMENTATION_HANDOFF.md
docs/guardian/30_TDD/GUARDIAN_G6_INDEPENDENT_REVIEW_HANDOFF.md
docs/adr/ADR-006-guardian-indicator-mechanism.md
docs/evidence/g6/G6_GNOME_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_XFCE_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GTK3_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GLIB_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_P0_IND_003_RECONNECT_EVIDENCE.md
docs/evidence/g6/G6_XFCE_KSNI_MENU_RESOLUTION.md
docs/evidence/g6/G6_XFCE_RECONNECT_EVIDENCE.md
docs/evidence/g6/G6_LOGOUT_LOGIN_LIFECYCLE_EVIDENCE.md
docs/evidence/g6/G6_DAEMON_UNAVAILABLE_EVIDENCE.md
docs/evidence/g6/G6_ICON_NAME_CORRECTION.md
docs/evidence/g6/G6_CANDIDATE_COMPARISON.md
tests/vm/g6-candidate-ksni/
tests/vm/g6-candidate-ayatana-gtk3/
tests/vm/g6-candidate-ayatana-glib/
tests/vm/g6-daemon-evidence-stub/
```
