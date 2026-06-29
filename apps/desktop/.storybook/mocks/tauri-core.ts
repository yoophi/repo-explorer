import type { RepositoryRecord, UpdateRepositoryMetadataRequest } from "../../src/entities/repository";
import { emitMockEvent } from "./tauri-event";

let repositories: RepositoryRecord[] = [
  {
    id: "/Users/yoophi/project/repo-explorer",
    name: "repo-explorer",
    path: "/Users/yoophi/project/repo-explorer",
    relativePath: "repo-explorer",
    parentId: null,
    isWorktree: false,
    originUrl: "git@github.com:yoophi/repo-explorer.git",
    readme: {
      path: "/Users/yoophi/project/repo-explorer/README.md",
      content: "# Repo Explorer\n\nLocal repository catalog app.",
    },
    metadata: {
      description: "Local repository catalog app",
      tags: ["tauri", "workspace"],
      pinned: true,
    },
    metadataPath: "/Users/yoophi/project/repo-explorer/.repo-explorer.json",
    lastSeenAt: 1_798_342_400,
  },
  {
    id: "/Users/yoophi/project/tauri-git-explorer",
    name: "tauri-git-explorer",
    path: "/Users/yoophi/project/tauri-git-explorer",
    relativePath: "tauri-git-explorer",
    parentId: null,
    isWorktree: false,
    originUrl: "git@github.com:yoophi/tauri-git-explorer.git",
    readme: null,
    metadata: {
      description: "",
      tags: ["reference"],
      pinned: false,
    },
    metadataPath: "/Users/yoophi/project/tauri-git-explorer/.repo-explorer.json",
    lastSeenAt: 1_798_342_400,
  },
  {
    id: "/Users/yoophi/project/tauri-git-explorer-worktree",
    name: "tauri-git-explorer-worktree",
    path: "/Users/yoophi/project/tauri-git-explorer-worktree",
    relativePath: "tauri-git-explorer-worktree",
    parentId: "/Users/yoophi/project/tauri-git-explorer",
    isWorktree: true,
    originUrl: "git@github.com:yoophi/tauri-git-explorer.git",
    readme: null,
    metadata: {
      description: "Linked worktree for feature work",
      tags: ["worktree"],
      pinned: false,
    },
    metadataPath: "/Users/yoophi/project/tauri-git-explorer-worktree/.repo-explorer.json",
    lastSeenAt: 1_798_342_400,
  },
];

export async function invoke<T>(command: string, args?: Record<string, unknown>) {
  switch (command) {
    case "app_info":
      return {
        name: "repo-explorer",
        version: "0.1.0",
      } as T;
    case "list_repositories":
      return repositories as T;
    case "scan_repositories":
      emitMockEvent("repository_scan_progress", {
        phase: "scanning",
        currentPath: "/Users/yoophi/project",
        visitedDirectories: 12,
        discoveredRepositories: repositories.length,
        message: "Scanning sample repositories",
      });
      emitMockEvent("repository_scan_progress", {
        phase: "finished",
        currentPath: null,
        visitedDirectories: 12,
        discoveredRepositories: repositories.length,
        message: "Repository scan finished",
      });
      return repositories as T;
    case "update_repository_metadata": {
      const request = args?.request as UpdateRepositoryMetadataRequest | undefined;
      repositories = repositories.map((repository) =>
        repository.id === request?.repositoryId
          ? {
              ...repository,
              metadata: {
                description: request.description,
                tags: request.tags.map((tag) => tag.trim()).filter(Boolean),
                pinned: request.pinned,
              },
            }
          : repository,
      );

      return repositories.find((repository) => repository.id === request?.repositoryId) as T;
    }
    default:
      throw new Error(`Unhandled Storybook Tauri command: ${command}`);
  }
}
