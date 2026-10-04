import assert from "node:assert/strict";
import test from "node:test";
import {
  createRepoPreferencesStore,
  defaultRepoPreferences,
  parseMaxDepthDraft,
  parseRepoPreferences,
  planMaxDepthCommit,
  syncMaxDepthDraft,
} from "../src/entities/preferences/model.ts";

function fixture(initial = null) {
  let text = initial;
  let writes = 0;
  let denied = false;
  const store = createRepoPreferencesStore(() => ({
    getItem: (key) => key === "repo-explorer.preferences" ? text : null,
    setItem: (key, value) => {
      assert.equal(key, "repo-explorer.preferences");
      if (denied) throw new Error("Storage denied");
      text = value;
      writes++;
    },
  }));
  return { store, text: () => text, writes: () => writes, deny: () => { denied = true; } };
}

test("preferences round trip uses v1 and preserves existing defaults", () => {
  const f = fixture();
  assert.deepEqual(f.store.getSnapshot().value, defaultRepoPreferences);
  assert.equal(f.writes(), 0);
  assert.equal(f.store.update((current) => ({ ...current, maxDepth: 0 })), true);
  assert.equal(f.store.update((current) => ({ ...current, rootPath: "/fixture/repos" })), true);
  assert.deepEqual(JSON.parse(f.text()), {
    version: 1,
    value: { rootPath: "/fixture/repos", maxDepth: 0 },
  });
  assert.deepEqual(fixture(f.text()).store.getSnapshot().value, { rootPath: "/fixture/repos", maxDepth: 0 });
});

test("invalid values and unsupported versions are preserved until explicit reset", () => {
  const invalid = [
    "{broken",
    JSON.stringify({ version: 2, value: defaultRepoPreferences }),
    JSON.stringify({ version: 1, value: { rootPath: "/fixture", maxDepth: "4" } }),
    JSON.stringify({ version: 1, value: { rootPath: "/fixture", maxDepth: 21 } }),
  ];
  for (const original of invalid) {
    const f = fixture(original);
    assert.ok(f.store.getSnapshot().error);
    assert.equal(f.store.update((current) => ({ ...current, maxDepth: 5 })), false);
    assert.equal(f.text(), original);
    assert.equal(f.store.reset(), true);
    assert.deepEqual(JSON.parse(f.text()), { version: 1, value: defaultRepoPreferences });
  }
});

test("strict parser and numeric draft reject NaN, empty and out-of-range values", () => {
  for (const input of ["", "-1", "1.5", "1e2", "21", "NaN"]) {
    assert.equal(parseMaxDepthDraft(input), null);
  }
  assert.equal(parseMaxDepthDraft("0"), 0);
  assert.equal(parseMaxDepthDraft("20"), 20);
  for (const invalid of [
    { rootPath: "/fixture", maxDepth: Number.NaN },
    { rootPath: "/fixture", maxDepth: -1 },
    { rootPath: "/fixture", maxDepth: 4, searchQuery: "not a preference" },
    { rootPath: 7, maxDepth: 4 },
  ]) {
    assert.throws(() => parseRepoPreferences(invalid));
  }
});

test("failed storage write retains confirmed value and exposes error", () => {
  const f = fixture();
  f.store.getSnapshot();
  f.deny();
  assert.equal(f.store.update((current) => ({ ...current, maxDepth: 7 })), false);
  assert.deepEqual(f.store.getSnapshot().value, defaultRepoPreferences);
  assert.match(f.store.getSnapshot().error, /Storage denied/);
  assert.equal(f.text(), null);
});

test("focus without edits follows external depth and blur does not overwrite it", () => {
  const f = fixture();
  f.store.getSnapshot();
  let draft = "4";
  const dirty = false; // focus alone does not mark the field dirty
  const otherWindow = fixture();
  otherWindow.store.update((current) => ({ ...current, maxDepth: 7 }));
  const externalText = otherWindow.text();
  const fAfterExternal = fixture(externalText);
  fAfterExternal.store.getSnapshot();

  draft = syncMaxDepthDraft(draft, dirty, fAfterExternal.store.getSnapshot().value.maxDepth);
  const plan = planMaxDepthCommit(draft, dirty, fAfterExternal.store.getSnapshot().value.maxDepth);
  assert.equal(draft, "7");
  assert.deepEqual(plan, { value: 7, needsWrite: false });
  assert.equal(fAfterExternal.writes(), 0);
  assert.equal(f.writes(), 0);
});

test("dirty draft survives external update until explicit commit", () => {
  const f = fixture();
  f.store.getSnapshot();
  const draft = syncMaxDepthDraft("6", true, 7);
  assert.equal(draft, "6");
  const plan = planMaxDepthCommit(draft, true, 7);
  assert.deepEqual(plan, { value: 6, needsWrite: true });
  assert.deepEqual(planMaxDepthCommit("", true, 7), { value: null, needsWrite: false });
});
