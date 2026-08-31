# Guardian Phase 0/1 TDD Contract
## Ubuntu 26.04.1 System Control Plane

**Status:** Governing implementation contract for Phase 0 and Phase 1  
**Target:** Ubuntu 26.04.1 LTS (Resolute)  
**Primary language:** Rust for privileged and safety-critical components  
**Development model:** Test-driven. A gate is not complete until every required test for that gate passes.  
**Scope boundary:** This contract defines the control-plane foundation only. It does not implement I/O Guardian, thermal control, Log Lens, USB authorization, or other later modules except where a minimal provider/mock is required to prove the architecture.

---

# 1. Purpose

Guardian is a local Ubuntu system-control plane that provides:

- one shared system daemon;
- typed D-Bus interfaces;
- per-action polkit authorization;
- GUI, TUI, CLI, and desktop-indicator clients;
- capability discovery;
- provider ownership/arbitration;
- transaction and rollback semantics;
- an incident/event model;
- a diagnostic-budget mechanism;
- a bounded flight recorder;
- safe degradation when a provider or desktop component is unavailable.

Phase 0 and Phase 1 exist to make later system-management features safe to add.

No later module may bypass these foundations.

---

# 2. Governing principles

These principles are normative.

## GP-01 — Orchestrate, do not reimplement

Guardian MUST prefer interfaces in this order:

1. native D-Bus or library API;
2. kernel interface (`/proc`, `/sys`, netlink, udev);
3. structured command output;
4. scraped command output only when no stronger interface exists.

## GP-02 — Read-only by default

Discovery, monitoring, and diagnosis MUST NOT require write authorization where the platform allows read access.

## GP-03 — Typed privileged operations only

Guardian MUST NOT expose a generic privileged shell/command execution API.

Prohibited public methods include semantic equivalents of:

```text
RunCommand(string)
RunShell(string)
ExecuteArbitrary(argv)
```

## GP-04 — Single-writer rule

Guardian MUST NOT write a mutable system capability while another authoritative provider owns that capability unless ownership is intentionally transferred through a defined transaction.

## GP-05 — Every write is transactional

Every real write action MUST follow:

```text
Snapshot
→ Validate
→ Authorize
→ Apply
→ Observe
→ Confirm
→ Commit
```

or, on failure:

```text
Apply
→ Observe failure
→ Roll back where supported
→ Record outcome
```

## GP-06 — Fail closed

Unknown provider state, stale identity, failed validation, authorization ambiguity, or changed preconditions MUST block the write.

## GP-07 — Diagnose without worsening the failure

The Diagnostic Budget Manager MUST be able to deny or downgrade diagnostic escalation when the host is already constrained in the same resource class.

## GP-08 — Preserve evidence

Guardian may collapse or normalize logs for presentation, but MUST NOT delete authoritative raw journal evidence as part of deduplication.

## GP-09 — Local-first

Guardian MUST operate without remote/cloud dependencies for core monitoring, diagnosis, transaction safety, and recovery.

A provider that performs a network operation MUST identify that operation explicitly.

## GP-10 — Explain decisions

Provider arbitration, safety denial, transaction failure, rollback unavailability, and diagnostic-budget decisions MUST produce machine-readable reason codes plus a human-readable explanation.

---

# 3. Phase boundaries

## Phase 0 — Contracts & Simulator

Phase 0 builds no production system-management feature beyond what is necessary to prove the architecture.

It MUST define and test:

- D-Bus API/versioning;
- caller identity and authorization flow;
- typed error model;
- Capability Registry;
- Provider Arbitrator;
- transaction state machine;
- event and incident schema;
- diagnostic budget;
- boot-availability model;
- persistence model;
- provider provenance;
- privilege topology;
- mock D-Bus providers;
- mock hardware/udev providers;
- indicator compatibility decision;
- package/install skeleton.

## Phase 1 — Control Plane

Phase 1 implements the production foundation:

- `guardian-daemon`;
- `guardian-cli`;
- `guardian-tui` shell;
- `guardian-gui` shell;
- `guardian-indicator`;
- D-Bus APIs;
- polkit actions;
- Capability Registry;
- Provider Arbitrator;
- transaction engine;
- event/incident store;
- diagnostic budget;
- flight recorder;
- initial read-only platform providers;
- service hardening;
- packaging;
- installation/uninstallation;
- basic recovery-access plumbing.

Phase 1 MUST NOT implement destructive high-risk provider actions.

---

# 4. Required repository layout

The exact crate names may change before the first commit, but the architecture MUST preserve separation equivalent to:

```text
guardian/
├── Cargo.toml
├── crates/
│   ├── guardian-core/
│   │   ├── capability/
│   │   ├── arbitration/
│   │   ├── transaction/
│   │   ├── incident/
│   │   ├── diagnostics/
│   │   ├── errors/
│   │   └── identity/
│   ├── guardian-provider-api/
│   ├── guardian-provider-systemd/
│   ├── guardian-provider-psi/
│   ├── guardian-provider-logind/
│   ├── guardian-provider-udisks/
│   ├── guardian-provider-upower/
│   ├── guardian-provider-accounts/
│   ├── guardian-daemon/
│   ├── guardian-cli/
│   ├── guardian-tui/
│   ├── guardian-gui/
│   ├── guardian-indicator/
│   └── guardian-testkit/
├── dbus/
│   ├── interfaces/
│   └── policy/
├── polkit/
├── systemd/
├── packaging/
│   └── debian/
├── tests/
│   ├── contract/
│   ├── integration/
│   ├── dbus/
│   ├── hardware/
│   ├── vm/
│   └── fixtures/
└── docs/
```

