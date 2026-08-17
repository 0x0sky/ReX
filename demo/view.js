export class BrowserSvgParser {
  #document;
  #createParser;

  constructor({ document, createParser = () => new DOMParser() }) {
    this.#document = document;
    this.#createParser = createParser;
  }

  parse(svgText) {
    const parsed = this.#createParser().parseFromString(svgText, "image/svg+xml");
    const parserError = parsed.querySelector("parsererror");

    if (parserError) {
      throw new Error("ReX returned an invalid SVG document.");
    }

    return this.#document.importNode(parsed.documentElement, true);
  }
}

export class DomDemoView {
  #sourceInput;
  #preview;
  #message;
  #svgParser;

  constructor({ document, svgParser }) {
    this.#sourceInput = document.querySelector("[data-source]");
    this.#preview = document.querySelector("[data-preview]");
    this.#message = document.querySelector("[data-message]");
    this.#svgParser = svgParser;

    if (!this.#sourceInput || !this.#preview || !this.#message) {
      throw new Error("Demo DOM contract is incomplete.");
    }
  }

  source() {
    return this.#sourceInput.value;
  }

  onSourceChanged(listener) {
    this.#sourceInput.addEventListener("input", listener);
  }

  showSvg(svgText) {
    const svg = this.#svgParser.parse(svgText);
    this.#preview.replaceChildren(svg);
    this.#message.textContent = "rendered by ReX · WASM";
    this.#message.dataset.state = "ready";
  }

  showError(message) {
    this.#preview.replaceChildren();
    this.#message.textContent = message;
    this.#message.dataset.state = "error";
  }

  setBusy(isBusy) {
    if (isBusy) {
      this.#message.textContent = "loading ReX…";
      this.#message.dataset.state = "busy";
    }
  }
}
