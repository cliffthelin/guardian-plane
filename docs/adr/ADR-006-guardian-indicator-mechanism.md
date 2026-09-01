# ADR-006: Guardian desktop indicator mechanism

- Status: Accepted — conditional selection (see Caveats)
- Date: 2026-09-01
- Governing gate: G6 — Desktop Indicator Decision

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
sessions were used for any pass/fail determination — see the five
checkpoint evidence documents in `docs/evidence/g6/` for full raw
evidence, and `G6_CANDIDATE_COMPARISON.md` for the consolidated matrix
this ADR is built from.

## Decision

**`ksni` (direct Rust SNI + canonical DBusMenu) is selected** as
Guardian's desktop indicator mechanism.

This is the only candidate with no directly-evidenced FAIL on any
required test it was actually run against, and the only one not
disqualified by a structural, uncorrectable defect:

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
- `ksni` passed every required test actually run against it on GNOME 50:
  icon appears, menu opens, menu actions invoke the handler, state/icon
  update propagates (favorably, with no extra API call needed — a real,
  disclosed contrast with the GTK3 candidate's status-without-icon-change
  finding), no X11 dependency (directly confirmed: launched with only
  `DBUS_SESSION_BUS_ADDRESS` set, no `DISPLAY`/`XAUTHORITY`, fully
  functional regardless), reconnect after panel/Shell restart, reconnect
  after daemon restart, and no duplicate icon across either reconnect
  scenario.

## Caveats — this selection is conditional, not a clean sweep

Per the contract's own instruction to report honestly rather than force
a tidy result, this ADR does not claim `ksni` passed every one of §30's
ten required tests with total certainty on both target environments:

- **Xfce 4.20 menu-open is UNRESOLVED for `ksni`**, not PASS. Three real
  synthetic-click attempts at confirmed icon coordinates produced no
  visible menu and zero `com.canonical.dbusmenu` D-Bus traffic. The
  identical click technique worked immediately for the (disqualified)
  GTK3 candidate on the same Xfce setup, which narrows this toward a
  candidate-specific interaction gap rather than an evidence-methodology
  flaw, but does not fully resolve it. **This must be investigated and
  resolved before Xfce-targeted G7+ production work begins on top of
  `ksni`.**
- "Daemon unavailable shows degraded state" was evidenced only indirectly
  (via `ksni`'s status-toggle mechanism), since no real Guardian daemon
  exists yet at G6 to be unavailable against. The eventual G7+ indicator
  built on `ksni` must implement and directly test this behavior against
  the real daemon.
- "Clean user logout/login lifecycle" was not tested for any candidate at
  this gate.
- Xfce-side reconnect (panel restart, daemon restart) and Xfce-side
  no-X11-dependency confirmation were not re-tested separately from
  GNOME, per this gate's explicit reduced-scope decision (see
  `G6_P0_IND_003_RECONNECT_EVIDENCE.md`'s scope note) — the underlying
  mechanism is expected to be desktop-independent since it is governed
  by `ksni`'s own D-Bus registration lifecycle, but this has not been
  separately confirmed on Xfce.

`ksni` remains the selection despite these gaps because both alternative
candidates are already disqualified project-wide by clean, directly
evidenced failures — there is no candidate to fall back to, and holding
the gate open indefinitely pending a fully exhaustive matrix would
contradict the same "record honestly, don't force a false result"
principle in the other direction (withholding a real, well-supported
answer). The open items above are carried forward explicitly as
pre-conditions for Xfce-targeted G7+ work, not silently dropped.

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

## What this ADR does not do

Consistent with this gate's explicit scope: **G6 selects an indicator
mechanism. It does not certify a production guardian-indicator
implementation.** This ADR does not authorize building the G7+ production
indicator daemon, expanding Guardian's public D-Bus surface, or beginning
any implementation work beyond this gate's own evidence prototypes. The
`ksni`-based prototype in `tests/vm/g6-candidate-ksni/` is an
evidence-only artifact (explicitly marked as such in its own module doc
comment) and is not the basis for G7's actual code — G7 must build its
own production indicator daemon using `ksni` as a library dependency,
subject to normal G7 TDD discipline, not by promoting this spike
prototype directly.

## Evidence

Full raw evidence, including screenshots with explicit provenance
labeling, VM setup/teardown records, and D-Bus introspection captures:

- `docs/evidence/g6/G6_GNOME_KSNI_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_XFCE_KSNI_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_AYATANA_GTK3_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_AYATANA_GLIB_SPIKE_EVIDENCE.md`
- `docs/evidence/g6/G6_P0_IND_003_RECONNECT_EVIDENCE.md`
- `docs/evidence/g6/G6_CANDIDATE_COMPARISON.md` (consolidated matrix this
  ADR's decision is drawn from)