Cross-cutting system logic MUST live in `guardian-core` or another shared crate, not be duplicated in GUI/TUI/daemon code.

---

# 5. Ubuntu 26.04.1 compatibility baseline

Contract development targets the researched Resolute stack:

| Component | Baseline |
|---|---:|
| systemd | 259.x |
| D-Bus | 1.16.x |
| polkit | 127 |
| NetworkManager | 1.54.x |
| UDisks2 | 2.10.x |
| AccountsService | 23.13.x |
| UPower | 1.91.x |
| power-profiles-daemon | 0.30 |
| thermald | 2.5.x |
| fwupd | 2.1.x |
| logrotate | 3.22 |
| Rust | Ubuntu-packaged current Resolute Rust toolchain |

Guardian MUST NOT key behavior only to these exact numbers.

Every adapter MUST probe the capabilities it needs.

---

# 6. External provider provenance

Each external provider MUST expose a provenance record internally:

```rust
ProviderProvenance {
    provider_id,
    package_name,
    package_version,
    interface_name,
    interface_version,
    introspection_hash,
    policy_hash,
    observed_at,
}
```

Fields that do not apply MUST be explicit `None`/`Unknown`, not fabricated.

## Acceptance rule

If the installed provider's D-Bus introspection differs from the fixture used by the test suite, Guardian MUST surface contract drift.

It MUST NOT silently assume the old contract still applies.

---

# 7. D-Bus contract

## 7.1 Naming decision gate

The reverse-domain / permanent bus namespace MUST be selected before G0 passes.

Until then, tests MAY use a temporary development namespace.

Once selected, public interface names MUST carry an explicit major version.

Illustrative shape only:

```text
<namespace>.Guardian1
<namespace>.Guardian.Capabilities1
<namespace>.Guardian.Transactions1
<namespace>.Guardian.Incidents1
```

## 7.2 Object hierarchy

The hierarchy MUST be stable and introspectable.

A recommended form is:

```text
/<namespace>/Guardian1
/<namespace>/Guardian1/Capabilities
/<namespace>/Guardian1/Transactions
/<namespace>/Guardian1/Incidents
/<namespace>/Guardian1/Diagnostics
```

Dynamic resources MAY use child objects, but the object-path construction rule MUST be deterministic.

## 7.3 Versioning

Compatible additive changes MAY remain in the same interface major.

The following require a new interface major:

- changing a method signature;
- removing a method/property/signal;
- changing the meaning of a return value;
- changing an error semantic incompatibly;
- changing authorization semantics incompatibly.

## 7.4 D-Bus implementation requirements

The daemon SHOULD use typed Rust D-Bus bindings such as `zbus`.

The implementation MUST provide introspection.

Clients MUST use typed proxies for Guardian's own public interfaces.

External provider adapters SHOULD use typed proxies when the provider contract is stable and known.

## 7.5 Bus policy

Packaged vendor policy belongs under:

```text
/usr/share/dbus-1/system.d/
```

Administrator overrides belong under:

```text
/etc/dbus-1/system.d/
```

Guardian MUST NOT use bus policy as a substitute for domain-specific authorization.

---

# 8. Caller identity and authorization

## 8.1 Identity source

Guardian MUST identify the caller from the actual system-bus message.

A client-supplied UID, username, PID, role, or `is_admin` flag MUST NOT be trusted as authorization evidence.

## 8.2 Subject model

For system-bus calls, Guardian SHOULD authorize against the caller's unique D-Bus system-bus name where supported by the polkit integration.

The implementation MUST retain enough caller/session information to create an auditable transaction record.

## 8.3 Interactive authorization rule

Only a user-initiated action may request interactive authorization.

Background monitoring and automatic Protect-mode logic MUST NOT unexpectedly trigger an authentication prompt.

## 8.4 TUI authentication

Phase 0 MUST prove textual authentication from a VT/recovery environment using a polkit text agent or equivalent supported mechanism.

## 8.5 SSH policy

SSH privileged actions MUST be intentionally defined.

The implementation MUST NOT accidentally allow or deny SSH solely because a graphical auth agent is absent.

---

# 9. polkit action taxonomy

Actions MUST be granular.

At minimum, Phase 0 fixtures MUST define distinct test actions equivalent to:

```text
guardian.test.read
guardian.test.low-risk-write
guardian.test.moderate-write
guardian.test.high-risk-write
```

Production actions later SHOULD follow domain-specific naming such as:

```text
guardian.storage.power-off
guardian.service.pause
guardian.session.set-default
guardian.logs.configure
guardian.resources.throttle
guardian.usb.authorize
guardian.recovery.advanced
```

Granting a low-risk action MUST NOT implicitly authorize a high-risk action.

---

# 10. Risk taxonomy

Every action MUST declare one risk class:

```text
OBSERVE
LOW
MODERATE
HIGH
VERY_HIGH
```

## OBSERVE

- no system mutation;
- no authorization required unless the underlying provider restricts read access.

## LOW

- temporary;
- reversible;
- bounded impact;
- automatic rollback expected.

## MODERATE

- meaningful local state change;
- reversible or recoverable;
- must show affected resources.

## HIGH

- can interrupt workloads or sessions;
- may have incomplete rollback;
- must live in Recovery/Advanced UI.

## VERY_HIGH

- may leave the machine degraded, offline, unbootable, or require manual recovery;
- never automated by default;
- explicit user acknowledgement required in addition to normal authorization.

---

# 11. Capability Registry schema

The canonical internal representation MUST contain at least:

