# Implementation Plan: repo 디렉토리 등록 앱으로 열기

**Branch**: `002-open-directory-app` | **Date**: 2026-07-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-open-directory-app/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Repo 상세 화면에서 현재 repo directory를 Finder, Ghostty, Kitty, WezTerm, VS Code 같은 등록 앱 중 하나로 열 수 있게 한다. 구현은 등록 앱 목록과 실행 요청을 하나의 repository app action contract로 통합하고, Rust application port와 outbound OS launcher adapter를 통해 경로 검증과 앱 실행을 처리한다. React UI는 repo 상세 action menu에서 deterministic app list, pending state, failure feedback을 표시한다.

## Technical Context

**Language/Version**: TypeScript ~5.8.3, React 19.1.0, Rust 2021 edition  
**Primary Dependencies**: Tauri 2, `@tauri-apps/api` v2, `@tauri-apps/plugin-opener` v2, `@tanstack/react-query` v5, `@yoophi/ui`, lucide-react, Storybook 10  
**Storage**: Existing single JSON persistence (`.repo-explorer.json`) remains unchanged; no schema change for this feature  
**Testing**: `pnpm -r exec tsc --noEmit`, `pnpm --filter desktop build`, `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`, `pnpm --filter desktop build-storybook`  
**Target Platform**: macOS desktop app through Tauri  
**Project Type**: pnpm workspace desktop app (`apps/desktop`) with shared UI package (`packages/ui`)  
**Performance Goals**: Registered app menu opens immediately from already-loaded repo detail state; app launch request returns success/failure feedback within 3 seconds in normal OS conditions  
**Constraints**: Must pass paths as structured process arguments, not shell strings; must support paths with spaces, Korean text, and special characters; failed launches must not reset repository selection, tree expansion, search, or metadata form state  
**Scale/Scope**: One repository detail action workflow, five requested app targets, compatibility support may retain Terminal and iTerm2 if already exposed

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Hexagonal Rust Boundaries**: PASS. The planned launch flow introduces application-facing commands, a repository/app launch use case, and an outbound OS launcher adapter. Existing monolithic scan/persistence code is outside this feature's behavioral scope, but touched app-launch code must move behind the new boundary.
- **Feature-Sliced React Structure**: PASS. API contracts stay under `entities/repository`; user action UI is planned under `features/open-repository-app`; page assembly remains in `pages/repository`.
- **Explicit State Ownership**: PASS. Repository data remains React Query server state. The selected repository/search/tree state remains existing page/session UI state. Launch pending/error state is transient local state scoped to the action UI.
- **JSON Persistence Behind Ports**: PASS. No JSON schema or persistence behavior changes are required.
- **Safe and Observable Desktop Actions**: PASS. The core requirement is visible success/failure feedback and robust path handling for app launch actions.
- **Storybook-Backed UI Changes**: PASS. New menu states require Storybook coverage for empty app list, long paths/names, pending, success, and failure.
- **Korean Documentation with Visual Flows**: PASS. No `docs/*.md` change is required for this feature; if added later, it must be Korean with Mermaid flow diagrams.

## Project Structure

### Documentation (this feature)

```text
specs/002-open-directory-app/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── open-repository-app.md
└── tasks.md              # Phase 2 output, not created by /speckit.plan
```

### Source Code (repository root)

```text
apps/desktop/
├── src/
│   ├── pages/repository/              # Route assembly; wires detail page to feature action
│   ├── features/open-repository-app/   # Registered app menu, pending/error/success UI state
│   ├── entities/repository/            # Tauri API types, query keys, app action contract
│   └── shared/                         # Shared Storybook providers/utilities
└── src-tauri/src/
    ├── domain/                         # Repository/app launch domain types, no Tauri/OS calls
    ├── application/                    # Open repository app use case
    ├── ports/                          # Repository validation and app launcher ports
    └── adapters/                       # Tauri commands and macOS process launcher

packages/ui/
└── src/components/                     # Shared primitive only if reusable UI is needed

apps/desktop/src/stories/               # Storybook stories following atomic design titles
```

**Structure Decision**: Use the existing monorepo and desktop app layout. Add a feature slice for the open-in-app interaction, keep repository command types in the repository entity, and isolate Rust launch behavior behind application ports and adapters. Do not add persistence files or modify the JSON schema.

## Phase 0: Research

Research completed in [research.md](./research.md).

Key decisions:

- Start with a deterministic known app registry for Finder, Ghostty, Kitty, WezTerm, and VS Code.
- Expose registered app listing and app launch through repository app action contracts.
- Validate repository path in Rust before launching.
- Use per-app launch descriptors and process arguments rather than shell command strings.
- Keep launch pending/error/success as transient UI action state.
- Add Storybook coverage for realistic action menu states.

## Phase 1: Design & Contracts

Design artifacts:

- [data-model.md](./data-model.md)
- [contracts/open-repository-app.md](./contracts/open-repository-app.md)
- [quickstart.md](./quickstart.md)

### Post-Design Constitution Check

- **Hexagonal Rust Boundaries**: PASS. The data model and contract require domain/app types separate from Tauri commands and OS launcher details.
- **Feature-Sliced React Structure**: PASS. The plan places reusable command types in `entities/repository` and action UI in `features/open-repository-app`.
- **Explicit State Ownership**: PASS. The quickstart and contract do not introduce new server cache ownership; transient launch state remains UI-local.
- **JSON Persistence Behind Ports**: PASS. Data model confirms no JSON schema changes.
- **Safe and Observable Desktop Actions**: PASS. Contract requires structured path arguments, pre-launch validation, and user-visible failure context.
- **Storybook-Backed UI Changes**: PASS. Story coverage is explicitly required for empty, pending, success, failure, and long text states.
- **Korean Documentation with Visual Flows**: PASS. No new `docs/*.md` document is part of this plan.

## Complexity Tracking

No constitution violations require exception tracking.
