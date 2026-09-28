---
title: "Wiki Update Workflow"
kind: "operations"
status: "active"
last_reviewed: "2026-08-30"
tags:
  - wiki
  - maintenance
---
# Wiki Update Workflow

## New feature

Create or update, in order:

1. `20_Control_Plane/` feature page or `40_Modules/` module page.
2. Relevant `10_Platform/` provider pages.
3. Relevant `90_Sources/wiki/` source snapshots and `SOURCE_REGISTRY.md`.
4. `LOOKUP_MAP.md` keywords.
5. TDD gate/test pointer.
6. ADR if the change makes or reverses an architectural decision.
7. `00_Project/GUARDIAN_ROADMAP.md` if the change alters what counts as
   complete, in progress, or planned — see "Roadmap maintenance" below.

## Roadmap maintenance

`00_Project/GUARDIAN_ROADMAP.md` is project memory: the wiki's own record
of completed, in-progress, and planned work, and the page `AGENTS.md`
requires every agent to consult first for a status question. It goes
stale silently if a change is committed without it. Update it, in the
same change, whenever:

- a gate, wave, or Master-Spec phase changes status (opened, passed,
  repaired, superseded);
- an R0/R1/R2… (or equivalent) decision is accepted or reopened;
- a Hardening Backlog entry opens or closes at a scope large enough to
  change the roadmap's "in progress" summary.

Bump the roadmap's `last_reviewed` field on every such update. Do not
restate detailed acceptance criteria there — link to the TDD Gate Index,
the relevant Phase Execution Spec, or the Hardening Backlog instead. A
change that alters project status without updating the roadmap is
incomplete; see `AGENTS.md`'s "Completion report" → "Wiki/roadmap
updated."

## Source separation

- Guardian-authored interpretation: `00_Project`, `10_Platform`, `20_Control_Plane`, `40_Modules`.
- Governing test documents: `30_TDD`.
- External-document snapshots/pointers: `90_Sources`.
