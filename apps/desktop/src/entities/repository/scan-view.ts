import type { RepositoryRecord } from "./api";

export type ScanViewStatus = "idle" | "running" | "cancelling";

export function displayedRepositories(
  canonical: RepositoryRecord[],
  preview: RepositoryRecord[],
  status: ScanViewStatus,
): RepositoryRecord[] {
  return status === "running" ? preview : canonical;
}

export function discardPreview(items: Map<string, RepositoryRecord>): RepositoryRecord[] {
  items.clear();
  return [];
}
