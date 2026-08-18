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