```rust
CapabilityRecord {
    capability_id,
    provider_id,
    provider_version,
    availability,
    health,
    read_support,
    write_support,
    authorization_mode,
    boot_availability,
    interface_kind,
    interface_name,
    interface_hash,
    diagnostic_cost,
    last_observed_at,
}
```

## Availability states

```text
AVAILABLE
DEGRADED
UNAVAILABLE
UNSUPPORTED
UNKNOWN
```

`UNKNOWN` MUST NOT be rendered as healthy.

## Health states

Recommended:

```text
HEALTHY
WARNING
ERROR
STALE
UNKNOWN
```

---

# 12. Provider interface contract

Providers MUST have a common lifecycle abstraction equivalent to:

```rust
trait Provider {
    fn identity(&self) -> ProviderIdentity;
    fn provenance(&self) -> ProviderProvenance;
    async fn probe(&self) -> ProbeResult;
    async fn capabilities(&self) -> Vec<CapabilityRecord>;
    async fn health(&self) -> ProviderHealth;
    async fn subscribe_events(&self) -> EventStream;
}
```

A mutable capability adapter MUST be able to express:

```rust
inspect()
validate(action)
snapshot(action)
apply(action)
observe(expectation)
rollback(snapshot)
```

Not every provider must support every operation.

Unsupported operations MUST return an explicit typed `Unsupported` result.

---

# 13. Provider Arbitrator

The arbitrator MUST answer:

> Which provider is authoritative for this capability right now, and is Guardian allowed to write it?

Canonical arbitration record:

```rust
ArbitrationDecision {
    capability_id,
    candidate_providers,
    authoritative_provider,
    current_owner,
    ownership_basis,
    conflicts,
    write_permitted,
    rollback_kind,
    risk_class,
    decision_reason,
}
```

## Rollback kind

```text
NATIVE
EMULATED
BEST_EFFORT
NONE
```

## Arbitration invariants

1. Two providers MUST NOT simultaneously receive writes for the same exclusively owned capability.
2. If ownership is ambiguous, writes fail closed.
3. Provider absence can produce degraded read-only capability but not guessed write ownership.
4. The reason for the chosen provider MUST be inspectable by clients.
5. Provider ownership changes MUST invalidate stale transaction preconditions.

---

# 14. Transaction state machine

The canonical Phase 0 state machine is:

```text
CREATED
VALIDATING
VALIDATED
AUTHORIZING
AUTHORIZED
APPLYING
OBSERVING
COMMITTED

ROLLING_BACK
ROLLED_BACK

REJECTED
FAILED
ROLLBACK_FAILED
EXPIRED
CANCELLED
```

Terminal states MUST be immutable.

## 14.1 Transaction record

```rust
TransactionRecord {
    transaction_id,
    idempotency_key,
    action_type,
    risk_class,
    initiating_bus_name,
    initiating_session,
    provider_id,
    capability_id,

    created_at,
    deadline,

    pre_state,
    validation_results,
    arbitration_result,
    authorization_result,

    requested_change,
    provider_request,
    provider_response,

    observation_policy,
    observations,

    commit_result,
    rollback_result,

    incident_ids,
}
```

## 14.2 Invariants

- Every real write has a transaction ID.
- An idempotency key prevents accidental duplicate execution.
- Validation is repeated at the privileged boundary immediately before apply.
- A changed resource identity invalidates the transaction.
- Native provider checkpoints are preferred.
- Rollback capability is disclosed before authorization.
- Provider calls have bounded timeout.
- Client disappearance does not erase the transaction audit record.
- Daemon restart must recover or clearly terminate nonterminal transaction state.
- A rollback failure becomes a distinct `ROLLBACK_FAILED` state.

---

# 15. Transaction observation contract

A transaction MUST define what "success" means before applying the change.

An observation policy contains:

```text
expected_properties
forbidden_properties
minimum_observation_duration
maximum_observation_duration
health_checks[]
commit_condition
rollback_condition
```

A provider returning "method call succeeded" MUST NOT automatically mean the transaction succeeded.

---

# 16. Error model

Public Guardian errors MUST use stable typed categories:

```text
NotAuthorized
AuthenticationUnavailable
Unsupported
ProviderUnavailable
ProviderChanged
PreconditionFailed
Conflict
Busy
TimedOut
Cancelled
InvalidRequest
Unsafe
ApplyFailed
ObservationFailed
RollbackFailed
PersistenceFailed
Internal
```

## Requirements

- Human text MUST supplement, not replace, the type.
- Provider stack traces MUST NOT be the stable external API.
- Arbitrary CLI stderr MUST NOT become the only error representation.
- Internal errors MUST log a correlation identifier.
- Sensitive information MUST be redacted from client-facing text.

---

# 17. Event schema

An event is an immutable normalized observation:

```rust
Event {
    event_id,
    timestamp_monotonic,
    timestamp_wall,
    source_provider,
    event_type,
    resource_refs,
    severity,
    normalized_key,
    raw_reference,
    attributes,
}
```

Monotonic time MUST be used where ordering/duration matters.

Wall-clock time MUST be retained for human correlation.

---

# 18. Incident schema

An incident is a correlation envelope:

```rust
Incident {
    incident_id,
    opened_at,
    closed_at,
    status,
    summary,
    confidence,
    primary_resource,
    event_ids,
    evidence,
    candidate_causes,
    recommended_actions,
    transaction_ids,
    outcome,
}
```

## Incident invariants

- an incident does not replace source events;
- correlation can be updated as evidence arrives;
- confidence changes are recorded;
- user actions and Guardian transactions can be linked to the incident;
- an incident can close without a known root cause.

---

