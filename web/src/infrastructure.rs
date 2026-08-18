use crate::application::{
    MathRenderer, RenderFuture, RenderMode, RenderPayload, RenderResult,
};
use rex::error::Error as RexError;
use rex::{
    svg, AutoRenderer, FallbackReason, RenderSettings, RenderedMath, Typesetter, WebGpuError,
    WebGpuRenderer,
};
use std::error;
use std::fmt;

pub struct RexRenderer {
    settings: RenderSettings,
}

impl RexRenderer {
    pub fn new(font_source: &str) -> Self {
        Self {
            settings: RenderSettings::default().font_src(font_source).debug(false),
        }
    }

    fn render_svg(&self, source: &str) -> Result<RenderResult, RexRenderError> {
        let output = svg::render_to_string(&self.settings, source)?;
        Ok(RenderResult::svg(output, None))
    }

    async fn render_webgpu(&self, source: &str) -> Result<RenderResult, RexRenderError> {
        let scene = Typesetter::new(&self.settings).typeset(source)?;
        let image = WebGpuRenderer::new(&self.settings).render_scene(&scene).await?;

        Ok(RenderResult::webgpu(
            image.width,
            image.height,
            image.rgba,
        ))
    }

    async fn render_auto(&self, source: &str) -> Result<RenderResult, RexRenderError> {
        let result = AutoRenderer::new(&self.settings).render(source).await?;
        let fallback_reason = result.fallback_reason.map(fallback_reason_text);

        match result.output {
            RenderedMath::WebGpu(image) => Ok(RenderResult::webgpu(
                image.width,
                image.height,
                image.rgba,
            )),
            RenderedMath::Svg(svg) => Ok(RenderResult::svg(svg, fallback_reason)),
        }
    }
}

impl MathRenderer for RexRenderer {
    type Error = RexRenderError;

    fn render<'a>(
        &'a self,
        source: &'a str,
        mode: RenderMode,
    ) -> RenderFuture<'a, RenderResult, Self::Error> {
        Box::pin(async move {
            match mode {
                RenderMode::Auto => self.render_auto(source).await,
                RenderMode::WebGpu => self.render_webgpu(source).await,
                RenderMode::Svg => self.render_svg(source),
            }
        })
    }
}

fn fallback_reason_text(reason: FallbackReason) -> String {
    match reason {
        FallbackReason::WebGpuFeatureDisabled => "WebGPU feature disabled".into(),
        FallbackReason::WebGpuUnavailable(message) => message,
    }
}

#[derive(Debug)]
pub enum RexRenderError {
    Core(RexError),
    WebGpu(WebGpuError),
}

impl fmt::Display for RexRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RexRenderError::Core(ref error) => write!(formatter, "{}", error),
            RexRenderError::WebGpu(ref error) => write!(formatter, "{}", error),
        }
    }
}

impl error::Error for RexRenderError {}

impl From<RexError> for RexRenderError {
    fn from(error: RexError) -> Self {
        RexRenderError::Core(error)
    }
}

impl From<WebGpuError> for RexRenderError {
    fn from(error: WebGpuError) -> Self {
        RexRenderError::WebGpu(error)
    }
}

#[cfg(test)]
mod tests {
    use super::RexRenderer;
    use crate::application::{MathRenderer, RenderBackend, RenderMode, RenderPayload};
    use std::future::Future;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn poll_ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("SVG render unexpectedly yielded"),
        }
    }

    #[test]
    fn svg_mode_preserves_the_supplied_browser_font_source() {
        let renderer = RexRenderer::new("./rex-xits.woff2");
        let result = poll_ready(renderer.render("x^2", RenderMode::Svg)).unwrap();

        assert_eq!(result.backend, RenderBackend::Svg);
        match result.payload {
            RenderPayload::Svg(svg) => {
                assert!(svg.contains("<svg"));
                assert!(svg.contains("./rex-xits.woff2"));
            }
            RenderPayload::Rgba { .. } => panic!("expected SVG output"),
        }
    }
}
