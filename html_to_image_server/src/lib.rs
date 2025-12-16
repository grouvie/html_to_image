#![allow(
    unused_crate_dependencies,
    reason = "Package shares dependencies across lib/bin/test targets; some are bin-only."
)]

use std::path::{Path, PathBuf};

// Ensure package-level unused dependency lint stays satisfied when building the library target.
#[allow(
    unused_imports,
    reason = "Binary-only dependencies are declared at package level."
)]
use anyhow as _;
#[allow(
    unused_imports,
    reason = "Binary-only dependencies are declared at package level."
)]
use dotenvy as _;
#[allow(
    unused_imports,
    reason = "Binary-only dependencies are declared at package level."
)]
use tracing_subscriber as _;

use html_to_image::{
    DEFAULT_ANIMATION_TIME, DEFAULT_SCALE, RenderError, render_html_to_png_bytes, render_template,
};
use poem::{
    Endpoint, EndpointExt, IntoResponse, Response, Route,
    endpoint::make_sync,
    error::ResponseError,
    http::StatusCode,
    middleware::{SizeLimit, Tracing},
    web::Json as PoemJson,
};
use poem_openapi::{
    ApiResponse, Object, OpenApi, OpenApiService,
    payload::{Binary, Json as OpenApiJson},
    types::Any,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use tokio::task;
use tracing::error;

pub const DEFAULT_MAX_BODY_SIZE: usize = 0x0010_0000; // 1 MiB
pub const MAX_DIMENSION: u32 = 4096;
pub const MAX_SCALE: f64 = 8.0;
pub const MAX_ANIMATION_TIME: f64 = 60.0;

#[derive(Debug, Clone)]
pub struct AppState {
    pub fonts_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AppLimits {
    pub max_dimension: u32,
    pub max_scale: f64,
    pub max_animation_time: f64,
}

impl Default for AppLimits {
    fn default() -> Self {
        Self {
            max_dimension: MAX_DIMENSION,
            max_scale: MAX_SCALE,
            max_animation_time: MAX_ANIMATION_TIME,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub state: AppState,
    pub limits: AppLimits,
    pub max_body_size: usize,
    pub server_base_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            state: AppState { fonts_dir: None },
            limits: AppLimits::default(),
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            server_base_url: None,
        }
    }
}

#[must_use]
pub fn create_app(config: &AppConfig) -> impl Endpoint<Output = Response> + 'static {
    let config = config.clone();
    let api = RenderApi::new(config.state.clone(), config.limits.clone());
    let mut api_service = OpenApiService::new(api, "HTML to Image API", "0.1.0");
    if let Some(server) = &config.server_base_url {
        api_service = api_service.server(server.clone());
    }

    let swagger = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    let spec_alias = api_service.spec_endpoint();
    let api_with_limit = api_service.with(SizeLimit::new(config.max_body_size));

    Route::new()
        .at("/healthz", make_sync(|_| "ok"))
        .nest("/", api_with_limit)
        .nest("/swagger", swagger)
        .nest("/spec", spec)
        .nest("/api/spec", spec_alias)
        .with(Tracing)
}

#[derive(Debug, Clone)]
struct RenderApi {
    state: AppState,
    limits: AppLimits,
}

impl RenderApi {
    fn new(state: AppState, limits: AppLimits) -> Self {
        Self { state, limits }
    }
}

#[OpenApi]
impl RenderApi {
    /// Render HTML (as a `MiniJinja` template) to PNG bytes.
    #[oai(path = "/render/png", method = "post")]
    async fn render_png(&self, req: OpenApiJson<RenderRequest>) -> ApiResult<RenderResponse> {
        validate_request(&req.0, &self.limits)?;

        let font_paths = resolve_requested_fonts(&self.state, req.0.font_paths.as_deref())?;
        let context = build_context(&req.0);
        let html = render_template(&req.html, &context).map_err(ApiError::from)?;

        let width = req.width;
        let height = req.height;
        let scale = req.scale;
        let animation_time = req.animation_time;

        let png_bytes = task::spawn_blocking(move || {
            render_html_to_png_bytes(&html, width, height, scale, animation_time, &font_paths)
        })
        .await
        .map_err(|err| {
            error!(%err, "render task join error");
            ApiError::internal("render task failed")
        })?
        .map_err(ApiError::from)?;

        Ok(RenderResponse::Png(Binary(png_bytes)))
    }
}

#[derive(Object, Debug, Deserialize)]
pub struct RenderRequest {
    /// HTML content that may contain `MiniJinja` placeholders.
    pub html: String,
    /// Output width in pixels (1..=4096 by default).
    pub width: u32,
    /// Output height in pixels (1..=4096 by default).
    pub height: u32,
    /// Scale factor applied during painting.
    #[oai(default = "default_scale")]
    pub scale: f64,
    /// Virtual animation time passed into the renderer.
    #[oai(default = "default_animation_time")]
    pub animation_time: f64,
    /// Optional font file names resolved against the configured fonts directory.
    #[oai(default)]
    pub font_paths: Option<Vec<String>>,
    /// Arbitrary template variables (free-form JSON).
    #[oai(default)]
    pub data: Option<Any<Value>>,
}

#[derive(ApiResponse)]
pub enum RenderResponse {
    #[oai(status = 200, content_type = "image/png")]
    Png(Binary<Vec<u8>>),
}

fn default_scale() -> f64 {
    DEFAULT_SCALE
}

fn default_animation_time() -> f64 {
    DEFAULT_ANIMATION_TIME
}

fn validate_request(req: &RenderRequest, limits: &AppLimits) -> Result<(), ApiError> {
    if req.width == 0 || req.width > limits.max_dimension {
        return Err(ApiError::validation(format!(
            "width must be between 1 and {}",
            limits.max_dimension
        )));
    }
    if req.height == 0 || req.height > limits.max_dimension {
        return Err(ApiError::validation(format!(
            "height must be between 1 and {}",
            limits.max_dimension
        )));
    }
    if !(req.scale.is_finite() && req.scale > 0.0 && req.scale <= limits.max_scale) {
        return Err(ApiError::validation(format!(
            "scale must be within (0, {}]",
            limits.max_scale
        )));
    }
    if !(req.animation_time.is_finite()
        && req.animation_time >= 0.0
        && req.animation_time <= limits.max_animation_time)
    {
        return Err(ApiError::validation(format!(
            "animation_time must be between 0 and {} seconds",
            limits.max_animation_time
        )));
    }

    Ok(())
}

fn resolve_requested_fonts(
    state: &AppState,
    requested: Option<&[String]>,
) -> Result<Vec<PathBuf>, ApiError> {
    let Some(entries) = requested else {
        return Ok(Vec::new());
    };

    let fonts_dir = state.fonts_dir.as_ref().ok_or(ApiError::FontsNotAllowed)?;

    resolve_font_paths(fonts_dir, entries)
}

fn resolve_font_paths(fonts_dir: &Path, requested: &[String]) -> Result<Vec<PathBuf>, ApiError> {
    let mut resolved = Vec::with_capacity(requested.len());
    for name in requested {
        if name.contains('/') || name.contains('\\') {
            return Err(ApiError::FontsNotAllowed);
        }

        let candidate = fonts_dir.join(name);
        let canonical = candidate
            .canonicalize()
            .map_err(|err| ApiError::validation(format!("font not found: {name} ({err})")))?;

        if !canonical.starts_with(fonts_dir) {
            return Err(ApiError::FontsNotAllowed);
        }
        resolved.push(canonical);
    }
    Ok(resolved)
}

fn build_context(req: &RenderRequest) -> Value {
    let mut map = Map::new();
    map.insert("width".into(), Value::from(req.width));
    map.insert("height".into(), Value::from(req.height));

    if let Some(Any(custom)) = &req.data {
        match custom {
            Value::Object(obj) => {
                for (key, value) in obj {
                    map.insert(key.clone(), value.clone());
                }
            }
            other => {
                map.insert("data".into(), other.clone());
            }
        }
    }

    Value::Object(map)
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid request: {0}")]
    Validation(String),
    #[error("font usage is not allowed on this server")]
    FontsNotAllowed,
    #[error("rendering failed: {0}")]
    Render(String),
    #[error("render task failed: {0}")]
    Task(String),
}

pub type ApiResult<T> = poem::Result<T>;

impl ApiError {
    fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::Task(message.into())
    }
}

impl From<RenderError> for ApiError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::RegisterTemplate { .. }
            | RenderError::LoadTemplate { .. }
            | RenderError::RenderTemplate { .. }
            | RenderError::ReadFont { .. }
            | RenderError::RegisterFont { .. } => ApiError::Validation(error.to_string()),
            _ => ApiError::Render(error.to_string()),
        }
    }
}

impl ResponseError for ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::Validation(_) | ApiError::FontsNotAllowed => StatusCode::BAD_REQUEST,
            ApiError::Render(_) | ApiError::Task(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn as_response(&self) -> Response {
        let payload = PoemJson(ErrorBody {
            error: self.to_string(),
        });
        let mut response = payload.into_response();
        response.set_status(self.status());
        response
    }
}
