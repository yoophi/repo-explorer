# Repo Explorer

로컬 Git 저장소를 검색하고 설명·태그·고정 상태와 README를 관리하는 Tauri 데스크톱 앱입니다.

## 주요 기능

- 루트 경로·최대 깊이를 지정한 Git 저장소 탐색
- 스캔 진행 상태·취소, 검사 완료 항목의 점진적 미리보기, 저장소 카탈로그 복원
- main/linked worktree 관계와 트리 탐색, 텍스트 검색
- origin·README 표시, 설명·태그·고정 상태 편집
- 탐색 경로·최대 깊이 설정 복원과 초기화

## 개발 환경과 실행

Node.js 22 이상과 pnpm, Rust/Cargo 및 운영체제에 필요한 Tauri 빌드 도구가 필요합니다. `.ts` 파일을 직접 실행하는 테스트에는 Node의 TypeScript type stripping을 지원하는 버전을 사용하세요.

공통 explorer-kit은 `pnpm@9.15.5`를 사용합니다. 이 앱도 pnpm으로 설치하며 `pnpm-lock.yaml`을 사용합니다.

이 앱과 `explorer-kit`을 같은 상위 디렉터리에 체크아웃해야 합니다. TypeScript는 `link:`, Rust는 Cargo `path` 의존성을 사용하므로 앱 저장소만으로는 설치·빌드할 수 없습니다. 아래 명령은 이 앱의 루트에서 시작합니다.

```sh
cd ../explorer-kit
pnpm install --frozen-lockfile
cd ../repo-explorer
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev`는 공통 개발 도구로 사용 가능한 포트를 찾아 Vite와 Tauri를 함께 실행합니다. 브라우저 UI만 실행하려면 `pnpm --filter desktop dev`를 사용합니다.

## 검증과 빌드

```sh
pnpm typecheck
pnpm test
pnpm build
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
pnpm tauri build
```

`pnpm build`는 프론트엔드 타입 검사·번들 생성이며 네이티브 패키징은 `pnpm tauri build`입니다.

앱별 Storybook: `pnpm storybook`, 정적 생성: `pnpm build-storybook`.

## 데이터와 설정

Tauri app_data_dir의 `repositories.json`에 카탈로그를 저장합니다. 각 Git 저장소의 설명·태그·고정 상태는 해당 디렉터리의 `.repo-explorer.json`에 저장됩니다. 탐색 UI 설정은 localStorage `repo-explorer.preferences` v1의 rootPath·maxDepth이며 깊이는 0~20입니다. 숫자 초안과 확정 값은 분리하고 실제 편집한 값만 blur 또는 Scan 시 저장합니다.

Git CLI가 PATH에 있어야 저장소 검사와 worktree 정보를 정상적으로 사용할 수 있습니다. 메타데이터 저장 시 대상 저장소에 `.repo-explorer.json` 파일이 생성될 수 있습니다.

스캔은 Git 디렉터리 발견을 마친 뒤 각 저장소를 검사합니다. 검사가 끝난 항목은 `repository_scan_item` 이벤트로 잠정 목록에 나타나며 50ms 단위로 화면에 반영됩니다. main 저장소를 먼저 검사했다면 worktree의 `parentId`를 첫 이벤트부터 채웁니다. worktree를 먼저 검사했다면 main 확인 시 같은 항목 ID로 수정 이벤트를 다시 보내 terminal 전에 부모관계를 맞춥니다. 그 전까지의 부모관계는 잠정 값입니다. 잠정 항목의 메타데이터 편집·저장은 비활성화됩니다. 성공 terminal에서만 최종 목록을 React Query 캐시에 반영하며, 취소를 누르거나 실패·시작 실패·새 작업·화면 종료 시 잠정 목록을 버립니다. 취소 시에는 즉시 기존 저장 목록으로 돌아가고, catalog 저장은 성공한 스캔의 commit 단계에서만 수행합니다.

