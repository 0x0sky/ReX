export class DemoController {
  #renderer;
  #view;
  #initialized = false;
  #rendering = false;
  #renderPending = false;
  #renderRevision = 0;

  constructor({ renderer, view }) {
    this.#renderer = renderer;
    this.#view = view;
  }

  async start() {
    this.#view.setBusy(true);
    this.#view.onSourceChanged(() => void this.#scheduleRender());
    this.#view.onModeChanged(() => void this.#scheduleRender());

    await this.#renderer.initialize();
    this.#initialized = true;
    await this.#scheduleRender();
  }

  #scheduleRender() {
    this.#renderRevision += 1;
    this.#renderPending = true;
    this.#view.setBusy(true);

    if (!this.#initialized || this.#rendering) {
      return;
    }

    return this.#drainRenders();
  }

  async #drainRenders() {
    this.#rendering = true;

    try {
      while (this.#renderPending) {
        this.#renderPending = false;
        const revision = this.#renderRevision;

        try {
          const result = await this.#renderer.render(this.#view.source(), this.#view.mode());
          if (revision === this.#renderRevision) {
            this.#view.showRender(result);
          }
        } catch (error) {
          if (revision !== this.#renderRevision) {
            continue;
          }

          const message = error instanceof Error ? error.message : String(error);
          this.#view.showError(message);
        }
      }
    } finally {
      this.#rendering = false;
    }
  }
}
