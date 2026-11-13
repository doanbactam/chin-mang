// Dev preview: node tools/art/preview.mjs <group> [nameRegex] [scale] [out.png] [--gray|--black]
// Renders matching sprites of one group, one animation per row, on a checkerboard (alpha-blended).
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { PALETTE, hex, writePNG } from './pixel.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const [group, re = '.', sc = '8', out = '/tmp/preview.png'] = process.argv.slice(2).filter((a) => !a.startsWith('--'));
const mode = process.argv.includes('--gray') ? 'gray' : process.argv.includes('--black') ? 'black' : '';
const S = +sc, rx = new RegExp(re);
const list = (await (await import(pathToFileURL(join(ROOT, 'tools/art/sprites', `${group}.mjs`)).href)).default()).filter((e) => rx.test(e.name));
const rows = [];
for (const e of list) { const b = e.name.replace(/_\d+$/, ''); if (!rows.length || rows.at(-1).b !== b) rows.push({ b, items: [] }); rows.at(-1).items.push(e); }
const W = Math.max(...rows.map((r) => r.items.reduce((s, e) => s + e.px.w + 2, 2))), H = rows.reduce((s, r) => s + Math.max(...r.items.map((e) => e.px.h)) + 2, 2);
const buf = Buffer.alloc(W * S * H * S * 4);
for (let y = 0; y < H * S; y++) for (let x = 0; x < W * S; x++) buf.set(((x >> 3) + (y >> 3)) & 1 ? [90, 95, 120, 255] : [74, 79, 102, 255], (y * W * S + x) * 4);
let oy = 2;
for (const r of rows) {
  const rh = Math.max(...r.items.map((e) => e.px.h)); let ox = 2;
  for (const e of r.items) {
    for (let y = 0; y < e.px.h; y++) for (let x = 0; x < e.px.w; x++) {
      const c = e.px.d[y][x]; if (c === '.') continue;
      let [R, G, B, A] = hex(PALETTE[c]);
      if (mode === 'gray') R = G = B = Math.round(0.299 * R + 0.587 * G + 0.114 * B);
      if (mode === 'black') R = G = B = 10;
      for (let j = 0; j < S; j++) for (let i = 0; i < S; i++) {
        const o = (((oy + rh - e.px.h + y) * S + j) * W * S + (ox + x) * S + i) * 4, t = A / 255;
        buf.set([R * t + buf[o] * (1 - t), G * t + buf[o + 1] * (1 - t), B * t + buf[o + 2] * (1 - t), 255].map(Math.round), o);
      }
    }
    ox += e.px.w + 2;
  }
  oy += rh + 2;
}
writePNG(out, W * S, H * S, buf);
console.log(`${list.length} sprites → ${out}`);
