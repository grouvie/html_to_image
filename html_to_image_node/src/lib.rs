use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::PathBuf;

#[napi(object)]
pub struct RenderRequest {
    pub template_path: String,
    pub out_path: String,
    pub width: u32,
    pub height: u32,

    /// Arbitrary JSON-like object from TS (Record<string, any>)
    pub data: Value,

    /// Optional render tuning
    pub scale: Option<f64>,
    pub animation_time: Option<f64>,

    /// Optional extra fonts (paths on disk)
    pub font_paths: Option<Vec<String>>,
}

/// Render a `MiniJinja` HTML template to a PNG on disk.
///
/// This is the Node-API entrypoint. Rendering is executed on a blocking thread via
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
///
/// # Errors
///
/// Returns a `napi::Error` with `Status::GenericFailure` if:
/// - The renderer fails to load or render the template (e.g., invalid `template_path`,
///   template/rendering error, missing assets/fonts, or other `html_to_image` failures).
/// - Writing the PNG fails (e.g., invalid `out_path` or permission/IO errors).
/// - The blocking task fails to join (e.g., the task panicked), in which case the join
///   error is surfaced as `GenericFailure`.
#[napi]
pub async fn render_template_to_png(req: RenderRequest) -> Result<()> {
    let template_path = PathBuf::from(req.template_path);
    let out_path = PathBuf::from(req.out_path);
    let width = req.width;
    let height = req.height;

    let data = req.data;
    let scale = req.scale.unwrap_or(1.0);
    let animation_time = req
        .animation_time
        .unwrap_or(html_to_image::DEFAULT_ANIMATION_TIME);

    let font_paths: Vec<PathBuf> = req
        .font_paths
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();

    spawn_blocking(move || {
        html_to_image::render_to_png(
            template_path.as_path(),
            &data,
            out_path.as_path(),
            width,
            height,
            scale,
            animation_time,
            &font_paths,
        )
        .map_err(|render_error| Error::new(Status::GenericFailure, render_error.to_string()))
    })
    .await
    .map_err(|join_error| Error::new(Status::GenericFailure, join_error.to_string()))??;

    Ok(())
}
