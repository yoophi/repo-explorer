import assert from "node:assert/strict";
import test from "node:test";
import { selectVisibleRepository } from "../src/entities/repository/selection.ts";

test("initial Repo selection keeps flat catalog first when tree order differs", () => {
  const repositories = [{ id: "child" }, { id: "parent" }];
  const visibleTreeOrder = ["parent", "child"];
  assert.equal(selectVisibleRepository(null, repositories, visibleTreeOrder), "child");
  assert.equal(selectVisibleRepository("child", repositories, visibleTreeOrder), "child");
});

test("filtered-out selection and unavailable flat first fall back to visible tree first", () => {
  const repositories = [{ id: "hidden" }, { id: "a" }, { id: "b" }];
  assert.equal(selectVisibleRepository(null, repositories, ["a", "b"]), "a");
  assert.equal(selectVisibleRepository("hidden", repositories, ["b", "a"]), "b");
  assert.equal(selectVisibleRepository("hidden", repositories, []), null);
});