# 19. Diagnostic Budget Manager

Diagnostic cost classes:

```text
NEGLIGIBLE
LOW
MODERATE
HIGH
```

Providers MUST declare cost metadata:

```rust
DiagnosticCost {
    cpu_cost,
    memory_cost,
    io_read_cost,
    io_write_cost,
    kernel_trace_cost,
    expected_duration,
}
```

## Required behavior

Examples that MUST be testable:

- severe I/O pressure can veto an I/O-write-heavy trace;
- severe memory pressure can veto a large diagnostic buffer;
- low root-disk space can switch the recorder to memory-only mode;
- thermal emergency can prevent expensive diagnostic escalation;
- a denied diagnostic action produces an explainable reason.

---

# 20. PSI event contract

Guardian MUST support:

1. periodic parsing of `/proc/pressure/{cpu,memory,io}`;
2. event-driven threshold monitoring where the kernel supports PSI triggers.

Tests MUST cover:

- valid `some` and `full` parsing;
- CPU lacking a `full` line;
- counter monotonicity;
- malformed/unavailable PSI source;
- threshold event triggering;
- threshold monitor teardown;
- no busy-loop when there is no event.

---

# 21. Boot availability model

Every capability/provider MUST declare one or more availability levels:

```text
EARLY_BOOT
SYSTEM_BUS
PRE_LOGIN
USER_SESSION
DESKTOP_ONLY
OPTIONAL
```

Clients MUST NOT assume desktop-only services exist in recovery mode.

The TUI MUST render unavailable desktop-only capabilities as unavailable, not as errors that prevent the application from starting.

---

# 22. Flight recorder contract

The Phase 1 recorder MUST:

- use a bounded in-memory ring buffer;
- never block critical monitoring on a removable/monitored storage target;
- have a bounded local persistence spill area if enabled;
- tolerate filesystem write failure;
- retain monotonic timestamp ordering;
- prioritize critical events when under capacity pressure;
- expose dropped-event counters.

It MUST NOT have an unbounded queue.

---

# 23. Persistent state layout

Recommended package-managed directories:

```text
/var/lib/guardian/
/var/log/guardian/       # only if a dedicated flat log is truly required
/run/guardian/
```

Use systemd directory directives such as:

```text
StateDirectory=
RuntimeDirectory=
LogsDirectory=
```

where appropriate.

Transaction and incident metadata SHOULD live under `/var/lib/guardian`, not only in journald.

Operational daemon logs SHOULD use journald.

---

# 24. systemd service hardening

Before any production write path is enabled, the service MUST be evaluated against controls including:

```text
NoNewPrivileges=
ProtectSystem=
ProtectHome=
ProtectKernelTunables=
ProtectKernelModules=
ProtectControlGroups=
PrivateTmp=
PrivateDevices=
CapabilityBoundingSet=
RestrictAddressFamilies=
RestrictNamespaces=
SystemCallFilter=
ReadWritePaths=
StateDirectory=
RuntimeDirectory=
```

Not every hardening option will be compatible.

Every intentionally omitted protection MUST have a documented reason.

`systemd-analyze security` output MUST be captured as a test/review artifact.

---

# 25. Privilege topology decision gate

Phase 0 MUST implement and compare:

## Model A — hardened privileged daemon

One Rust system daemon performs reads and typed writes, but is heavily sandboxed and authorizes each write through polkit.

## Model B — unprivileged core + privileged mechanism

A dedicated unprivileged long-running Guardian core performs monitoring/correlation while a smaller privileged D-Bus mechanism performs narrowly typed writes and can be activated on demand.

## Decision criteria

Evaluate:

- attack surface;
- amount of code running privileged;
- D-Bus complexity;
- transaction atomicity;
- crash recovery;
- audit coherence;
- provider access needs;
- boot/recovery availability;
- systemd sandbox score;
- implementation complexity.

The contract MUST record the chosen model and rejected model rationale before Phase 1 write infrastructure is accepted.

---

# 26. Initial read-only providers for Phase 1

Phase 1 MUST prove the provider architecture with a bounded set of real providers.

Required:

1. **systemd provider**
   - list/inspect selected units;
   - unit state;
   - startup/failure metadata;
   - no production write required yet.

2. **PSI provider**
   - CPU/memory/I/O pressure;
   - threshold event source.

3. **logind provider**
   - list current inhibitors / "System Blockers".

4. **UDisks provider**
   - drive/block topology;
   - read `CanPowerOff`;
   - read sibling identity;
   - production `PowerOff()` deferred until I/O Guardian phase.

5. **UPower provider**
   - display device;
   - battery/UPS/power-device enumeration where available.

6. **AccountsService provider**
   - enumerate provider availability;
   - discover user/session context;
   - production session write deferred until system-management phase.

Optional if cheap:

7. power-profiles-daemon read-only ownership state.

No additional provider is required for Phase 1 completion.

---

# 27. UDisks invariants

The UDisks adapter MUST distinguish:

```text
physical/topological device
Drive
Block
Partition
Filesystem
Mount
```

It MUST NOT model a disk solely by `/dev/sdX`.

For a future `PowerOff()` action, tests MUST already exist proving:

- `CanPowerOff == false` rejects;
- sibling drives are discovered;
- stale object identity rejects;
- device removal between validation and apply rejects;
- action is marked user-initiated only;
- affected siblings are returned to the client before authorization.

The production write implementation may remain deferred.

---

# 28. Session-provider invariants

Guardian MUST:

- enumerate valid installed sessions;
- validate requested session IDs;
- prefer AccountsService `SetSession()` on supported targets;
- treat `SetXSession()`/`.dmrc` as compatibility mechanisms, not primary state;
- never accept an arbitrary unvalidated session string as a privileged write target.

