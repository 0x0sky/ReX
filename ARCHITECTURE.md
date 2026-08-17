# ReX architecture

ReX separates mathematical meaning, layout, scene construction, and output backends.

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
      +----> SVG renderer
      |
      +----> future renderer (WebGPU, native, canvas, ...)
```

## Responsibilities

- **parser** owns syntax and produces the mathematical parse tree.
- **layout** owns TeX-style geometry and produces layout boxes in font units.
- **scene** converts layout boxes into absolute renderer-independent drawing data.
- **renderers** consume `MathScene`; they do not parse TeX and do not calculate mathematical layout.
- **Typesetter** orchestrates parse -> layout -> scene construction.

## SOLID boundaries

### Single responsibility

Parsing, mathematical layout, scene construction, and backend serialization are separate responsibilities. A renderer is no longer responsible for walking layout boxes or invoking the parser.

### Open/closed

A new output backend implements the scene-rendering contract and consumes `MathScene`. Adding a backend should not require changes to parser or layout code.

### Liskov substitution

Backends consume the same scene contract. Any conforming renderer can replace SVG at the rendering boundary without changing typesetting behavior.

### Interface segregation

`SceneRenderer` only knows how to render a `MathScene`. The higher-level `Renderer` compatibility trait adds TeX-to-output convenience methods for renderers that also expose `RenderSettings`.

### Dependency inversion

High-level typesetting produces the abstract `MathScene` contract. SVG depends on that contract instead of the typesetting pipeline depending on SVG primitives.

## Scene contract

`MathScene` contains absolute geometry in font units and an ordered list of scene nodes:

- glyphs;
- rules;
- color groups;
- optional debug boxes.

The ordered scene preserves painter order. Color groups remain hierarchical so a backend can implement scoped style without reconstructing layout semantics.

The scene intentionally does not contain SVG-specific concepts. Font identity is still global through the current render settings; moving font selection into the scene is a future compatibility boundary rather than part of this first extraction.
