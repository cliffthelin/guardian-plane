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
Status:             Accepted — conditional selection (see ADR-006 Caveats)
```

Candidate 1 (legacy GTK3 Ayatana AppIndicator) and candidate 2
(GLib-only Ayatana AppIndicator 2.x) were both disqualified by real,
directly-evidenced failures against §30's required-test list — an
X11/XWayland dependency on GNOME 50/Wayland for candidate 1, and a
menu-protocol incompatibility with both required desktops' actual
indicator-rendering code for candidate 2. `ksni` is the only candidate
with no directly-evidenced FAIL on any required test actually run
against it, and is selected on that basis, with open items (Xfce
menu-open UNRESOLVED, several tests not yet exercised against a real
daemon or a real logout/login cycle) carried forward explicitly rather
than silently dropped. Full reasoning: `ADR-006`.

## Evidence-gathering method

All evidence was gathered against real, disposable GNOME 50 and Xfce
4.20 sessions in qemu VMs (bare `qemu-system-x86_64` + QMP
`screendump`/`input-send-event`, no VNC client, no mocks, no headless
fixtures) — never against the primary workstation, and never inferred
from library documentation alone. Every VM was torn down and its overlay
disk destroyed after evidence capture; base cloud images were never
written to. Six checkpoint documents record this work in full, each with
explicit environment details, real captured process output, and
provenance-labeled screenshots:

```text
docs/evidence/g6/G6_GNOME_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_XFCE_KSNI_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GTK3_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_AYATANA_GLIB_SPIKE_EVIDENCE.md
docs/evidence/g6/G6_P0_IND_003_RECONNECT_EVIDENCE.md
docs/evidence/g6/G6_CANDIDATE_COMPARISON.md   (consolidated matrix)
```

## Required-test status (§30), consolidated

See `G6_CANDIDATE_COMPARISON.md` for the full per-candidate,
per-desktop matrix. Summary for the selected mechanism (`ksni`):

```text
icon appears                          GNOME PASS / Xfce PASS
menu opens                            GNOME PASS / Xfce UNRESOLVED
menu actions invoke handler           GNOME PASS / Xfce N/T (blocked)
state/icon update propagates          GNOME PASS / Xfce N/T
no X11 dependency                     GNOME PASS / Xfce N/A (X11 by design)
reconnect after panel/Shell restart   GNOME PASS / Xfce N/T
reconnect after daemon restart        GNOME PASS / Xfce N/T
daemon unavailable shows degraded     indirect only (no real daemon exists yet)
no duplicate icon                     GNOME PASS / Xfce N/T
clean user logout/login lifecycle     N/T (both desktops)
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

## Open items explicitly carried forward (not silently dropped)

```text
1. Xfce 4.20 menu-open for ksni: UNRESOLVED. Investigate before
   Xfce-targeted G7+ work begins on top of ksni.
2. "Daemon unavailable shows degraded state": must be implemented and
   directly tested against the real Guardian daemon once one exists.
3. "Clean user logout/login lifecycle": not tested for any candidate
   this gate; close before or during G7+ work.
4. Xfce-side reconnect (panel restart, daemon restart) and Xfce-side
   no-X11-dependency confirmation for ksni: not separately re-tested
   from GNOME this gate (explicit reduced-scope decision); the
   underlying mechanism is expected to be desktop-independent but this
   has not been separately confirmed.
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
docs/evidence/g6/G6_CANDIDATE_COMPARISON.md
tests/vm/g6-candidate-ksni/
tests/vm/g6-candidate-ayatana-gtk3/
tests/vm/g6-candidate-ayatana-glib/
```
