# ReX architecture

ReX separates mathematical meaning, layout, scene construction, backend rendering, and backend selection.

```text
TeX-like input
      |
      v
    parser
      |
      v
     AST
      |
      v
 layout engine
      |
      v
  MathScene
      |
      +----> WebGPU renderer ----> RGBA image
      |
      +----> SVG renderer -------> SVG document
                 ^
                 |
          automatic fallback
```

`AutoRenderer` typesets once, tries the WebGPU backend first, and reuses the same
`MathScene` for SVG if WebGPU cannot acquire an adapter/device or cannot finish
rendering. Fallback never reparses or recalculates mathematical layout.

## Responsibilities

- **parser** owns syntax and produces the mathematical parse tree.
- **layout** owns TeX-style geometry and produces layout boxes in font units.
- **scene** converts layout boxes into absolute renderer-independent drawing data.
- **WebGPU renderer** rasterizes bundled font glyphs into a glyph atlas and uses GPU quads/WGSL for positioning, rules, colors, debug geometry, compositing, and offscreen rendering.
- **SVG renderer** is the stable document/software fallback and preserves the existing SVG entry points.
- **Typesetter** orchestrates parse -> layout -> scene construction.
- **AutoRenderer** owns backend policy only: WebGPU first, SVG fallback.

The core does not own a window, browser canvas, or event loop. WebGPU renders to an offscreen RGBA target, keeping host/surface integration outside the typesetting library.

## SOLID boundaries

### Single responsibility

Parsing, mathematical layout, scene construction, GPU rendering, SVG serialization, and backend selection are separate responsibilities.

### Open/closed

A new output backend consumes `MathScene`. Adding another backend does not require parser or layout changes.

### Liskov substitution

Backends consume the same scene semantics. Backend choice changes output representation, not mathematical layout.

### Interface segregation

`SceneRenderer` remains the small synchronous contract used by document renderers such as SVG. WebGPU exposes an async scene operation because adapter and device acquisition are asynchronous by contract. `AutoRenderer` bridges those execution models without forcing SVG callers into async APIs.

### Dependency inversion

High-level typesetting produces `MathScene`. Both SVG and WebGPU depend on that scene contract; the parser/layout pipeline does not depend on either backend.

## Scene contract

`MathScene` contains absolute geometry in font units and an ordered list of scene nodes:

- glyphs;
- rules;
- color groups;
- optional debug boxes.

The ordered scene preserves painter order. Color groups remain hierarchical so a backend can implement scoped style without reconstructing layout semantics.

Font identity is still global through `RenderSettings`. WebGPU currently uses the bundled `rex-xits.otf` for glyph rasterization, while SVG retains the existing `font_src` behavior. Moving font identity into `MathScene` is a separate compatibility change.

## Backend contract

The default crate feature enables `webgpu`.

- `AutoRenderer` prefers WebGPU.
- Any WebGPU initialization or rendering failure becomes an explicit `FallbackReason` and produces SVG from the same scene.
- Disabling the `webgpu` feature keeps ReX buildable as an SVG-only renderer; `AutoRenderer` reports `WebGpuFeatureDisabled`.
- Direct `WebGpuRenderer` callers receive `WebGpuError` instead of silent fallback.
- Direct SVG APIs remain available and unchanged.

The WebGPU dependency establishes a Rust 1.87 minimum toolchain for this fork.
