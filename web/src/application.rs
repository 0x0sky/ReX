use std::convert::TryFrom;
use std::error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Auto,
    WebGpu,
    Svg,
}

impl TryFrom<&str> for RenderMode {
    type Error = RenderModeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "auto" => Ok(RenderMode::Auto),
            "webgpu" => Ok(RenderMode::WebGpu),
            "svg" => Ok(RenderMode::Svg),
            other => Err(RenderModeError(other.into())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderModeError(String);

impl fmt::Display for RenderModeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported render mode: {}", self.0)
    }
}

impl error::Error for RenderModeError {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderBackend {
    WebGpu,
    Svg,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderPayload {
    Svg(String),
    Rgba {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderResult {
    pub backend: RenderBackend,
    pub payload: RenderPayload,
    pub fallback_reason: Option<String>,
}

impl RenderResult {
    pub fn svg(svg: String, fallback_reason: Option<String>) -> Self {
        Self {
            backend: RenderBackend::Svg,
            payload: RenderPayload::Svg(svg),
            fallback_reason,
        }
    }

    pub fn webgpu(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            backend: RenderBackend::WebGpu,
            payload: RenderPayload::Rgba {
                width,
                height,
                pixels,
            },
            fallback_reason: None,
        }
    }
}

pub type RenderFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>;

pub trait MathRenderer {
    type Error;

    fn render<'a>(
        &'a self,
        source: &'a str,
        mode: RenderMode,
    ) -> RenderFuture<'a, RenderResult, Self::Error>;
}

pub struct RenderMath<R> {
    renderer: R,
}

impl<R> RenderMath<R> {
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }
}

impl<R> RenderMath<R>
where
    R: MathRenderer,
{
    pub async fn execute(&self, source: &str, mode: RenderMode) -> Result<RenderResult, R::Error> {
        self.renderer.render(source, mode).await
    }
}

#[cfg(test)]
mod tests {
    use super::{MathRenderer, RenderBackend, RenderFuture, RenderMath, RenderMode, RenderResult};
    use std::convert::TryFrom;

    struct StubRenderer;

    impl MathRenderer for StubRenderer {
        type Error = ();

        fn render<'a>(
            &'a self,
            source: &'a str,
            mode: RenderMode,
        ) -> RenderFuture<'a, RenderResult, Self::Error> {
            Box::pin(async move {
                assert_eq!(mode, RenderMode::WebGpu);
                Ok(RenderResult::svg(format!("rendered:{source}"), None))
            })
        }
    }

    #[test]
    fn render_modes_are_explicit_at_the_application_boundary() {
        assert_eq!(RenderMode::try_from("auto").unwrap(), RenderMode::Auto);
        assert_eq!(RenderMode::try_from("webgpu").unwrap(), RenderMode::WebGpu);
        assert_eq!(RenderMode::try_from("svg").unwrap(), RenderMode::Svg);
        assert!(RenderMode::try_from("canvas").is_err());
    }

    #[test]
    fn result_constructors_preserve_backend_semantics() {
        let svg = RenderResult::svg("<svg/>".into(), Some("fallback".into()));
        assert_eq!(svg.backend, RenderBackend::Svg);
        assert_eq!(svg.fallback_reason.as_deref(), Some("fallback"));

        let webgpu = RenderResult::webgpu(2, 3, vec![0; 24]);
        assert_eq!(webgpu.backend, RenderBackend::WebGpu);
        assert!(webgpu.fallback_reason.is_none());
    }

    #[test]
    fn render_use_case_delegates_to_renderer_port() {
        fn poll_ready<F: std::future::Future>(future: F) -> F::Output {
            use std::sync::Arc;
            use std::task::{Context, Poll, Wake, Waker};

            struct NoopWake;

            impl Wake for NoopWake {
                fn wake(self: Arc<Self>) {}
            }

            let waker = Waker::from(Arc::new(NoopWake));
            let mut context = Context::from_waker(&waker);
            let mut future = Box::pin(future);

            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => output,
                Poll::Pending => panic!("stub future unexpectedly yielded"),
            }
        }

        let use_case = RenderMath::new(StubRenderer);
        let output = poll_ready(use_case.execute("x^2", RenderMode::WebGpu)).unwrap();

        assert_eq!(output.backend, RenderBackend::Svg);
    }
}
