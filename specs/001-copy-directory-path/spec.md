# Feature Specification: repo 상세 디렉토리 경로 복사

**Feature Branch**: `001-copy-directory-path`  
**Created**: 2026-07-01  
**Status**: Draft  
**Input**: User description: "repo 상세에서 디렉토리 absolute path 를 클립보드에 복사할 수 있다"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 디렉토리 절대 경로 복사 (Priority: P1)

사용자는 repo 상세 화면에서 확인 중인 디렉토리의 absolute path를 즉시 클립보드에 복사할 수 있다.

**Why this priority**: repo 상세 화면에서 디렉토리 경로를 터미널, 에디터, 다른 도구에 붙여넣는 흐름은 작업 전환 비용을 줄이는 핵심 사용 가치다.

**Independent Test**: repo 상세 화면에서 디렉토리를 선택하거나 확인한 뒤 복사 동작을 실행하고, 클립보드에 해당 디렉토리의 absolute path가 들어갔는지 확인하면 독립적으로 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 사용자가 repo 상세 화면에서 디렉토리 항목을 보고 있음, **When** 사용자가 해당 디렉토리의 경로 복사 동작을 실행함, **Then** 클립보드에는 그 디렉토리의 absolute path가 저장된다.
2. **Given** 복사 가능한 디렉토리 경로가 표시되어 있음, **When** 사용자가 복사 동작을 실행함, **Then** 사용자는 복사가 완료되었음을 명확하게 알 수 있다.

---

### User Story 2 - 복사 실패 인지 (Priority: P2)

사용자는 클립보드 접근이 실패했을 때 실패 사실과 다음 행동 가능성을 알 수 있다.

**Why this priority**: 복사 실패가 조용히 무시되면 사용자는 잘못된 경로를 붙여넣거나 작업을 반복하게 된다.

**Independent Test**: 클립보드 사용이 불가능한 상황을 만들고 복사 동작을 실행했을 때, 실패 안내가 표시되고 화면 상태가 유지되는지 확인하면 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 클립보드에 쓸 수 없는 상태임, **When** 사용자가 디렉토리 경로 복사 동작을 실행함, **Then** 사용자는 복사가 실패했음을 알 수 있고 기존 repo 상세 화면은 유지된다.

---

### User Story 3 - 긴 경로 식별과 복사 (Priority: P3)

사용자는 긴 absolute path를 가진 디렉토리도 혼동 없이 복사할 수 있다.

**Why this priority**: 실제 저장소 경로는 길어질 수 있으며, 복사 대상이 불명확하면 잘못된 디렉토리 경로를 사용할 위험이 있다.

**Independent Test**: 긴 이름과 깊은 중첩 경로를 가진 디렉토리를 repo 상세 화면에 표시하고 복사 동작을 실행해 정확한 full path가 클립보드에 들어갔는지 확인하면 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 긴 absolute path를 가진 디렉토리 항목이 있음, **When** 사용자가 해당 항목의 복사 동작을 실행함, **Then** 클립보드에는 생략되지 않은 전체 absolute path가 저장된다.

### Edge Cases

- 디렉토리 이름 또는 상위 경로에 공백, 한글, 특수 문자가 포함되어도 클립보드에는 원본 absolute path가 그대로 저장되어야 한다.
- 사용자가 파일 항목과 디렉토리 항목을 함께 보고 있을 때 복사 동작의 대상이 디렉토리임을 구분할 수 있어야 한다.
- 경로가 화면에서 축약되어 표시되더라도 복사되는 값은 축약되지 않은 absolute path여야 한다.
- 복사 동작을 여러 번 빠르게 실행해도 마지막으로 선택한 디렉토리의 absolute path가 클립보드에 저장되어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 사용자는 repo 상세 화면에서 각 디렉토리의 absolute path를 복사하는 동작을 실행할 수 있어야 한다.
- **FR-002**: 복사되는 값은 사용자에게 보이는 표시 축약 여부와 관계없이 디렉토리의 전체 absolute path여야 한다.
- **FR-003**: 복사 동작은 사용자가 현재 보고 있는 repo 상세 화면의 선택, 탐색 위치, 표시 상태를 변경하지 않아야 한다.
- **FR-004**: 복사 성공 시 사용자는 복사가 완료되었음을 2초 이내에 인지할 수 있어야 한다.
- **FR-005**: 복사 실패 시 사용자는 실패 사실을 명확히 알 수 있어야 하며, 실패로 인해 repo 상세 화면의 기존 정보가 사라지면 안 된다.
- **FR-006**: 공백, 한글, 특수 문자를 포함한 디렉토리 absolute path도 원문 그대로 복사되어야 한다.
- **FR-007**: 사용자는 파일 경로 복사와 혼동하지 않도록 디렉토리 경로 복사 대상과 동작을 구분할 수 있어야 한다.

### Key Entities

- **Repo**: 사용자가 상세 정보를 확인하는 저장소이며, 디렉토리 목록 또는 트리의 기준이 된다.
- **Directory**: repo 상세 화면에서 복사 대상이 되는 폴더 항목이며, 이름과 absolute path를 가진다.
- **Clipboard Copy Result**: 경로 복사 시도의 결과이며, 성공 또는 실패 상태와 사용자에게 전달할 피드백을 포함한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자는 repo 상세 화면에서 디렉토리 absolute path 복사를 2번 이하의 명시적 동작으로 완료할 수 있다.
- **SC-002**: 테스트된 디렉토리 경로의 100%가 화면 표시 축약 여부와 관계없이 클립보드에 전체 absolute path로 저장된다.
- **SC-003**: 복사 성공 또는 실패 후 사용자는 2초 이내에 결과를 확인할 수 있다.
- **SC-004**: 공백, 한글, 특수 문자가 포함된 경로 샘플 10개 이상에서 복사된 값이 원본 absolute path와 정확히 일치한다.
- **SC-005**: 사용성 확인에서 참여자의 90% 이상이 repo 상세 화면에서 디렉토리 경로 복사 동작을 별도 설명 없이 찾을 수 있다.

## Assumptions

- repo 상세 화면에는 사용자가 디렉토리 항목 또는 현재 디렉토리 정보를 인지할 수 있는 영역이 이미 존재한다.
- 이 기능의 범위는 디렉토리 absolute path 복사이며, 파일 경로 복사나 relative path 복사는 별도 기능으로 다룬다.
- 사용자의 실행 환경은 클립보드 사용을 일반적으로 허용하지만, 권한 또는 OS 상태에 따라 실패할 수 있다.
- absolute path는 repo 상세 화면이 다루는 로컬 파일시스템 기준의 전체 경로를 의미한다.
