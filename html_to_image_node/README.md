# html_to_image_node

Node.js bindings (via Node-API / N-API) for the Rust `html_to_image` library.

This crate is built and shipped to JavaScript consumers through the npm package in this repo:

- `@grouvie/html-to-image` (root `package.json`)

That npm package includes:

- `index.node` (native addon)
- `index.d.ts` (generated TypeScript types)
- `templates/` (sample HTML templates)
- `assets/` (bundled fonts and other assets)

The Rust side lives here (`html_to_image_node`), while the JS entrypoint and build pipeline live at the repo root.

---

## Build flow (how it works)

From the repo root, npm runs a `prepare` script:

1. `npm run build`
2. `napi build --release -p html_to_image_node -o . --dts index.d.ts`

That produces `index.node` and `index.d.ts` in the repo root, which is what the npm package exports.

---

## Build from repo root

```bash
# From repo root
npm install

# Or explicitly run the native build:
npm run build
```

Notes:

- `npm install` runs `prepare`, which triggers the native build.
- The native build requires a Rust toolchain on the machine.

---

## Run the local example

```bash
# From repo root
npm install

cd examples/ts-local
npm install
npm run start
```

This runs `examples/ts-local/src/index.ts`, which renders `templates/card.html` to `examples/ts-local/out/card.png`.

---

## Use from another TypeScript project (GitHub install)

The npm package is marked `"private": true` to avoid accidental publishing, but you can install it directly from GitHub.

```bash
# install from GitHub (default branch)
npm install github:grouvie/html_to_image

# pin to a tag or commit for reproducibility
npm install github:grouvie/html_to_image#<tag-or-commit-sha>
```

Install triggers `prepare`, which compiles the native addon on the consumer machine. That machine must have Rust installed.

---

## API overview

The addon exposes a single async function:

```ts
import { renderTemplateToPng } from "@grouvie/html-to-image";

await renderTemplateToPng({
  templatePath: "/absolute/path/to/templates/card.html",
  outPath: "/absolute/path/to/out/card.png",
  width: 420,
  height: 155,
  data: {
    user: "TypeScript",
    message: "Rendered via Rust N-API",
    icon: "🚀",
    width: 420,
    height: 155,
  },
  scale: 1.0,
  animationTime: 5.0,
  fontPaths: [
    "/absolute/path/to/assets/fonts/FiraSans-Regular.ttf",
    "/absolute/path/to/assets/fonts/NotoEmoji-Regular.ttf",
  ],
});
```

The `RenderRequest` fields (from `index.d.ts`) are:

- `templatePath` (string, required): path to a MiniJinja HTML template file.
- `outPath` (string, required): where to write the PNG.
- `width` / `height` (number, required): output image dimensions.
- `data` (any, required): JSON data passed into the template.
- `scale` (number, optional): renderer scale factor (default 1.0).
- `animationTime` (number, optional): virtual time for animations (Rust: `animation_time`).
- `fontPaths` (string[], optional): extra font files to load.

High-level flow:

1. Load the template from disk.
2. Render it with `data` (MiniJinja).
3. Rasterize to a PNG using the CPU-only renderer.
4. Write the PNG to `outPath`.

---

## Emoji troubleshooting (important)

If emoji render in the Rust CLI but not through the Node addon:

1. Pass the emoji font explicitly via `fontPaths` (for example `assets/fonts/NotoEmoji-Regular.ttf`).
2. Ensure your CSS actually selects that font where emoji appear.

Example update to `templates/card.html`:

```html
<style>
  .badge {
    font-family: "Noto Emoji", "Fira Sans", sans-serif;
  }
</style>
```

Without both of these, emoji can render as missing glyphs even if letters (A, X, etc.) work.

---

## Notes

- The Node addon is a thin wrapper around the same Rust renderer used by the CLI and server crates.
- The template and assets folders shipped in the npm package mirror the paths referenced in the root README and CLI docs.
