export class DemoController {
  #renderer;
  #view;
  #renderQueued = false;

  constructor({ renderer, view }) {
    this.#renderer = renderer;
    this.#view = view;
  }

  async start() {
    this.#view.setBusy(true);
    this.#view.onSourceChanged(() => this.#scheduleRender());

    await this.#renderer.initialize();
    this.#render();
  }

  #scheduleRender() {
    if (this.#renderQueued) {
      return;
    }

    this.#renderQueued = true;
    queueMicrotask(() => {
      this.#renderQueued = false;
      this.#render();
    });
  }

  #render() {
    try {
      const svg = this.#renderer.render(this.#view.source());
      this.#view.showSvg(svg);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.#view.showError(message);
    }
  }
}
