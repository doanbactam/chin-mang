// Atlas builder + art lint (SPEC §8.3, docs/CONTRACT.md).
//   node tools/art/build-atlas.mjs [--allow-missing]
// Loads every tools/art/sprites/<group>.mjs (default export → [{ name, px, ax?, ay? }]), checks it against
// manifest.mjs, packs one atlas (1 px padding) and writes:
//   assets/art/atlas.png, assets/art/atlas.json  { size:[w,h], frames:{ name:{x,y,w,h,ax,ay,slice?} } }
//   docs/art/contact-<group>.png  (×4, checkerboard, one animation per row)
//   docs/art/lineup.png           (4 cats + 8 enemies + 5 bosses on zone-1 ground, ×4)
//   docs/art/check/*.png          (self-review helpers: grayscale lineup, black silhouettes, ground seams)
// Exit code ≠ 0 on any lint error.
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { PALETTE, hex, writePNG } from './pixel.mjs';
import { MANIFEST, NAMES, GROUPS, CATS, ENEMIES, BOSSES } from './manifest.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const OUT = join(ROOT, 'assets/art'), DOCS = join(ROOT, 'docs/art');
const MAX = 1024, PAD = 1;
const RGBA = Object.fromEntries(Object.entries(PALETTE).map(([k, v]) => [k, hex(v)]));
const DARK = new Set(['k', 'n']);
const N4 = [[1, 0], [-1, 0], [0, 1], [0, -1]];

// Default anchors (CONTRACT): bottom-centre for unit/boss/drop/prop, centre for fx/proj, top-left otherwise.
// Always integers so no sprite lands on a half pixel (SPEC §5).
function defaultAnchor(kind, w, h) {
  if (['unit', 'boss', 'drop', 'prop'].includes(kind)) return [w >> 1, h];
  if (kind === 'fx' || kind === 'proj') return [w >> 1, h >> 1];
  return [0, 0];
}
const MIN_COLORS = { unit: 5, boss: 6, proj: 2, fx: 2, drop: 3, tile: 3, prop: 4, icon: 3, ui: 2, font: 2 };

// ── lint one sprite ──────────────────────────────────────────────────────────────
function lintSprite(m, px) {
  const errs = [], n = m.name;
  if (!px || !Array.isArray(px.d)) return [`${n}: not a Px`];
  if (px.w !== m.w || px.h !== m.h) errs.push(`${n}: size ${px.w}×${px.h}, manifest says ${m.w}×${m.h}`);
  if (px.d.length !== px.h || px.d.some((r) => r.length !== px.w)) errs.push(`${n}: ragged pixel rows`);
  const colors = new Set(), bad = new Set();
  let opaque = 0, edge = 0, edgeDark = 0;
  for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) {
    const c = px.d[y][x]; if (c === '.') continue;
    if (!RGBA[c]) { bad.add(c); continue; }
    opaque++; colors.add(c);
    if (N4.some(([i, j]) => px.get(x + i, y + j) === '.')) { edge++; if (DARK.has(c)) edgeDark++; }
  }
  if (bad.size) errs.push(`${n}: colours not in palette: ${[...bad].join(' ')}`);
  if (!opaque) return [...errs, `${n}: empty sprite`];
  if (colors.size < MIN_COLORS[m.kind]) errs.push(`${n}: only ${colors.size} colours (min ${MIN_COLORS[m.kind]} for ${m.kind}) — flat shape?`);
  if ((m.kind === 'unit' || m.kind === 'boss') && edgeDark < edge) errs.push(`${n}: ${edge - edgeDark} silhouette-edge pixel(s) not dark outline (k/n)`);
  if (m.slice !== undefined && (2 * m.slice >= px.w || 2 * m.slice >= px.h)) errs.push(`${n}: slice ${m.slice} leaves no centre in ${px.w}×${px.h}`);
  return errs;
}
const key = (px) => px.d.map((r) => r.join('')).join('|');

// ── packing: shelves, tallest first, try widths and keep the smallest square-ish result ──
function packShelves(items, width) {
  const rects = new Map(); let x = PAD, y = PAD, shelf = 0;
  for (const it of items) {
    if (x + it.w + PAD > width) { y += shelf + PAD; x = PAD; shelf = 0; }
    rects.set(it.name, { x, y }); x += it.w + PAD; shelf = Math.max(shelf, it.h);
  }
  return { W: width, H: y + shelf + PAD, rects };
}
function pack(items) {
  const order = [...items].sort((a, b) => b.h - a.h || b.w - a.w || (a.name < b.name ? -1 : 1));
  let best = null;
  for (let w = 128; w <= MAX; w += 32) {
    const r = packShelves(order, w); r.H = Math.ceil(r.H / 4) * 4;
    if (r.H > MAX) continue;
    const score = Math.max(r.W, r.H) * 4 + (r.W * r.H) / 1e4;
    if (!best || score < best.score) best = { ...r, score };
  }
  return best;
}

