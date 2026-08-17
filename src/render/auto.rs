use crate::error::Error;
use crate::render::svg;
use crate::render::{RenderSettings, Typesetter};
use crate::scene::MathScene;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderBackend {
    WebGpu,
    Svg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackReason {
    WebGpuFeatureDisabled,
    WebGpuUnavailable(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderedMath {
    #[cfg(feature = "webgpu")]
    WebGpu(super::webgpu::WebGpuImage),
    Svg(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AutoRenderResult {
    pub backend: RenderBackend,
    pub output: RenderedMath,
    pub fallback_reason: Option<FallbackReason>,
}

#[derive(Copy, Clone)]
pub struct AutoRenderer<'a> {
    settings: &'a RenderSettings,
}

impl<'a> AutoRenderer<'a> {
    pub fn new(settings: &'a RenderSettings) -> AutoRenderer<'a> {
        AutoRenderer { settings }
    }

    pub async fn render(&self, tex: &str) -> Result<AutoRenderResult, Error> {
        let scene = Typesetter::new(self.settings).typeset(tex)?;
        self.render_scene(&scene).await
    }

    pub async fn render_scene(&self, scene: &MathScene) -> Result<AutoRenderResult, Error> {
        #[cfg(feature = "webgpu")]
        {
            let renderer = super::webgpu::WebGpuRenderer::new(self.settings);
            match renderer.render_scene(scene).await {
                Ok(image) => {
                    return Ok(AutoRenderResult {
                        backend: RenderBackend::WebGpu,
                        output: RenderedMath::WebGpu(image),
                        fallback_reason: None,
                    });
                }
                Err(error) => {
                    return self
                        .svg_fallback(scene, FallbackReason::WebGpuUnavailable(error.to_string()));
                }
            }
        }

        #[cfg(not(feature = "webgpu"))]
        {
            self.svg_fallback(scene, FallbackReason::WebGpuFeatureDisabled)
        }
    }

    fn svg_fallback(
        &self,
        scene: &MathScene,
        reason: FallbackReason,
    ) -> Result<AutoRenderResult, Error> {
        let output = svg::render_scene_to_string(self.settings, scene)?;
        Ok(AutoRenderResult {
            backend: RenderBackend::Svg,
            output: RenderedMath::Svg(output),
            fallback_reason: Some(reason),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontUnit;

    #[test]
    fn svg_fallback_reuses_an_existing_scene() {
        let settings = RenderSettings::default().font_src("rex-xits.otf");
        let renderer = AutoRenderer::new(&settings);
        let scene = MathScene {
            width: FontUnit::from(1000),
            height: FontUnit::from(1000),
            nodes: Vec::new(),
        };

        let result = renderer
            .svg_fallback(&scene, FallbackReason::WebGpuFeatureDisabled)
            .unwrap();

        assert_eq!(result.backend, RenderBackend::Svg);
        assert_eq!(
            result.fallback_reason,
            Some(FallbackReason::WebGpuFeatureDisabled)
        );
        match result.output {
            RenderedMath::Svg(svg) => assert!(svg.contains("<svg")),
            #[cfg(feature = "webgpu")]
            RenderedMath::WebGpu(_) => panic!("expected SVG fallback"),
        }
    }
}
