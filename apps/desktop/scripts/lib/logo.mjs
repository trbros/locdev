// The official logo (branding/logo.png) as a square master image, shared by
// the scripts that derive icons and installer artwork from it.

import sharp from "sharp";

/** The logo cropped to what is visible, centered on a square with a small margin. */
export async function square(source) {
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
