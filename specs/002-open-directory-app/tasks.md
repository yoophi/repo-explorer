# Tasks: repo 디렉토리 등록 앱으로 열기

**Input**: Design documents from `/specs/002-open-directory-app/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/open-repository-app.md, quickstart.md

**Tests**: Automated test-authoring tasks are not generated because the feature specification does not explicitly request TDD or new automated tests. Storybook coverage and validation commands are included because the constitution requires them for UI changes.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **React desktop UI**: `apps/desktop/src/{app,pages,widgets,features,entities,shared}/`
- **Tauri/Rust core**: `apps/desktop/src-tauri/src/{domain,application,ports,adapters}/`
- **Shared UI primitives**: `packages/ui/`
- **Storybook**: `apps/desktop/src/stories/`
- **Feature artifacts**: `specs/002-open-directory-app/`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the feature and Rust boundary structure needed before implementation.

- [ ] T001 Create Rust hexagonal module directories in `apps/desktop/src-tauri/src/domain`, `apps/desktop/src-tauri/src/application`, `apps/desktop/src-tauri/src/ports`, and `apps/desktop/src-tauri/src/adapters`
- [ ] T002 Create React feature slice directories in `apps/desktop/src/features/open-repository-app/ui` and `apps/desktop/src/features/open-repository-app/model`
- [ ] T003 [P] Create initial feature barrel file in `apps/desktop/src/features/open-repository-app/index.ts`
- [ ] T004 [P] Create Storybook fixture directory for repository action states in `apps/desktop/src/stories/fixtures`
- [ ] T005 [P] Review existing repository exports and reserve app-action API names in `apps/desktop/src/entities/repository/index.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define shared contracts and boundaries that all user stories depend on.

**Critical**: No user story work can begin until this phase is complete.

- [ ] T006 Define `RepositoryOpenApp`, `RepositoryOpenAppKind`, `RegisteredRepositoryOpenApp`, and `OpenRepositoryInAppRequest` types in `apps/desktop/src/entities/repository/api.ts`
- [ ] T007 Add `listRepositoryOpenApps` and `openRepositoryInApp` Tauri invoke wrappers in `apps/desktop/src/entities/repository/api.ts`
- [ ] T008 Define Rust domain types for registered app id, app kind, app descriptor, and open request in `apps/desktop/src-tauri/src/domain/repository_open_app.rs`
- [ ] T009 Register the new Rust domain module in `apps/desktop/src-tauri/src/domain/mod.rs`
- [ ] T010 Define Rust application ports for repository validation and app launching in `apps/desktop/src-tauri/src/ports/repository_open_app.rs`
- [ ] T011 Register Rust port modules in `apps/desktop/src-tauri/src/ports/mod.rs`
- [ ] T012 Define the open repository app use case and registry access functions in `apps/desktop/src-tauri/src/application/repository_open_app.rs`
- [ ] T013 Register Rust application modules in `apps/desktop/src-tauri/src/application/mod.rs`
- [ ] T014 Define Rust adapter module declarations for Tauri command and macOS launcher adapters in `apps/desktop/src-tauri/src/adapters/mod.rs`
- [ ] T015 Update `apps/desktop/src-tauri/src/lib.rs` to load `domain`, `application`, `ports`, and `adapters` modules without moving unrelated scan or persistence behavior

**Checkpoint**: Foundation ready - user story implementation can now begin.

---

## Phase 3: User Story 1 - repo 디렉토리를 선택한 앱으로 열기 (Priority: P1)

**Goal**: User can choose Finder, Ghostty, Kitty, WezTerm, or VS Code from the repo detail screen and open the selected repo directory.

**Independent Test**: From repo detail, open the action menu, choose a registered app, and verify the selected app targets the current repo directory.

### Implementation for User Story 1

