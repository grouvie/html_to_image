import path from "path";
import fs from "fs";
import { renderTemplateToPng } from "@grouvie/html-to-image";

const pkgRoot = path.dirname(
  require.resolve("@grouvie/html-to-image/package.json")
);

const outDir = path.join(process.cwd(), "out");
fs.mkdirSync(outDir, { recursive: true });

const templatePath = path.join(pkgRoot, "templates", "card.html");
const outPath = path.join(outDir, "card.png");

async function main() {
  await renderTemplateToPng({
    templatePath,
    outPath,
    width: 420,
    height: 155,
    data: {
      width: 420,
      height: 155,
      user: "TypeScript",
      message: "Rendered via Rust N-API",
      icon: "🚀",
    },
    scale: 1.0,
    // animationTime: 5.0,
    fontPaths: [
      path.join(pkgRoot, "assets", "fonts", "FiraSans-Regular.ttf"),
      path.join(pkgRoot, "assets", "fonts", "NotoEmoji-Regular.ttf"),
    ],
  });

  console.log("Wrote:", outPath);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
