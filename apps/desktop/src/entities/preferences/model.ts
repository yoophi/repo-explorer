import { createSettingsStore, createSettingsDraft, editSettingsDraft, syncSettingsDraft, planSettingsDraft, type SettingsStorage } from "@yoophi/settings-core";

export type RepoPreferences = { rootPath: string; maxDepth: number };

export const defaultRepoPreferences: RepoPreferences = {
  rootPath: "/Users/yoophi/project",
  maxDepth: 4,
};

export function parseMaxDepthDraft(text: string): number | null {
  if (!/^(0|[1-9]\d*)$/.test(text)) return null;
  const value = Number(text);
  return Number.isInteger(value) && value >= 0 && value <= 20 ? value : null;
}

export function syncMaxDepthDraft(draft: string, dirty: boolean, confirmed: number): string {
  const base = createSettingsDraft(confirmed, String);
  return syncSettingsDraft(dirty ? editSettingsDraft(base, draft, String) : base, confirmed, String).text;
}

export function planMaxDepthCommit(draft: string, dirty: boolean, confirmed: number) {
  const base = createSettingsDraft(confirmed, String);
  return planSettingsDraft(dirty ? editSettingsDraft(base, draft, String) : base, parseMaxDepthDraft);
}

export function parseRepoPreferences(value: unknown): RepoPreferences {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("잘못된 탐색 설정입니다.");
  }
  const keys = Object.keys(value).sort();
  if (keys.length !== 2 || keys[0] !== "maxDepth" || keys[1] !== "rootPath"
    || !("rootPath" in value) || typeof value.rootPath !== "string"
    || !("maxDepth" in value) || typeof value.maxDepth !== "number"
    || !Number.isInteger(value.maxDepth) || value.maxDepth < 0 || value.maxDepth > 20) {
    throw new Error("잘못된 탐색 설정입니다.");
  }
  return { rootPath: value.rootPath, maxDepth: value.maxDepth };
}

export function createRepoPreferencesStore(storage?: () => SettingsStorage) {
  return createSettingsStore({
    key: "repo-explorer.preferences",
    version: 1,
    defaults: defaultRepoPreferences,
    parse: parseRepoPreferences,
    storage,
  });
}

export const repoPreferencesStore = createRepoPreferencesStore();