The actual session write remains deferred from Phase 1.

---

# 29. logind "System Blockers"

Phase 1 SHOULD expose inhibitors from `org.freedesktop.login1`.

The normalized model SHOULD include:

```text
what
who
why
mode
uid
pid
```

A missing logind provider MUST degrade the page/command without blocking Guardian startup.

---

# 30. Desktop indicator decision gate

Phase 0 MUST compare at least:

- legacy GTK3 Ayatana AppIndicator;
- GLib-only Ayatana AppIndicator 2.x;
- direct Rust SNI + canonical DBusMenu (e.g. `ksni`).

Target environments:

```text
Ubuntu 26.04.1 GNOME 50 / Wayland
Xfce 4.20 / Status Tray
```

## Required indicator tests

- icon appears;
- menu opens;
- menu actions invoke the client-side handler;
- state/icon update propagates;
- no X11 dependency;
- reconnect after panel/Shell restart;
- reconnect after daemon restart;
- daemon unavailable shows degraded state;
- no duplicate icon;
- clean user logout/login lifecycle.

The winning implementation is the simplest candidate that passes all required targets.

The test result, not library recency, selects the implementation.

---

# 31. GUI/TUI/CLI responsibilities

All clients MUST remain thin.

Clients MAY:

- render state;
- request transactions;
- show authorization result;
- show transaction progress;
- show incidents;
- subscribe to signals;
- provide local UI preferences.

Clients MUST NOT:

- directly write system configuration;
- call `sudo`;
- directly manipulate `/sys` write controls;
- duplicate provider arbitration;
- implement independent safety logic that differs from daemon policy.

---

# 32. GUI shell acceptance

Phase 1 GUI is intentionally a shell, not the finished Guardian dashboard.

It MUST demonstrate:

- daemon connection state;
- overall Guardian state;
- capabilities list;
- provider ownership details;
- incidents list;
- current system blockers;
- read-only PSI summary;
- transaction history view;
- graceful provider unavailable state.

It does not need final visual polish.

---

# 33. TUI shell acceptance

The Phase 1 TUI MUST:

- run in a normal terminal;
- run from a VT without a desktop;
- display daemon connectivity;
- display the same capability/incident data as the GUI at a basic level;
- exercise text polkit in a test action;
- remain usable if desktop-only providers are unavailable.

---

# 34. CLI acceptance

The CLI MUST have structured output support.

Minimum commands equivalent to:

```text
guardian status
guardian capabilities
guardian providers
guardian incidents
guardian blockers
guardian psi
guardian transactions
```

A machine-readable mode such as JSON MUST be provided.

CLI output MUST NOT require scraping by future automation when a structured mode exists.

---

# 35. Test infrastructure

## Layer 1 — pure Rust unit/contract tests

No system bus and no real hardware.

Required domains:

- schema serialization;
- state-machine transitions;
- typed errors;
- risk ordering;
- arbitration;
- idempotency;
- timeout policy;
- diagnostic budget;
- event normalization;
- incident correlation.

## Layer 2 — isolated D-Bus tests

Use a private bus / `dbus-run-session` and mocks.

Test:

- provider appears/disappears;
- owner changes;
- method times out;
- malformed response;
- unsupported member;
- signal storm;
- daemon/client disconnect;
- polkit decision mock;
- stale proxy.

## Layer 3 — mocked hardware

Use `umockdev` and fixtures where appropriate.

Test:

- hotplug;
- removal;
- re-enumeration under another `/dev` name;
- missing serial;
- same device on new port;
- multiple block devices under one physical parent;
- invalid sensor values;
- disappearing sysfs node.

## Layer 4 — disposable Ubuntu 26.04.1 VM

Required for:

- real system D-Bus;
- polkit graphical auth;
- polkit text auth;
- systemd hardening;
- transient scopes;
- cgroup behavior;
- AccountsService behavior;
- UDisks behavior;
- GNOME indicator;
- Xfce indicator;
- package install/uninstall;
- daemon restart recovery.

---

# 36. Required Phase 0 tests

The following test IDs are normative.

## P0-DBUS

### P0-DBUS-001 — introspection exists
**Given** Guardian daemon is registered on a private bus  
**When** introspection is requested  
**Then** the documented object/interface tree is returned.

### P0-DBUS-002 — interface major present
Every public interface name contains the selected major version.

### P0-DBUS-003 — no generic execution method
Introspection contains no generic shell/command execution method.

### P0-DBUS-004 — typed error mapping
Each defined internal error category maps deterministically to the public D-Bus error contract.

### P0-DBUS-005 — unknown method fails normally
Unknown calls do not crash the daemon.

## P0-AUTH

### P0-AUTH-001 — caller identity cannot be spoofed
Supplying another UID/PID in method data does not alter the authenticated subject.

### P0-AUTH-002 — denied action does not apply
A denied polkit decision leaves provider state untouched.

### P0-AUTH-003 — background action cannot prompt
A noninteractive request cannot trigger interactive authentication.

### P0-AUTH-004 — explicit GUI-style action may prompt
A user-interactive test request can enter authentication flow.

### P0-AUTH-005 — VT text auth
A text authorization agent can complete a test action in the VM.

## P0-REGISTRY

### P0-REG-001 — provider unavailable
Unavailable provider yields `UNAVAILABLE`, not healthy.

### P0-REG-002 — degraded provider
Partially working provider yields `DEGRADED` with reason.

### P0-REG-003 — contract provenance
Provider provenance is stored.

