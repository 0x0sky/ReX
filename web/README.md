# ReX Web Adapter

This crate exposes the existing ReX SVG renderer to browser code through WebAssembly. It is intentionally an adapter, not a second rendering engine.

## Dependency direction

`browser UI -> WASM API -> RenderMath -> MathRenderer <- RexSvgRenderer -> rex`

The application layer owns the `MathRenderer` port. The ReX-specific adapter implements that port, and the WASM boundary only translates browser-friendly input/output and errors.

## SOLID boundaries

- **Single responsibility:** the use case orchestrates rendering; the ReX adapter knows ReX; the WASM module knows `wasm-bindgen`; browser classes own rendering transport, DOM presentation, and orchestration separately.
- **Open/closed:** another renderer can implement `MathRenderer` without changing `RenderMath`.
- **Liskov substitution:** `RenderMath` accepts any implementation that satisfies the renderer port contract.
- **Interface segregation:** the browser-facing use case depends on one operation only: `render`.
- **Dependency inversion:** application code owns the abstraction and does not depend on ReX or browser APIs.

No DOM, GitHub Pages, or deployment dependency belongs in the ReX core crate.
