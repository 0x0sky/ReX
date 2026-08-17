// extern crate rex;
// extern crate font_types as font;
// #[macro_use]
// extern crate serde_derive;

use font::FontUnit;
use rex::parser::color::RGBA;
use rex::render::{RenderSettings, Renderer, SceneRenderer};
use rex::scene::{Color, MathScene, SceneNode};
use std::cell::Cell;

type Objects = Vec<Object>;

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub tex: String,
    pub description: String,
    pub width: FontUnit,
    pub height: FontUnit,
    pub render: Objects,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Symbol(DebugSymbol),
    Rule(DebugRule),
    Color(RGBA, Vec<Object>),
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugSymbol {
    pub scale: f64,
    pub codepoint: u32,
    pub x: FontUnit,
    pub y: FontUnit,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugRule {
    pub width: FontUnit,
    pub height: FontUnit,
    pub x: FontUnit,
    pub y: FontUnit,
}

#[derive(Clone, Default)]
pub struct DebugRenderer {
    settings: RenderSettings,
    pub width: Cell<FontUnit>,
    pub height: Cell<FontUnit>,
}

impl DebugRenderer {
    fn render_nodes(out: &mut Objects, nodes: &[SceneNode]) {
        for node in nodes {
            match *node {
                SceneNode::Glyph(ref glyph) => out.push(Object::Symbol(DebugSymbol {
                    codepoint: glyph.unicode,
                    scale: glyph.scale,
                    x: glyph.position.x,
                    y: glyph.position.y,
                })),
                SceneNode::Rule(ref rule) => out.push(Object::Rule(DebugRule {
                    width: rule.width,
                    height: rule.height,
                    x: rule.position.x,
                    y: rule.position.y,
                })),
                SceneNode::Color(ref color) => {
                    let mut inner = Objects::default();
                    Self::render_nodes(&mut inner, &color.contents);
                    out.push(Object::Color(to_rgba(color.color), inner));
                }
                SceneNode::DebugBox(_) => {}
            }
        }
    }
}

fn to_rgba(color: Color) -> RGBA {
    RGBA(color.red, color.green, color.blue, color.alpha)
}

impl SceneRenderer for DebugRenderer {
    type Out = Objects;

    fn render_scene_to(&self, out: &mut Objects, scene: &MathScene) -> Result<(), rex::error::Error> {
        self.width.set(scene.width);
        self.height.set(scene.height);
        Self::render_nodes(out, &scene.nodes);
        Ok(())
    }
}

impl Renderer for DebugRenderer {
    fn settings(&self) -> &RenderSettings {
        &self.settings
    }
}
