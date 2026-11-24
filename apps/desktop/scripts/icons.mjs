// Derives every icon from the one source, branding/logo.png (the official
// logo, used as it is: only cropped, squared and resized):
//
//   pnpm icons
//
// - src-tauri/icons/*: the app icon for each platform (`tauri icon`);
// - src-tauri/icons/tray-*.png: the tray icon, lit and dimmed, plus a
//   one-color template for the macOS menu bar (the logo's alpha in black);
// - public/logo.png (app and splash) and public/favicon.ico, and the same
//   two files for the docs site (apps/docs/public: logo.png, favicon.ico).
//
// The result is committed; rerun after changing the logo.

import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const icons = join(root, "src-tauri/icons");
const source = join(root, "branding/logo.png");

/** The logo cropped to what is visible, centered on a square with a small margin. */
async function square() {
  const { data, info } = await sharp(source).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  // The bounds of clearly visible pixels: the faint haze around the cut-out
  // must not decide where the logo sits.
  let [x0, y0, x1, y1] = [info.width, info.height, 0, 0];
  for (let y = 0; y < info.height; y++) {
    for (let x = 0; x < info.width; x++) {
      if (data[(y * info.width + x) * 4 + 3] > 64) {
        x0 = Math.min(x0, x);
        x1 = Math.max(x1, x);
        y0 = Math.min(y0, y);
        y1 = Math.max(y1, y);
      }
    }
  }
  const width = x1 - x0 + 1;
  const height = y1 - y0 + 1;
  const side = Math.round(Math.max(width, height) * 1.04);
  const cropped = await sharp(source).extract({ left: x0, top: y0, width, height }).png().toBuffer();
  return sharp({ create: { width: side, height: side, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite([{ input: cropped, left: Math.round((side - width) / 2), top: Math.round((side - height) / 2) }])
    .png()
    .toBuffer();
}

const resized = (master, size) => sharp(master).resize(size, size, { kernel: "lanczos3" }).png();

/** A .ico holding PNG images (Vista+ format), one per size. */
async function ico(master, sizes) {
  const images = await Promise.all(sizes.map((s) => resized(master, s).toBuffer()));
  const header = Buffer.alloc(6 + 16 * images.length);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = header.length;
  images.forEach((img, i) => {
    const at = 6 + 16 * i;
    header.writeUInt8(sizes[i] >= 256 ? 0 : sizes[i], at);
    header.writeUInt8(sizes[i] >= 256 ? 0 : sizes[i], at + 1);
    header.writeUInt16LE(1, at + 4);
    header.writeUInt16LE(32, at + 6);
    header.writeUInt32LE(img.length, at + 8);
    header.writeUInt32LE(offset, at + 12);
    offset += img.length;
  });
  return Buffer.concat([header, ...images]);
}

const tmp = mkdtempSync(join(tmpdir(), "locdev-icons-"));

try {
  const master = await square();
  await resized(master, 1024).toFile(join(tmp, "icon.png"));
  execFileSync("pnpm", ["exec", "tauri", "icon", join(tmp, "icon.png"), "-o", icons], { cwd: root, stdio: "inherit" });
  // Desktop only: drop the mobile sets `tauri icon` also writes.
  for (const dir of ["android", "ios"]) rmSync(join(icons, dir), { recursive: true, force: true });

  // Tray: 64px reads well on HiDPI panels and scales down cleanly.
  await resized(master, 64).toFile(join(icons, "tray-on.png"));
  await resized(master, 64).grayscale().linear(0.75, 10).toFile(join(icons, "tray-off.png"));
  // macOS menu bar template: black plus the logo's alpha, tinted by the system.
  const alpha = await resized(master, 44).extractChannel("alpha").toBuffer();
  await sharp({ create: { width: 44, height: 44, channels: 3, background: "#000" } })
    .joinChannel(alpha)
    .png()
    .toFile(join(icons, "tray-template.png"));

  const favicon = await ico(master, [16, 32, 48]);
  for (const dir of [join(root, "public"), join(root, "../docs/public")]) {
    await resized(master, 256).toFile(join(dir, "logo.png"));
    writeFileSync(join(dir, "favicon.ico"), favicon);
  }
  console.log("icons written");
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
