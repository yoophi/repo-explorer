import { reconcileSelection } from "@yoophi/collection-core";
import type { RepositoryRecord } from "./api.ts";

/** Initial selection follows the flat catalog order; filtered fallback follows visible tree order. */
export function selectVisibleRepository(
  selectedId: string | null,
  repositories: readonly Pick<RepositoryRecord, "id">[],
  visibleIds: readonly string[],
): string | null {
  return reconcileSelection(selectedId, visibleIds, (visible) => {
    const firstCatalogId = repositories[0]?.id;
    if (selectedId === null && firstCatalogId && visible.includes(firstCatalogId)) return firstCatalogId;
    return visible[0] ?? null;
  });
}
