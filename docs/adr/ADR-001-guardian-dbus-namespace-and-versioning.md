# ADR-001: Guardian D-Bus namespace and interface versioning

- Status: Pending owner namespace decision; development mechanics accepted for G0
- Date: 2026-08-30
- Governing gate: G0 — Public Contracts

## Context

Guardian needs a stable well-known D-Bus name, interface namespace, and object-path convention before a public release. Reverse-domain names imply ownership. The repository contains no domain, organization URL, remote, or other evidence that establishes legitimate ownership of a permanent namespace.

G0 still needs a real, introspectable interface so its mechanics can be tested without fabricating ownership.

## Decision

Until the owner selects and documents a legitimate permanent namespace, G0 uses this explicitly non-production namespace:

```text
Interface:   org.guardianproject.Development.Guardian1
Object path: /org/guardianproject/Development/Guardian1
```

`Development` is mandatory in every temporary public name and path. No production package may claim this is permanent.

Public interface names carry a numeric major suffix from their first version. Compatible additive changes may remain in `Guardian1`; incompatible signature, removal, meaning, error-semantic, or authorization-semantic changes require a new major. Object paths mirror the namespace as slash-separated components and end in the interface-major root.

The harmless read-only G0 methods report contract version and service state. They do not imply system-management capability.

## Namespace legitimacy

No permanent namespace is selected. `org.guardianproject.Development` is a conspicuous development label, not an assertion that this repository owns `guardianproject.org`. This prevents temporary tests from being mistaken for a production ownership claim.

The repository owner must provide evidence for a domain or organization namespace before final G0 acceptance. The resulting namespace decision must supersede this ADR status and replace every development name consistently.

## Alternatives considered

- Inventing a reverse-DNS domain was rejected because the repository provides no ownership evidence.
- Reusing a personal or GitHub namespace was rejected because no repository remote or organization is established.
- An unqualified name was rejected because it would not exercise the intended D-Bus naming and object-path mechanics.
- Delaying all D-Bus work was rejected because isolated G0 contract mechanics can be proven safely under a development name.

## Evidence and tests

- `P0-DBUS-001` introspects the live registered object on a private bus.
- `P0-DBUS-002` enforces the interface-major suffix.
- `P0-DBUS-003` audits the live Guardian interface for generic execution surfaces.
- `P0-DBUS-005` verifies ordinary unknown-method failure and continued service operation.

These tests are rooted in `docs/guardian/20_Control_Plane/D-Bus_API_Contract.md` and the local Ubuntu/zbus source snapshots.

## Consequences

The mechanics can become green, but the gate verdict must remain:

```text
MECHANICALLY GREEN / NAMESPACE DECISION PENDING
```

Changing the namespace later is intentionally treated as a migration, not a cosmetic rename. It changes well-known names, interface identities, object paths, generated client bindings, fixtures, policies, service activation, documentation, and any installed client expectations.

## Rollback and migration

There is no compatibility promise for the development namespace. Before a release, select the legitimate permanent namespace, update all names atomically, regenerate/capture introspection evidence, rerun G0, and record the ownership basis in a superseding ADR decision. Once a permanent namespace ships, a later rename requires a versioned compatibility/migration plan rather than silent replacement.

