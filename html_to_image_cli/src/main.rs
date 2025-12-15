use std::{
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::Parser;
use rand::{SeedableRng, rngs::StdRng, seq::IndexedRandom};
use serde::Serialize;

use html_to_image::{DEFAULT_ANIMATION_TIME, DEFAULT_SCALE, render_to_png};

#[derive(Debug, Clone, Serialize)]
struct CardData {
    user: String,
    icon: String,
    message: String,
    width: u32,
    height: u32,
}

#[derive(Parser, Debug)]
#[command(
    name = "html-to-image",
    version,
    about = "Render an HTML template to a PNG using Blitz + anyrender_vello_cpu (no browser, CPU-only)."
)]
struct Cli {
    /// Path to the HTML template (`MiniJinja` syntax)
    #[arg(short, long, default_value = "templates/card.html")]
    template: PathBuf,

    /// Output PNG file path (directories will be created)
    #[arg(short, long, default_value = "card.png")]
    out: PathBuf,

    /// Name to render into the greeting
    #[arg(short, long, default_value = "User")]
    name: String,

    /// Fixed output width in pixels
    #[arg(long, default_value_t = 420)]
    width: u32,

    /// Fixed output height in pixels
    #[arg(long, default_value_t = 155)]
    height: u32,

    /// Scale factor used by the painter (1.0 is normal)
    #[arg(long, default_value_t = DEFAULT_SCALE)]
    scale: f64,

    /// Virtual time passed to Blitz layout for animations (seconds); tweak if your template animates
    #[arg(long, default_value_t = DEFAULT_ANIMATION_TIME)]
    animation_time: f64,

    /// Additional font files to load (repeatable, or comma-separated)
    #[arg(
        long = "font-path",
        value_name = "PATH",
        num_args = 1..,
        value_delimiter = ','
    )]
    font_paths: Vec<PathBuf>,

    /// Override the random icon (e.g. "★", "🚀")
    #[arg(long)]
    icon: Option<String>,

    /// Override the random message
    #[arg(long)]
    message: Option<String>,

    /// Seed for deterministic random icon/message selection
    #[arg(long)]
    seed: Option<u64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut rng = match cli.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_os_rng(),
    };

    let icon = cli.icon.unwrap_or_else(|| pick_icon(&mut rng).to_owned());
    let message = cli
        .message
        .unwrap_or_else(|| pick_message(&mut rng).to_owned());

    let data = CardData {
        user: cli.name,
        icon,
        message,
        width: cli.width,
        height: cli.height,
    };

    render_to_png(
        &cli.template,
        &data,
        &cli.out,
        cli.width,
        cli.height,
        cli.scale,
        cli.animation_time,
        &cli.font_paths,
    )
    .with_context(|| {
        format!(
            "render failed (template={}, out={})",
            cli.template.display(),
            cli.out.display()
        )
    })?;

    writeln!(io::stdout(), "Wrote {}", cli.out.display())?;
    Ok(())
}

fn pick_icon(rng: &mut StdRng) -> &'static str {
    const ICONS: &[&str] = &[
        "★", "✨", "🚀", "🎉", "✅", "💎", "🌙", "☕", "⚡", "🔔", "🧠",
    ];
    ICONS.choose(rng).copied().unwrap_or("★")
}

fn pick_message(rng: &mut StdRng) -> &'static str {
    const MESSAGES: &[&str] = &[
        "Your shiny Discord-sized card is ready. Crisp, compact, and screenshot-friendly.",
        "New render dropped: clean edges, smooth gradients, zero browser drama.",
        "Everything compiled. Nothing exploded. This is your sign to ship it. ✅",
        "A small card with big energy. Have a great one. ✨",
        "Pixels are aligned and vibes are immaculate.",
    ];
    MESSAGES
        .choose(rng)
        .copied()
        .unwrap_or("Your shiny Discord-sized card is ready.")
}