// ── raster helpers (RGBA buffers with alpha blending) ────────────────────────────
class Img {
  constructor(w, h, fill) { this.w = w; this.h = h; this.d = Buffer.alloc(w * h * 4); if (fill) for (let i = 0; i < w * h; i++) this.d.set(fill, i * 4); }
  blend(x, y, [r, g, b, a]) {
    if (x < 0 || y < 0 || x >= this.w || y >= this.h || !a) return;
    const o = (y * this.w + x) * 4, t = a / 255, s = 1 - t;
    if (this.d[o + 3] === 0) { this.d.set([r, g, b, a], o); return; }
    this.d.set([r * t + this.d[o] * s, g * t + this.d[o + 1] * s, b * t + this.d[o + 2] * s, Math.max(a, this.d[o + 3])].map(Math.round), o);
  }
  sprite(px, ox, oy, scale = 1, map = (c) => c) {
    for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) {
      const c = px.d[y][x]; if (c === '.') continue;
      const col = map(RGBA[c]);
      for (let j = 0; j < scale; j++) for (let i = 0; i < scale; i++) this.blend(ox * scale + x * scale + i, oy * scale + y * scale + j, col);
    }
  }
  save(path) { mkdirSync(dirname(path), { recursive: true }); writePNG(path, this.w, this.h, this.d); }
}
function checker(w, h, cell = 8) {
  const img = new Img(w, h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) img.d.set(((x / cell | 0) + (y / cell | 0)) & 1 ? [0x5a, 0x5f, 0x78, 255] : [0x4a, 0x4f, 0x66, 255], (y * w + x) * 4);
  return img;
}
const gray = ([r, g, b, a]) => { const l = Math.round(0.299 * r + 0.587 * g + 0.114 * b); return [l, l, l, a]; };
const black = ([, , , a]) => [16, 16, 16, a];

// Contact sheet: one animation (or one base name) per row, rows wrap at maxW logical px.
function contactSheet(path, entries, scale = 4, maxW = 300) {
  const rows = []; let cur = null;
  for (const e of entries) {
    const base = e.anim ?? e.name.replace(/_[^_]+$/, '');
    if (!cur || cur.base !== base || cur.w + e.px.w + 3 > maxW) { cur = { base, items: [], w: 3, h: 0 }; rows.push(cur); }
    cur.items.push(e); cur.w += e.px.w + 3; cur.h = Math.max(cur.h, e.px.h);
  }
  // pack short rows side by side
  const lines = []; let line = null;
  for (const r of rows) {
    if (!line || line.w + r.w > maxW) { line = { rows: [], w: 0, h: 0 }; lines.push(line); }
    line.rows.push(r); line.w += r.w + 4; line.h = Math.max(line.h, r.h);
  }
  const W = Math.max(...lines.map((l) => l.w)) + 3, H = lines.reduce((s, l) => s + l.h + 4, 4);
  const img = checker(W * scale, H * scale);
  let y = 3;
  for (const l of lines) {
    let x = 3;
    for (const r of l.rows) { for (const e of r.items) { img.sprite(e.px, x, y + l.h - e.px.h, scale); x += e.px.w + 3; } x += 4; }
    y += l.h + 4;
  }
  img.save(path);
}

async function loadGroup(g) {
  try {
    const mod = await import(pathToFileURL(join(ROOT, 'tools/art/sprites', `${g}.mjs`)).href);
    return { entries: await mod.default() };
  } catch (err) {
    if (err?.code === 'ERR_MODULE_NOT_FOUND' && String(err.message).includes(`${g}.mjs`)) return { absent: true };
    return { error: `sprites/${g}.mjs: ${err?.stack?.split('\n').slice(0, 4).join(' | ') ?? err}` };
  }
}

