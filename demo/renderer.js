export const RenderMode = Object.freeze({
  AUTO: "auto",
  WEBGPU: "webgpu",
  SVG: "svg",
});

export class WasmRendererAdapter {
  #initializeWasm;
  #renderWasm;
  #fontSource;

  constructor({ initializeWasm, renderWasm, fontSource }) {
    this.#initializeWasm = initializeWasm;
    this.#renderWasm = renderWasm;
    this.#fontSource = fontSource;
  }

  async initialize() {
    await this.#initializeWasm();
  }

  async render(source, mode) {
    const result = await this.#renderWasm(source, this.#fontSource, mode);

    try {
      return this.#normalize(result);
    } finally {
      result.free?.();
    }
  }

  #normalize(result) {
    const shared = {
      backend: result.backend,
      fallbackReason: result.fallback_reason ?? null,
    };

    if (result.kind === "svg") {
      if (typeof result.svg !== "string") {
        throw new Error("ReX returned an invalid SVG render payload.");
      }

      return {
        ...shared,
        payload: {
          kind: "svg",
          svg: result.svg,
        },
      };
    }

    if (result.kind === "rgba") {
      const pixels = result.rgba();
      const expectedLength = result.width * result.height * 4;

      if (!(pixels instanceof Uint8Array) || pixels.length !== expectedLength) {
        throw new Error("ReX returned an invalid WebGPU pixel payload.");
      }

      return {
        ...shared,
        payload: {
          kind: "rgba",
          width: result.width,
          height: result.height,
          pixels,
        },
      };
    }

    throw new Error(`ReX returned an unsupported render payload: ${result.kind}`);
  }
}
