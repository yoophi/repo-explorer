import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export type AppInfo = {
  name: string;
  version: string;
};

export type RepositoryMetadata = {
  description: string;
  tags: string[];
  pinned: boolean;
};

export type ReadmeContent = {
  path: string;
  content: string;
};

export type GitStatusSummary = {
  uncommittedChanges: number;
  ahead: number;
  behind: number;
  hasUpstream: boolean;
};

export type RepositoryRecord = {
  id: string;
  name: string;
  path: string;
  relativePath: string;
  parentId: string | null;
  isWorktree: boolean;
  originUrl: string | null;
  gitStatus: GitStatusSummary;
  readme: ReadmeContent | null;
  metadata: RepositoryMetadata;
  metadataPath: string;
  lastSeenAt: number;
};

export type ScanRepositoriesRequest = {
  scanId: string;
  rootPath: string;
  maxDepth?: number;
};

export type RepositoryScanProgress = {
  scanId: string;
  phase: "started" | "scanning" | "found" | "inspecting";
  currentPath: string | null;
  visitedDirectories: number;
  discoveredRepositories: number;
  message: string | null;
};

export type RepositoryScanItem = {
  scanId: string;
  repository: RepositoryRecord;
};

export type RepositoryScanTerminal = {
  scanId: string;
  status: "completed" | "cancelled" | "failed";
  repositories: RepositoryRecord[] | null;
  error: string | null;
};

export type ScanAcknowledgement = { scanId: string };

export type UpdateRepositoryMetadataRequest = {
  repositoryId: string;
  description: string;
  tags: string[];
  pinned: boolean;
};

export type TerminalApp = "terminal" | "iterm2" | "ghostty" | "wezterm";

export type OpenRepositoryInTerminalRequest = {
  repositoryId: string;
  terminalApp: TerminalApp;
};

export function getAppInfo() {
  return invoke<AppInfo>("app_info");
}

export function listRepositories() {
  return invoke<RepositoryRecord[]>("list_repositories");
}

export function scanRepositories(request: ScanRepositoriesRequest) {
  return invoke<ScanAcknowledgement>("scan_repositories", { request });
}

export function cancelRepositoryScan(scanId: string) {
  return invoke<boolean>("cancel_repository_scan", { scanId });
}

export function updateRepositoryMetadata(request: UpdateRepositoryMetadataRequest) {
  return invoke<RepositoryRecord>("update_repository_metadata", { request });
}

export function openRepositoryInFinder(repository: RepositoryRecord) {
  return revealItemInDir(repository.path);
}

export function openRepositoryInTerminal(request: OpenRepositoryInTerminalRequest) {
  return invoke<void>("open_repository_in_terminal", { request });
}
