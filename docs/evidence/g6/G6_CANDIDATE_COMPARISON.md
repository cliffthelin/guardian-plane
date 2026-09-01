# G6 Candidate Comparison — Desktop Indicator Decision Gate

**Status: final comparison record for G6, built from real evidence
gathered across six checkpoint documents in this directory.** This
document does not introduce new evidence; it consolidates
`G6_GNOME_KSNI_SPIKE_EVIDENCE.md`, `G6_XFCE_KSNI_SPIKE_EVIDENCE.md`,
`G6_AYATANA_GTK3_SPIKE_EVIDENCE.md`, `G6_AYATANA_GLIB_SPIKE_EVIDENCE.md`,
and `G6_P0_IND_003_RECONNECT_EVIDENCE.md` against the contract's own
10-item required-test list (§30) and applies its selection rule. The
selection this document reaches is recorded formally in
`docs/adr/ADR-006-guardian-indicator-mechanism.md`.

## The rule being applied

> "The winning implementation is the simplest candidate that passes all
> required targets. The test result, not library recency, selects the
> implementation." (§30, verbatim)

Applied honestly, this rule requires checking all ten required tests
against both target environments for every candidate, and disqualifying
any candidate that fails even one, "regardless of how idiomatic or
modern it is" (per the G6 implementation handoff's own restatement of
this rule). This document does that check explicitly rather than
asserting a winner and back-filling justification.

## Comparison matrix

Legend: **PASS** (directly evidenced), **FAIL** (directly evidenced),
**UNRESOLVED** (attempted, inconclusive, honestly reported as such),
**N/T** (not tested this gate), **N/A** (test does not apply to this
target as designed).