- [ ] T016 [P] [US1] Implement deterministic registered app registry for Finder, Ghostty, Kitty, WezTerm, VS Code, Terminal, and iTerm2 in `apps/desktop/src-tauri/src/application/repository_open_app.rs`
- [ ] T017 [P] [US1] Implement repository path validation adapter using existing git repository checks in `apps/desktop/src-tauri/src/adapters/repository_validator.rs`
- [ ] T018 [US1] Implement macOS app launcher descriptors and structured `open` process arguments in `apps/desktop/src-tauri/src/adapters/macos_app_launcher.rs`
- [ ] T019 [US1] Implement `list_repository_open_apps` and `open_repository_in_app` Tauri command adapters in `apps/desktop/src-tauri/src/adapters/repository_open_app_commands.rs`
- [ ] T020 [US1] Register `list_repository_open_apps` and `open_repository_in_app` in the Tauri invoke handler in `apps/desktop/src-tauri/src/lib.rs`
- [ ] T021 [US1] Keep existing terminal command compatibility or route `open_repository_in_terminal` through the new use case in `apps/desktop/src-tauri/src/lib.rs`
- [ ] T022 [P] [US1] Build `RepositoryOpenAppMenu` UI with grouped file manager, terminal, and editor actions in `apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx`
- [ ] T023 [P] [US1] Create action state helpers for pending, success, and error states in `apps/desktop/src/features/open-repository-app/model/action-state.ts`
- [ ] T024 [US1] Wire `listRepositoryOpenApps` and `openRepositoryInApp` into the repository detail page in `apps/desktop/src/pages/repository/ui/RepositoryPage.tsx`
- [ ] T025 [US1] Replace the existing hard-coded Finder and terminal menu with `RepositoryOpenAppMenu` in `apps/desktop/src/pages/repository/ui/RepositoryPage.tsx`
- [ ] T026 [US1] Export the feature UI from `apps/desktop/src/features/open-repository-app/index.ts`

**Checkpoint**: User Story 1 can open a selected repository directory in at least Finder and any installed requested app.

---

## Phase 4: User Story 2 - 사용할 수 없는 앱 처리 (Priority: P2)

**Goal**: User sees clear failure feedback when a registered app is unavailable, launch fails, or the repository directory is invalid.

**Independent Test**: Select an unavailable app or removed repository path and verify the failure message names the failed app while the repo detail state remains intact.

### Implementation for User Story 2

- [ ] T027 [US2] Map unsupported app ids, missing repositories, non-git paths, and launch failures to user-readable Rust errors in `apps/desktop/src-tauri/src/application/repository_open_app.rs`
- [ ] T028 [US2] Return failed app context from the macOS launcher adapter in `apps/desktop/src-tauri/src/adapters/macos_app_launcher.rs`
- [ ] T029 [US2] Render app-specific pending, success, and failure messages in `apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx`
- [ ] T030 [US2] Preserve selected repository, search query, expanded tree ids, and metadata form state when app launch fails in `apps/desktop/src/pages/repository/ui/RepositoryPage.tsx`
- [ ] T031 [US2] Add disabled or explanatory unavailable app rendering in `apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx`
- [ ] T032 [US2] Add Storybook page states for unavailable app and launch failure in `apps/desktop/src/stories/pages.stories.tsx`

**Checkpoint**: User Story 2 can be verified independently by forcing an unavailable app or invalid repo path.

---

## Phase 5: User Story 3 - 앱 목록의 예측 가능성 (Priority: P3)

**Goal**: User sees a stable, recognizable app list every time the repo directory open menu is opened.

**Independent Test**: Open the menu repeatedly with multiple registered apps and verify labels and order remain stable.

### Implementation for User Story 3

- [ ] T033 [US3] Enforce deterministic app ordering and stable labels in `apps/desktop/src-tauri/src/application/repository_open_app.rs`
- [ ] T034 [US3] Sort and render registered apps by registry order without client-side reordering in `apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx`
- [ ] T035 [US3] Handle long app labels and long repository paths without text overlap in `apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx`
- [ ] T036 [US3] Add Storybook page state for long repository path and long app names in `apps/desktop/src/stories/pages.stories.tsx`
- [ ] T037 [US3] Add reusable repository and app fixtures for deterministic menu stories in `apps/desktop/src/stories/fixtures/repository-open-app.ts`

**Checkpoint**: User Story 3 can be verified by repeatedly opening the menu and comparing app order and labels.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, documentation alignment, and cleanup across stories.