### P0-REG-004 — drift detection
Changed introspection hash is observable and testable.

## P0-ARBITRATION

### P0-ARB-001 — single writer
Two exclusive providers cannot both receive write ownership.

### P0-ARB-002 — ambiguous owner
Ambiguity blocks write.

### P0-ARB-003 — owner change invalidates transaction
A transaction validated under provider A cannot apply after ownership moves to B without revalidation.

### P0-ARB-004 — rollback disclosure
Arbitration result reports rollback kind.

## P0-TRANSACTION

### P0-TXN-001 — happy path
Transaction reaches `COMMITTED` only through valid state transitions.

### P0-TXN-002 — validation failure
Invalid precondition ends in `REJECTED` and no apply occurs.

### P0-TXN-003 — authorization denied
Denied authorization performs no apply.

### P0-TXN-004 — apply failure
Apply error reaches `FAILED` or rollback path according to whether state changed.

### P0-TXN-005 — observation failure
Provider call success followed by failed health observation does not commit.

### P0-TXN-006 — rollback success
Failed observation with supported rollback ends `ROLLED_BACK`.

### P0-TXN-007 — rollback failure
Rollback failure ends `ROLLBACK_FAILED`.

### P0-TXN-008 — terminal immutability
A terminal transaction cannot re-enter an active state.

### P0-TXN-009 — idempotent retry
Same idempotency key cannot perform the same write twice.

### P0-TXN-010 — stale resource identity
Resource replacement between validation/apply blocks apply.

### P0-TXN-011 — daemon restart
Persisted nonterminal transaction is recovered into a defined state after daemon restart.

### P0-TXN-012 — client disconnect
Client disappearance does not lose the audit record.

## P0-DIAGNOSTICS

### P0-DIAG-001 — I/O budget veto
Critical I/O pressure prevents a high I/O-write-cost diagnostic.

### P0-DIAG-002 — memory budget veto
Critical memory pressure prevents large-memory diagnostic allocation.

### P0-DIAG-003 — disk-full degradation
Critical free-space condition forces memory-first recorder policy.

### P0-DIAG-004 — explain denial
A denied escalation returns a reason code.

### P0-DIAG-005 — lower-cost alternative
Budget manager can select a cheaper available diagnostic path.

## P0-EVENT

### P0-EVT-001 — monotonic ordering
Events preserve monotonic ordering despite wall-clock adjustment.

### P0-EVT-002 — normalized key
Equivalent volatile log/event variants can share a normalized key.

### P0-EVT-003 — raw reference preserved
Normalization does not destroy source-reference linkage.

### P0-EVT-004 — incident linking
Multiple correlated events can be linked into one incident without deletion.

## P0-RECORDER

### P0-REC-001 — bounded memory
Ring buffer never grows beyond configured limit.

### P0-REC-002 — dropped counter
Overflow increments a dropped-event counter.

### P0-REC-003 — storage failure
Persistence failure does not block the monitoring loop.

### P0-REC-004 — removable target rejected
Critical recorder path cannot be configured to a monitored removable device.

## P0-INDICATOR

### P0-IND-001 — GNOME compatibility
Chosen indicator works on Ubuntu GNOME 50/Wayland.

### P0-IND-002 — Xfce compatibility
Chosen indicator works on Xfce 4.20 Status Tray.

### P0-IND-003 — reconnect
Indicator reconnects after daemon or host restart.

## P0-PRIVILEGE

### P0-PRIV-001 — model A measurement
Hardened privileged-daemon prototype has documented required privileges and security review.

### P0-PRIV-002 — model B measurement
Split-privilege prototype has documented required privileges and security review.

### P0-PRIV-003 — decision record
One topology is selected with a written comparison.

---

# 37. Required Phase 1 tests

## P1-DAEMON

### P1-DMN-001 — boot start
Daemon starts successfully under systemd before graphical login.

### P1-DMN-002 — restart
Daemon restart preserves required persistent state.

### P1-DMN-003 — no desktop dependency
Daemon runs without GNOME/Xfce/session bus.

### P1-DMN-004 — clean stop
Stop completes without leaving corrupt transaction persistence.

### P1-DMN-005 — crash recovery
Forced daemon termination does not corrupt persisted state.

## P1-SYSTEMD

### P1-SYS-001 — unit read
Systemd provider can inspect an allowed unit through its provider interface.

### P1-SYS-002 — unavailable unit
Missing unit returns typed unavailable/not-found behavior.

### P1-SYS-003 — no direct client systemctl
GUI/TUI/CLI test confirms clients do not execute systemctl directly.

## P1-PSI

### P1-PSI-001 — parse CPU
CPU PSI parses valid data.

### P1-PSI-002 — parse memory
Memory PSI parses `some`/`full`.

### P1-PSI-003 — parse I/O
I/O PSI parses `some`/`full`.

### P1-PSI-004 — event trigger
VM PSI trigger path produces an event without busy polling.

### P1-PSI-005 — unavailable
Missing PSI produces explicit unsupported/unavailable state.

## P1-LOGIND

### P1-LGI-001 — list inhibitors
Inhibitors normalize into Guardian blocker records.

### P1-LGI-002 — no blockers
Empty result is healthy, not provider error.

## P1-UDISKS

### P1-UDS-001 — topology
Drive/block relationships are preserved.

### P1-UDS-002 — sibling relation
Shared physical parent/sibling information is visible.

### P1-UDS-003 — volatile name
Changing `/dev` name does not make Guardian treat the same hardware identity as automatically unrelated.

### P1-UDS-004 — removed device
Removal produces an event and invalidates stale resource references.

