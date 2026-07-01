# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]
**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

[Extract from feature spec: primary requirement + technical approach from research]

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: [e.g., Python 3.11, Swift 5.9, Rust 1.75 or NEEDS CLARIFICATION]  
**Primary Dependencies**: [e.g., FastAPI, UIKit, LLVM or NEEDS CLARIFICATION]  
**Storage**: [if applicable, e.g., PostgreSQL, CoreData, files or N/A]  
**Testing**: [e.g., pytest, XCTest, cargo test or NEEDS CLARIFICATION]  
**Target Platform**: [e.g., Linux server, iOS 15+, WASM or NEEDS CLARIFICATION]
**Project Type**: [e.g., library/cli/web-service/mobile-app/compiler/desktop-app or NEEDS CLARIFICATION]  
**Performance Goals**: [domain-specific, e.g., 1000 req/s, 10k lines/sec, 60 fps or NEEDS CLARIFICATION]  
**Constraints**: [domain-specific, e.g., <200ms p95, <100MB memory, offline-capable or NEEDS CLARIFICATION]  
**Scale/Scope**: [domain-specific, e.g., 10k users, 1M LOC, 50 screens or NEEDS CLARIFICATION]

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Document PASS/FAIL for each gate. Any FAIL requires an entry in Complexity
Tracking with an exception reason and follow-up remediation.

- **Hexagonal Rust Boundaries**: Domain logic is free of Tauri, filesystem,
  git CLI, OS API, and JSON file format dependencies; external access is
  behind application ports and outbound adapters.
- **Feature-Sliced React Structure**: Planned React files stay within
  `app`, `pages`, `widgets`, `features`, `entities`, `shared`, or
  `packages/ui` responsibilities.
- **Explicit State Ownership**: React Query owns server/async/cache state;
  Zustand or an explicitly named store owns client-global/session state;
  local state remains component-local.
- **JSON Persistence Behind Ports**: Single JSON file persistence remains
  isolated in Rust outbound adapters; schema changes include migration or
  backward-compatible handling.
- **Safe and Observable Desktop Actions**: Filesystem, git, launch, and
  clipboard actions provide visible success/failure feedback and handle spaces,
  Korean text, and special characters in paths.
- **Storybook-Backed UI Changes**: New components or meaningful UI states have
  Storybook coverage in the appropriate atomic design category.
- **Korean Documentation with Visual Flows**: `docs/*.md` files use English
  kebab-case filenames, Korean body text, and Mermaid diagrams for flows.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
apps/desktop/
├── src/                    # React app using feature sliced design
│   ├── app/
│   ├── pages/
│   ├── widgets/
│   ├── features/
│   ├── entities/
│   └── shared/
└── src-tauri/src/          # Rust hexagonal architecture
    ├── domain/
    ├── application/
    ├── ports/
    └── adapters/

packages/ui/                # Shared UI primitives
docs/                       # Korean documentation, English kebab-case names
specs/[###-feature]/        # Spec Kit artifacts for this feature
```

**Structure Decision**: [Document the selected structure and reference the real
directories captured above]

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
