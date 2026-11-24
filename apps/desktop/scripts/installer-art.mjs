// The installers' artwork, derived from the official logo (branding/logo.png):
//
//   pnpm installer-art
//
// Written to src-tauri/installer/ and referenced from tauri.conf.json:
// - installer.ico: the Windows installer's icon (the logo with a download
//   badge), so the setup file isn't mistaken for the installed app;
// - nsis-sidebar.bmp (164x314) and nsis-header.bmp (150x57): the NSIS
//   welcome/finish pages and the header of the other pages (left-aligned);
// - wix-dialog.bmp (493x312) and wix-banner.bmp (493x58): the MSI welcome/
//   finish dialogs (their text sits on the white area right of x=180) and
//   the banner of the other dialogs (title on the left, so the logo goes right);
// - dmg-background.png (660x400): the macOS disk image window, with an
//   arrow from the app to the Applications folder (positions in tauri.conf).
//
// No text on purpose: the installers are the same for every language, and
// rendering text here would depend on the fonts of the machine running it.
// Linux packages (.deb, .rpm, .AppImage) have no installer window; they use
// the app icon. The result is committed; rerun after changing the logo.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";
import { square } from "./lib/logo.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src-tauri/installer");
const source = join(root, "branding/logo.png");

// The site's palette (apps/docs/app/globals.css, sRGB of the oklch tokens).
const color = {
  paper: "#f9f6f2",
  ink: "#241e1a",
  ember: "#d55c13",
  night: "#13100e",
  nightCard: "#1a1614",
};

const logo = (master, size) => sharp(master).resize(size, size, { kernel: "lanczos3" }).png().toBuffer();

/** A canvas from an SVG background, with the logo composited on top. */
async function canvas(width, height, svg, logos = []) {
  const layers = await Promise.all(
    logos.map(async ({ master, size, left, top }) => ({ input: await logo(master, size), left, top })),
  );
  return sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">${svg}</svg>`))
    .composite(layers)
    .png()
    .toBuffer();
}

/** A 24-bit uncompressed BMP, the format NSIS and WiX both read. */
async function bmp(png, background = color.paper) {
  const { data, info } = await sharp(png)
    .flatten({ background })
    .removeAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  const stride = Math.ceil((info.width * 3) / 4) * 4;
  const pixels = Buffer.alloc(stride * info.height);
  for (let y = 0; y < info.height; y++) {
    // Rows go bottom-up, pixels as BGR.
    const row = (info.height - 1 - y) * stride;
    for (let x = 0; x < info.width; x++) {
      const at = (y * info.width + x) * 3;
      pixels[row + x * 3] = data[at + 2];
      pixels[row + x * 3 + 1] = data[at + 1];
      pixels[row + x * 3 + 2] = data[at];
    }
  }
  const header = Buffer.alloc(54);
  header.write("BM", 0, "ascii");
  header.writeUInt32LE(54 + pixels.length, 2);
  header.writeUInt32LE(54, 10);
  header.writeUInt32LE(40, 14);
  header.writeInt32LE(info.width, 18);
  header.writeInt32LE(info.height, 22);
  header.writeUInt16LE(1, 26);
  header.writeUInt16LE(24, 28);
  header.writeUInt32LE(pixels.length, 34);
  header.writeInt32LE(2835, 38); // 72 dpi
  header.writeInt32LE(2835, 42);
  return Buffer.concat([header, pixels]);
}

