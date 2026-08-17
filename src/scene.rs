use crate::dimensions::Float;
use crate::font::FontUnit;
use crate::layout::{Alignment, Layout, LayoutNode, LayoutVariant};
use crate::parser::color::RGBA;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: FontUnit,
    pub y: FontUnit,
}

impl Point {
    pub fn translate(self, dx: FontUnit, dy: FontUnit) -> Point {
        Point {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn left(self, dx: FontUnit) -> Point {
        Point {
            x: self.x - dx,
            y: self.y,
        }
    }

    pub fn right(self, dx: FontUnit) -> Point {
        Point {
            x: self.x + dx,
            y: self.y,
        }
    }

    pub fn up(self, dy: FontUnit) -> Point {
        Point {
            x: self.x,
            y: self.y - dy,
        }
    }

    pub fn down(self, dy: FontUnit) -> Point {
        Point {
            x: self.x,
            y: self.y + dy,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub fn has_alpha(&self) -> bool {
        self.alpha != 0xff
    }
}

impl From<RGBA> for Color {
    fn from(color: RGBA) -> Color {
        Color {
            red: color.0,
            green: color.1,
            blue: color.2,
            alpha: color.3,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugBoxKind {
    HorizontalBox,
    VerticalBox,
    Glyph,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneGlyph {
    pub position: Point,
    pub unicode: u32,
    pub scale: Float,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneRule {
    pub position: Point,
    pub width: FontUnit,
    pub height: FontUnit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneDebugBox {
    pub position: Point,
    pub width: FontUnit,
    pub height: FontUnit,
    pub kind: DebugBoxKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneColor {
    pub color: Color,
    pub contents: Vec<SceneNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneNode {
    Glyph(SceneGlyph),
    Rule(SceneRule),
    Color(SceneColor),
    DebugBox(SceneDebugBox),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MathScene {
    pub width: FontUnit,
    pub height: FontUnit,
    pub nodes: Vec<SceneNode>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneSettings {
    pub horizontal_padding: FontUnit,
    pub vertical_padding: FontUnit,
    pub debug: bool,
}

pub trait SceneBuilder {
    fn build(&self, layout: &Layout) -> MathScene;
}

#[derive(Copy, Clone, Debug)]
pub struct LayoutSceneBuilder {
    settings: SceneSettings,
}

impl LayoutSceneBuilder {
    pub fn new(settings: SceneSettings) -> LayoutSceneBuilder {
        LayoutSceneBuilder { settings: settings }
    }

    fn build_hbox(
        &self,
        mut position: Point,
        nodes: &[LayoutNode],
        height: FontUnit,
        nodes_width: FontUnit,
        alignment: Alignment,
    ) -> Vec<SceneNode> {
        if let Alignment::Centered(width) = alignment {
            position.x += (nodes_width - width) / 2;
        }

        let mut output = Vec::new();

        if self.settings.debug {
            output.push(SceneNode::DebugBox(SceneDebugBox {
                position: position.up(height),
                width: nodes_width,
                height: height,
                kind: DebugBoxKind::HorizontalBox,
            }));
        }

        for node in nodes {
            match node.node {
                LayoutVariant::Glyph(ref glyph) => {
                    if self.settings.debug {
                        output.push(SceneNode::DebugBox(SceneDebugBox {
                            position: position.up(node.height),
                            width: node.width,
                            height: node.height - node.depth,
                            kind: DebugBoxKind::Glyph,
                        }));
                    }

                    output.push(SceneNode::Glyph(SceneGlyph {
                        position: position,
                        unicode: glyph.unicode,
                        scale: f64::from(glyph.scale),
                    }));
                }
                LayoutVariant::Rule => output.push(SceneNode::Rule(SceneRule {
                    position: position.up(node.height),
                    width: node.width,
                    height: node.height,
                })),
                LayoutVariant::VerticalBox(ref vbox) => {
                    if self.settings.debug {
                        output.push(SceneNode::DebugBox(SceneDebugBox {
                            position: position.up(node.height),
                            width: node.width,
                            height: node.height - node.depth,
                            kind: DebugBoxKind::VerticalBox,
                        }));
                    }
                    output.extend(self.build_vbox(position.up(node.height), &vbox.contents));
                }
                LayoutVariant::HorizontalBox(ref hbox) => {
                    output.extend(self.build_hbox(
                        position,
                        &hbox.contents,
                        node.height,
                        node.width,
                        hbox.alignment,
                    ));
                }
                LayoutVariant::Color(ref color) => {
                    let contents = self.build_hbox(
                        position,
                        &color.inner,
                        node.height,
                        node.width,
                        Alignment::Default,
                    );
                    output.push(SceneNode::Color(SceneColor {
                        color: color.color.into(),
                        contents: contents,
                    }));
                }
                LayoutVariant::Kern => {}
            }

            position.x += node.width;
        }

        output
    }

    fn build_vbox(&self, mut position: Point, nodes: &[LayoutNode]) -> Vec<SceneNode> {
        let mut output = Vec::new();

        for node in nodes {
            match node.node {
                LayoutVariant::Rule => output.push(SceneNode::Rule(SceneRule {
                    position: position,
                    width: node.width,
                    height: node.height,
                })),
                LayoutVariant::HorizontalBox(ref hbox) => {
                    output.extend(self.build_hbox(
                        position.down(node.height),
                        &hbox.contents,
                        node.height,
                        node.width,
                        hbox.alignment,
                    ));
                }
                LayoutVariant::VerticalBox(ref vbox) => {
                    if self.settings.debug {
                        output.push(SceneNode::DebugBox(SceneDebugBox {
                            position: position,
                            width: node.width,
                            height: node.height - node.depth,
                            kind: DebugBoxKind::VerticalBox,
                        }));
                    }
                    output.extend(self.build_vbox(position, &vbox.contents));
                }
                LayoutVariant::Glyph(ref glyph) => {
                    if self.settings.debug {
                        output.push(SceneNode::DebugBox(SceneDebugBox {
                            position: position,
                            width: node.width,
                            height: node.height - node.depth,
                            kind: DebugBoxKind::Glyph,
                        }));
                    }
                    output.push(SceneNode::Glyph(SceneGlyph {
                        position: position.down(node.height),
                        unicode: glyph.unicode,
                        scale: f64::from(glyph.scale),
                    }));
                }
                LayoutVariant::Color(ref color) => {
                    let contents = self.build_hbox(
                        position.down(node.height),
                        &color.inner,
                        node.height,
                        node.width,
                        Alignment::Default,
                    );
                    output.push(SceneNode::Color(SceneColor {
                        color: color.color.into(),
                        contents: contents,
                    }));
                }
                LayoutVariant::Kern => {}
            }

            position.y += node.height;
        }

        output
    }
}

impl SceneBuilder for LayoutSceneBuilder {
    fn build(&self, layout: &Layout) -> MathScene {
        let width = layout.width + 2 * self.settings.horizontal_padding;
        let height = layout.height - layout.depth + 2 * self.settings.vertical_padding;
        let position = Point {
            x: self.settings.horizontal_padding,
            y: self.settings.vertical_padding + layout.height,
        };

        MathScene {
            width: width,
            height: height,
            nodes: self.build_hbox(
                position,
                &layout.contents,
                layout.height,
                layout.width,
                Alignment::Default,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutGlyph, LayoutVariant};

    #[test]
    fn builds_absolute_glyph_geometry_with_padding() {
        let mut layout = Layout::new();
        layout.add_node(LayoutNode {
            node: LayoutVariant::Glyph(LayoutGlyph {
                unicode: 'x' as u32,
                scale: FontUnit::from(1),
                offset: FontUnit::from(0),
                attachment: FontUnit::from(0),
                italics: FontUnit::from(0),
            }),
            width: FontUnit::from(100),
            height: FontUnit::from(80),
            depth: FontUnit::from(-20),
        });

        let builder = LayoutSceneBuilder::new(SceneSettings {
            horizontal_padding: FontUnit::from(10),
            vertical_padding: FontUnit::from(5),
            debug: false,
        });
        let scene = builder.build(&layout);

        assert_eq!(scene.width, FontUnit::from(120));
        assert_eq!(scene.height, FontUnit::from(110));
        assert_eq!(scene.nodes.len(), 1);

        match scene.nodes[0] {
            SceneNode::Glyph(ref glyph) => {
                assert_eq!(glyph.position.x, FontUnit::from(10));
                assert_eq!(glyph.position.y, FontUnit::from(85));
                assert_eq!(glyph.unicode, 'x' as u32);
                assert_eq!(glyph.scale, 1.0);
            }
            _ => panic!("expected glyph scene node"),
        }
    }

    #[test]
    fn debug_geometry_is_scene_data_not_backend_behavior() {
        let layout = Layout {
            width: FontUnit::from(20),
            height: FontUnit::from(10),
            ..Layout::default()
        };
        let builder = LayoutSceneBuilder::new(SceneSettings {
            debug: true,
            ..SceneSettings::default()
        });
        let scene = builder.build(&layout);

        assert_eq!(scene.nodes.len(), 1);
        match scene.nodes[0] {
            SceneNode::DebugBox(ref debug) => {
                assert_eq!(debug.kind, DebugBoxKind::HorizontalBox);
            }
            _ => panic!("expected debug box scene node"),
        }
    }
}
