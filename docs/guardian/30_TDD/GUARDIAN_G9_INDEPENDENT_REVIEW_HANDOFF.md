# Guardian Phase 1 Independent Review Handoff
## G9 — Clients & Packaging

# 1. Baseline verification

Confirm independently, before reading anything else in this handoff or
the implementer's completion report:

- Exact `git log -1` HEAD and exact `git status --short` output. G9's
  candidate should sit on top of G8's (by then hopefully tagged)
  baseline as further uncommitted working-tree state, or on top of a
  fresh commit if G8 was tagged in the interim — state which is true.
- No G9 tag exists yet (`git tag -l`).
- Whether G8 itself is tagged at the time of this review. If it is not,
  determine whether G9's candidate depends on any G8 code that is still
  only a candidate (it should not — G9 only *reads* G8's
  `CapabilityRecord`/registry types, never modifies G8 provider code) —
  flag if G9 work required touching any G8 provider file, since that
  would be out of scope for this gate.

# 2. Governing material

- `docs/guardian/30_TDD/GUARDIAN_PHASE_0_1_TDD_CONTRACT.md` §7 (D-Bus
  contract), §21 (boot availability), §30 (indicator decision — already
  closed, G6), §31-34 (client responsibilities/acceptance), §35 (test
  infrastructure), §37 (P1-CLIENTS/P1-PACKAGING), §38 (G9 subsection),
  §39-41.
- `docs/guardian/30_TDD/GUARDIAN_G9_IMPLEMENTATION_HANDOFF.md` (the
  paired implementation handoff this review checks the candidate
  against).
- `docs/adr/ADR-006-guardian-indicator-mechanism.md` (G6's decision —
  the review must confirm G9 reused it, not re-litigated it).
- `docs/evidence/g8/G8_EVIDENCE_REPORT.md` (what real data actually
  exists for G9's clients to render — the review should cross-check the
  GUI/CLI/TUI's rendered content against this, not just against the
  implementer's own claims).

Treat the implementer's completion report only as navigation, not
evidence.

# 3. Required verdict

Exactly one:

```text
PASS — G9 IMPLEMENTATION ACCEPTED
PASS WITH NON-BLOCKING FINDINGS
FAIL — G9 CLIENT-THINNESS VIOLATED
FAIL — G9 FABRICATED WRITE CAPABILITY
FAIL — G9 INDICATOR DECISION RE-LITIGATED OR REGRESSED
FAIL — G9 PACKAGING EVIDENCE INCOMPLETE
FAIL — G9 REQUIRED EVIDENCE INCOMPLETE
FAIL — G9 PRIOR-GATE REGRESSION
```

# 4. Mechanically re-derive G9's normative scope — do not trust the candidate's own framing

Re-read contract §37's "P1-CLIENTS"/"P1-PACKAGING" subsections and §38's
G9 subsection directly. Confirm independently that the ID set is exactly
`P1-CLI-001..002`, `P1-TUI-001`, `P1-GUI-001..002`, `P1-IND-001..002`,
`P1-PKG-001..005` — 12 IDs. Confirm `P1-DMN-*`/`P1-SEC-*` are G7's (not
re-claimed by G9) and `P0-IND-*` are G6's (not re-claimed by G9). If the
candidate's own matrix claims a different ID set, that is itself a
finding.

# 5. The central-finding audit — the primary audit question

The implementation handoff's own §1 states G9 has **no real write-
capable capability to trigger**, and that any "request transaction" UI
must render this honestly (a real, correctly-empty transaction history;
no fabricated capability with `write_support: true` invented merely to
give a demo something to click).

Independently verify, from source, not from the report:

- Does any new capability record introduced or exposed by G9 have
  `write_support: true`? Grep for it. There should be **zero** such
  records anywhere G9 touches.
- Does any client-facing "request transaction" code path actually
  invoke a real mutation, or silently no-op and report success? Trace
  it end to end. The only acceptable behaviors are: (a) a typed "no
  write-capable capability exists" response for every real request, or
  (b) the UI affordance simply does not exist yet.
- Does the GUI/TUI/CLI transaction-history view show a real (likely
  empty) list, or does it show fabricated/hardcoded sample data dressed
  up as real? Reject fabricated sample data presented as live state.

This is G9's equivalent of G8's write-scope audit — treat any violation
found here as blocking (`FAIL — G9 FABRICATED WRITE CAPABILITY`).

# 6. Client-thinness audit (§31)

For each of CLI/TUI/GUI/indicator, independently confirm none of them:

- directly writes system configuration;
- calls `sudo` or shells out to a privileged helper directly (bypassing
  `guardian-daemon`);
- directly manipulates `/sys` write controls;
- duplicates provider arbitration logic (re-implements any part of
  what belongs to the Provider Arbitrator);
- implements independent safety logic that differs from daemon policy
  (e.g., a client-side "is this safe to power off" check that
  duplicates or diverges from `validate_power_off_preconditions`-style
  daemon logic).

Grep each client crate for D-Bus proxy construction and confirm every
one only ever talks to `guardian-daemon`'s public interfaces — never to
`guardian-helper`, never to a provider's own D-Bus service directly.

# 7. Public D-Bus surface-expansion audit

