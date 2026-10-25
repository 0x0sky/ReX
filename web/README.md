# ReX Web Adapter

This crate exposes ReX rendering to browser code through WebAssembly. It adapts the existing renderer-independent ReX scene pipeline; it does not create a second typesetting engine.

## Browser modes

- `AUTO` asks ReX to render with WebGPU first and returns the SVG fallback only when WebGPU cannot initialize or render.
- `WEBGPU` requires the WebGPU backend and surfaces adapter/device/render failures instead of hiding them.
- `SVG` uses the deterministic SVG backend directly.

The WebGPU path returns the RGBA image produced by the existing ReX `WebGpuRenderer`. The browser presentation layer only copies that result into a canvas; it does not re-render the mathematics in JavaScript.

## Dependency direction

`browser UI -> WASM API -> RenderMath -> MathRenderer <- RexRenderer -> rex`

The application layer owns render mode, backend, and payload contracts. The ReX-specific infrastructure adapter implements that port. The WASM boundary only translates the application result into browser-friendly values. Browser classes separately own WASM transport, payload presentation, DOM state, and orchestration.

## SOLID boundaries

- **Single responsibility:** `RenderMath` orchestrates one use case; `RexRenderer` adapts ReX; the WASM module translates ABI values; the browser renderer adapter normalizes transport; presenters turn one payload into one DOM node; the controller coordinates user intent.
- **Open/closed:** another rendering implementation can satisfy `MathRenderer`, and another browser payload can be added as a presenter without changing the use case.
- **Liskov substitution:** `RenderMath` accepts any `MathRenderer` implementation that preserves the render result contract.
- **Interface segregation:** the use case depends on one narrow `render(source, mode)` port instead of ReX, WebGPU, SVG, or DOM details.
- **Dependency inversion:** application code owns `RenderMode`, `RenderResult`, and `MathRenderer`; infrastructure depends on those abstractions, not the reverse.

No DOM, GitHub Pages, or deployment dependency belongs in the ReX core crate.
