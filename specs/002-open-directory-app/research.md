# Research: repo 디렉토리 등록 앱으로 열기

## Decision: 등록 앱 목록은 고정된 known app registry로 시작한다

**Rationale**: Spec은 Ghostty, Kitty, WezTerm, VS Code, Finder 같은 등록 앱을 요구하지만, 등록 앱 관리 UI는 범위 밖으로 명시되어 있다. 현재 앱도 terminal options를 코드에 고정해 제공하고 있으므로, 첫 구현은 사용자가 기대하는 known apps를 일관된 순서로 표시하는 registry가 가장 작고 검증 가능하다.

**Alternatives considered**:

- 사용자 설정 JSON에 앱 registry 저장: 사용자 커스텀 앱에는 유리하지만 이번 feature의 persistence scope가 커진다.
- OS에서 설치 앱 자동 탐색: macOS bundle 탐색, PATH 탐색, 앱별 실행 규칙이 복잡해지고 결과 순서가 불안정해진다.

## Decision: Finder와 terminal/editor 열기를 하나의 repository app action contract로 통합한다

**Rationale**: 현재 UI는 Finder와 terminal actions가 분리되어 있다. Spec은 Ghostty, Kitty, WezTerm, VS Code, Finder 등 등록 앱 중 하나를 선택하는 흐름을 요구한다. 하나의 contract로 `app`을 선택하게 하면 pending/error 상태와 path validation을 동일하게 처리할 수 있다.

**Alternatives considered**:

- 기존 `openRepositoryInFinder`와 `openRepositoryInTerminal` 유지 후 UI만 묶기: 구현은 작지만 실패 처리와 앱 목록 contract가 갈라진다.
- 모든 앱 열기를 frontend plugin에서 처리: Finder는 가능하지만 terminal/editor별 경로 전달 규칙과 검증이 frontend에 새어 나온다.

## Decision: 등록 앱 목록도 Rust contract로 노출한다

**Rationale**: 앱 목록의 순서, label, 지원 여부는 launch contract와 같은 registry에서 나와야 한다. UI가 별도 배열을 유지하면 목록과 실제 실행 가능 앱이 어긋날 수 있다. Rust application use case가 registry를 제공하고 frontend entity API가 이를 조회하면 deterministic order와 availability message를 한 곳에서 관리할 수 있다.

**Alternatives considered**:

- Frontend에 registered app 배열 유지: 구현은 빠르지만 launch enum과 쉽게 불일치한다.
- OS에서 매번 설치 여부 자동 탐색: 사용자 환경 반영에는 유리하지만 feature 범위를 넘어가고 list 안정성이 낮아진다.

## Decision: Rust app launch code는 application port와 outbound adapter로 분리한다

**Rationale**: 새 constitution은 Tauri command와 OS process 실행이 domain logic에 직접 들어가지 않도록 요구한다. `open_repository_in_app` Tauri command는 inbound adapter로 request만 변환하고, application use case가 repository validation port와 app launcher port를 호출해야 한다.

**Alternatives considered**:

- 기존 `open_repository_in_terminal` 함수 확장: 변경량은 작지만 Tauri command, path validation, OS process call이 계속 한 파일에 섞인다.
- 완전한 scan/persistence 리팩터링 동시 수행: 이상적이지만 이 feature의 범위를 넘어가고 delivery risk가 커진다.

## Decision: 앱 실행은 Rust command에서 repository path 검증 후 수행한다

**Rationale**: repo id는 absolute path이고, 열기 전에 해당 path가 여전히 git repository인지 검증해야 한다. Rust layer는 이미 git repository 검증과 OS process 실행을 담당하고 있으므로 path validation과 launch error를 일관되게 반환할 수 있다.

**Alternatives considered**:

- frontend에서 path 존재 여부를 판단: Tauri 권한과 OS 차이를 frontend에 노출한다.
- 검증 없이 앱 실행: 삭제된 repo나 잘못된 path에서 사용자에게 모호한 OS 오류만 보일 수 있다.

## Decision: macOS launch semantics는 앱별로 명시한다

**Rationale**: Finder는 directory reveal/open 계열이고, terminal/editor 앱은 directory를 working directory 또는 open target으로 받는 방식이 다르다. 앱별 launch descriptor를 두면 경로 공백/한글/특수 문자를 shell escaping 없이 process args로 전달할 수 있다.

**Alternatives considered**:

- shell command string 조립: quoting 문제로 경로 특수 문자 요구사항을 깨기 쉽다.
- 앱마다 별도 command 생성: contract와 UI 상태가 중복된다.

## Decision: UI 상태는 selected repository detail의 local action state로 유지한다

**Rationale**: 앱 열기는 서버 캐시를 바꾸지 않는 transient action이다. 현재 코드도 repository action pending/error를 local state로 보관한다. React Query mutation을 사용할 수 있지만 cache invalidation 대상이 없어 local async handler가 더 단순하다.

**Alternatives considered**:

- React Query mutation: pending/error 모델은 편하지만 server state 변경이 아니고 query key 설계가 불필요하다.
- Zustand store: 다른 route와 공유할 상태가 아니므로 전역 store가 과하다.

## Decision: Storybook은 Pages/Repository에서 action menu states를 보강한다

**Rationale**: 현재 Storybook에는 `Pages/Repository` default만 있다. Spec과 AGENTS.md는 긴 경로, pending/error, empty/loading 같은 상태를 요구한다. 이번 feature는 registered app list, unavailable/failure, long path를 보여주는 story가 필요하다.

**Alternatives considered**:

- 별도 component story만 추가: 현재 `RepositoryDetail`이 내부 함수라 바로 story하기 어렵다. 구현 중 component 추출이 발생하면 Organisms/Molecules story로 이동한다.
