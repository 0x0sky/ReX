extern crate rex;

use rex::render::auto::{AutoRenderer, RenderedMath};
use rex::render::svg;
#[cfg(feature = "webgpu")]
use rex::render::webgpu::WebGpuRenderer;
use rex::{RenderSettings, Typesetter};
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

const DEFAULT_FORMULA: &str = r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}";
const REX_FONTS_SOURCE: &str = "ReTeX/rex-fonts: out/stix2/rex-stix2.otf";
const REX_FONTS_BLOB_SHA: &str = "386d595c9eee71a85bc95e52fa76377dbb741843";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let formula = args.get(1).map(String::as_str).unwrap_or(DEFAULT_FORMULA);
    let out_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("smoke-output"));

    fs::create_dir_all(&out_dir).expect("failed to create smoke-output directory");

    let settings = RenderSettings::default()
        .font_size(96)
        .font_src("rex-xits.otf");
    let scene = Typesetter::new(&settings)
        .typeset(formula)
        .expect("failed to typeset smoke formula");

    let svg_output =
        svg::render_scene_to_string(&settings, &scene).expect("failed to render SVG smoke output");
    write_text(out_dir.join("smoke.svg"), &svg_output);

    #[cfg(feature = "webgpu")]
    match pollster::block_on(WebGpuRenderer::new(&settings).render_scene(&scene)) {
        Ok(image) => {
            let path = out_dir.join("smoke.webgpu.png");
            write_png(&path, image.width, image.height, &image.rgba);
            eprintln!("direct WebGPU: {}", path.display());
        }
        Err(error) => eprintln!("direct WebGPU unavailable: {}", error),
    }

    let auto = pollster::block_on(AutoRenderer::new(&settings).render_scene(&scene))
        .expect("automatic renderer failed");

    match auto.output {
        RenderedMath::Svg(output) => {
            let path = out_dir.join("smoke.auto.svg");
            write_text(&path, &output);
            eprintln!(
                "AutoRenderer: SVG fallback ({:?}) -> {}",
                auto.fallback_reason,
                path.display()
            );
        }
        #[cfg(feature = "webgpu")]
        RenderedMath::WebGpu(image) => {
            let path = out_dir.join("smoke.auto.png");
            write_png(&path, image.width, image.height, &image.rgba);
            eprintln!("AutoRenderer: WebGPU -> {}", path.display());
        }
    }

    eprintln!("SVG reference: {}", out_dir.join("smoke.svg").display());
    eprintln!("font source: {}", REX_FONTS_SOURCE);
    eprintln!("font git blob: {}", REX_FONTS_BLOB_SHA);
}

fn write_text(path: impl AsRef<Path>, contents: &str) {
    fs::write(path.as_ref(), contents).expect("failed to write SVG smoke output");
}

fn write_png(path: impl AsRef<Path>, width: u32, height: u32, rgba: &[u8]) {
    let file = File::create(path.as_ref()).expect("failed to create PNG smoke output");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut png = encoder.write_header().expect("failed to write PNG header");
    png.write_image_data(rgba)
        .expect("failed to write PNG smoke output");
}
