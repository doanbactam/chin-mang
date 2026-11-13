// Shared drawing helpers for the sprite modules (built on pixel.mjs).
// Convention for creatures: parts are typed WITHOUT the outer outline; frames are composed from parts
// (head, body, legs, tail, accessories) re-posed per frame, then fin() adds the 1 px dark outline last.
import { Px } from '../pixel.mjs';

Px.strict = false; // role letters allowed while drawing; final pixels are linted by build-atlas.mjs

// Grid from rows, with an optional role map: role chars (any char) → palette chars. '.' stays transparent.
export function G(rows, map = {}) {
  const w = Math.max(...rows.map((r) => r.length));
  const p = new Px(w, rows.length);
  rows.forEach((r, y) => [...r.padEnd(w, '.')].forEach((ch, x) => { const c = map[ch] ?? ch; if (c !== '.' && c !== ' ') p.set(x, y, c); }));
  return p;
}
// Stamp src at (x,y). selout: draw a 1 px dark rim around src first (keeps overlapping parts readable).
export function put(dst, src, x = 0, y = 0, selout = false, rim = 'k') {
  if (selout) {
    const r = new Px(src.w + 2, src.h + 2).stamp(src, 1, 1).outline('k');
    for (let j = 0; j < r.h; j++) for (let i = 0; i < r.w; i++) if (r.d[j][i] === 'k') dst.set(x + i - 1, y + j - 1, rim);
  }
  return dst.stamp(src, x, y);
}
export const fin = (px, c = 'k') => px.outline(c);
// Creatures: clear the 1 px canvas border first (anything posed into it would lose its outline), then outline.
export const finUnit = (px, c = 'k') => {
  let n = 0; const cl = (x, y) => { if (px.d[y][x] !== '.') { n++; px.d[y][x] = '.'; } };
  for (let x = 0; x < px.w; x++) { cl(x, 0); cl(x, px.h - 1); }
  for (let y = 1; y < px.h - 1; y++) { cl(0, y); cl(px.w - 1, y); }
  const out = px.outline(c); out.clipped = n; return out; // build-atlas warns about clipped frames
};
export function dots(px, list) { for (const [x, y, c] of list) px.set(x, y, c); return px; }
export function recolor(px, map) { const o = px.clone(); for (const r of o.d) for (let x = 0; x < r.length; x++) if (map[r[x]] !== undefined) r[x] = map[r[x]]; return o; }
export function flipY(px) { const p = px.clone(); p.d.reverse(); return p; }
export function rotCW(px) { const p = new Px(px.h, px.w); for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) p.d[x][px.h - 1 - y] = px.d[y][x]; return p; }
export function rotCCW(px) { const p = new Px(px.h, px.w); for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) p.d[px.w - 1 - x][y] = px.d[y][x]; return p; }
export function bbox(px) {
  let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
  for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) if (px.d[y][x] !== '.') { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
  return { x0, y0, x1, y1, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}
// Move rows [y0,y1] by (dx,dy); rows left behind become transparent.
export function shiftRows(px, y0, y1, dx, dy) {
  const band = px.crop(0, y0, px.w, y1 - y0 + 1), out = px.clone();
  for (let y = y0; y <= y1; y++) for (let x = 0; x < px.w; x++) out.d[y][x] = '.';
  return out.stamp(band, dx, y0 + dy);
}
// Erase pixels of colour set `cs` (or all if null) in a rect.
export function clear(px, x, y, w, h) { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) if (px.in(x + i, y + j)) px.d[y + j][x + i] = '.'; return px; }
// Deterministic hash in [0,1).
export function hash(a, b = 0, c = 0) { let n = (a * 374761393 + b * 668265263 + c * 974711) | 0; n = (n ^ (n >>> 13)) * 1274126177; n = (n ^ (n >>> 16)) >>> 0; return n / 4294967296; }
// Seat content of px into a w×h canvas, its lowest pixel on row `bottom`, horizontally centred + dx.
export function seat(px, w, h, bottom, dx = 0) {
  const b = bbox(px); return new Px(w, h).stamp(px.crop(b.x0, b.y0, b.w, b.h), Math.floor((w - b.w) / 2) + dx, bottom - b.h + 1);
}
// Dither-free ring / disc helpers for fx.
export function ring(px, cx, cy, r0, r1, c) {
  for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) { const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy); if (d >= r0 && d < r1) px.set(x, y, c); }
  return px;
}
export function disc(px, cx, cy, r, c) { return ring(px, cx, cy, -1, r, c); }
// Pixels of colour `c` that touch transparency (4-neighbour) become `edge` — cheap rim lighting/shading.
export function rim(px, c, edge, dirs = [[0, -1], [-1, 0]]) {
  const src = px.clone();
  for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) if (src.d[y][x] === c && dirs.some(([i, j]) => src.get(x + i, y + j) === '.')) px.d[y][x] = edge;
  return px;
}
export const entries = (list, kind) => list.map(([name, px, extra = {}]) => ({ name, px, ...extra }));

// Shaded ellipse ("blob"): 3-tone volume lit from the top-left. ramp = [shadow, base, light] (+ optional deep shadow).
export function blob(px, cx, cy, rx, ry, ramp, { light = 0.5, dark = -0.45 } = {}) {
  const [lo, mid, hi, deep] = ramp;
  for (let y = Math.floor(cy - ry - 1); y <= Math.ceil(cy + ry + 1); y++)
    for (let x = Math.floor(cx - rx - 1); x <= Math.ceil(cx + rx + 1); x++) {
      const nx = (x + 0.5 - cx) / rx, ny = (y + 0.5 - cy) / ry, r = nx * nx + ny * ny;
      if (r > 1) continue;
      const l = -0.6 * nx - 0.8 * ny;
      let c = mid;
      if (l > light && r > 0.3) c = hi;
      else if (l < dark && r > 0.45) c = deep && l < dark - 0.4 && r > 0.75 ? deep : lo;
      px.set(x, y, c);
    }
  return px;
}
// Thick polyline through points, colour per segment index function.
export function tube(px, pts, col, thick = 1) {
  for (let i = 0; i < pts.length - 1; i++) {
    const [x0, y0] = pts[i], [x1, y1] = pts[i + 1], n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0), 1);
    for (let k = 0; k <= n; k++) {
      const x = Math.round(x0 + ((x1 - x0) * k) / n), y = Math.round(y0 + ((y1 - y0) * k) / n);
      for (let j = 0; j < thick; j++) for (let ii = 0; ii < thick; ii++) px.set(x + ii, y + j, typeof col === 'function' ? col(i, k, j) : col);
    }
  }
  return px;
}
