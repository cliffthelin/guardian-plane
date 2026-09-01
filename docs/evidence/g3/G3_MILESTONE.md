# Guardian Phase 0 — G3 Milestone Record

## Project identity

```text
Project display name:  Guardian Plane
Repository slug:       guardian-plane
GitHub repository:     github.com/cliffthelin/guardian-plane
Permanent D-Bus namespace: io.github.cliffthelin.Guardian1
```

## Accepted commit and tag

```text
Accepted commit: d21d0c4f41f7032db969b0b50c20c72c17b836c5
G3 tag:          phase0-g3-core-data-models (annotated, points to d21d0c4)
```

Implementation landed as two commits: `3e75200` (capability/provider models,
`guardian-provider-api`) and `d21d0c4` (arbitration/event/incident models,
`guardian-core`).

## Independent review

- Independent G3 implementation audit: **PASS WITH NON-BLOCKING FINDINGS**.
  The auditor independently re-verified the 24-row G2 inventory fixture
  row-by-row, and performed live mutation testing against a `/tmp` copy of
  the repository (never the tracked tree) to confirm the load-bearing tests
  actually catch real regressions — not just that they exist.
- No blocking findings. Four non-blocking findings carried forward as
  explicit G4 planning constraints (§ below).

## Normative test status

```text
P0-REG-001 — provider unavailable         PASS
P0-REG-002 — degraded provider            PASS
P0-REG-003 — contract provenance          PASS (reused unchanged from G0; byte-identical to baseline)
P0-REG-004 — drift detection              PASS (reused unchanged from G0; byte-identical to baseline)
P0-ARB-001 — single writer                PASS
P0-ARB-002 — ambiguous owner              PASS
P0-ARB-003 — owner change invalidates transaction   PASS (data-model level; see NB-2 below)
P0-ARB-004 — rollback disclosure          PASS
P0-EVT-001 — monotonic ordering           PASS
P0-EVT-002 — normalized key               PASS
P0-EVT-003 — raw reference preserved      PASS
P0-EVT-004 — incident linking             PASS
```

`cargo test --workspace`: **75 passed, 0 failed** (23 pre-existing G0/G1/G2
tests unmodified + 52 new G3 tests). `cargo fmt --check` and `cargo clippy
--workspace --all-targets --all-features -- -D warnings` both clean.

## Immutable G3 model rules

These are the accepted, load-bearing decisions this milestone freezes.
Later gates build on top of them; they are not to be silently reopened.

**Identity**
- `CapabilityId != ProviderId` — structurally distinct types, no conversion
  between them.
- Capability identity does not change when its realizing provider changes
  (`CapabilityRecord::with_provider`, tested).
- No positional, discovery-order, or UI-label-derived identity anywhere.

**Authorization architecture**
- The only known authorization modes are exactly `NoAuthorizationRequired`,
  `ProviderOwnedAuthorization`, `GuardianOwnedAuthorization`.
- This describes *who owns the authorization mechanism for a capability*,
  never *whether the current caller is authorized*.

**Authorization knowledge**
- `Knowledge::Known(AuthorizationMode)` is structurally and semantically
  distinct from `Knowledge::Unknown`.
- Unknown/unresearched authorization ownership never becomes
  `NoAuthorizationRequired` — proven by mutation testing during the
  independent audit (collapsing the two was caught immediately by the test
  suite).

**Privilege/access**
- A separate `PrivilegeRequirement` model
  (`NoDirectPrivilege | SpecificFileOrDeviceAccess | SpecificLinuxCapability
  | RootOrSystemPrivilege | Unknown`) exists independent of authorization
  ownership — neither dimension may be inferred from the other.

**Single writer**
- `Ownership` distinguishes at minimum `NoWriter`, `GuardianOwnedWriter`,
  `ProviderOwnedWriter`, `ExternalWriter`, `Conflict`.
- Ambiguous ownership (equal-priority competing write claimants) fails
  closed: `write_permitted = false`, `Ownership::Conflict` — never resolved
  by list order, `HashMap` iteration, or lexicographic tie-break.

**G2 security boundary**
- `ArbitrationDecision.write_permitted` means only *"control-plane policy
  permits proceeding."* It is never proof that a caller is authorized.
- The privileged helper (established at G2, unchanged) remains solely and
  independently responsible for resolving the real caller and performing
  real authorization immediately before mutation. No G3 type may be read
  as security proof by that boundary.

## Forward constraints for G4+

The independent G3 audit's four non-blocking findings are recorded here
verbatim in substance, neutrally, as explicit constraints the G4 gate must
resolve. None of them is a G3 defect — the auditor explicitly adjudicated
each as correct scope for a deterministic data-model gate. They become
load-bearing the moment a real transaction engine exists to act on this
data, which is G4's job, not G3's.

