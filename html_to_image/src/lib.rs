use std::{
    fs, io,
    path::{Path, PathBuf},
    result::Result as StdResult,
    sync::Arc,
};

use anyrender::ImageRenderer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz::{dom::DocumentConfig, html::HtmlDocument, paint};
use image::{ImageEncoder, codecs::png::PngEncoder};
use linebender_resource_handle::Blob;
use parley::FontContext;
use serde::Serialize;
use thiserror::Error;

pub const DEFAULT_SCALE: f64 = 1.0;
pub const DEFAULT_ANIMATION_TIME: f64 = 5.0;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to read template file: {path}")]
    ReadTemplate { source: io::Error, path: PathBuf },
    #[error("failed to register template")]
    RegisterTemplate {
        #[source]
        source: minijinja::Error,
    },
    #[error("failed to load template from environment")]
    LoadTemplate {
        #[source]
        source: minijinja::Error,
    },
    #[error("failed to render template")]
    RenderTemplate {
        #[source]
        source: minijinja::Error,
    },
    #[error("failed to create output directory: {path}")]
    CreateOutputDir { source: io::Error, path: PathBuf },
    #[error("failed to write png: {path}")]
    WritePng {
        source: image::ImageError,
        path: PathBuf,
    },
    #[error("failed to read font at {path}")]
    ReadFont { source: io::Error, path: PathBuf },
    #[error("no loadable fonts found at {path}")]
    RegisterFont { path: PathBuf },
}

pub type Result<T> = StdResult<T, RenderError>;

/// Load an HTML template from disk.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn load_template(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| RenderError::ReadTemplate {
        source,
        path: path.to_path_buf(),
    })
}

/// Render the `MiniJinja` template into HTML using arbitrary serializable data.
///
/// # Errors
/// Returns an error if the template cannot be registered or rendered.
pub fn render_template<T: Serialize>(template: &str, data: &T) -> Result<String> {
    let mut env = minijinja::Environment::new();

    // Treat this as HTML and escape user-provided values safely.
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);

    env.add_template("card.html", template)
        .map_err(|source| RenderError::RegisterTemplate { source })?;

    let html = env
        .get_template("card.html")
        .map_err(|source| RenderError::LoadTemplate { source })?
        .render(data)
        .map_err(|source| RenderError::RenderTemplate { source })?;

    Ok(html)
}

/// Render raw HTML to a PNG file.
///
/// # Errors
/// Returns an error if the output directory cannot be created or the PNG cannot be written.
pub fn render_html_to_png(
    html: &str,
    out_path: &Path,
    width: u32,
    height: u32,
    scale: f64,
    current_time_for_animations: f64,
    font_paths: &[PathBuf],
) -> Result<()> {
    let rgba = render_html_to_rgba(
        html,
        width,
        height,
        scale,
        current_time_for_animations,
        font_paths,
    )?;

    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| RenderError::CreateOutputDir {
            source,
            path: parent.to_path_buf(),
        })?;
    }

    image::save_buffer(out_path, &rgba, width, height, image::ColorType::Rgba8).map_err(
        |source| RenderError::WritePng {
            source,
            path: out_path.to_path_buf(),
        },
    )?;

    Ok(())
}

/// Render raw HTML to PNG bytes (in-memory).
///
/// This avoids filesystem I/O and is useful for HTTP responses.
///
/// # Errors
/// Returns an error if fonts cannot be loaded or the PNG encoding fails.
pub fn render_html_to_png_bytes(
    html: &str,
    width: u32,
    height: u32,
    scale: f64,
    current_time_for_animations: f64,
    font_paths: &[PathBuf],
) -> Result<Vec<u8>> {
    let rgba = render_html_to_rgba(
        html,
        width,
        height,
        scale,
        current_time_for_animations,
        font_paths,
    )?;
    encode_png(&rgba, width, height)
}

