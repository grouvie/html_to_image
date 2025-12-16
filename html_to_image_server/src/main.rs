use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dotenvy::dotenv;
use html_to_image_server::{AppConfig, AppLimits, AppState, DEFAULT_MAX_BODY_SIZE, create_app};
use poem::{Server, listener::TcpListener};
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

const DEFAULT_ADDR: &str = "0.0.0.0:3000";
const DEFAULT_FONTS_DIR: &str = "assets/fonts";
const BYTES_PER_MEGABYTE: usize = 1024 * 1024;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_tracing();

    let addr = read_addr()?;
    let fonts_dir = read_fonts_dir()?;
    let max_body_size = read_max_body_size();

    let state = AppState {
        fonts_dir: Some(fonts_dir),
    };
    let config = AppConfig {
        state,
        limits: AppLimits::default(),
        max_body_size,
        server_base_url: Some(format!("http://{addr}")),
    };

    let listener = TcpListener::bind(addr);
    let app = create_app(&config);

    info!(%addr, "listening");
    Server::new(listener)
        .run_with_graceful_shutdown(app, shutdown_signal(), None)
        .await
        .context("server error")?;
    info!("server stopped");
    Ok(())
}

fn read_addr() -> Result<SocketAddr> {
    let raw = env::var("HTML_TO_IMAGE_SERVER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned());
    raw.parse().with_context(|| format!("invalid addr {raw}"))
}

fn read_fonts_dir() -> Result<PathBuf> {
    let raw = env::var("HTML_TO_IMAGE_FONTS_DIR").unwrap_or_else(|_| DEFAULT_FONTS_DIR.to_owned());
    validate_fonts_dir(Path::new(&raw))
}

fn read_max_body_size() -> usize {
    match env::var("HTML_TO_IMAGE_MAX_BODY") {
        Ok(value) => match value.trim().parse::<usize>() {
            Ok(mb) => mb.checked_mul(BYTES_PER_MEGABYTE).unwrap_or_else(|| {
                tracing::warn!(%value, "HTML_TO_IMAGE_MAX_BODY overflow, using default");
                DEFAULT_MAX_BODY_SIZE
            }),
            Err(err) => {
                tracing::warn!(%value, %err, "failed to parse HTML_TO_IMAGE_MAX_BODY (MiB), using default");
                DEFAULT_MAX_BODY_SIZE
            }
        },
        Err(_) => DEFAULT_MAX_BODY_SIZE,
    }
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(env_filter).init();
}

fn validate_fonts_dir(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("failed to read fonts dir {}", path.display()))?;
    let metadata = fs::metadata(&canonical)
        .with_context(|| format!("failed to stat fonts dir {}", canonical.display()))?;
    if !metadata.is_dir() {
        anyhow::bail!("fonts dir is not a directory: {}", canonical.display());
    }
    Ok(canonical)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            error!(%err, "failed to listen for Ctrl-C");
        }
        info!("received Ctrl-C, shutting down");
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
                info!("received SIGTERM, shutting down");
            }
            Err(err) => {
                error!(%err, "failed to install SIGTERM handler");
            }
        }
    };

    #[cfg(unix)]
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    };

    #[cfg(not(unix))]
    ctrl_c.await;
}