- [ ] T038 [P] Update quickstart verification notes if implementation behavior differs from the plan in `specs/002-open-directory-app/quickstart.md`
- [ ] T039 [P] Update contract details if final command names, enum values, or payloads differ in `specs/002-open-directory-app/contracts/open-repository-app.md`
- [ ] T040 [P] Run TypeScript typecheck for the workspace and fix issues in `apps/desktop/src/entities/repository/api.ts`, `apps/desktop/src/features/open-repository-app/index.ts`, and `apps/desktop/src/pages/repository/ui/RepositoryPage.tsx`
- [ ] T041 [P] Run Rust tests for the Tauri crate and fix issues in `apps/desktop/src-tauri/src/application/repository_open_app.rs`, `apps/desktop/src-tauri/src/adapters/macos_app_launcher.rs`, and `apps/desktop/src-tauri/src/lib.rs`
- [ ] T042 [P] Run desktop build and fix bundling issues in `apps/desktop/package.json` and `apps/desktop/src/pages/repository/ui/RepositoryPage.tsx`
- [ ] T043 [P] Run Storybook build and fix story or provider issues in `apps/desktop/src/stories/pages.stories.tsx` and `apps/desktop/src/shared/storybook/storybook-providers.tsx`
- [ ] T044 Validate manual quickstart scenarios for installed apps and special-character paths using `specs/002-open-directory-app/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion - blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion.
- **User Story 2 (Phase 4)**: Depends on User Story 1 because it extends the app launch flow with failure and unavailable states.
- **User Story 3 (Phase 5)**: Depends on User Story 1 registry/menu implementation and can run in parallel with parts of User Story 2 after T016-T026 complete.
- **Polish (Phase 6)**: Depends on all implemented user stories.

### User Story Dependencies

- **US1 (P1)**: Base MVP. Required before US2 and US3.
- **US2 (P2)**: Requires US1 command and UI flow; independently testable through failure scenarios after US1 exists.
- **US3 (P3)**: Requires US1 registry and menu; independently testable through repeated menu inspection after US1 exists.

### Within Each User Story

- Rust domain/application/adapter tasks before Tauri command registration.
- Entity API types before React feature wiring.
- Feature UI before page replacement.
- Storybook states after UI behavior exists.

### Parallel Opportunities

- T003, T004, and T005 can run in parallel during setup.
- T016 and T017 can run in parallel once foundational Rust modules exist.
- T022 and T023 can run in parallel with Rust launcher work after T006-T007 exist.
- T032 can run after T029-T031 while T027-T028 are being refined.
- T036 and T037 can run together after T034 starts.
- T038-T043 can run in parallel during polish, then T044 validates the integrated behavior.

---

## Parallel Example: User Story 1

```bash
# Rust registry and validation can proceed together:
Task: "T016 [US1] Implement deterministic registered app registry in apps/desktop/src-tauri/src/application/repository_open_app.rs"
Task: "T017 [US1] Implement repository path validation adapter in apps/desktop/src-tauri/src/adapters/repository_validator.rs"

# React feature UI and state helpers can proceed together:
Task: "T022 [US1] Build RepositoryOpenAppMenu UI in apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx"
Task: "T023 [US1] Create action state helpers in apps/desktop/src/features/open-repository-app/model/action-state.ts"
```

## Parallel Example: User Story 2

```bash
# Rust error mapping and UI failure rendering can proceed together after US1:
Task: "T027 [US2] Map launch failures in apps/desktop/src-tauri/src/application/repository_open_app.rs"
Task: "T029 [US2] Render app-specific failure messages in apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx"
```

## Parallel Example: User Story 3

```bash
# Deterministic rendering and story fixtures can proceed together:
Task: "T034 [US3] Sort and render registered apps in apps/desktop/src/features/open-repository-app/ui/RepositoryOpenAppMenu.tsx"
Task: "T037 [US3] Add deterministic menu fixtures in apps/desktop/src/stories/fixtures/repository-open-app.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundational contracts and boundaries.
3. Complete Phase 3 User Story 1.
4. Validate that Finder and at least one installed requested app opens the selected repo directory.

### Incremental Delivery

1. Deliver US1 to provide the core open-in-app workflow.
2. Add US2 to harden unavailable app and failure feedback.
3. Add US3 to lock down deterministic app list behavior and long-label rendering.
4. Complete polish validation commands and manual quickstart checks.

### Validation Commands

```bash
pnpm -r exec tsc --noEmit
pnpm --filter desktop build
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
pnpm --filter desktop build-storybook
```
