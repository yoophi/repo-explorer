# Feature Specification: repo 디렉토리 등록 앱으로 열기

**Feature Branch**: `002-open-directory-app`  
**Created**: 2026-07-01  
**Status**: Draft  
**Input**: User description: "repo directory 를 ghostty, kitty, wezterm, vscode, finder 등 등록된 앱으로 열 수 있다"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - repo 디렉토리를 선택한 앱으로 열기 (Priority: P1)

사용자는 repo 상세 화면에서 repo directory를 Ghostty, Kitty, WezTerm, VS Code, Finder 등 등록된 앱 중 하나로 열 수 있다.

**Why this priority**: repo를 확인한 직후 터미널, 에디터, 파일 관리자에서 같은 디렉토리를 여는 작업은 개발 흐름의 핵심 전환이다.

**Independent Test**: repo 상세 화면에서 repo directory 열기 동작을 실행하고 등록된 앱 하나를 선택했을 때, 선택한 앱이 해당 repo directory를 대상으로 열리는지 확인하면 독립적으로 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 사용자가 repo 상세 화면을 보고 있고 등록된 앱 목록이 있음, **When** 사용자가 앱 하나를 선택해 repo directory 열기를 실행함, **Then** 선택한 앱이 해당 repo directory를 대상으로 열린다.
2. **Given** 사용자가 repo directory 열기 메뉴를 열었음, **When** 등록된 앱 목록이 표시됨, **Then** 사용자는 Ghostty, Kitty, WezTerm, VS Code, Finder 등 사용 가능한 앱을 구분해 선택할 수 있다.

---

### User Story 2 - 사용할 수 없는 앱 처리 (Priority: P2)

사용자는 등록된 앱이 현재 환경에서 사용할 수 없거나 실행에 실패했을 때 문제를 명확히 알 수 있다.

**Why this priority**: 앱 실행 실패가 조용히 무시되면 사용자는 repo가 열렸다고 오해하거나 같은 동작을 반복하게 된다.

**Independent Test**: 등록되어 있지만 실행할 수 없는 앱을 선택했을 때 실패 안내가 표시되고 repo 상세 화면이 유지되는지 확인하면 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 선택한 앱이 현재 환경에서 실행될 수 없음, **When** 사용자가 그 앱으로 repo directory 열기를 실행함, **Then** 사용자는 실행 실패 사실과 실패한 앱을 알 수 있다.
2. **Given** 앱 실행에 실패했음, **When** 실패 안내가 표시됨, **Then** repo 상세 화면의 현재 선택과 탐색 상태는 유지된다.

---

### User Story 3 - 앱 목록의 예측 가능성 (Priority: P3)

사용자는 repo directory를 열 수 있는 등록 앱 목록을 예측 가능한 순서와 이름으로 확인할 수 있다.

**Why this priority**: 여러 앱이 등록되어 있을 때 목록이 불안정하거나 이름이 모호하면 사용자가 잘못된 앱을 선택할 가능성이 높다.

**Independent Test**: 여러 앱이 등록된 상태에서 repo directory 열기 메뉴를 반복해서 열었을 때, 같은 앱들이 일관된 이름과 순서로 표시되는지 확인하면 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 여러 앱이 repo directory 열기 대상으로 등록되어 있음, **When** 사용자가 열기 메뉴를 확인함, **Then** 앱 목록은 사용자가 식별 가능한 이름으로 일관되게 표시된다.

### Edge Cases

- 등록된 앱이 하나도 없을 때 사용자는 열 수 있는 앱이 없다는 사실을 알 수 있어야 한다.
- repo directory가 삭제되었거나 접근할 수 없는 상태이면 앱 실행 전에 사용자는 열 수 없다는 사실을 알 수 있어야 한다.
- 앱 이름이 길거나 비슷한 앱이 여러 개 있어도 사용자는 선택 대상을 구분할 수 있어야 한다.
- 같은 앱 열기 동작을 빠르게 반복해도 repo 상세 화면은 깨지거나 중복 상태로 전환되지 않아야 한다.
- repo directory 경로에 공백, 한글, 특수 문자가 포함되어도 선택한 앱은 올바른 directory를 대상으로 열려야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 사용자는 repo 상세 화면에서 현재 repo directory를 등록된 앱으로 여는 동작을 실행할 수 있어야 한다.
- **FR-002**: 시스템은 repo directory를 열 수 있도록 등록된 앱 목록을 사용자에게 표시해야 한다.
- **FR-003**: 앱 목록에는 사용자가 앱을 식별할 수 있는 이름이 표시되어야 하며, 같은 조건에서는 일관된 순서로 표시되어야 한다.
- **FR-004**: 사용자가 등록 앱을 선택하면 해당 앱은 현재 repo directory를 대상으로 열려야 한다.
- **FR-005**: 등록된 앱이 현재 환경에서 사용할 수 없거나 실행에 실패하면 사용자는 실패한 앱과 실패 사실을 알 수 있어야 한다.
- **FR-006**: repo directory가 존재하지 않거나 접근할 수 없는 경우 사용자는 앱을 열 수 없다는 사실을 알 수 있어야 한다.
- **FR-007**: 앱 열기 동작은 repo 상세 화면의 현재 선택, 탐색 위치, 표시 상태를 변경하지 않아야 한다.
- **FR-008**: 공백, 한글, 특수 문자를 포함한 repo directory 경로도 선택한 앱에서 올바르게 열려야 한다.

### Key Entities

- **Repo**: 사용자가 상세 정보를 확인하는 저장소이며, 열기 대상이 되는 repo directory를 가진다.
- **Registered App**: repo directory를 열 수 있는 사용자 선택 대상이며, 표시 이름과 사용 가능 상태를 가진다.
- **Open Result**: 등록 앱으로 repo directory 열기를 시도한 결과이며, 성공 또는 실패 상태와 사용자 피드백을 포함한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자는 repo 상세 화면에서 등록 앱으로 repo directory 열기를 3번 이하의 명시적 동작으로 완료할 수 있다.
- **SC-002**: 사용 가능한 등록 앱을 선택한 테스트의 95% 이상에서 선택한 앱이 올바른 repo directory를 대상으로 열린다.
- **SC-003**: 앱 실행 성공 또는 실패 후 사용자는 3초 이내에 결과를 인지할 수 있다.
- **SC-004**: 공백, 한글, 특수 문자가 포함된 repo directory 경로 샘플 10개 이상에서 선택한 앱이 올바른 directory를 대상으로 열린다.
- **SC-005**: 사용성 확인에서 참여자의 90% 이상이 별도 설명 없이 원하는 앱 열기 동작을 찾고 완료할 수 있다.

## Assumptions

- repo 상세 화면에는 현재 repo directory를 식별할 수 있는 정보가 이미 존재한다.
- 등록된 앱은 사용자가 repo directory를 여는 대상으로 기대하는 앱 목록을 의미하며, Ghostty, Kitty, WezTerm, VS Code, Finder 같은 앱이 포함될 수 있다.
- 등록 앱 관리 자체는 별도 설정 또는 기존 등록 체계를 따르며, 이 기능의 핵심 범위는 등록된 앱을 선택해 repo directory를 여는 것이다.
- 앱 실행 가능 여부는 사용자 환경에 따라 달라질 수 있으며, 실패 가능성은 정상적인 사용자 흐름으로 다룬다.
