export class BrowserSvgPresenter {
  #document;
  #createParser;

  constructor({ document, createParser = () => new DOMParser() }) {
    this.#document = document;
    this.#createParser = createParser;
  }

  present(payload) {
    const parsed = this.#createParser().parseFromString(payload.svg, "image/svg+xml");
    const parserError = parsed.querySelector("parsererror");

    if (parserError) {
      throw new Error("ReX returned an invalid SVG document.");
    }

    return this.#document.importNode(parsed.documentElement, true);
  }
}

export class BrowserRgbaPresenter {
  #document;

  constructor({ document }) {
    this.#document = document;
  }

  present(payload) {
    const canvas = this.#document.createElement("canvas");
    canvas.width = payload.width;
    canvas.height = payload.height;
    canvas.setAttribute("aria-label", "ReX WebGPU render");

    const context = canvas.getContext("2d");
    if (!context) {
      throw new Error("2D canvas context is unavailable for WebGPU readback presentation.");
    }

    const image = context.createImageData(payload.width, payload.height);
    image.data.set(payload.pixels);
    context.putImageData(image, 0, 0);

    return canvas;
  }
}

export class DomDemoView {
  #sourceInput;
  #preview;
  #message;
  #modeButtons;
  #presenters;

  constructor({ document, presenters }) {
    this.#sourceInput = document.querySelector("[data-source]");
    this.#preview = document.querySelector("[data-preview]");
    this.#message = document.querySelector("[data-message]");
    this.#modeButtons = [...document.querySelectorAll("[data-render-mode]")];
    this.#presenters = presenters;

    if (
      !this.#sourceInput ||
      !this.#preview ||
      !this.#message ||
      this.#modeButtons.length === 0
    ) {
      throw new Error("Demo DOM contract is incomplete.");
    }
  }

  source() {
    return this.#sourceInput.value;
  }

  mode() {
    const active = this.#modeButtons.find(
      (button) => button.getAttribute("aria-pressed") === "true",
    );

    if (!active) {
      throw new Error("No render mode is selected.");
    }

    return active.dataset.renderMode;
  }

  onSourceChanged(listener) {
    this.#sourceInput.addEventListener("input", listener);
  }

  onModeChanged(listener) {
    for (const button of this.#modeButtons) {
      button.addEventListener("click", () => {
        for (const candidate of this.#modeButtons) {
          candidate.setAttribute("aria-pressed", String(candidate === button));
        }
        listener();
      });
    }
  }

  showRender(result) {
    const presenter = this.#presenters[result.payload.kind];
    if (!presenter) {
      throw new Error(`No presenter for render payload: ${result.payload.kind}`);
    }

    this.#preview.replaceChildren(presenter.present(result.payload));
    this.#message.textContent = this.#statusText(result);
    this.#message.dataset.state = "ready";
    this.#message.dataset.backend = result.backend;
  }

  showError(message) {
    this.#preview.replaceChildren();
    this.#message.textContent = message;
    this.#message.dataset.state = "error";
    delete this.#message.dataset.backend;
  }

  setBusy(isBusy) {
    if (isBusy) {
      this.#message.textContent = "rendering…";
      this.#message.dataset.state = "busy";
    }
  }

  #statusText(result) {
    if (result.backend === "webgpu") {
      return "WEBGPU · ReX/WASM";
    }

    if (result.fallbackReason) {
      return `SVG FALLBACK · ${result.fallbackReason}`;
    }

    return "SVG · ReX/WASM";
  }
}
