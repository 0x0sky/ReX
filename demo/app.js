import init, { render_math as renderMath } from "./pkg/rex_web.js";
import { DemoController } from "./controller.js";
import { WasmRendererAdapter } from "./renderer.js";
import { BrowserSvgParser, DomDemoView } from "./view.js";

const renderer = new WasmRendererAdapter({
  initializeWasm: init,
  renderWasm: renderMath,
  fontSource: "./rex-xits.woff2",
});

const svgParser = new BrowserSvgParser({ document });
const view = new DomDemoView({ document, svgParser });
const controller = new DemoController({ renderer, view });

controller.start().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  view.showError(message);
});
