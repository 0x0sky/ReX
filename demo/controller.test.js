import assert from "node:assert/strict";
import test from "node:test";
import { DemoController } from "./controller.js";

test("passes the selected render mode through the application boundary", async () => {
  const calls = [];
  const rendered = [];
  let sourceListener;
  let modeListener;

  const renderer = {
    initialize: async () => {
      calls.push(["initialize"]);
    },
    render: async (source, mode) => {
      calls.push(["render", source, mode]);
      return {
        backend: "webgpu",
        fallbackReason: null,
        payload: { kind: "rgba", width: 1, height: 1, pixels: new Uint8Array(4) },
      };
    },
  };

  const view = {
    source: () => "x^2",
    mode: () => "webgpu",
    onSourceChanged: (listener) => {
      sourceListener = listener;
    },
    onModeChanged: (listener) => {
      modeListener = listener;
    },
    setBusy: () => {},
    showRender: (result) => rendered.push(result),
    showError: (message) => assert.fail(message),
  };

  const controller = new DemoController({ renderer, view });
  await controller.start();

  assert.deepEqual(calls, [
    ["initialize"],
    ["render", "x^2", "webgpu"],
  ]);
  assert.equal(rendered.length, 1);
  assert.equal(typeof sourceListener, "function");
  assert.equal(typeof modeListener, "function");
});

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function waitFor(condition) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (condition()) return;
    await Promise.resolve();
  }
  assert.ok(condition(), "expected async renderer progress");
}

test("serializes WebGPU renders, coalesces edits, and displays only the newest result", async () => {
  const calls = [];
  const rendered = [];
  const errors = [];
  let currentSource = "x";
  let currentMode = "auto";
  let sourceChanged;
  let modeChanged;
  let active = 0;
  let peakActive = 0;

  const renderer = {
    initialize: async () => {},
    render: (source, mode) => {
      const pending = deferred();
      active += 1;
      peakActive = Math.max(peakActive, active);
      calls.push({ source, mode, pending });
      return pending.promise.finally(() => { active -= 1; });
    },
  };
  const view = {
    source: () => currentSource,
    mode: () => currentMode,
    onSourceChanged: (callback) => { sourceChanged = callback; },
    onModeChanged: (callback) => { modeChanged = callback; },
    setBusy: () => {},
    showRender: (result) => rendered.push(result),
    showError: (message) => errors.push(message),
  };

  const started = new DemoController({ renderer, view }).start();
  await waitFor(() => calls.length === 1);
  currentSource = "x^2";
  sourceChanged();
  currentSource = "x^3";
  sourceChanged();
  currentMode = "webgpu";
  modeChanged();
  assert.equal(calls.length, 1, "edits must not start concurrent GPU work");

  calls[0].pending.resolve({ backend: "svg", payload: { kind: "svg", svg: "old" } });
  await waitFor(() => calls.length === 2);
  assert.deepEqual(calls.map(({ source, mode }) => [source, mode]), [
    ["x", "auto"],
    ["x^3", "webgpu"],
  ]);
  assert.deepEqual(rendered, [], "stale first result must not update the preview");
  assert.equal(peakActive, 1);

  const newest = { backend: "webgpu", payload: { kind: "rgba", width: 1, height: 1 } };
  calls[1].pending.resolve(newest);
  await started;
  assert.deepEqual(rendered, [newest]);
  assert.deepEqual(errors, []);
  assert.equal(active, 0);
});

test("suppresses stale errors and reports the latest failure", async () => {
  const pending = [];
  const errors = [];
  let sourceChanged;
  let source = "x";
  const view = {
    source: () => source,
    mode: () => "webgpu",
    onSourceChanged: (callback) => { sourceChanged = callback; },
    onModeChanged: () => {},
    setBusy: () => {},
    showRender: () => assert.fail("unexpected render"),
    showError: (message) => errors.push(message),
  };
  const renderer = {
    initialize: async () => {},
    render: () => {
      const work = deferred();
      pending.push(work);
      return work.promise;
    },
  };

  const started = new DemoController({ renderer, view }).start();
  await waitFor(() => pending.length === 1);
  source = "y";
  sourceChanged();
  pending[0].reject(new Error("old failure"));
  await waitFor(() => pending.length === 2);
  assert.deepEqual(errors, []);
  pending[1].reject(new Error("current failure"));
  await started;
  assert.deepEqual(errors, ["current failure"]);
});

test("does not render before WASM initialization and uses the latest input", async () => {
  const initialization = deferred();
  const calls = [];
  let source = "first";
  let onSourceChanged;
  const view = {
    source: () => source,
    mode: () => "svg",
    onSourceChanged: (callback) => { onSourceChanged = callback; },
    onModeChanged: () => {},
    setBusy: () => {},
    showRender: () => {},
    showError: (message) => assert.fail(message),
  };
  const renderer = {
    initialize: () => initialization.promise,
    render: async (value) => {
      calls.push(value);
      return { backend: "svg", payload: { kind: "svg", svg: value } };
    },
  };

  const started = new DemoController({ renderer, view }).start();
  source = "latest";
  onSourceChanged();
  assert.deepEqual(calls, []);
  initialization.resolve();
  await started;
  assert.deepEqual(calls, ["latest"]);
});