## P1-UPOWER

### P1-UPW-001 — display device
Display device is read when present.

### P1-UPW-002 — absent battery
Desktop without battery remains healthy with "not present".

## P1-ACCOUNTS

### P1-ACC-001 — provider detection
AccountsService is discovered.

### P1-ACC-002 — installed sessions enumerated
Valid graphical sessions are enumerated through the selected session-discovery adapter.

### P1-ACC-003 — invalid session rejected
Invalid session identifier fails validation before any write.

## P1-CLIENTS

### P1-CLI-001 — JSON output
CLI structured output parses as valid JSON.

### P1-CLI-002 — daemon offline
CLI returns deterministic exit/error behavior when daemon is unavailable.

### P1-TUI-001 — VT startup
TUI starts without graphical session.

### P1-GUI-001 — provider dashboard
GUI renders available/degraded/unavailable providers.

### P1-GUI-002 — transaction history
GUI renders transaction records from daemon state.

### P1-IND-001 — healthy state
Indicator renders healthy state.

### P1-IND-002 — degraded daemon
Indicator renders offline/degraded state without hanging.

## P1-PACKAGING

### P1-PKG-001 — install
Fresh Ubuntu 26.04.1 VM install succeeds.

### P1-PKG-002 — service files
Installed service/D-Bus/polkit files are in correct vendor locations.

### P1-PKG-003 — uninstall
Uninstall removes package-owned files without deleting user/admin state unless explicitly purging.

### P1-PKG-004 — purge
Purge semantics are documented and tested.

### P1-PKG-005 — no vendor-file mutation
Guardian does not modify another package's files during normal install.

## P1-SECURITY

### P1-SEC-001 — hardening review
`systemd-analyze security` artifact exists.

### P1-SEC-002 — path access
Service cannot write outside declared writable paths except through authoritative external provider APIs.

### P1-SEC-003 — no arbitrary shell API
Public interface remains free of generic command execution.

### P1-SEC-004 — privilege denial
Unauthorized client cannot use a test write action.

---

# 38. Gate model

No gate advances until all required tests for that gate pass.

## G0 — Public contracts

Required:

- D-Bus namespace selected;
- interface major selected;
- introspection fixtures committed;
- typed errors committed;
- no generic root command;
- provider provenance schema committed.

Tests:

```text
P0-DBUS-001..005
P0-REG-003..004
```

## G1 — Identity & authorization

Required:

- caller identity source fixed;
- polkit subject model fixed;
- graphical auth test;
- text auth test;
- noninteractive background behavior fixed.

Tests:

```text
P0-AUTH-001..005
```

## G2 — Privilege topology

Required:

- both privilege prototypes measured;
- topology selected;
- hardening requirements documented.

Tests:

```text
P0-PRIV-001..003
```

## G3 — Core data models

Required:

- Capability Registry;
- Provider Arbitrator;
- typed provider API;
- event/incident models.

Tests:

```text
P0-REG-001..004
P0-ARB-001..004
P0-EVT-001..004
```

## G4 — Transaction engine

Required:

- state machine;
- persistence;
- idempotency;
- authorization hook;
- observation;
- rollback.

Tests:

```text
P0-TXN-001..012
```

## G5 — Diagnostic safety

Required:

- Diagnostic Budget Manager;
- bounded recorder;
- PSI test fixtures.

Tests:

```text
P0-DIAG-001..005
P0-REC-001..004
```

## G6 — Indicator decision

Required:

- GNOME 50 test;
- Xfce 4.20 test;
- implementation selected and documented.

Tests:

```text
P0-IND-001..003
```

## G7 — Production daemon

Required:

- systemd unit;
- D-Bus service;
- persistent state;
- selected privilege topology implemented.

Tests:

```text
P1-DMN-001..005
P1-SEC-001..004
```

## G8 — Initial providers

Required:

- systemd;
- PSI;
- logind;
- UDisks;
- UPower;
- AccountsService.

Tests:

```text
P1-SYS-*
P1-PSI-*
P1-LGI-*
P1-UDS-*
P1-UPW-*
P1-ACC-*
```

## G9 — Clients & packaging

Required:

- CLI;
- TUI;
- GUI shell;
- indicator;
- Debian package;
- clean install/uninstall.

Tests:

```text
P1-CLI-*
P1-TUI-*
P1-GUI-*
P1-IND-*
P1-PKG-*
```

---

# 39. Implementation order

The required implementation sequence is:

```text
1. Test harness + schemas
2. D-Bus contract tests
3. Error model
4. Capability Registry
5. Provider API
6. Provider Arbitrator
7. Event/incident model
8. Transaction engine
9. Diagnostic Budget
10. Recorder
11. Authorization integration
12. Privilege topology prototypes and decision
13. Daemon systemd/D-Bus skeleton
14. Initial read-only providers
15. CLI
16. TUI shell
17. GUI shell
18. Indicator compatibility spike and selected implementation
19. Packaging
20. Full Ubuntu 26.04.1 VM acceptance pass
```

Do not implement later Guardian feature modules ahead of this order merely because their underlying Linux command is easy to call.

---

# 40. Forbidden shortcuts

Phase 0/1 MUST NOT:

- call `sudo` from GUI/TUI/indicator;
- run the whole GUI as root;
- create a generic root command broker;
- shell out where a stable D-Bus API is already selected for the provider;
- write `/etc` directly from an unprivileged client;
- auto-edit another package's vendor configuration during install;
- assume `/dev/sdX` is persistent identity;
- assume GNOME is always GDM or Xfce is always LightDM;
- assume a provider is healthy because its package is installed;
- use raw utilization alone as proof of a bottleneck;
- create an unbounded recorder queue;
- put critical recorder storage on monitored removable media;
- suppress raw journal evidence to implement deduplication;
- enable high/very-high recovery actions in Phase 1;
- silently continue after rollback failure.

