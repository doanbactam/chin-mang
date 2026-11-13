// Projectiles, combat effects and contact shadows (SPEC §8.3). Anchor: centre.
import { Px } from '../pixel.mjs';
import { G, fin, dots, ring, disc, tube, rotCW, hash } from './_lib.mjs';

const ol = (p) => fin(p, 'k');
// ── projectiles 8×8 ───────────────────────────────────────────────────────────────
const FISHBONE = G([
  '........',
  '...v.v..',
  '.wvvvvvw',
  'wkwvwvwv',
  '.wvvvvvw',
  '...v.v..',
  '........',
  '........',
]);
function fishbone(i) {
  // frame 0: level; frame 1: tumbling (tail kicked up, ribs flipped)
  if (i === 0) return ol(new Px(8, 8).stamp(G(['......', '..v.vw', 'wvvvvv', 'kwvwvw', 'wvvvvv', '..v.vw']), 1, 1));
  return ol(new Px(8, 8).stamp(G(['....vw', '..vvv.', '.vvv.v', 'wvwv..', 'kwv...', 'w.....']), 1, 1));
}
function curse(i) {
  const p = new Px(8, 8);
  disc(p, 4, 4, 3, 'M'); disc(p, 3.6, 3.6, 1.8, 'L');
  const sw = i ? [[2, 5, 'm'], [5, 2, 'm'], [3, 3, 'w'], [6, 5, 'L']] : [[2, 2, 'm'], [5, 5, 'm'], [4, 3, 'w'], [1, 5, 'L']];
  dots(p, sw);
  return ol(p);
}
function feather(i) {
  const g = i ? G(['.....ih', '..iihhg', 'iihhgg.', '.ggf...']) : G(['....ii.', '.iihhhg', 'ihhggg.', '..gf...']);
  return ol(new Px(8, 8).stamp(g, 0, 2));
}
function spark(i) {
  const g = i ? G(['..z...', '.zw..y', 'zwwz.z', '..zwwz', '...wz.', '...z..']) : G(['...z..', 'y..wz.', 'z.zwwz', 'zwwz..', '.zw...', '..z...']);
  return ol(new Px(8, 8).stamp(g, 1, 1));
}
function boneRain() {
  return ol(new Px(6, 8).stamp(G(['v..v', '.vv.', 'vvvv', '.v..', 'vvvv', '.wk.']), 1, 1));
}