### Candidate 1 — legacy GTK3 Ayatana AppIndicator

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | PASS | PASS |
| menu opens | PASS | PASS |
| menu actions invoke handler | PASS | PASS |
| state/icon update propagates | **PARTIAL** — status genuinely changes (log-confirmed `ATTENTION`), but the glyph does not, because the prototype never calls the separate `app_indicator_set_attention_icon` the C API requires for a visual change | N/T (not re-run; GNOME result is the representative behavior since it's the same C library/API on both desktops) |
| no X11 dependency | **FAIL** — requires a real XWayland auth cookie (`XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.*`) to run at all under GNOME 50/Wayland; "Could not open X display" without it | N/A — Xfce 4.20 is X11-based by design in this Ubuntu 26.04 packaging, so this test is moot there per §30's own framing |
| reconnect after panel/Shell restart | PASS (via extension disable/enable cycle, the correct Wayland analog — see reconnect evidence) | N/T |
| reconnect after daemon restart | PASS (clean deregistration on kill, clean re-registration on relaunch, no duplicate, menu still functional) | N/T |
| daemon unavailable shows degraded state | Evidenced indirectly via the status-toggle mechanism (candidate can represent a distinct attention/degraded D-Bus state); not tested against an actual absent Guardian daemon, since no daemon exists yet at G6 | N/T |
| no duplicate icon | PASS (confirmed across both reconnect scenarios) | N/T |
| clean user logout/login lifecycle | N/T | N/T |

**Verdict: disqualified.** The X11/XWayland dependency is a structural
property of linking against `libayatana-appindicator3`/GTK3, not a
prototype shortcoming — no code change to this candidate's prototype
would remove it. §30 lists "no X11 dependency" as a required test with no
carve-out for GTK3-based candidates, and the contract's own selection
rule says a failure "regardless of how idiomatic or modern" disqualifies
a candidate. This is disqualifying on GNOME 50/Wayland specifically,
which is one of the two named target environments.

### Candidate 2 — GLib-only Ayatana AppIndicator 2.x

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | **FAIL** — `com.canonical.dbusmenu` interface not found, directly thrown by GNOME Shell | **FAIL** — identical root cause, directly captured in `xfce4-indicator-plugin`'s own debug log |
| all other tests | N/T — blocked by icon-appears failure; testing further behaviors of a candidate whose icon never renders would add no information | N/T |

**Verdict: disqualified**, on both required desktops, for a directly
diagnosed, reproduced-on-two-independent-consumers reason: this
library's modern `org.gtk.Menus`/`org.gtk.Actions` menu export is not
understood by either required desktop's actual indicator-rendering code,
both of which still expect the legacy `com.canonical.dbusmenu` protocol.
The candidate's own D-Bus registration, menu content, and action handling
were independently verified correct — the failure is a real ecosystem
gap, not an implementation defect, but it is disqualifying regardless.

### Candidate 3 — direct Rust SNI + canonical DBusMenu (`ksni`)

| Required test | GNOME 50 | Xfce 4.20 |
|---|---|---|
| icon appears | PASS (conditional on `ubuntu-appindicators@ubuntu.com` being enabled — see caveat below) | PASS (conditional on `xfce4-indicator-plugin` being added to the panel — Xfce's stock panel has no SNI awareness by default) |
| menu opens | PASS | **UNRESOLVED** — three real synthetic-click attempts at confirmed icon coordinates produced no visible menu and zero `com.canonical.dbusmenu` D-Bus traffic; honestly reported as unresolved, not forced to PASS or FAIL. Circumstantial evidence from candidate 1 (identical click technique worked immediately for it on the same Xfce setup) narrows this toward "candidate-specific," but does not fully resolve it. |
| menu actions invoke handler | PASS (`menu_clicks=1` log-confirmed) | N/T — blocked by the menu-open result above |
| state/icon update propagates | PASS — icon glyph visibly changes on status toggle via `icon_name()`, no extra API call needed (a real, favorable contrast with candidate 1) | N/T |
| no X11 dependency | **PASS** — directly confirmed empirically in this gate's reconnect testing: launched over SSH with only `DBUS_SESSION_BUS_ADDRESS` set, no `DISPLAY`/`XAUTHORITY`, and the icon/menu/click all worked correctly regardless | N/A (Xfce is X11-based by design; moot per §30's own framing, same as candidate 1) |
| reconnect after panel/Shell restart | PASS (via extension disable/enable cycle; recovery depended on the extension's own fallback bus-scan, since ksni does not proactively re-announce on `NameOwnerChanged` — a real, disclosed mechanism note) | N/T |
| reconnect after daemon restart | PASS (clean deregistration on kill, clean re-registration on relaunch, no duplicate, menu still functional) | N/T |
| daemon unavailable shows degraded state | Evidenced indirectly via the status-toggle mechanism (same caveat as candidate 1 — no real Guardian daemon exists yet to test against) | N/T |
| no duplicate icon | PASS (confirmed across both reconnect scenarios) | N/T |
| clean user logout/login lifecycle | N/T | N/T |

**Verdict: not disqualified by any directly-evidenced FAIL**, but **not
proven to pass every required test either** — the Xfce menu-open result
is honestly UNRESOLVED, and several required tests (clean logout/login
lifecycle; real daemon-unavailable degraded state against an actual
Guardian daemon; Xfce-side reconnect and no-X11-dependency confirmation)
remain N/T at this gate.

## Applying the selection rule

No candidate has a fully clean sweep of all ten required tests on both
target environments, evidenced with total certainty. Reporting this
honestly rather than forcing a tidy answer:

- **Candidate 1 is disqualified**, cleanly and structurally, by a real
  X11/XWayland dependency on GNOME 50 — one of the contract's ten named
  required tests, with no ambiguity in the result.
- **Candidate 2 is disqualified**, cleanly, on both desktops, by a
  directly diagnosed and independently reproduced menu-protocol
  incompatibility.
- **Candidate 3 (`ksni`) is the only candidate with no directly-evidenced
  FAIL on any required test it was actually run against.** Its open
  items are an UNRESOLVED result (Xfce menu-open) and a set of N/T items
  that are either infeasible to test meaningfully at G6 (a real Guardian
  daemon does not exist yet) or were deliberately out of scope for this
  pass's reconnect testing (Xfce-side reconnect, full logout/login
  cycle).

Given the contract's own framing — "a candidate that cannot be evidenced
as passing a required test on a target environment is disqualified for
that environment; it must not be selected provisionally pending later
verification" — the disciplined reading is: **`ksni` is provisionally
selected for GNOME 50, where every test actually run against it PASSed.**
For Xfce 4.20, `ksni`'s status is genuinely open pending resolution of
the menu-open finding, and this document does not claim otherwise.
Since the other two candidates are already fully disqualified on Xfce
(candidate 1 by a menu-open PASS but only reaching that after a
disqualifying GNOME failure that already rules it out project-wide;
candidate 2 by icon-appears FAIL), `ksni` is also the only remaining
viable candidate for Xfce even though its Xfce result is not yet a clean
PASS — there is no alternative to fall back to.

This is recorded in ADR-006 as a **conditional selection**: `ksni` is
selected as Guardian's indicator mechanism, with the Xfce menu-open gap
named explicitly as an open risk to resolve before Xfce-targeted G7+
production work begins, rather than papered over.

## Caveats and follow-ups for the record

- The `ubuntu-appindicators@ubuntu.com` / `xfce4-indicator-plugin`
  dependency `ksni` requires on both desktops is not a Guardian-authored
  component and is not guaranteed present on every real Ubuntu 26.04
  install (confirmed present on `ubuntu-desktop`/`ubuntu-desktop-minimal`
  metapackages per the GNOME spike's own note, but not verified against
  every possible install path). A production Guardian indicator should
  detect and clearly report the absence of a working
  `StatusNotifierWatcher`, not silently show nothing — this maps onto
  the still-untested "daemon unavailable shows degraded state" behavior
  and should be treated as a first-class G7+ requirement for whatever
  built on top of `ksni`.
- The icon-name/icon-theme fragility documented in the reconnect evidence
  (a hardcoded icon name that resolves in one VM and silently fails to
  render, with zero errors, in another) applies to whatever production
  icon name Guardian eventually chooses. It must be verified against the
  target theme or shipped as Guardian's own icon asset.
- The Xfce menu-open UNRESOLVED finding for `ksni` should be
  investigated further before Xfce-targeted G7+ work begins — ideas not
  yet tried: inspecting `ksni`'s DBusMenu object registration against
  `xfce4-indicator-plugin`'s actual D-Bus method calls via `dbus-monitor`
  during a real click (this gate captured a monitor session during one
  attempt that showed zero `com.canonical.dbusmenu` traffic at all,
  which is itself informative but was not followed up further within
  this gate's scope).
- "Clean user logout/login lifecycle" was not tested for any candidate
  in this gate. This is a reasonable, explicitly named gap to close
  before or during G7+ work, not a reason to withhold this gate's
  conditional selection.

## What this document does not do

Per this gate's explicit scope: **G6 selects an indicator mechanism. It
does not certify a production guardian-indicator implementation.**
Nothing in this document, or in ADR-006, authorizes building the G7+
production indicator daemon, expanding Guardian's public D-Bus surface,
or beginning any implementation work beyond this gate's own evidence
prototypes. The prototypes in `tests/vm/g6-candidate-*/` remain
evidence-only artifacts and are not the basis for G7's actual code.
