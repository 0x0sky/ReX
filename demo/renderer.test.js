import assert from "node:assert/strict";
import test from "node:test";
import { WasmRendererAdapter } from "./renderer.js";

test("normalizes WebGPU output and releases the wasm result", async () => {
  let initialized = false;
  let freed = false;
  const pixels = new Uint8Array([0, 0, 0, 255]);

  const adapter = new WasmRendererAdapter({
    initializeWasm: async () => {
      initialized = true;
    },
    renderWasm: async (source, fontSource, mode) => {
      assert.equal(source, "x");
      assert.equal(fontSource, "./font.woff2");
      assert.equal(mode, "webgpu");

      return {
        backend: "webgpu",
        fallback_reason: undefined,
        kind: "rgba",
        width: 1,
        height: 1,
        rgba: () => pixels,
        free: () => {
          freed = true;
        },
      };
    },
    fontSource: "./font.woff2",
  });

  await adapter.initialize();
  const result = await adapter.render("x", "webgpu");

  assert.equal(initialized, true);
  assert.equal(freed, true);
  assert.equal(result.backend, "webgpu");
  assert.equal(result.payload.kind, "rgba");
  assert.deepEqual(result.payload.pixels, pixels);
});

test("preserves AUTO fallback metadata for the view", async () => {
  const adapter = new WasmRendererAdapter({
    initializeWasm: async () => {},
    renderWasm: async () => ({
      backend: "svg",
      fallback_reason: "WebGPU adapter unavailable",
      kind: "svg",
      svg: "<svg></svg>",
      free: () => {},
    }),
    fontSource: "./font.woff2",
  });

  const result = await adapter.render("x", "auto");

  assert.equal(result.backend, "svg");
  assert.equal(result.fallbackReason, "WebGPU adapter unavailable");
  assert.deepEqual(result.payload, { kind: "svg", svg: "<svg></svg>" });
});
