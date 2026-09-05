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

This TDD contract is the implementation-governing document for Phase 0 and
Phase 1. See §50 for an amendment extending its governance to Wave 1.

---

# 50. Amendment — Wave 1: first production mutation capability

- Status: Accepted
- Date: 2026-09-04 (revised 2026-09-04 after independent planning review;
  see "Revision history" at the end of this section)
- Clarifies sequencing relative to §47 ("Handoff to TDD-contract Phase
  2"). Does not rename, renumber, or relocate §47's own scope.

## Context

G0 through G9 are independently gated, accepted, committed, and tagged
(`phase0-g0-public-contracts` through `phase0-g9-clients-packaging`) —
Phase 1's exit criteria (§46) are satisfied. §47, as originally written,
characterized the next TDD-contract phase ("TDD-contract Phase 2") as
expanding **read-only observability and correlation** only, with no
mention of a production write path. Planning work for a first production
mutation capability was requested at this point in the project's
history. §47's original framing does not authorize that work as
written, and per this contract's own §48/AGENTS.md discipline ("stop and
record the ambiguity as a contract issue rather than silently choosing a
new architecture"), that gap was surfaced rather than silently bridged.

**This contract uses the unqualified term "Phase 2" nowhere below this
point — only "TDD-contract Phase 2" or "master-spec Phase 2," always
qualified.** An independent review of the first version of this section
found that its own text simultaneously called Wave 1's mutation work
"TDD-contract Phase 2" (to justify a `P2-*` ID prefix) while *also*
continuing to call §47's original, undisturbed scope "TDD-contract
Phase 2" — two different things sharing one unqualified label inside
the same governing section, with no
rule for which one a `P2-*` ID would belong to once the read-only work
actually starts. That defect is fixed by this revision: **Wave 1 is not
TDD-contract Phase 2, was never renamed to TDD-contract Phase 2, and does
not consume that label in any sense.** It is a separate, unnumbered,
interstitial phase, and its own normative IDs use a distinct prefix
(`W1-`, never `P2-`) precisely so they can never be confused with the
numbered `P0-`/`P1-` sequence or with whatever prefix a future, real
TDD-contract Phase 2 implementation eventually adopts for its own IDs.

This section preserves §47 unchanged above (per AGENTS.md's "supersede,
don't hide" rule for historical decisions) and adds, alongside it, a
narrowly-scoped, evidence-gated phase named **Wave 1**, whose sole
purpose is proving the G4 transaction engine and G1/G2 authorization/
privilege model against one real production write path. §47's own
read-only observability/correlation expansion (TDD-contract Phase 2)
remains valid future work and is not itself started by this amendment —
Wave 1 and TDD-contract Phase 2 are independent, not sequential; nothing
here requires one to finish before the other begins, and nothing here
implies Wave 1 precedes or follows TDD-contract Phase 2 in any required
order.

**Disambiguation rule, binding on every future reference in this
document and in any Wave 1 handoff**: this contract has, after this
amendment, exactly three distinct things that must never be referred to
by the bare word "Phase 2" without a qualifier:

```text
TDD-contract Phase 2   — §47's read-only observability/correlation
                          expansion. Unchanged, unrenamed, unstarted.
master-spec Phase 2    — the master spec's own, differently-numbered
                          staged rollout ("Phase 2 — I/O Guardian"),
                          an entirely separate numbering system.
Wave 1                 — this section's own interstitial phase. Not a
                          numbered phase in either system above.
```

## Decision

Wave 1 authorizes exactly one first production mutation capability,
selected and planned under the acceptance bar below, before any other
new write capability. It does not authorize master-spec-Phase-2 (I/O
Guardian), TDD-contract-Phase-2 read-only correlation/incident
intelligence, or any other master-spec-numbered phase — those remain
separately gated, exactly as before this amendment, and exactly as
before this revision.

## Wave 1 acceptance bar

A candidate capability is eligible for selection as Guardian's first
production mutation only if it satisfies every one of the following,
with real evidence, not assertion:

- a narrow, typed operation — never a generic action-name/JSON broker,
  never `RunCommand`/`RunShell` or an equivalent (AGENTS.md Privilege
  rules, unchanged and non-negotiable here);
- a real, already-evidenced G8 read provider to extend, or an accepted
  G2 privilege classification (`docs/evidence/g2/
  PRIVILEGE_REQUIREMENT_INVENTORY.md`) if no G8 provider exists yet;
- an identified, non-`Conflict` authorization owner, decided under the
  **relay-authorization rule** below — "provider-owned authorization"
  (per G2's inventory classification) is a real, useful signal about
  *which provider is authoritative*, but it is **not**, by itself,
  sufficient reason to skip a real, independent authorization check for
  the real caller — that check must query the provider's own action, not
  a Guardian-invented substitute (see the relay-authorization rule);
- deterministic preconditions the G4 `Validate` step can check before
  any mutation begins;
- a real, externally observable postcondition Guardian can `Observe`
  without guessing (TDD contract §14/§19's Apply/Observe/Confirm
  discipline, unmodified);
- bounded, narrow blast radius — one explicitly-named target, never an
  arbitrary or client-suppliable target set;
- a real rollback or compensating action, or an honest `RollbackKind::
  None` disclosure if none exists (§13's existing `RollbackKind`
  taxonomy, unmodified — no new rollback category is introduced by this
  amendment);
- no irreversible physical-hardware action;
- fully closable with real disposable-VM evidence (§35 Layer 4), with no
  physical-hardware layer (§35 Layer 5) required to reach acceptance;
- a `RecoveryClassification` (`crates/guardian-core/src/transaction/
  recovery.rs`, unmodified) provably derivable for every crash point in
  its own Apply/Observe sequence — `SafeToResume` must be earned per
  candidate, never inherited from G7's `GuardedWrite` evidence fixture;
  if it cannot be proven, the conservative classification (`MustObserve`/
  `MustRollback`/`StateAmbiguous`/`RequiresHumanRecovery`, whichever the
  actual uncertainty demands) is used, not `SafeToResume` by default;
- resolvable single-writer/competing-writer semantics using the existing
  Provider Arbitrator (`crates/guardian-core/src/arbitration.rs`,
  unmodified — this amendment does not authorize changing it) — if the
  Arbitrator's existing `Ownership`/`ArbitrationDecision` model cannot
  represent the candidate's real writer landscape, that candidate is not
  eligible until the Arbitrator itself is extended under its own gate
  discipline, not silently worked around.

## The relay-authorization rule (added by this revision; corrected below, three times)

An independent planning review of the first version of this section
found, and empirically confirmed against real `systemd`/polkit behavior
in a disposable VM, a defect this rule closes: "provider-owned
authorization" (G2's classification: *the provider performs its own
polkit check*) is a true statement about the provider's own D-Bus
surface, but it silently stops being a meaningful authorization
boundary once Guardian's own privileged process (`guardian-helper`,
`User=root` per ADR-002/G7) is the one that actually issues the D-Bus
call. Polkit authorizes *whoever is on the wire* — for a call
`guardian-helper` makes on the system bus, that is `guardian-helper`
itself (root), not the real end client, because an ordinary D-Bus method
call carries no forwarded/impersonated subject. A provider's own
`auth_admin`-class check therefore passes unconditionally for a root
caller, regardless of who the real requester was — this is not a
theoretical risk, it was reproduced directly (an unprivileged, non-admin
VM user's own `RestartUnit` call was correctly denied against that
user's identity; the identical call issued as root returned a job path
with no check performed at all).

A second independent review, focused specifically on the fix the first
review's finding produced, found that fix itself defective: requiring a
brand-new **Guardian-owned** `PolkitAction`/`.policy` action for every
relayed "provider-owned" capability does not preserve provider-owned
authorization — it silently replaces it. Two distinct claims were being
conflated:

```text
trusted to query polkit's authorization decision for another subject
≠
owner of the policy that decision is made against
```

`guardian-helper` running as root, with `resolve_caller_identity`
resolving the real caller from its own inbound D-Bus connection, gives
it the first property (per ADR-002's trusted-caller finding). Nothing
about that trusted-caller property requires or implies the second:
`guardian-helper` may query the *provider's own, real, already-shipped*
polkit action for the resolved caller, and that continues to be a
provider-owned decision — the provider's own `.policy` file, and
whatever an OS administrator has already configured against it, remains
authoritative. A Guardian-invented action instead makes Guardian's own
policy the effective authority for the operation (any existing OS-admin
grant/denial against the provider's real action becomes irrelevant; a
separate, Guardian-only grant could diverge from the provider's own
policy in either direction) — which is a real reclassification from
provider-owned to Guardian-owned authorization that the first fix made
silently, never stating or justifying it as such.

A third independent review, examining whether this corrected rule was
actually *implementable* against the real, existing authorization
machinery, found it was not, as originally worded: `PolkitAction`
(`crates/guardian-core/src/authorization.rs`) is a closed enum with no
generic, string-carrying variant, and `Authorizer::authorize()` takes a
typed `PolkitAction`, not a raw action-id string — there was, and is, no
way to check `org.freedesktop.systemd1.manage-units` through that exact,
unmodified type without either adding a new `PolkitAction` variant
(reintroducing the second defect above) or otherwise changing
`authorization.rs`'s shape, which the rule as first corrected never
disclosed or scoped. The rule is corrected a second time below to close
this gap, deliberately, without collapsing into a generic raw-string
authorization surface.

A fourth independent review, checking whether the type-level split alone
was sufficient, found it was not: **checking the correct action id is
necessary but not sufficient to reproduce a provider's real authorization
decision.** Empirically confirmed by snooping the real `CheckAuthorization`
call in a disposable VM (`busctl monitor --system`) while running
`systemctl restart cups.service` as an unprivileged user, systemd's own
request for `manage-units` carries non-empty polkit **details**, not an
empty map:

```text
action_id: "org.freedesktop.systemd1.manage-units"
details:
  unit                  = "cups.service"
  verb                  = "restart"
  polkit.message        = "Authentication is required to restart '$(unit)'."
  polkit.gettext_domain = "systemd"
```

A real administrator's polkit `.rules` file may legitimately branch on
`action.lookup("unit")`/`action.lookup("verb")` — a standard, documented
pattern for exactly this action (e.g. "allow `manage-units` only when
`unit == "cups.service"` and `verb == "restart"`"). The existing
`PolkitAuthorizer::authorize()` (`crates/guardian-core/src/authorization/
polkit.rs`) hardcodes an **empty** `details` map for every current
`PolkitAction` variant — reusing that transport unmodified, as the third
correction assumed, would send systemd's real action id with no `unit`/
`verb`, silently diverging from what a direct systemd call would present
to the same admin policy. This is a real provider-policy-fidelity defect,
not a cosmetic one: it means "provider policy remains authoritative"
would be false for any detail-sensitive rule.

**Binding rule (corrected a third time)**: `PolkitAction` remains
reserved for **Guardian-owned** authorization decisions only — nothing
here adds a systemd (or other provider) variant to it. A mediated
provider-policy authorization request is more than an action id: it is
the action id **plus every authorization-relevant detail the provider's
own real request would carry**, all of it internally derived from
already-validated, Guardian-controlled data — never from caller input,
and never as an open `details: HashMap<String, String>`-shaped
parameter a caller (or careless future call site) could populate
arbitrarily. For Wave 1's one candidate, the complete, closed
representation is conceptually:

```text
enum ProviderAuthorizationRequest {
    SystemdRestart { capability: RestartCapability },
    // action id, and details {unit, verb, polkit.message,
    // polkit.gettext_domain}, are derived internally from `capability`
    // -- never accepted as separate caller-suppliable fields.
}
```

(exact Rust naming/shape is not binding — a fieldless `ProviderPolkitAction`
enum may still exist internally if useful, but the handoff must make
clear that *action identity alone is not the complete provider
authorization request*; the resolved capability row is what the request
is actually built from, and it must carry the complete details a genuine
direct call would send — for Wave 1's evidenced request, all four
observed fields: `unit`, `verb`, `polkit.message`, `polkit.gettext_domain`
— not a partial subset chosen for convenience). A new, equally typed
entry point (e.g. `authorize_provider_request(subject: CallerIdentity,
request: ProviderAuthorizationRequest, interactive: bool)`) issues the
real `CheckAuthorization` call built from that closed request against
the resolved caller — it may share the existing `PolkitAuthorizer`/
`CheckAuthorization` D-Bus plumbing internally (via a dedicated internal
path that accepts the derived, closed details, not a globally-widened
`details` parameter on every existing call), but its *public, typed*
boundary remains closed exactly like `PolkitAction`'s: no caller-supplied
raw action-id string, no caller-supplied detail keys or values, no
generic `authorize_provider(action_id: &str, details: HashMap<String,
String>, ...)`-shaped interface. The semantic split is explicit and
permanent: `authorize(PolkitAction)` decides a Guardian-owned policy;
`authorize_provider_request(ProviderAuthorizationRequest)` mediates a
provider's own policy, complete with its real authorization-relevant
context, for the resolved caller. Both may issue a `CheckAuthorization`
D-Bus request underneath — what differs is *policy ownership*, and which
action identifiers and detail values are even expressible, not the
transport.

If a future implementation pass discovers that the currently-shipped
Ubuntu/systemd version sends additional or different authorization
details for this exact operation than the four evidenced above,
implementation MUST stop and reconcile this contract rather than
silently dropping or inventing fields — the evidenced request above is
the acceptance bar, not a floor implementation may freely fall below.

Whenever a Wave 1 (or later) capability is realized by having
`guardian-helper` issue a call to a "provider-owned authorization" D-Bus
method on the real end client's behalf, `guardian-helper` MUST call
`authorize_provider_request` against the real, resolved caller identity
**before** making that call — using the *provider's own, real,
already-shipped* action id **and its complete, evidenced authorization
details** for the operation being relayed (e.g. action
`org.freedesktop.systemd1.manage-units` with details `{unit:
"cups.service", verb: "restart", polkit.message: "Authentication is
required to restart '$(unit)'.", polkit.gettext_domain: "systemd"}` for
Wave 1's own candidate — confirmed present at `/usr/share/polkit-1/
actions/org.freedesktop.systemd1.policy` and empirically observed on the
D-Bus wire in a disposable Ubuntu 26.04.1 VM), never a Guardian-invented
`PolkitAction` substitute and never a partial details map missing fields
the real provider request would carry. The target provider's own
subsequent internal check is real and may still run — it is not
redundant, since it still governs whatever policy the provider itself
applies once the call arrives from root — but Guardian's own mediating
check is what makes the decision discriminate between real end users,
since the provider's own check alone cannot do that once relayed
through root. This means "provider-owned authorization" no longer
implies "no independent Guardian-side check is needed" (the first
review's finding, still true), does NOT mean "therefore Guardian needs
its own new, Guardian-owned `PolkitAction`" (the second review's
finding, still true), and does NOT mean "the correct action id alone is
sufficient" (the third review's finding, still true, and this
revision's own subject) — the correct reading is "Guardian mediates the
provider's own authorization decision for the real caller, reproducing
the complete request the provider itself would issue, through its own
closed, typed representation of that provider's policy and context; it
does not become a second, competing authority, and it does not silently
drop authorization-relevant context the provider's own admin policy may
depend on." A relayed capability under this rule requires **no new
`PolkitAction` variant and no new Guardian `.policy` file** — it
requires a new, closed provider-authorization-request representation
(action id and complete details, both internally derived from an
already-resolved, Guardian-controlled capability, never from caller
input) for the specific provider operation being mediated, which is a
small, disclosed, independently-reviewable typed addition, not an
open-ended authorization surface.

This rule also explains, retroactively, why G7's own independent audit
rejected an earlier `Guardian1.Transactions1.AttemptProviderDelegatedWrite`
addition as "an unjustified permanent production API addition"
(`docs/evidence/g7/G7_MILESTONE.md`, Round 1 finding) — that addition
was the same relay shape this rule now governs explicitly, rather than
leaving each future gate to rediscover the same defect independently.
It does not, however, justify treating `GuardedWrite`'s own action
(`io.github.cliffthelin.guardian.g7.bounded-write`) as a template for
relayed *provider-owned* capabilities: `GuardedWrite`'s action is
correct precisely because the write it gates has no other provider and
is genuinely Guardian-owned — mirroring its shape for a capability that
G2 already classified as provider-owned would mirror the wrong
precedent, which is exactly the error this corrected rule fixes.

## Required governing material for the Wave 1 candidate

A Wave 1 implementation handoff and independent-review handoff, in the
same paired structure every G0–G9 gate used, are required before any
Wave 1 code is written — this amendment authorizes planning and
candidate selection, not implementation. New normative IDs use the
prefix **`W1-`** (e.g. `W1-MUT-*`, `W1-AUTH-*`, `W1-TXN-*`, `W1-REC-*`,
`W1-VM-*`) — never `P2-*` or any other prefix from the `P<phase>-`
numbered sequence, per the disambiguation rule above.

## Scope exclusions (restated for this amendment)

Wave 1 does not authorize, and any implementation handoff produced under
it must not silently begin:

- I/O Guardian (master-spec Phase 2 — UDisks/udev/PSI correlation chain,
  recovery ladder, incident recorder);
- TDD-contract Phase 2 read-only observability/correlation (§47's own
  original scope, independent of Wave 1, not started by this amendment);
- any master-spec-later-phase capability (thermal/power profiles beyond
  a read-only extension, logs/incidents, general system management)
  unless it is itself the one selected Wave 1 candidate;
- the master spec's explicitly-deferred "highest-risk tier" operations
  (general fan overclocking, kernel parameter tuning, automatic driver
  changes, forced USB resets, automatic service disabling) under any
  circumstance as part of Wave 1.

## Consequences

Guardian's transaction/safety framework (G1–G4) is now tested against a
real external write, not only G7's evidence fixture — the actual
architectural claim Phase 1 exists to support. TDD-contract Phase 2's
read-only observability/correlation expansion remains available as
independent future work, ungated by Wave 1's outcome either way. The
relay-authorization rule is now standing architecture for any future
gate that considers relaying a "provider-owned authorization" write
through `guardian-helper` — it is not specific to Wave 1's own selected
candidate.

## Revision history

- 2026-09-04, initial: authorized Wave 1, used a `P2-*` ID prefix and a
  "provider-owned authorization... requires no new Guardian-side
  privilege decision" acceptance-bar criterion.
- 2026-09-04, second revision: an independent planning review found the
  initial version's own text ambiguous about whether Wave 1 *was* Phase
  2 (fixed by the disambiguation rule and the `W1-` prefix), and found,
  with real VM evidence, that the "provider-owned authorization requires
  no new Guardian-side decision" criterion was actually false for any
  capability relayed through `guardian-helper` (fixed by the first
  version of the relay-authorization rule, which required a new
  Guardian-owned `PolkitAction`/`.policy` action for every such
  capability).
- 2026-09-04, third revision: a further independent review found the
  second revision's own relay-authorization rule defective — requiring a
  Guardian-owned action for a relayed provider-owned capability silently
  converts that capability's authorization from provider-owned to
  Guardian-owned, without ever stating or justifying that
  reclassification, and discards any authorization meaning an OS
  administrator's existing grants/denials against the provider's own
  real action (e.g. `org.freedesktop.systemd1.manage-units`) already
  had. The rule was corrected: `guardian-helper` mediates the provider's
  own real polkit action for the resolved caller, rather than
  substituting a Guardian-invented one.
- 2026-09-04, fourth revision (this one): a further independent review
  found the third revision's own rule unimplementable as worded against
  the real, existing `PolkitAction`/`Authorizer` machinery — that
  machinery has no way to represent or check a provider's own action id
  without either reintroducing a Guardian-owned substitute (the second
  revision's defect) or an undisclosed change to `authorization.rs`. The
  rule is corrected above a second time: `PolkitAction` remains reserved
  for Guardian-owned decisions only; a new, equally closed
  `ProviderPolkitAction` enum (one variant for Wave 1,
  `SystemdManageUnits`) represents provider-owned policy actions Guardian
  is permitted to mediate, through a new, equally typed entry point —
  never a generic raw-action-id interface. This is a small, disclosed,
  independently-reviewable production change to `authorization.rs`
  (previously described as remaining unmodified — that framing is
  corrected here), not a new authorization model. Nothing in this
  revision weakens the acceptance bar to fit a specific candidate — it
  corrects the bar itself, again.
- 2026-09-04, fifth revision (this one): a further independent review
  found that the correct action id alone does not reproduce a
  provider's real authorization decision — empirically confirmed by
  observing systemd's real `manage-units` `CheckAuthorization` request
  on the D-Bus wire in a disposable VM, which carries non-empty details
  (`unit`, `verb`, `polkit.message`, `polkit.gettext_domain`) that a
  real admin polkit rule may branch on, while the fourth revision's
  design would have issued the mediated check with an empty details map.
  The rule is corrected a third time: a mediated provider-authorization
  request now means the action id **and** its complete, evidenced
  authorization details, both derived internally from an already-
  resolved, Guardian-controlled capability — never from caller input,
  and never through an open `details` map any call site could populate
  freely. All four evidenced detail fields are preserved for Wave 1's
  candidate, not a subset chosen for convenience — distinguishing
  normative policy context from presentation-only metadata is deferred
  to a possible future general provider-authorization abstraction, not
  decided prematurely here. Nothing in this revision weakens the
  acceptance bar to fit a specific candidate — it corrects the bar
  itself, a third time.

## Rollback / migration implications

If Wave 1's selected candidate is later found unsafe in practice, this
amendment's acceptance bar — including the relay-authorization rule — is
what any successor candidate must also satisfy; the bar itself is not
weakened to accommodate a specific candidate's shortcoming. The
candidate is replaced, or the Provider Arbitrator is extended under its
own gate discipline first, as the specific failure requires.
