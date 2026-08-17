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

  render(source) {
    return this.#renderWasm(source, this.#fontSource);
  }
}
