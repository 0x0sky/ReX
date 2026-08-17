use crate::error::Error;
use crate::font::constants::UNITS_PER_EM;
use crate::font::FontUnit;
use crate::render::{RenderSettings, Renderer, SceneRenderer};
use crate::scene::{Color, DebugBoxKind, MathScene, Point, SceneNode};
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;

pub fn render_to_path<P: AsRef<Path>>(path: P, settings: &RenderSettings, input: &str) {
    render_to_file(
        &mut File::create(path.as_ref()).expect("could not create file"),
        settings,
        input,
    );
}

pub fn render_to_file(file: &mut File, settings: &RenderSettings, input: &str) {
    let s: Vec<u8> = SVGRenderer::new(settings)
        .render(input)
        .expect("failed to render");
    file.write_all(&s).expect("failed to write to file");
}

pub fn render_to_string(settings: &RenderSettings, input: &str) -> Result<String, Error> {
    let s: Vec<u8> = SVGRenderer::new(settings).render(input)?;
    Ok(String::from_utf8(s).unwrap())
}

pub fn render_scene_to_string(
    settings: &RenderSettings,
    scene: &MathScene,
) -> Result<String, Error> {
    let renderer = SVGRenderer::<Vec<u8>>::new(settings);
    let bytes = renderer.render_scene(scene)?;
    Ok(String::from_utf8(bytes).unwrap())
}

#[derive(Clone)]
pub struct SVGRenderer<'a, W: Write> {
    settings: &'a RenderSettings,
    _marker: PhantomData<W>,
}

impl<'a, W: Write> SVGRenderer<'a, W> {
    pub fn new(settings: &'a RenderSettings) -> SVGRenderer<'a, W> {
        SVGRenderer {
            settings,
            _marker: PhantomData,
        }
    }

    fn prepare(&self, out: &mut W, width: FontUnit, height: FontUnit) {
        let px_width = f64::from(width) / f64::from(UNITS_PER_EM) * self.settings.font_size as f64;
        let px_height =
            f64::from(height) / f64::from(UNITS_PER_EM) * self.settings.font_size as f64;

        writeln!(
            out,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg width="{:2}" height="{:2}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
    <defs>
    <style type="text/css">@font-face{{font-family:rex;src:url('{}');}}</style>
    </defs>
    <g font-family="rex" font-size="{}">"#,
            px_width,
            px_height,
            width,
            height,
            self.settings.font_src,
            f64::from(UNITS_PER_EM)
        )
        .expect("Failed to write to buffer!");
    }

    fn finish(&self, out: &mut W) {
        writeln!(
            out,
            "\
    </g>
</svg>
"
        )
        .unwrap();
    }

    fn debug_box(
        &self,
        out: &mut W,
        position: Point,
        width: FontUnit,
        height: FontUnit,
        kind: DebugBoxKind,
    ) {
        let color = match kind {
            DebugBoxKind::HorizontalBox => "blue",
            DebugBoxKind::VerticalBox => "red",
            DebugBoxKind::Glyph => "green",
        };

        writeln!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{}" stroke-width="8"/>"#,
            position.x, position.y, width, height, color
        )
        .expect("Failed to write to buffer!");
    }

    fn symbol(&self, out: &mut W, position: Point, symbol: u32, scale: f64) {
        use std::char;

        if scale != 1.0 {
            writeln!(
                out,
                r#"<text transform="translate({}, {}) scale({:.2})">{}</text>"#,
                position.x,
                position.y,
                scale,
                char::from_u32(symbol).expect("Unable to decode Unicode code point!")
            )
            .expect("Failed to write to buffer!");
        } else {
            writeln!(
                out,
                r#"<text transform="translate({}, {})">{}</text>"#,
                position.x,
                position.y,
                char::from_u32(symbol).expect("Unable to decode Unicode code point!")
            )
            .expect("Failed to write to buffer!");
        }
    }

    fn rule(&self, out: &mut W, position: Point, width: FontUnit, height: FontUnit) {
        writeln!(
            out,
            r##"<rect x="{}" y ="{}" width="{}" height="{}" fill="#000"/>"##,
            position.x, position.y, width, height
        )
        .expect("Failed to write to buffer!");
    }

    fn begin_color(&self, out: &mut W, color: Color) {
        if color.has_alpha() {
            writeln!(
                out,
                r##"<g fill="#{}{}{}">"##,
                color.red, color.green, color.blue
            )
            .expect("failed to write to buffer!");
        } else {
            writeln!(
                out,
                r#"<g fill="rgba({},{},{},{})">"#,
                color.red, color.green, color.blue, color.alpha
            )
            .expect("Failed to write to buffer!");
        }
    }

    fn render_nodes(&self, out: &mut W, nodes: &[SceneNode]) {
        for node in nodes {
            match *node {
                SceneNode::Glyph(ref glyph) => {
                    self.symbol(out, glyph.position, glyph.unicode, glyph.scale)
                }
                SceneNode::Rule(ref rule) => self.rule(out, rule.position, rule.width, rule.height),
                SceneNode::DebugBox(ref debug) => {
                    self.debug_box(out, debug.position, debug.width, debug.height, debug.kind)
                }
                SceneNode::Color(ref color) => {
                    self.begin_color(out, color.color);
                    self.render_nodes(out, &color.contents);
                    writeln!(out, "</g>").expect("Failed to write to buffer!");
                }
            }
        }
    }
}

impl<'a, W: Write> SceneRenderer for SVGRenderer<'a, W> {
    type Out = W;

    fn render_scene_to(&self, out: &mut W, scene: &MathScene) -> Result<(), Error> {
        self.prepare(out, scene.width, scene.height);
        self.render_nodes(out, &scene.nodes);
        self.finish(out);
        Ok(())
    }
}

impl<'a, W: Write> Renderer for SVGRenderer<'a, W> {
    fn settings(&self) -> &RenderSettings {
        self.settings
    }
}
