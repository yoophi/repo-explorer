import assert from "node:assert/strict";
import test from "node:test";
import { ScanSession, installScanSubscriptions } from "../src/entities/repository/scan-session.ts";

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

function fixture(start, cancel) {
  const events = [];
  const session = new ScanSession(
    { start, cancel },
    {
      onProgress: (value) => events.push(["progress", value.scanId]),
      onItem: (value) => events.push(["item", value.scanId, value.repository.id]),
      onTerminal: (value) => events.push(["terminal", value.scanId, value.status]),
      onStartError: (error) => events.push(["start-error", String(error)]),
      onCancelError: (error) => events.push(["cancel-error", String(error)]),
    },
  );
  return { session, events };
}

const request = { scanId: "mine", rootPath: "/fixture", maxDepth: 2 };
const progress = (scanId) => ({ scanId, phase: "scanning", currentPath: "/fixture", visitedDirectories: 1, discoveredRepositories: 0, message: null });
const item = (scanId, id = "/fixture/repo") => ({ scanId, repository: { id } });
const terminal = (scanId, status) => ({ scanId, status, repositories: null, error: null });

test("cancel before registration retries after acknowledgement and filters other jobs", async () => {
  const ack = deferred();
  const calls = [];
  const { session, events } = fixture(() => ack.promise, async (id) => {
    calls.push(id);
    return calls.length > 1;
  });

  assert.equal(session.start(request), true);
  assert.equal(session.cancel(), true);
  session.progress(progress("other"));
  session.item(item("other"));
  session.terminal(terminal("other", "completed"));
  assert.deepEqual(events, []);

  ack.resolve({ scanId: "mine" });
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(calls, ["mine", "mine"]);
  session.progress(progress("mine"));
  session.item(item("mine"));
  session.terminal(terminal("mine", "cancelled"));
  session.progress(progress("mine"));
  session.item(item("mine", "late"));
  assert.deepEqual(events, [["progress", "mine"], ["item", "mine", "/fixture/repo"], ["terminal", "mine", "cancelled"]]);
});

test("item and progress before acknowledgement follow the active scan, then terminal closes it", async () => {
  const ack = deferred();
  const { session, events } = fixture(() => ack.promise, async () => true);
  assert.equal(session.start(request), true);
  session.progress(progress("mine"));
  session.item(item("mine", "first"));
  session.item(item("other", "foreign"));
  session.terminal(terminal("mine", "completed"));
  session.item(item("mine", "late"));
  ack.resolve({ scanId: "mine" });
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, [
    ["progress", "mine"], ["item", "mine", "first"], ["terminal", "mine", "completed"],
  ]);
});

test("cancel transport failure is reported and a second cancel can succeed", async () => {
  const calls = [];
  const { session, events } = fixture(
    async () => ({ scanId: "mine" }),
    async (id) => {
      calls.push(id);
      if (calls.length === 1) throw new Error("transport unavailable");
      return true;
    },
  );
  session.start(request);
  await new Promise((resolve) => setImmediate(resolve));
  session.cancel();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, [["cancel-error", "Error: transport unavailable"]]);
  session.cancel();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(calls, ["mine", "mine"]);
});

test("dispose before ack still retries cancellation without delivering events", async () => {
  const ack = deferred();
  const calls = [];
  const { session, events } = fixture(() => ack.promise, async (id) => {
    calls.push(id);
    return calls.length > 1;
  });
  session.start(request);
  session.dispose();
  ack.resolve({ scanId: "mine" });
  await new Promise((resolve) => setImmediate(resolve));
  session.terminal(terminal("mine", "cancelled"));
  assert.deepEqual(calls, ["mine", "mine"]);
  assert.deepEqual(events, []);
});

test("all event listeners are ready before a scan can start", async () => {
  const progressListener = deferred();
  const itemListener = deferred();
  const terminalListener = deferred();
  const released = [];
  let ready = false;
  const { session } = fixture(async ({ scanId }) => ({ scanId }), async () => true);
  const installation = installScanSubscriptions([
    progressListener.promise, itemListener.promise, terminalListener.promise,
  ], () => true).then((release) => { ready = true; return release; });
  progressListener.resolve(() => released.push("progress"));
  itemListener.resolve(() => released.push("item"));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(ready, false);
  terminalListener.resolve(() => released.push("terminal"));
  const release = await installation;
  assert.equal(ready && session.start(request), true);
  release();
  assert.deepEqual(released, ["progress", "item", "terminal"]);
});

test("partial listener failure and late subscription both release successful listeners", async () => {
  const late = deferred();
  const released = [];
  await assert.rejects(installScanSubscriptions([
    Promise.resolve(() => released.push("progress")),
    Promise.reject(new Error("item listener failed")),
    late.promise,
  ], () => true), /item listener failed/);
  late.resolve(() => released.push("terminal"));
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(released, ["progress", "terminal"]);
});