- Confirm `Guardian1` itself is byte-for-byte unchanged from G0's frozen
  `ContractVersion`/`ServiceState` — any addition belongs on a separate
  interface major per §7.1's illustrative shape. Reject an
  implementation that bolts new methods directly onto `Guardian1`.
- Confirm the new interface(s) are genuinely read-only — no method on
  them can mutate daemon or provider state. Search for any method that
  isn't a pure getter/lister.
- Confirm `CapabilityRecord`/incident/transaction data serialized over
  the new interface(s) matches G3's/G8's real typed models — not a
  hand-rolled second schema that could drift from the real one (in
  particular: does the exposed capability data still show real
  `write_support: false` and real `Knowledge::Unknown` authorization
  ownership, or does the client-facing serialization quietly drop or
  default those fields to something friendlier-looking?).
- Confirm introspection works (`busctl introspect` or equivalent) and
  matches the documented shape.

# 8. Indicator reuse audit — confirm G6 was not re-litigated

- Confirm the candidate uses `ksni`, per ADR-006, without re-running or
  second-guessing the three-candidate comparison.
- Confirm `guardian-indicator` is real production code (in a real
  workspace member crate), not the disposable `tests/vm/g6-candidate-
  ksni/` prototype copied wholesale — check for a genuine dependency on
  real `guardian-daemon` D-Bus state vs. G6's stub/fixture data.
- Confirm the required §30 test list (icon appears, menu opens, menu
  actions invoke the handler, state/icon update propagates, no X11
  dependency, reconnect after panel/Shell restart, reconnect after
  **daemon** restart — not indicator-process restart, no duplicate
  icon, clean logout/login) is evidenced again for real, against real
  data, not merely cited as "G6 already proved this."
- If the candidate found and reports a genuine defect in the G6
  decision itself, verify that finding independently rather than either
  rubber-stamping it or dismissing it — a correct "G6 was wrong" finding
  here is a real prior-gate regression, not a G9 defect.

# 9. Per-surface acceptance validation

- **CLI**: real JSON-mode output for every one of the seven minimum
  commands, independently parsed with a real JSON parser (not eyeballed
  formatting). Real, deterministic daemon-offline exit code/message,
  reproduced independently (stop the real daemon, run the real CLI).
- **TUI**: reproduce a real VT-only session (no `DISPLAY`, no
  `WAYLAND_DISPLAY`) and confirm real startup.
- **GUI**: confirm every §32-listed element is genuinely rendered from
  real daemon state (daemon connection state, overall Guardian state,
  capabilities list, provider ownership, incidents list, blockers,
  PSI summary, transaction history, graceful provider-unavailable
  state) — not a static mockup.
- **Packaging**: reproduce (or closely audit if reproduction is
  impractical within review budget) real install on a clean VM, real
  vendor-path placement, real uninstall/purge behavior, and confirm no
  other package's files were touched during install.

# 10. Forward-constraint audit

Confirm G9 did not silently close: G4 FC-3, G5 FC-2, G7 SafeToResume/
idempotency, or any of G8's forward constraints (single-writer,
`Knowledge::Unknown` authorization ownership, `write_support: false`).
None of these should even be *touchable* by a read-only client layer —
verify that's actually true in the code, not merely asserted.

# 11. Real-VM evidence audit

- Confirm indicator evidence includes real visual proof (screenshots or
  equivalent) on both GNOME 50/Wayland and Xfce 4.20, not text-only
  D-Bus introspection presented as if it proved visual rendering.
- Confirm packaging evidence used a genuinely fresh Ubuntu 26.04.1
  image, not a VM already carrying prior gates' leftover state.
- Confirm daemon-restart-while-indicator-alive evidence is the
  genuinely correct scenario (daemon process restarts, indicator
  process does not) — this exact confusion was a real defect G6's own
  audit history caught once already; check it was not repeated here.
- Confirm every VM artifact is attributable to the exact final source
  digest of the candidate under review (same discipline the G8 audit
  required) — reject evidence that cannot be tied to the reviewed
  source.

# 12. Regression audit

Run the full workspace test suite and confirm the pre-G9 baseline count
is preserved (no G0-G8 test was weakened, skipped, or deleted to make
G9 pass). Confirm no G8 provider file was modified by this gate (G9's
own scope should not require touching `crates/guardian-core/src/
providers/*`).

# 13. Validation

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Report the exact before/after test counts and confirm they match the
candidate's own claim.

# 14. Required report

1. Git/baseline state;
2. exhaustive changed-file reconciliation (reported vs. actual, flagging
   any G8-provider-file or prior-gate-production-file touch);
3. independently re-derived 12-ID matrix;
4. central-finding (§5) audit — fabricated-write-capability check;
5. client-thinness audit (§6);
6. public D-Bus surface-expansion audit (§7);
7. indicator reuse audit (§8);
8. per-surface acceptance validation (§9);
9. forward-constraint audit (§10);
10. real-VM evidence audit (§11);
11. regression audit (§12);
12. validation (§13);
13. blocking findings;
14. non-blocking findings;
15. exact next action.

Then STOP. Do not modify the candidate. Do not commit. Do not push. Do
not tag G9. Do not begin any Phase 2 work.