---

# 41. CI / validation matrix

At minimum:

## Fast CI

Runs on every change:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
contract/introspection fixture tests
serialization compatibility tests
```

## D-Bus integration CI

Runs in Linux environment with private D-Bus:

```text
dbus-run-session ...
python3-dbusmock / test provider harness
```

## Hardware mock CI

Runs `umockdev` fixtures where practical.

## Ubuntu 26.04.1 VM validation

Runs for phase gates and release candidates:

- GNOME environment;
- Xfce environment;
- system bus;
- polkit;
- systemd;
- packaging;
- install/uninstall;
- recovery TUI access;
- indicator matrix.

VM tests MAY be slower but are mandatory gate evidence.

---

# 42. Test artifact retention

Phase gates MUST retain:

- test report;
- failing/passing counts;
- D-Bus introspection fixture;
- polkit policy fixture;
- provider provenance fixture;
- `systemd-analyze security` result;
- selected privilege-topology decision record;
- indicator compatibility report;
- Ubuntu package/version baseline;
- known deviations.

These artifacts belong in version control where appropriate.

---

# 43. Decision records required before Phase 1 completion

At minimum:

```text
ADR-001 Guardian D-Bus namespace and interface versioning
ADR-002 Privilege topology
ADR-003 Provider arbitration rules
ADR-004 Transaction persistence and recovery
ADR-005 Guardian local state format
ADR-006 Desktop indicator implementation
ADR-007 GUI/TUI client separation
ADR-008 Package/service filesystem layout
```

---

# 44. Deferred work

The following are explicitly outside this contract's completion requirement:

- actual UDisks power-off;
- USB reset/deauthorize/re-authorize;
- USBGuard write integration;
- fan PWM control;
- CoolerControl/LACT control;
- thermald tuning;
- NVIDIA power caps;
- SMART/NVMe deep health analysis;
- Log Lens finished UI;
- CUPS auto-remediation;
- automatic crash-loop pausing;
- GNOME/Xfce write switch;
- firmware updates;
- network configuration writes;
- firewall writes;
- AppArmor policy editor;
- kdump management;
- Dracut recovery module;
- pre-login full recovery target;
- eBPF deep diagnostic execution.

Tests and abstractions MAY anticipate these features, but Phase 1 must not expand into implementing them.

---

# 45. Phase 0 exit criteria

Phase 0 is complete only when:

- G0 through G6 pass;
- every P0 required test is green;
- privilege topology is selected;
- indicator implementation is selected;
- package/interface provenance fixtures are captured;
- transaction engine handles rollback failure and restart recovery;
- Diagnostic Budget can veto dangerous escalation;
- recorder is bounded;
- D-Bus/polkit contracts are frozen at major version 1 for Phase 1.

---

# 46. Phase 1 exit criteria

Phase 1 is complete only when:

- G7 through G9 pass;
- all required Phase 1 tests are green;
- daemon runs before login;
- clients operate without privilege;
- TUI works without a desktop;
- GUI, TUI, CLI, and indicator use the daemon rather than direct system mutation;
- initial read-only providers work or degrade safely;
- package installs cleanly on a fresh Ubuntu 26.04.1 VM;
- uninstall/purge behavior is tested;
- hardening review is captured;
- no high/very-high production write path exists;
- no open P0/P1 defect is classified as safety-critical or contract-breaking.

---

# 47. Handoff to Phase 2

Phase 2 may begin only after the Phase 1 exit criteria are satisfied.

The Phase 2 planning input will be:

- stable Guardian daemon/API;
- stable Capability Registry;
- stable Provider Arbitrator;
- stable transaction framework;
- stable event/incident model;
- stable Diagnostic Budget;
- initial platform providers;
- proven client shells;
- package/install foundation.

Phase 2 then expands **read-only observability and correlation** without needing to redesign the privilege/control plane.

---

# 48. Definition of done for this TDD contract

This contract is ready for coding when the implementation agent can answer all of the following from this document without inventing architecture:

1. What is allowed to run privileged?
2. How is caller identity established?
3. How is a write authorized?
4. How is a provider selected?
5. What happens if two providers conflict?
6. What makes a transaction commit?
7. What happens if rollback fails?
8. What happens if the client disappears?
9. What happens if the daemon restarts?
10. How does diagnostic load get constrained?
11. How are provider API changes detected?
12. How are GUI/TUI/CLI prevented from bypassing the daemon?
13. How is storage identity modeled safely?
14. What must be tested on real Ubuntu 26.04.1 rather than mocks?
15. What work is explicitly deferred?

If any of these answers becomes ambiguous during implementation, the coding agent MUST stop that affected implementation path and record the ambiguity as a contract issue rather than silently choosing a new architecture.

---

# 49. Source basis

This contract is derived from:

- `Guardian: An Ubuntu 26.04 System Control Plane` merged specification;
- `GUARDIAN_PHASE_0_CONTRACT_RESEARCH.md`;
- Ubuntu 26.04.1 package/API research covering systemd, D-Bus, polkit, NetworkManager, UDisks2, AccountsService, UPower, power-profiles-daemon, thermald, fwupd, logrotate, Ayatana indicator libraries, zbus, ksni, `python3-dbusmock`, and `umockdev`.

The research artifact remains the evidence/reference document.

This TDD contract is the implementation-governing document for Phase 0 and Phase 1.
