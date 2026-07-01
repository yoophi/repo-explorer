<!--
Sync Impact Report
Version change: template -> 1.0.0
Modified principles:
- Placeholder principles -> I. Hexagonal Rust Boundaries
- Placeholder principles -> II. Feature-Sliced React Structure
- Placeholder principles -> III. Explicit State Ownership
- Placeholder principles -> IV. JSON Persistence Behind Ports
- Placeholder principles -> V. Safe and Observable Desktop Actions
- Placeholder principles -> VI. Storybook-Backed UI Changes
- Placeholder principles -> VII. Korean Documentation with Visual Flows
Added sections:
- Technology Constraints
- Development Workflow & Quality Gates
Removed sections:
- None
Templates requiring updates:
- ✅ .specify/templates/plan-template.md
- ✅ .specify/templates/spec-template.md
- ✅ .specify/templates/tasks-template.md
- ⚠ .specify/templates/commands/*.md (not present in this project)
Runtime guidance:
- ✅ AGENTS.md reviewed; already aligned with constitution principles
- ✅ README.md reviewed; no principle references required
- ✅ docs/dynamic-dev-port.md reviewed; already follows documentation rule
Follow-up TODOs:
- None
-->
# Repo Explorer Constitution

## Core Principles

### I. Hexagonal Rust Boundaries
Rust code MUST follow hexagonal architecture. Domain logic MUST NOT directly
depend on Tauri commands, filesystem access, git CLI calls, OS APIs, or JSON
file formats. Inbound adapters MUST translate UI and Tauri requests into
application ports. Outbound adapters MUST own access to external systems such
as filesystem, git, OS integration, and persistence. Application and service
code MUST compose use cases through port interfaces.

Rationale: repository discovery, git state, app launch behavior, and
persistence are OS-dependent concerns. Keeping them outside domain rules makes
the core behavior testable and replaceable.

### II. Feature-Sliced React Structure
React code MUST follow feature sliced design. App bootstrap concerns belong in
`app`; route assembly belongs in `pages`; large cross-feature UI blocks belong
in `widgets`; user actions belong in `features`; domain models, APIs, query
keys, and types belong in `entities`; reusable UI, hooks, and libraries belong
in `shared` or the workspace UI package. New UI MUST NOT cross layer
responsibilities without an explicit plan justification.

Rationale: the desktop source tree already follows an `app`, `pages`,
`entities`, and `shared` direction. Clear ownership prevents route files from
absorbing feature and entity logic as the app grows.

### III. Explicit State Ownership
Server state, asynchronous requests, caching, and revalidation MUST use React
Query. Client-global state such as UI preferences, selected repositories, and
temporary session state MUST use Zustand or another explicitly named client
state store. React local state MUST be limited to state whose meaning ends
inside the component.

Rationale: repository scans, metadata updates, selected repositories, and
pending actions are easy to mix together unless ownership is fixed at the
architecture level.

### IV. JSON Persistence Behind Ports
The default persistence model is a single JSON file. JSON reading and writing
MUST be isolated in Rust outbound adapters. Domain and application layers MUST
access stored data only through repository ports. Any JSON schema change MUST
include a migration plan or backward-compatible handling in the feature spec
or implementation plan.

Rationale: local storage such as `.repo-explorer.json` is user data. If file
format assumptions spread through the codebase, future schema changes become
risky and expensive.

### V. Safe and Observable Desktop Actions
User-visible desktop actions MUST report success or failure clearly. This
includes filesystem operations, git CLI calls, terminal/editor/Finder launch,
and clipboard access. Failures MUST NOT corrupt screen state or leave the user
without feedback. Paths containing spaces, Korean text, and special characters
MUST be handled correctly.

Rationale: the app's core value is connecting local repositories with external
tools. Silent OS action failures cause immediate user confusion and data-risky
retries.

### VI. Storybook-Backed UI Changes
New React components and meaningful new UI states MUST be registered in
Storybook. Stories MUST follow atomic design categories: Atoms, Molecules,
Organisms, and Pages. Stories MUST include realistic states for the component,
such as empty, loading, error, long path or name, pending, success, and failure
where applicable.

Rationale: repo explorer UI displays many paths and statuses. Storybook gives
the project a practical way to catch visual regressions before desktop flows
are exercised manually.

### VII. Korean Documentation with Visual Flows
Files under `docs/*.md` MUST use English kebab-case filenames and Korean body
text. Documentation that explains architecture, execution flow, state
transitions, or dependency relationships MUST include Mermaid.js diagrams.

Rationale: existing documentation follows this convention, and local desktop
workflows are easier to maintain when their flows are visible.

## Technology Constraints

Package management MUST use the pnpm workspace. The desktop app MUST use
Tauri with React. Shared UI primitives belong in `packages/ui`; app-specific
composition MUST follow the `apps/desktop/src` layer rules. OS integration
MUST be implemented through Rust adapters or explicitly approved Tauri plugin
boundaries.

## Development Workflow & Quality Gates

Spec Kit features MUST follow the `/speckit.specify` -> `/speckit.plan` ->
`/speckit.tasks` workflow. The plan phase MUST run a constitution check for
Rust boundaries, React layer ownership, state ownership, persistence
boundaries, safe desktop actions, Storybook coverage, and documentation
requirements. Each feature plan MUST pass the constitution check or document
the exception reason and follow-up remediation.

Implementation verification MUST match the changed surface area. Plans and
tasks MUST include the necessary subset of typecheck, Rust tests,
Storybook/build validation, and quickstart verification. Existing dirty
worktree changes MUST be preserved; unrelated changes MUST NOT be reverted.

## Governance

This constitution is the long-term authority for Repo Explorer engineering
principles. `AGENTS.md` provides execution guidance and detailed working rules
but does not supersede this constitution.

Amendments MUST update the constitution version and review affected Spec Kit
templates and runtime guidance. Adding a new principle or materially expanding
guidance requires a MINOR version bump. Changing or removing the meaning of an
existing principle requires a MAJOR version bump. Editorial clarification that
does not change requirements requires a PATCH version bump.

Every feature plan MUST verify compliance with the current constitution. A
plan that cannot comply MUST document the exception, the reason it is
necessary, and a follow-up cleanup plan.

**Version**: 1.0.0 | **Ratified**: 2026-07-01 | **Last Amended**: 2026-07-01
