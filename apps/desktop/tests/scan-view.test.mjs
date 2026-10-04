import assert from "node:assert/strict";
import test from "node:test";
import { discardPreview, displayedRepositories } from "../src/entities/repository/scan-view.ts";

test("running scan shows only provisional items; immediate cancellation restores catalog", () => {
  const catalog = [{ id: "saved" }];
  const preview = [{ id: "new" }];
  assert.equal(displayedRepositories(catalog, preview, "running"), preview);
  assert.equal(displayedRepositories(catalog, preview, "cancelling"), catalog);
  const buffered = new Map([["new", preview[0]]]);
  assert.deepEqual(discardPreview(buffered), []);
  assert.equal(buffered.size, 0);
});

test("failed terminal restores catalog even if provisional items had arrived", () => {
  const catalog = [{ id: "saved" }];
  const preview = [{ id: "new" }];
  assert.equal(displayedRepositories(catalog, preview, "idle"), catalog);
});