/** A .ico holding PNG images (Vista+ format), one per size. */
async function ico(master, sizes) {
  const images = await Promise.all(
    sizes.map((s) => sharp(master).resize(s, s, { kernel: "lanczos3" }).png().toBuffer()),
  );
  const header = Buffer.alloc(6 + 16 * images.length);
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

/** The dark panel with the logo: the NSIS sidebar and the left of the WiX dialog. */
function panel(master, width, height) {
  const size = Math.round(width * 0.54);
  const cy = Math.round(height * 0.4);
  return canvas(
    width,
    height,
    `<defs>
      <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="${color.nightCard}"/><stop offset="1" stop-color="${color.night}"/>
      </linearGradient>
      <radialGradient id="glow" cx="${width / 2}" cy="${cy}" r="${width * 0.95}" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="${color.ember}" stop-opacity="0.42"/>
        <stop offset="0.55" stop-color="${color.ember}" stop-opacity="0.08"/>
        <stop offset="1" stop-color="${color.ember}" stop-opacity="0"/>
      </radialGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#bg)"/>
    <rect width="100%" height="100%" fill="url(#glow)"/>
    <rect y="${height - 3}" width="100%" height="3" fill="${color.ember}" opacity="0.85"/>`,
    [{ master, size, left: Math.round((width - size) / 2), top: cy - Math.round(size / 2) }],
  );
}

/** The logo with a download badge in the corner. */
async function installerIcon(master) {
  const s = 1024;
  const r = s * 0.17;
  const [cx, cy] = [s - r - s * 0.02, s - r - s * 0.02];
  const badge = `<svg xmlns="http://www.w3.org/2000/svg" width="${s}" height="${s}">
    <circle cx="${cx}" cy="${cy}" r="${r}" fill="${color.ink}" stroke="#fff" stroke-width="${s * 0.035}"/>
    <g fill="none" stroke="#fff" stroke-width="${s * 0.05}" stroke-linecap="round" stroke-linejoin="round">
      <path d="M${cx} ${cy - r * 0.5} V${cy + r * 0.38}"/>
      <path d="M${cx - r * 0.42} ${cy - r * 0.02} L${cx} ${cy + r * 0.4} L${cx + r * 0.42} ${cy - r * 0.02}"/>
    </g>
  </svg>`;
  return sharp(await logo(master, s))
    .composite([{ input: Buffer.from(badge) }])
    .png()
    .toBuffer();
}

mkdirSync(out, { recursive: true });
const master = await square(source);

writeFileSync(join(out, "installer.ico"), await ico(await installerIcon(master), [16, 24, 32, 48, 64, 128, 256]));

writeFileSync(join(out, "nsis-sidebar.bmp"), await bmp(await panel(master, 164, 314)));
writeFileSync(
  join(out, "nsis-header.bmp"),
  await bmp(
    await canvas(150, 57, `<rect width="100%" height="100%" fill="#fff"/>`, [{ master, size: 41, left: 12, top: 8 }]),
    "#fff",
  ),
);

const side = await panel(master, 164, 312);
const dialog = await sharp({ create: { width: 493, height: 312, channels: 4, background: "#fff" } })
  .composite([{ input: side, left: 0, top: 0 }])
  .png()
  .toBuffer();
writeFileSync(join(out, "wix-dialog.bmp"), await bmp(dialog, "#fff"));
writeFileSync(
  join(out, "wix-banner.bmp"),
  await bmp(
    await canvas(493, 58, `<rect width="100%" height="100%" fill="#fff"/>`, [
      { master, size: 42, left: 493 - 42 - 12, top: 8 },
    ]),
    "#fff",
  ),
);

// Icons sit at (180, 190) and (480, 190), as in tauri.conf.json (bundle.macOS.dmg).
const dmg = await canvas(
  660,
  400,
  `<defs>
    <radialGradient id="a" cx="0" cy="0" r="520" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${color.ember}" stop-opacity="0.13"/><stop offset="1" stop-color="${color.ember}" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="b" cx="660" cy="400" r="460" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${color.ember}" stop-opacity="0.07"/><stop offset="1" stop-color="${color.ember}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="100%" height="100%" fill="${color.paper}"/>
  <rect width="100%" height="100%" fill="url(#a)"/>
  <rect width="100%" height="100%" fill="url(#b)"/>
  <g fill="none" stroke="${color.ember}" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" opacity="0.8">
    <path d="M286 190 H372" stroke-dasharray="0.1 12"/>
    <path d="M362 176 L378 190 L362 204"/>
  </g>`,
);
writeFileSync(join(out, "dmg-background.png"), dmg);

console.log("installer artwork written");
