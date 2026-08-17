use crate::application::MathRenderer;
use rex::error::Error;
use rex::{svg, RenderSettings};

pub struct RexSvgRenderer {
    settings: RenderSettings,
}

impl RexSvgRenderer {
    pub fn new(font_source: &str) -> Self {
        Self {
            settings: RenderSettings::default().font_src(font_source).debug(false),
        }
    }
}

impl MathRenderer for RexSvgRenderer {
    type Error = Error;

    fn render(&self, source: &str) -> Result<String, Self::Error> {
        svg::render_to_string(&self.settings, source)
    }
}

#[cfg(test)]
mod tests {
    use super::RexSvgRenderer;
    use crate::application::MathRenderer;

    #[test]
    fn rex_adapter_returns_svg_document() {
        let renderer = RexSvgRenderer::new("./rex-xits.woff2");

        let svg = renderer.render("x^2").unwrap();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("./rex-xits.woff2"));
    }
}