### NB-1 — Unknown privilege/access enforcement is not yet wired to any write gate

Accepted G3 behavior: `PrivilegeRequirement::Unknown` is faithfully
represented as metadata on `CapabilityRecord`. G3's arbitration model
(`CandidateProvider`/`arbitrate()`) does not consume `privilege_requirement`
at all — it is structurally absent from the arbitration input. Therefore it
is currently possible for `write_permitted = true` while the capability's
`privilege_requirement` remains `Unknown`.

The independent auditor adjudicated this as correct for G3's scope: the
governing TDD contract's arbitration invariants (§13) say nothing about
privilege/access requirement, and the G3 implementation handoff's fail-closed
arbitration invariant is scoped only to `authorization_ownership`. G4
planning must explicitly decide the stage at which `PrivilegeRequirement::Unknown`
becomes a fail-closed execution precondition — see
`GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` §14 for the resolution.

**The transaction engine must never accidentally interpret
`PrivilegeRequirement::Unknown` as `NoDirectPrivilege`.**

### NB-2 — Arbitration `revision` is caller-supplied, not model-derived

G3 implements `revision: u64` on `ArbitrationInput`/`ArbitrationDecision`
to make staleness *mechanically comparable*. The independent audit found
that `revision` is presently a plain caller-supplied field with no
computation, hash, or derivation tying it to the actual contents of the
candidate/ownership set — `arbitrate()` only copies it through unchanged.
The existing P0-ARB-003 test proves two decisions built from inputs with
different `revision` values compare unequal, which is true by construction
regardless of whether the underlying model enforces anything.

This is accepted as correct for G3's scope: the G3 implementation handoff
explicitly permits "a monotonic arbitration epoch/generation number **or
an equivalent**" without requiring G3 to build the mechanism that keeps
`revision` honest. G3's job was to make staleness *representable*; G4 is
where that value becomes operationally load-bearing. G4 planning must
therefore establish who owns revision generation, what changes increment
it, how it is captured/re-read, and how staleness is actually detected in
a real transaction — see `GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` §15/§20.

**The transaction engine must not accept a caller-asserted `revision` as
trustworthy precondition proof until G4 defines who actually owns and
increments it.**

### NB-3 — Serialization coverage is uneven across governed G3 enums

`BootAvailability`, `RollbackKind`, and `Risk` have `Display` but no
`FromStr`; `Ownership`, `IncidentStatus`, and `Confidence` have neither —
no wire representation exists for these six types. This is not a G3
blocker: none of these types currently crosses a real process/persistence/
D-Bus boundary. G4 must determine which *transaction-related* types
genuinely cross persistence, IPC, evidence logs, or restart recovery, and
require stable serialization only for those — not add serialization
everywhere merely for symmetry, but also never persist a type implicitly
through `Debug` formatting. See `GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` §32.

### NB-4 — `EventId`/`IncidentId` reuse `CapabilityId`/`ProviderId`'s strict domain-identity validator

All four ID types are produced by the same dotted-lowercase-domain
validation macro. The independent audit confirmed a UUID-style value
(digit-led, hyphenated) is rejected by `EventId::new`. Nothing in G3 is
currently broken by this — all G3 tests use hand-picked literal IDs that
happen to satisfy the stricter rule — but `CapabilityId`/`ProviderId` are
*deterministic semantic identity* while `EventId`/`IncidentId` (and G4's
new `TransactionId`) are *generated record identity*, and the same
validation rule should not have been allowed to spread across that
distinction by accident. G4 must decide `TransactionId`'s own identity
semantics deliberately (not by reusing `CapabilityId`'s validator) and, in
doing so, explicitly decide whether `EventId`/`IncidentId` should be
corrected to match or left as-is until a later gate — see
`GUARDIAN_G4_IMPLEMENTATION_HANDOFF.md` §17.

**G3's `ids.rs` is not modified by this milestone record or by G4 planning.**
Any correction to `EventId`/`IncidentId` validation is a deliberate,
separately-reviewed code change, not an automatic consequence of this note.

## Evidence index (referenced, not duplicated here)

```text
docs/guardian/30_TDD/GUARDIAN_G3_IMPLEMENTATION_HANDOFF.md
docs/guardian/30_TDD/GUARDIAN_G3_INDEPENDENT_REVIEW_HANDOFF.md
crates/guardian-provider-api/src/{ids,capability,provider}.rs
crates/guardian-provider-api/tests/{ids,capability,provider,authorization_dimension,g2_inventory}_contract.rs
crates/guardian-core/src/{risk,arbitration,event,incident}.rs
crates/guardian-core/tests/{risk,arbitration,event,incident}_contract.rs
```
