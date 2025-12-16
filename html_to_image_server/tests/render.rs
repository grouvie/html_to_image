#![allow(
    unused_crate_dependencies,
    reason = "Integration test does not exercise all package-level dependencies."
)]
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration test crate is the test module."
)]

use html_to_image_server::{AppConfig, AppLimits, AppState, DEFAULT_MAX_BODY_SIZE, create_app};
use poem::{http::StatusCode, test::TestClient};
use serde_json::json;

#[tokio::test]
async fn render_png_endpoint_returns_png() -> poem::Result<()> {
    let app_config = AppConfig {
        state: AppState { fonts_dir: None },
        limits: AppLimits::default(),
        max_body_size: DEFAULT_MAX_BODY_SIZE,
        server_base_url: None,
    };
    let app = create_app(&app_config);
    let client = TestClient::new(app);

    let payload = json!({
        "html": "<html><body><div>{{ name }}</div></body></html>",
        "width": 64,
        "height": 48,
        "data": { "name": "Test User" }
    });
    let body = payload.to_string();

    let response = client
        .post("/render/png")
        .header("content-length", body.len())
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await;

    response.assert_status(StatusCode::OK);

    let bytes = response.0.into_body().into_vec().await?;
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(())
    } else {
        Err(poem::Error::from_string(
            "response should be a PNG",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
