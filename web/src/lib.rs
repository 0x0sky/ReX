mod application;
mod infrastructure;

use application::RenderMath;
use infrastructure::RexSvgRenderer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn render_math(source: &str, font_source: &str) -> Result<String, JsValue> {
    let renderer = RexSvgRenderer::new(font_source);
    let use_case = RenderMath::new(renderer);

    use_case
        .execute(source)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
