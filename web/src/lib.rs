mod application;
mod infrastructure;

use application::{RenderBackend, RenderMath, RenderMode, RenderPayload, RenderResult};
use infrastructure::RexRenderer;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmRenderResult {
    backend: String,
    fallback_reason: Option<String>,
    kind: String,
    svg: Option<String>,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl WasmRenderResult {
    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn fallback_reason(&self) -> Option<String> {
        self.fallback_reason.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> Option<String> {
        self.svg.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
}

impl From<RenderResult> for WasmRenderResult {
    fn from(result: RenderResult) -> Self {
        let backend = match result.backend {
            RenderBackend::WebGpu => "webgpu",
            RenderBackend::Svg => "svg",
        }
        .into();

        match result.payload {
            RenderPayload::Svg(svg) => Self {
                backend,
                fallback_reason: result.fallback_reason,
                kind: "svg".into(),
                svg: Some(svg),
                width: 0,
                height: 0,
                rgba: Vec::new(),
            },
            RenderPayload::Rgba {
                width,
                height,
                pixels,
            } => Self {
                backend,
                fallback_reason: result.fallback_reason,
                kind: "rgba".into(),
                svg: None,
                width,
                height,
                rgba: pixels,
            },
        }
    }
}

#[wasm_bindgen]
pub async fn render_math(
    source: &str,
    font_source: &str,
    mode: &str,
) -> Result<WasmRenderResult, JsValue> {
    let mode = RenderMode::try_from(mode).map_err(js_error)?;
    let renderer = RexRenderer::new(font_source);
    let use_case = RenderMath::new(renderer);

    use_case
        .execute(source, mode)
        .await
        .map(WasmRenderResult::from)
        .map_err(js_error)
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