fn render_html_to_rgba(
    html: &str,
    width: u32,
    height: u32,
    scale: f64,
    current_time_for_animations: f64,
    font_paths: &[PathBuf],
) -> Result<Vec<u8>> {
    let mut font_ctx = FontContext::new();
    register_fonts(&mut font_ctx, font_paths)?;

    let cfg = DocumentConfig {
        font_ctx: Some(font_ctx),
        ..Default::default()
    };

    let mut doc = HtmlDocument::from_html(html, cfg);
    doc.resolve(current_time_for_animations);
    doc.resolve_layout();

    let mut renderer = VelloCpuImageRenderer::new(width, height);
    let mut rgba = vec![0_u8; (width * height * 4) as usize];

    renderer.render(
        |scene| {
            paint::paint_scene(scene, &doc, scale, width, height);
        },
        &mut rgba,
    );

    Ok(rgba)
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let encoder = PngEncoder::new(&mut buffer);
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|source| RenderError::WritePng {
            source,
            path: PathBuf::from("in-memory"),
        })?;
    Ok(buffer)
}

/// Render any `MiniJinja` template with arbitrary serializable data.
///
/// # Errors
/// Returns an error if reading the template, rendering HTML, or writing the PNG fails.
#[allow(
    clippy::too_many_arguments,
    reason = "Render configuration is explicit and stable for callers"
)]
pub fn render_to_png<T: Serialize>(
    template_path: &Path,
    data: &T,
    out_path: &Path,
    width: u32,
    height: u32,
    scale: f64,
    animation_time: f64,
    font_paths: &[PathBuf],
) -> Result<()> {
    let template = load_template(template_path)?;
    let html = render_template(&template, data)?;
    render_html_to_png(
        &html,
        out_path,
        width,
        height,
        scale,
        animation_time,
        font_paths,
    )
}

fn register_fonts(font_ctx: &mut FontContext, font_paths: &[PathBuf]) -> Result<()> {
    if font_paths.is_empty() {
        return Ok(());
    }

    for path in font_paths {
        let data = fs::read(path).map_err(|source| RenderError::ReadFont {
            source,
            path: path.clone(),
        })?;
        let added = font_ctx
            .collection
            .register_fonts(Blob::new(Arc::new(data)), None);
        if added.is_empty() {
            return Err(RenderError::RegisterFont { path: path.clone() });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::{error::Error as StdError, result::Result as StdResult};

    use super::*;
    use tempfile::tempdir;
    type TestResult<T = ()> = StdResult<T, Box<dyn StdError>>;

    const DEFAULT_WIDTH: u32 = 420;

    #[test]
    fn load_template_reads_file() -> TestResult {
        let dir = tempdir()?;
        let path = dir.path().join("template.html");
        fs::write(&path, "hello template")?;

        let contents = load_template(&path)?;
        if contents != "hello template" {
            return Err(format!("unexpected contents: {contents}").into());
        }
        Ok(())
    }

    #[derive(Serialize)]
    struct TestData {
        user: &'static str,
        icon: &'static str,
        message: &'static str,
        width: u32,
    }

    #[test]
    fn render_template_injects_data() -> TestResult {
        let template = "<div>{{ user }} {{ icon }} {{ message }} {{ width }}</div>";
        let data = TestData {
            user: "User",
            icon: "★",
            message: "hi",
            width: DEFAULT_WIDTH,
        };

        let rendered = render_template(template, &data)?;
        if !rendered.contains("User") {
            return Err("user missing".into());
        }
        if !rendered.contains("★") {
            return Err("icon missing".into());
        }
        if !rendered.contains("hi") {
            return Err("message missing".into());
        }
        if !rendered.contains(&DEFAULT_WIDTH.to_string()) {
            return Err("width missing".into());
        }
        Ok(())
    }

    #[test]
    fn render_html_to_png_creates_png_file() -> TestResult {
        let dir = tempdir()?;
        let out = dir.path().join("card.png");
        let html = "<html><body><div>Hello</div></body></html>";

        render_html_to_png(html, &out, 64, 48, 1.0, DEFAULT_ANIMATION_TIME, &[])?;

        let bytes = fs::read(&out)?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("output is not a PNG".into());
        }
        Ok(())
    }

    #[test]
    fn render_html_to_png_bytes_returns_png() -> TestResult {
        let html = "<html><body><div>Hello bytes</div></body></html>";

        let bytes = render_html_to_png_bytes(html, 64, 48, 1.0, DEFAULT_ANIMATION_TIME, &[])?;

        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("output is not a PNG".into());
        }
        Ok(())
    }
}
