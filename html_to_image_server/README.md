# html-to-image-server

Poem-based HTTP server that renders MiniJinja HTML templates to PNG bytes using the `html_to_image` library (Blitz + Vello CPU renderer). It serves a JSON OpenAPI spec and Swagger UI for easy exploration.

## Run

```bash
cargo run -p html-to-image-server
```

Configuration is environment-first (loaded via `.env` with [`dotenvy`](https://crates.io/crates/dotenvy)):

- `HTML_TO_IMAGE_SERVER_ADDR` (default `0.0.0.0:3000`)
- `HTML_TO_IMAGE_MAX_BODY` (default `1`, MiB)
- `HTML_TO_IMAGE_FONTS_DIR` (default `assets/fonts`; must resolve within this directory)

Example `.env`:

```env
HTML_TO_IMAGE_SERVER_ADDR=127.0.0.1:3000
HTML_TO_IMAGE_MAX_BODY=1
HTML_TO_IMAGE_FONTS_DIR=assets/fonts
```

## REST API

- `GET /healthz` → `ok`
- `POST /render/png` → `image/png` bytes
- `GET /spec` and `GET /api/spec` → OpenAPI JSON
- `GET /swagger` → Swagger UI

Example request (writes `card.png`):

```bash
curl -X POST http://127.0.0.1:3000/render/png \
  -H "Content-Type: application/json" \
  -o card.png \
  -d '{
    "html": "<!doctype html><html><head><meta charset=\"utf-8\"/><style>:root{--w:{{ width }}px;--h:{{ height }}px}*{box-sizing:border-box}html,body{margin:0;width:var(--w);height:var(--h);overflow:hidden;background:transparent;font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Arial;color:#eef2ff}.card{width:var(--w);height:var(--h);padding:18px;border-radius:18px;display:flex;flex-direction:column;justify-content:center;gap:8px;position:relative;overflow:hidden;background:radial-gradient(110% 90% at 15% 20%,rgba(255,255,255,.22),transparent 55%),radial-gradient(90% 70% at 85% 30%,rgba(255,255,255,.14),transparent 60%),linear-gradient(135deg,#7c3aed 0%,#2563eb 45%,#06b6d4 100%);border:1px solid rgba(255,255,255,.18);box-shadow:0 18px 45px rgba(0,0,0,.35)}h1{margin:0;font-size:26px;line-height:1.1;font-weight:850;letter-spacing:-.02em;text-shadow:0 2px 12px rgba(0,0,0,.25)}p{margin:0;font-size:13px;line-height:1.35;opacity:.92}</style></head><body><div class=\"card\"><h1>{{ title }}</h1><p>{{ body }} {{ width }}</p></div></body></html>",
    "width": 420,
    "height": 200,
    "scale": 1.0,
    "animation_time": 5.0,
    "data": { "title": "Hello", "body": "Rendered via HTTP" }
  }'


```

### More examples

- Render with bundled font allowlist (defaults to `assets/fonts`):

```bash
curl -X POST http://127.0.0.1:3000/render/png \
  -H "content-type: application/json" \
  -d '{
    "html": "<style>*{font-family:FiraSans-Regular;}</style><h2>{{ title }}</h2>",
    "width": 320,
    "height": 120,
    "font_paths": ["FiraSans-Regular.ttf"],
    "data": { "title": "Font test" }
  }' \
  -o font-test.png
```

- Serve locally with a custom port and body limit:

```bash
cat > .env <<'EOF'
HTML_TO_IMAGE_SERVER_ADDR=127.0.0.1:4000
HTML_TO_IMAGE_MAX_BODY=2097152
EOF
cargo run -p html-to-image-server
```

The request body is validated for size, dimensions (defaults: max 4096x4096), and scale range `(0, 8]`. Rendering work is offloaded to `spawn_blocking` to keep the async runtime responsive.
