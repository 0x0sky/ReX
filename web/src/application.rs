pub trait MathRenderer {
    type Error;

    fn render(&self, source: &str) -> Result<String, Self::Error>;
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
    pub fn execute(&self, source: &str) -> Result<String, R::Error> {
        self.renderer.render(source)
    }
}

#[cfg(test)]
mod tests {
    use super::{MathRenderer, RenderMath};

    struct StubRenderer;

    impl MathRenderer for StubRenderer {
        type Error = ();

        fn render(&self, source: &str) -> Result<String, Self::Error> {
            Ok(format!("rendered:{source}"))
        }
    }

    #[test]
    fn render_use_case_delegates_to_renderer_port() {
        let use_case = RenderMath::new(StubRenderer);

        let output = use_case.execute("x^2").unwrap();

        assert_eq!(output, "rendered:x^2");
    }
}
