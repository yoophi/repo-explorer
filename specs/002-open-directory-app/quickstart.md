# Quickstart: repo 디렉토리 등록 앱으로 열기

## Prerequisites

- macOS desktop environment
- At least one scanned repository in Repo Explorer
- Optional apps installed for manual verification: Ghostty, Kitty, WezTerm, VS Code

## Run The App

```bash
pnpm install
pnpm dev
```

`pnpm dev` uses the dynamic Tauri dev port wrapper described in `docs/dynamic-dev-port.md`.

## Manual Verification

1. Launch the desktop app.
2. Scan a root directory that contains git repositories.
3. Select a repository from the tree.
4. Open the repository action menu.
5. Verify the registered app list appears in deterministic order: Finder, Ghostty, Kitty, WezTerm, VS Code.
6. Choose Finder.
7. Verify Finder opens the selected repository directory and a visible success cue appears.
8. Repeat with Ghostty, Kitty, WezTerm, or VS Code when installed.
9. Verify the selected repository, search query, expanded tree, and metadata form remain unchanged after each action.

## Edge Case Verification

1. Use a repository path containing spaces, Korean text, or special characters.
2. Open it with each available registered app.
3. Verify the app targets the exact repository directory.
4. Temporarily choose an unavailable registered app.
5. Verify a clear failure message appears and the repo detail screen remains intact.
6. Verify the registered app menu handles long app labels and long repository paths without text overlap.

## Expected Validation Commands

```bash
pnpm -r exec tsc --noEmit
pnpm --filter desktop build
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
pnpm --filter desktop build-storybook
```