export async function build({ allowMissing = false } = {}) {
  const errors = [], warnings = [], clipped = [], byName = new Map(), byMan = new Map(MANIFEST.map((m) => [m.name, m]));
  for (const g of GROUPS) {
    const r = await loadGroup(g);
    if (r.error) { errors.push(r.error); continue; }
    if (r.absent) { (allowMissing ? warnings : errors).push(`sprites/${g}.mjs missing`); continue; }
    for (const e of r.entries) {
      const m = byMan.get(e.name);
      if (!m) { errors.push(`${g}: "${e.name}" is not in manifest.mjs`); continue; }
      if (m.group !== g) { errors.push(`${g}: "${e.name}" belongs to group ${m.group}`); continue; }
      if (byName.has(e.name)) { errors.push(`${g}: duplicate name ${e.name}`); continue; }
      errors.push(...lintSprite(m, e.px).map((s) => `lint ${s}`));
      if (e.px.clipped) clipped.push(`${e.name}:${e.px.clipped}`);
      byName.set(e.name, { ...m, ...e });
    }
  }
  if (clipped.length) warnings.push(`${clipped.length} creature frame(s) had pose tips trimmed by the 1 px outline border (max ${Math.max(...clipped.map((c) => +c.split(':')[1]))} px)` + (process.argv.includes('--verbose') ? `: ${clipped.join(' ')}` : ' — --verbose to list'));
  const missing = MANIFEST.filter((m) => !byName.has(m.name)).map((m) => m.name);
  if (missing.length) (allowMissing ? warnings : errors).push(`${missing.length} manifest sprite(s) missing: ${missing.slice(0, 12).join(', ')}${missing.length > 12 ? ' …' : ''}`);
  // identical frames inside one animation
  const anims = new Map();
  for (const e of byName.values()) if (e.anim) anims.set(e.anim, [...(anims.get(e.anim) ?? []), e]);
  for (const [a, list] of anims) {
    const seen = new Map();
    for (const e of list) { const k = key(e.px); if (seen.has(k)) errors.push(`lint ${e.name}: identical to ${seen.get(k)} in animation ${a}`); else seen.set(k, e.name); }
  }

  const all = MANIFEST.filter((m) => byName.has(m.name)).map((m) => byName.get(m.name));
  const packed = pack(all.map((e) => ({ name: e.name, w: e.px.w, h: e.px.h })));
  if (!packed) { errors.push(`atlas does not fit ${MAX}²`); return { errors, warnings }; }
  const atlas = new Img(packed.W, packed.H), frames = {};
  for (const e of all) {
    const { x, y } = packed.rects.get(e.name);
    atlas.sprite(e.px, x, y);
    const [dax, day] = defaultAnchor(e.kind, e.px.w, e.px.h);
    frames[e.name] = { x, y, w: e.px.w, h: e.px.h, ax: e.ax ?? dax, ay: e.ay ?? day, ...(e.slice !== undefined ? { slice: e.slice } : {}) };
  }
  mkdirSync(OUT, { recursive: true });
  atlas.save(join(OUT, 'atlas.png'));
  writeFileSync(join(OUT, 'atlas.json'), JSON.stringify({ size: [packed.W, packed.H], frames }).replace(/},"/g, '},\n"') + '\n');

  // contact sheets
  for (const g of GROUPS) {
    const list = all.filter((e) => e.group === g); if (!list.length) continue;
    contactSheet(join(DOCS, `contact-${g}.png`), list, 4, g === 'bosses' ? 180 : g === 'world' ? 200 : 160);
  }
  lineup(byName, warnings);
  return { errors, warnings, size: [packed.W, packed.H], count: all.length, missing };
}

// Lineup: zone-1 ground strip with the whole cast standing on it (cats facing right, foes facing left).
function lineup(byName, warnings) {
  const need = [...CATS.map((c) => `cat_${c}_idle_0`), ...ENEMIES.map((e) => `en_${e}_walk_0`), ...BOSSES.map((b) => `boss_${b}_walk_0`)];
  if (need.some((n) => !byName.has(n)) || !byName.has('ground_z1_0')) { warnings.push('lineup skipped (sprites missing)'); return; }
  const W = 4 * 18 + 8 * 18 + 5 * 34 + 24, H = 48, ground = 6, S = 4;
  const draw = (path, map, opts = {}) => {
    const img = opts.checker ? checker(W * S, H * S) : new Img(W * S, H * S, [0xc8, 0xd6, 0xe4, 255]);
    for (let x = 0, i = 0; x < W; x += 16, i++) img.sprite(byName.get(`ground_z1_${(i * 7 + (i >> 1)) % 4}`).px, x, H - ground, S, map);
    let x = 6;
    const foot = H - ground + 1; // feet sink 1 px into the top lip of the ground strip
    const shadow = (n, cx) => { const s = byName.get(n); if (s) img.sprite(s.px, cx - (s.px.w >> 1), foot - (s.px.h >> 1) - 1, S, map); };
    for (const n of need) {
      const e = byName.get(n), big = n.startsWith('boss_');
      if (!opts.silhouette) shadow(big ? 'shadow_l' : 'shadow_s', x + (e.px.w >> 1));
      const lift = n === 'en_crow_walk_0' ? 10 : 0;
      img.sprite(e.px, x, foot - e.px.h - lift, S, opts.silhouette ? black : map);
      x += e.px.w + (big ? 2 : 2) + (n === `cat_mun_idle_0` ? 8 : 0);
    }
    img.save(path);
  };
  draw(join(DOCS, 'lineup.png'), (c) => c);
  draw(join(DOCS, 'check/lineup-gray.png'), gray);
  draw(join(DOCS, 'check/lineup-silhouette.png'), (c) => c, { silhouette: true, checker: true });
  // every zone's ground variants in a pseudo-random order to catch seams
  const zs = new Img(16 * 20 * S, 5 * 10 * S, [0xc8, 0xd6, 0xe4, 255]);
  for (let z = 1; z <= 5; z++) for (let i = 0; i < 20; i++) {
    const g = byName.get(`ground_z${z}_${(i * 5 + (i * i >> 2)) % 4}`); if (g) zs.sprite(g.px, i * 16, z * 10 - 7, S);
  }
  zs.save(join(DOCS, 'check/ground-seams.png'));
  plateCheck(byName);
}

// ui_plate legibility: plate 9-sliced (centre tiled) to 40×14 with an icon + digits, and contact shadows,
// over white, black and a busy patch (deco + ground). ×4.
function plateCheck(byName) {
  const get = (n) => byName.get(n)?.px; const plate = get('ui_plate'); if (!plate) return;
  const S = 4, PW = 40, PH = 14, cell = 56, img = new Img(cell * 3 * S, 30 * S, [255, 255, 255, 255]);
  const bgs = [[255, 255, 255, 255], [0, 0, 0, 255], null];
  bgs.forEach((bg, k) => {
    const ox = k * cell;
    for (let y = 0; y < 30; y++) for (let x = 0; x < cell; x++) for (let j = 0; j < S; j++) for (let i = 0; i < S; i++) {
      const o = (((y * S + j) * img.w) + (ox + x) * S + i) * 4;
      img.d.set(bg ?? [0xc8, 0xd6, 0xe4, 255], o);
    }
    if (!bg) {
      for (let x = 0; x < cell; x += 16) img.sprite(get('ground_z4_' + ((x >> 4) % 4)), ox + x, 24, S);
      img.sprite(get('deco_z4_1'), ox + 2, 2, S); img.sprite(get('deco_z3_0'), ox + 22, 4, S); img.sprite(get('deco_z5_1'), ox + 38, 10, S);
    }
    // 9-slice with tiled centre
    const sl = 4, px0 = ox + 8, py0 = 4;
    for (let y = 0; y < PH; y++) for (let x = 0; x < PW; x++) {
      const sx = x < sl ? x : x >= PW - sl ? plate.w - (PW - x) : sl + ((x - sl) % (plate.w - 2 * sl));
      const sy = y < sl ? y : y >= PH - sl ? plate.h - (PH - y) : sl + ((y - sl) % (plate.h - 2 * sl));
      const c = plate.d[sy][sx]; if (c === '.') continue;
      for (let j = 0; j < S; j++) for (let i = 0; i < S; i++) img.blend((px0 + x) * S + i, (py0 + y) * S + j, RGBA[c]);
    }
    img.sprite(get('ic_fish'), px0 + 2, py0 + 1, S);
    ['1', 'dot', '2', '3', 'K'].forEach((g, i) => img.sprite(get('fd_' + g), px0 + 16 + i * 4, py0 + 4, S));
    img.sprite(get('shadow_l'), ox + 10, 25, S); img.sprite(get('shadow_s'), ox + 36, 26, S);
  });
  img.save(join(DOCS, 'check/plate.png'));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const r = await build({ allowMissing: process.argv.includes('--allow-missing') });
  for (const w of r.warnings) console.warn(`warn: ${w}`);
  for (const e of r.errors) console.error(`error: ${e}`);
  if (r.size) console.log(`atlas ${r.size[0]}×${r.size[1]}, ${r.count}/${MANIFEST.length} sprites`);
  console.log(r.errors.length ? `atlas: FAILED (${r.errors.length} error(s))` : 'atlas: OK');
  process.exit(r.errors.length ? 1 : 0);
}
export { NAMES };
