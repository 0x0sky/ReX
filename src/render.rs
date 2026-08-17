pub mod auto;
pub mod svg;
#[cfg(feature = "webgpu")]
pub mod webgpu;

pub use self::auto::{AutoRenderResult, AutoRenderer, FallbackReason, RenderBackend, RenderedMath};
pub use self::svg::SVGRenderer;
#[cfg(feature = "webgpu")]
pub use self::webgpu::{WebGpuError, WebGpuImage, WebGpuRenderer};

use crate::error::Error;
use crate::font::FontUnit;
use crate::layout::engine::layout as build_layout;
use crate::layout::{Layout, LayoutSettings, Style};
use crate::parser::parse;
use crate::scene::{LayoutSceneBuilder, MathScene, SceneBuilder, SceneSettings};

pub use crate::scene::Point as Cursor;

#[derive(Clone)]
pub struct RenderSettings {
    pub font_size: u16,
    pub font_src: String,
    pub horz_padding: FontUnit,
    pub vert_padding: FontUnit,
    pub strict: bool,
    pub style: Style,
    pub debug: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            font_size: 48,
            font_src: "http://rex.breeden.cc/rex-xits.otf".into(),
            horz_padding: FontUnit::from(250),
            vert_padding: FontUnit::from(100),
            strict: true,
            style: Style::Display,
            debug: false,
        }
    }
}

impl RenderSettings {
    pub fn font_size(self, size: u16) -> Self {
        RenderSettings {
            font_size: size,
            ..self
        }
    }

    pub fn font_src(self, src: &str) -> Self {
        RenderSettings {
            font_src: src.into(),
            ..self
        }
    }

    pub fn horz_padding(self, size: FontUnit) -> RenderSettings {
        RenderSettings {
            horz_padding: size,
            ..self
        }
    }

    pub fn vert_padding(self, size: FontUnit) -> RenderSettings {
        RenderSettings {
            vert_padding: size,
            ..self
        }
    }

    pub fn style(self, style: Style) -> RenderSettings {
        RenderSettings { style, ..self }
    }

    pub fn debug(self, debug: bool) -> RenderSettings {
        RenderSettings { debug, ..self }
    }

    pub fn layout_settings(&self) -> LayoutSettings {
        LayoutSettings {
            font_size: self.font_size,
            style: self.style,
        }
    }

    pub fn scene_settings(&self) -> SceneSettings {
        SceneSettings {
            horizontal_padding: self.horz_padding,
            vertical_padding: self.vert_padding,
            debug: self.debug,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Typesetter<'a> {
    settings: &'a RenderSettings,
}

impl<'a> Typesetter<'a> {
    pub fn new(settings: &'a RenderSettings) -> Typesetter<'a> {
        Typesetter { settings }
    }

    pub fn layout(&self, tex: &str) -> Result<Layout, Error> {
        let parse_tree = parse(tex)?;
        let layout = build_layout(&parse_tree, self.settings.layout_settings());

        trace!("Parse: {:?}", parse_tree);
        trace!("Layout: {:?}", layout);

        Ok(layout)
    }

    pub fn typeset(&self, tex: &str) -> Result<MathScene, Error> {
        let layout = self.layout(tex)?;
        let builder = LayoutSceneBuilder::new(self.settings.scene_settings());
        Ok(builder.build(&layout))
    }
}

pub trait SceneRenderer {
    type Out;

    fn render_scene_to(&self, out: &mut Self::Out, scene: &MathScene) -> Result<(), Error>;

    fn render_scene(&self, scene: &MathScene) -> Result<Self::Out, Error>
    where
        Self::Out: Default,
    {
        let mut out = Self::Out::default();
        self.render_scene_to(&mut out, scene)?;
        Ok(out)
    }
}

pub trait Renderer: SceneRenderer {
    fn settings(&self) -> &RenderSettings;

    fn render_to(&self, out: &mut Self::Out, tex: &str) -> Result<(), Error> {
        let scene = Typesetter::new(self.settings()).typeset(tex)?;
        self.render_scene_to(out, &scene)
    }

    fn render(&self, tex: &str) -> Result<Self::Out, Error>
    where
        Self::Out: Default,
    {
        let mut out = Self::Out::default();
        self.render_to(&mut out, tex)?;
        Ok(out)
    }
}
