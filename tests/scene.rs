extern crate rex;

use rex::scene::SceneNode;
use rex::{RenderSettings, Typesetter};

#[test]
fn typesetter_exposes_renderer_independent_scene() {
    let settings = RenderSettings::default();
    let scene = Typesetter::new(&settings).typeset("x + y").unwrap();

    assert!(!scene.nodes.is_empty());
    assert!(scene.nodes.iter().any(|node| {
        match *node {
            SceneNode::Glyph(_) => true,
            _ => false,
        }
    }));
}

#[test]
fn svg_renders_from_the_scene_pipeline() {
    let settings = RenderSettings::default().font_src("rex-xits.otf");
    let svg = rex::render::svg::render_to_string(&settings, "x").unwrap();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("<text"));
}
