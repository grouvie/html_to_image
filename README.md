# html_to_image

Render a MiniJinja-based HTML template to a PNG using [Blitz](https://github.com/dioxuslabs/blitz) and [`anyrender_vello_cpu`](https://crates.io/crates/anyrender_vello_cpu) - no browser required, CPU-only. The workspace ships a library (`html_to_image`) and a CLI (`html-to-image-cli`) so you can embed rendering in other projects or run it directly from the command line.

## Features

- CPU-only HTML → PNG rendering via [Blitz](https://github.com/dioxuslabs/blitz) + [Vello](https://github.com/linebender/vello) (no headless Chrome)
- [MiniJinja](https://crates.io/crates/minijinja) templating with safe HTML auto-escaping
- Deterministic card content with `--seed`; overridable icon/message
- Ships with a sample Discord-card-sized template you can tweak

## Workspace layout

- `html_to_image/`: library crate exposing rendering functions and defaults
- `html_to_image_cli/`: CLI crate (`html-to-image`) wrapping the library
- `templates/`: sample `card.html` MiniJinja template
- `card.png`: example output

## Key dependencies

- [Blitz](https://github.com/dioxuslabs/blitz)
- [anyrender_vello_cpu](https://crates.io/crates/anyrender_vello_cpu)
- [Vello](https://github.com/linebender/vello)
- [MiniJinja](https://crates.io/crates/minijinja) ([GitHub](https://github.com/mitsuhiko/minijinja))

## Prerequisites

- Rust (edition 2024-capable toolchain; stable 1.80+ recommended)

## Usage

Render a card with the CLI (output path and directories are created automatically):

```bash
cargo run -p html-to-image-cli -- \
  --template templates/card.html \
  --out card.png \
  --name "Your Name" \
  --width 420 \
  --height 155 \
  --scale 1.0 \
  --font-path assets/fonts/FiraSans-Regular.ttf \
  --animation-time 5.0
```

Optional flags:

- `--icon "🚀"` and `--message "Custom text"` to override template content
- `--seed 42` for deterministic icon/message selection
- `--scale` to adjust rendering scale (defaults to `1.0`)
- `--animation-time` to tweak the virtual time passed to the layout engine (useful if your template has animations)
- `--font-path` to load one or more additional font files (repeat or comma-separate)

## Template notes

- The sample template uses MiniJinja variables: `{{ user }}`, `{{ icon }}`, `{{ message }}`, `{{ width }}`, `{{ height }}` (used to size the viewport).
- The renderer escapes user-provided values with HTML auto-escaping.
- Adjust dimensions via CLI flags; width/height are passed into the template so the card fills the render area.
- The library accepts any serializable data structure; the CLI’s `CardData` is just a convenience layer.

## Library usage

```rust
use html_to_image::{render_to_png, DEFAULT_ANIMATION_TIME};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct Context {
    title: String,
    body: String,
}

fn render() -> html_to_image::Result<()> {
    let ctx = Context {
        title: "Hello".into(),
        body: "Dynamic payload".into(),
    };
    render_to_png(
        Path::new("templates/custom.html"),
        &ctx,
        Path::new("out.png"),
        420,
        155,
        1.0,
        DEFAULT_ANIMATION_TIME,
        &[],
    )
}
```

## Development

- Lint: `cargo clippy --workspace`
- Tests: `cargo test --workspace`
- Format: `cargo fmt --all`

## CI

A minimal GitHub Actions workflow in `.github/workflows/ci.yml` runs fmt, clippy, and tests on pushes and pull requests. It keeps the workspace buildable, but does not publish releases—add a separate release workflow if you want automated tagging/assets.

## License

Dual-licensed under either:

- MIT (`LICENSE-MIT`)
- Apache-2.0 (`LICENSE-APACHE`)

You may choose either license. Contributions are accepted under the same dual-licensing terms.
