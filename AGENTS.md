# Agent Guidelines

## Architecture

- Rust 영역은 hexagonal architecture를 사용한다.
  - 도메인 로직은 framework, Tauri command, filesystem, git CLI 같은 외부 어댑터에 직접 의존하지 않는다.
  - inbound adapter는 Tauri command와 UI 요청을 application port로 전달한다.
  - outbound adapter는 filesystem, git, OS API, persistence 같은 외부 시스템 접근을 담당한다.
  - application/service 계층은 use case를 조합하고 port interface를 통해 외부 기능을 호출한다.

- React 영역은 feature sliced design을 사용한다.
  - `app`은 provider, router, global style 같은 앱 부트스트랩을 담당한다.
  - `pages`는 route 단위 화면 조립을 담당한다.
  - `widgets`는 여러 feature/entity를 조합한 큰 UI 블록을 담당한다.
  - `features`는 사용자 행동 중심 기능을 담당한다.
  - `entities`는 도메인 모델, API, query key, 타입을 담당한다.
  - `shared` 또는 workspace UI package는 범용 UI, lib, hooks를 담당한다.

## State Management

- 서버 상태, 비동기 요청, 캐시, revalidation은 React Query를 사용한다.
- 클라이언트 전역 상태, UI preference, 선택 상태, 임시 session state는 Zustand를 사용한다.
- React local state는 컴포넌트 내부에서만 의미가 끝나는 상태에 사용한다.

## Persistence

- 저장소는 단일 JSON 파일을 사용한다.
- Rust outbound adapter가 JSON 파일 읽기/쓰기를 담당한다.
- 도메인 및 application/service 계층은 JSON 파일 형식에 직접 의존하지 않고 repository port를 통해 접근한다.

## Documentation

- `docs/*.md` 문서 파일명은 영어 kebab-case를 사용한다.
- 문서 본문은 한국어로 작성한다.
- 아키텍처, 실행 흐름, 상태 전이, 의존 관계를 시각화할 때는 Mermaid.js 코드블록을 사용한다.

## Storybook

- Storybook은 atomic design 규칙에 따라 구성한다.
  - `Atoms`는 단일 primitive, 입력, 버튼, 아이콘성 표시 요소를 다룬다.
  - `Molecules`는 여러 primitive를 조합한 데이터 표시, 입력 그룹, 테이블, 패널 단위를 다룬다.
  - `Organisms`는 feature/widget 수준의 사용자 워크플로와 여러 entity 데이터를 조합한 컴포넌트를 다룬다.
  - `Pages`는 route 단위 화면 조립 결과를 다룬다.
- 새로운 React 컴포넌트를 생성할 때는 Storybook에 등록한다.
- Storybook에 컴포넌트를 추가할 때는 컴포넌트가 처리하는 데이터의 종류와 상태를 알기 쉽도록 다양한 샘플을 포함한다.
  - 예: empty/loading/error, 긴 경로/긴 이름, 추가/수정/삭제/이름 변경 상태, 로컬/원격 브랜치, main/linked worktree.

<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan
<!-- SPECKIT END -->
