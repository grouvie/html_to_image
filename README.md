# html_to_image

Render MiniJinja HTML to PNG using Blitz + Vello (CPU-only). This workspace ships a reusable library, a CLI, and a Poem-based HTTP server.

## Crates

- `html_to_image/`: library with rendering functions and defaults.
- `html_to_image_cli/`: CLI wrapper (`html-to-image`) – see `html_to_image_cli/README.md`.
- `html_to_image_server/`: Poem HTTP server – see `html_to_image_server/README.md`.
- `templates/`: sample `card.html`; `assets/fonts/` contains bundled fonts.

## Quick start

### Library

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
    render_to_png(
        Path::new("templates/custom.html"),
        &Context {
            title: "Hello".into(),
            body: "Dynamic payload".into(),
        },
        Path::new("out.png"),
        420,
        155,
        1.0,
        DEFAULT_ANIMATION_TIME,
        &[],
    )
}
```

### CLI

```bash
cargo run -p html-to-image-cli -- --template templates/card.html --out card.png --name "Your Name"
```

More flags and details: `html_to_image_cli/README.md`.

### Server

```bash
# Optional: .env with HTML_TO_IMAGE_SERVER_ADDR, HTML_TO_IMAGE_MAX_BODY (MiB), HTML_TO_IMAGE_FONTS_DIR
cargo run -p html-to-image-server &

curl -X POST http://127.0.0.1:3000/render/png \
  -H "content-type: application/json" \
  -d '{"html":"<h1>{{ title }}</h1>","width":320,"height":180,"data":{"title":"Hi"}}' \
  -o render.png
```

See `html_to_image_server/README.md` for full API details and Swagger UI.

## Features

- CPU-only HTML → PNG (no headless browser).
- MiniJinja templating with HTML auto-escaping.
- Optional custom fonts and render tuning (scale, animation time).

## Development

- Lint: `cargo clippy --workspace`
- Tests: `cargo test --workspace`
- Format: `cargo fmt --all`

## License

Dual-licensed under either MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`).