// ── combat fx ───────────────────────────────────────────────────────────────────────
function hit(i) {
  const p = new Px(10, 10), r = [2, 3.5, 4.5][i];
  const c1 = ['w', 'z', 'y'][i], c2 = ['z', 'y', 'x'][i];
  for (let a = 0; a < 8; a++) {
    const an = (a * Math.PI) / 4 + (i ? 0.39 : 0), len = a % 2 ? r * 0.6 : r;
    tube(p, [[Math.round(5 + Math.cos(an) * (i === 2 ? 2 : 0)), Math.round(5 + Math.sin(an) * (i === 2 ? 2 : 0))], [Math.round(5 + Math.cos(an) * len), Math.round(5 + Math.sin(an) * len)]], (s, k) => (k ? c2 : c1));
  }
  if (i < 2) disc(p, 5, 5, i ? 1.6 : 1.2, 'w');
  return p;
}
function slash(i) {
  // three parallel curved claw strokes, drawn on (0), full (1), trailing off (2, 3)
  const p = new Px(16, 16), [t0, t1] = [[0, 0.45], [0, 1], [0.3, 1], [0.65, 1]][i];
  for (let k = 0; k < 3; k++) for (let s = 0; s <= 40; s++) {
    const t = t0 + ((t1 - t0) * s) / 40, x = Math.round(1 + k * 3.5 + 8 * t - 3 * t * t), y = Math.round(1 + 13 * t + k * 0.5);
    const edge = s > 34;
    p.set(x, y, i === 3 ? (s % 3 ? (k === 1 ? 'E' : 'D') : '.') : i === 2 ? (edge ? 'w' : 'E') : edge ? 'D' : 'w');
    if (i === 1 && t > 0.2 && t < 0.8) p.set(x + 1, y, 'E');
  }
  return p;
}
function aoe(i) {
  const p = new Px(24, 24), r = [3, 6, 9, 11, 11.5][i];
  if (i === 0) { disc(p, 12, 12, 3.5, 'L'); disc(p, 12, 12, 2, 'w'); return p; }
  ring(p, 12, 12, r - (i === 4 ? 1 : 2.2), r, i === 4 ? 'm' : 'M');
  ring(p, 12, 12, r - (i === 4 ? 0.6 : 1.2), r - 0.2, i === 4 ? 'M' : 'L');
  if (i < 3) disc(p, 12, 12, r - 3.5, i === 1 ? 'L' : 'M');
  if (i === 1) disc(p, 12, 12, 2, 'w');
  for (let k = 0; k < 8; k++) {
    const a = k * 0.785 + i * 0.3, d = r + (i === 4 ? 0 : 1);
    if (i >= 2) p.set(Math.round(12 + Math.cos(a) * d), Math.round(12 + Math.sin(a) * d), k % 2 ? 'w' : 'L');
  }
  return p;
}
function poof(i) {
  const p = new Px(16, 16);
  const puffs = [[[8, 9, 3]], [[6, 9, 3], [10, 8, 3], [8, 11, 2.5]], [[4, 8, 2.6], [11, 7, 2.8], [8, 11, 2.6], [7, 5, 2]], [[3, 6, 1.8], [13, 5, 1.6], [9, 12, 1.6], [6, 3, 1.3]]][i];
  for (const [x, y, r] of puffs) { disc(p, x, y, r, i === 3 ? 'u' : 'v'); disc(p, x - 0.6, y - 0.6, r * 0.6, i === 3 ? 'v' : 'w'); }
  for (const [x, y, r] of puffs) { const yy = Math.round(y + r - 1); if (i < 3) p.set(Math.round(x), yy, 'u'); }
  return p;
}
function levelup(i) {
  const p = new Px(16, 24), y0 = 16 - i * 3;
  // arrow
  const arrow = G(['...z...', '..zyz..', '.zyyyz.', 'zyyyyyz', '..yyy..', '..yxy..', '..yxy..', '..xxx..']);
  if (i < 4) p.stamp(arrow, 4, y0);
  // sparkles
  const sp = [[2, 20], [13, 18], [3, 12], [12, 9], [7, 4], [1, 6], [14, 3]];
  sp.forEach(([x, y], k) => {
    const show = (k + i) % 3 !== 0, yy = y - i * 2;
    if (!show || yy < 1) return;
    const big = (k + i) % 2 === 0;
    p.set(x, yy, 'w'); if (big) dots(p, [[x - 1, yy, 'z'], [x + 1, yy, 'z'], [x, yy - 1, 'z'], [x, yy + 1, 'z']]);
  });
  if (i === 4) { ring(p, 8, 12, 5, 6.2, 'z'); ring(p, 8, 12, 6.2, 7, 'y'); }
  return p;
}
function catnip(i) {
  const p = new Px(16, 16);
  const leaf = (x, y, flip) => dots(p, flip ? [[x, y, 'c'], [x + 1, y, 'd'], [x + 1, y - 1, 'e'], [x + 2, y - 1, 'd'], [x, y + 1, 'b']] : [[x, y, 'd'], [x + 1, y, 'c'], [x, y - 1, 'e'], [x - 1, y - 1, 'd'], [x + 1, y + 1, 'b']]);
  for (let k = 0; k < 5; k++) {
    const a = k * 1.256 + i * 0.6, r = 3 + i * 1.2;
    leaf(Math.round(8 + Math.cos(a) * r), Math.round(9 + Math.sin(a) * r * 0.8 - i), k & 1);
  }
  // little hearts / sparkles
  const hs = [[7, 7 - i], [3, 3 + (i & 1)], [12, 2 + i]];
  hs.forEach(([x, y], k) => { if ((k + i) % 2 === 0) dots(p, [[x, y, 'H'], [x + 2, y, 'H'], [x, y + 1, 'H'], [x + 1, y + 1, 'H'], [x + 2, y + 1, 'H'], [x + 1, y + 2, 'H'], [x + 1, y, 'o']]); else p.set(x + 1, y + 1, 'w'); });
  return p;
}
function stun(i) {
  const p = new Px(16, 8);
  for (let k = 0; k < 3; k++) {
    const a = (k * 2 * Math.PI) / 3 + (i * Math.PI) / 6, x = Math.round(8 + Math.cos(a) * 5.5), y = Math.round(4 + Math.sin(a) * 2);
    const front = Math.sin(a) > 0;
    dots(p, [[x, y, front ? 'w' : 'z'], [x - 1, y, 'z'], [x + 1, y, 'z'], [x, y - 1, 'z'], [x, y + 1, front ? 'y' : 'x']]);
  }
  return p;
}
function poison(i) {
  const p = new Px(12, 12);
  const bubbles = [[3, 10, 1.6], [8, 9, 1.2], [6, 6, 1.8], [9, 4, 1.0], [4, 3, 1.1]];
  bubbles.forEach(([x, y, r], k) => {
    const yy = y - i * 1.5 - (k % 2), rr = r * (1 + (i === 2 && k % 2 ? -0.4 : 0.1 * i));
    if (yy < 1) return;
    ring(p, x, yy, rr - 0.9, rr, 'c'); disc(p, x, yy, rr - 0.9, 'd'); p.set(Math.round(x - rr / 2), Math.round(yy - rr / 2), 'e');
  });
  if (i === 2) dots(p, [[2, 2, 'e'], [10, 7, 'e']]);
  return p;
}
function crit(i) {
  const p = new Px(16, 16), R = [4, 7, 7.5, 6][i];
  for (let a = 0; a < 12; a++) {
    const an = (a * Math.PI) / 6 + 0.2, len = a % 2 ? R * 0.55 : R, inner = i === 3 ? R * 0.6 : 0;
    tube(p, [[Math.round(8 + Math.cos(an) * inner), Math.round(8 + Math.sin(an) * inner)], [Math.round(8 + Math.cos(an) * len), Math.round(8 + Math.sin(an) * len)]], (s, k) => (i === 3 ? (k < 1 ? 'y' : 'x') : k < 2 ? 'w' : k < 4 ? 'z' : 'R'));
  }
  if (i < 3) { disc(p, 8, 8, [2.5, 3.5, 2.5][i], i === 2 ? 'z' : 'w'); }
  if (i === 1) ring(p, 8, 8, 3.5, 4.4, 'R');
  return p;
}
function shadow(w, h) {
  const p = new Px(w, h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const nx = (x + 0.5 - w / 2) / (w / 2), ny = (y + 0.5 - h / 2) / (h / 2), r = nx * nx + ny * ny;
    if (r <= 1.05) p.set(x, y, r > 0.55 ? 'Q' : 'P');
  }
  return p;
}

export default function build() {
  const L = [];
  const anim = (base, n, f) => { for (let i = 0; i < n; i++) L.push({ name: `${base}_${i}`, px: f(i) }); };
  anim('proj_fishbone', 2, fishbone); anim('proj_curse', 2, curse); anim('proj_feather', 2, feather); anim('proj_spark', 2, spark);
  L.push({ name: 'proj_bone_rain', px: boneRain() });
  anim('fx_hit', 3, hit); anim('fx_slash', 4, slash); anim('fx_aoe', 5, aoe); anim('fx_poof', 4, poof); anim('fx_levelup', 5, levelup);
  anim('fx_catnip', 4, catnip); anim('fx_stun', 4, stun); anim('fx_poison', 3, poison); anim('fx_crit', 4, crit);
  L.push({ name: 'shadow_s', px: shadow(10, 3) }, { name: 'shadow_l', px: shadow(20, 4) });
  return L;
}