## 현재 아키텍처

프론트엔드는 app/pages/entities/shared 구조와 로컬 workspace UI 패키지를 사용합니다. 서버 데이터는 TanStack Query, 탐색 설정은 settings-core, 숫자 편집 초안은 settings-core의 순수 draft 함수, 스캔 수명은 scan-client의 `ScanLifecycle`을 감싼 앱 `ScanSession`이 관리합니다. 가시 저장소 선택은 collection-core의 `reconcileSelection`에 앱의 평면 목록 초기 선택 정책을 전달합니다. 진행·취소·재시도 표시는 ui-base `ScanStatusPanel`, 기존 Button API는 ui-radix의 호환 버튼을 재export합니다. preferences는 entities의 공개 index에서 제공하며, Storybook의 앱 provider 조립은 `.storybook/app.tsx`에 둡니다.

Rust `src-tauri/src/lib.rs`는 Tauri command, 이벤트, 스캔 등록·취소와 catalog commit 경계를 조립합니다. `application.rs`는 Tauri와 파일 형식에 의존하지 않는 스캔·목록·메타데이터 유스케이스 및 inspector/catalog/metadata/progress port를 제공합니다. `domain.rs`에는 DTO와 순수 변환을 두고, `infrastructure.rs`가 Git 검사·메타데이터·JSON 저장을 구현합니다. 디렉터리 순회는 fs-core의 `RepositoryBfs` 정책을 사용하고 Git 저장소 판정과 결과 DTO는 앱에 남습니다. catalog 경로는 저장소 port 호출 시 해석하며 JSON 갱신은 기존 단일 update lock을 사용합니다.

| 위치 | 책임 |
| --- | --- |
| `apps/desktop/src` | React 화면·모델·API adapter |
| `apps/desktop/src-tauri/src/lib.rs` | Tauri inbound adapter와 scan-job 수명·commit 경계 |
| `apps/desktop/src-tauri/src/application.rs` | 유스케이스와 outbound port |
| `apps/desktop/src-tauri/src/domain.rs` | 저장소 DTO와 순수 변환 |
| `apps/desktop/src-tauri/src/infrastructure.rs` | Git·파일시스템·JSON outbound adapter |
| `packages/ui` | 앱 로컬 UI primitive |

## 공통 코드 연결

- 프론트엔드/개발 도구: `@yoophi/explorer-dev-tools`, `@yoophi/settings-core`, `@yoophi/settings-ui`, `@yoophi/scan-client`, `@yoophi/collection-core`, `@yoophi/ui-base`, `@yoophi/ui-radix`
- Rust: `explorer-json-store`, `explorer-scan-job`, `explorer-fs-core`.

공통 React 컴포넌트는 앱 데이터를 props와 callback으로 받고, 앱의 Tauri·라우팅·도메인 정책은 호출부에 유지합니다. 공통 저장소 UI를 보려면 `explorer-kit`에서 `pnpm storybook`을 실행하세요. 앱별 Storybook과는 별도이며 기본 포트 6006을 사용합니다.

## 관련 문서

- [공통 저장소 안내](../explorer-kit/README.md)
- [공통 UI Storybook](../explorer-kit/docs/storybook.md)
- [설정 공통화 결과](../explorer-kit/docs/settings-promotion-report.md)
- [Hexagonal·FSD 리뷰](../explorer-kit/docs/architecture-review.md)
- [아키텍처 지적 수정 결과](../explorer-kit/docs/architecture-fix-report.md)
- [두 앱 이상 공통 기능 후보](../explorer-kit/docs/shared-feature-candidates.md)
- [공통 코드 기능 리뷰와 미해결 항목](../explorer-kit/docs/shared-code-review.md)

문서 기준: 2026-10-05 로컬 구현. 아키텍처 리뷰의 개선 권고와 공통 기능 후보는 완료된 구현과 구분합니다.
