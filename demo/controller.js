export class DemoController {
  #renderer;
  #view;
  #renderQueued = false;
  #renderRevision = 0;

  constructor({ renderer, view }) {
    this.#renderer = renderer;
    this.#view = view;
  }

  async start() {
    this.#view.setBusy(true);
    this.#view.onSourceChanged(() => this.#scheduleRender());
    this.#view.onModeChanged(() => this.#scheduleRender());

    await this.#renderer.initialize();
    await this.#render();
  }

  #scheduleRender() {
    if (this.#renderQueued) {
      return;
    }

    this.#renderQueued = true;
    queueMicrotask(() => {
      this.#renderQueued = false;
      void this.#render();
    });
  }

  async #render() {
    const revision = ++this.#renderRevision;
    this.#view.setBusy(true);

    try {
      const result = await this.#renderer.render(this.#view.source(), this.#view.mode());
      if (revision === this.#renderRevision) {
        this.#view.showRender(result);
      }
    } catch (error) {
      if (revision !== this.#renderRevision) {
        return;
      }

      const message = error instanceof Error ? error.message : String(error);
      this.#view.showError(message);
    }
  }
}
