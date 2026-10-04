# Contract: Open Repository Directory In Registered App

## Purpose

Allow the repository detail UI to list deterministic registered desktop apps and open the selected repository directory in one of them with clear success or failure reporting.

## Registered Apps

The UI displays apps in this deterministic order when available:

1. Finder
2. Ghostty
3. Kitty
4. WezTerm
5. VS Code

Existing Terminal or iTerm2 support may remain available after these entries if the product chooses to keep backward compatibility.

## Tauri Commands

### `list_repository_open_apps`

Request:

```json
{}
```

Success response:

```json
[
  {
    "id": "finder",
    "label": "Finder",
    "kind": "fileManager",
    "available": true,
    "availabilityMessage": null
  },
  {
    "id": "vscode",
    "label": "VS Code",
    "kind": "editor",
    "available": true,
    "availabilityMessage": null
  }
]
```

`kind` enum values:

```text
fileManager | terminal | editor
```

### `open_repository_in_app`

Request:

```json
{
  "request": {
    "repositoryId": "/absolute/path/to/repo",
    "app": "vscode"
  }
}
```

`app` enum values:

```text
finder | ghostty | kitty | wezterm | vscode | terminal | iterm2
```

`terminal` and `iterm2` are compatibility values for existing behavior.

Success response:

```json
null
```

Failure response:

```text
Repository not found: /absolute/path/to/repo
Path is not a git repository: /absolute/path/to/repo
Registered app is not supported: <app>
Failed to open <label>: <reason>
```

## Frontend API

```ts
export type RepositoryOpenApp =
  | "finder"
  | "ghostty"
  | "kitty"
  | "wezterm"
  | "vscode"
  | "terminal"
  | "iterm2";

export type RepositoryOpenAppKind = "fileManager" | "terminal" | "editor";

export type RegisteredRepositoryOpenApp = {
  id: RepositoryOpenApp;
  label: string;
  kind: RepositoryOpenAppKind;
  available: boolean;
  availabilityMessage: string | null;
};

export type OpenRepositoryInAppRequest = {
  repositoryId: string;
  app: RepositoryOpenApp;
};

export function listRepositoryOpenApps(): Promise<RegisteredRepositoryOpenApp[]>;
export function openRepositoryInApp(request: OpenRepositoryInAppRequest): Promise<void>;
```

## UI Contract

- The repository detail action menu MUST show registered apps with stable labels.
- Selecting an available app MUST close the menu and mark that app action as pending.
- Unavailable apps MUST either be disabled with a reason or return a clear launch failure.
- Success MUST clear previous action errors and show a visible success cue.
- Failure MUST show a user-readable error that includes the failed app context.
- Action failure MUST NOT clear the current repository selection, search query, expanded tree state, or metadata form state.

## Path Handling Contract

- Repository paths are passed as structured command arguments, not shell command strings.
- Paths containing spaces, Korean text, or special characters MUST be opened as the exact target directory.
- The command MUST validate the repository before launching the app.
