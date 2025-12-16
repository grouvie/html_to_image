# html-to-image-cli

CLI wrapper around the `html_to_image` library. It renders MiniJinja HTML templates to PNG files using Blitz + Vello (CPU-only).

## Usage

Render the bundled `templates/card.html` to `card.png` (directories are created automatically):

```bash
cargo run -p html-to-image-cli -- \
  --template templates/card.html \
  --out card.png \
  --name "Your Name" \
  --width 420 \
  --height 155 \
  --scale 1.0 \
  --animation-time 5.0
```

Common flags:

- `--font-path assets/fonts/FiraSans-Regular.ttf` to load additional fonts (repeatable or comma-separated).
- `--icon "🚀"` or `--message "Custom text"` to override template content.
- `--seed 42` for deterministic icon/message selection.
- `--scale` and `--animation-time` to tweak render output.

The CLI accepts any MiniJinja template and arbitrary serializable data; see `src/main.rs` for the data structure passed to the template.
