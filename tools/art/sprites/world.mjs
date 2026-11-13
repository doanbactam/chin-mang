// Zone ground strips (16×6, tile horizontally) and background deco (SPEC §2.1 zones, §8.3).
// Ground: rows share one structure per zone so any variant sits next to any other without a seam;
// variants only differ in details kept away from the left/right columns.
import { Px } from '../pixel.mjs';
import { G, fin, dots, blob, tube, disc, ring, hash, put } from './_lib.mjs';

// ── ground ────────────────────────────────────────────────────────────────────────
function ground(z, v) {
  const p = new Px(16, 6), R = (x, y, s = 0) => hash(x + 16 * v * 0, y, z * 31 + s);
  for (let y = 0; y < 6; y++) for (let x = 0; x < 16; x++) {
    let c;
    if (z === 1) { // alley: kerb stone on top, asphalt below
      c = y === 0 ? (x % 8 === 7 ? 't' : 'v') : y === 1 ? (x % 8 === 7 ? 's' : 'u') : 's';
      if (y >= 2 && R(x, y) < 0.18) c = 'n';
      if (y >= 2 && R(x, y, 1) < 0.1) c = 't';
    }
    if (z === 2) { // roof tiles: scalloped terracotta rows
      const sx = (x + (y >> 1 & 1) * 2) % 4;
      c = y === 0 ? (x % 2 ? 'o' : 'R') : (y % 2 === 1 ? (sx === 3 ? 'r' : 'R') : (sx === 3 ? 'r' : sx === 0 ? 'o' : 'R'));
      if (y === 5) c = sx === 3 ? 'm' : 'r';
    }
    if (z === 3) { // wet planks of the fish market
      c = y === 0 ? 'i' : x % 8 === 7 ? 'f' : y === 1 ? 'h' : (y + (x >> 3)) % 4 === 0 ? 'g' : 'h';
      if (y >= 2 && x % 8 !== 7 && R(x, y) < 0.12) c = 'g';
      if (y === 5 && x % 8 !== 7) c = 'g';
    }
    if (z === 4) { // park: grass lip over dark soil
      const tuft = (x * 5 + 3) % 7 < 3;
      c = y === 0 ? (tuft ? 'd' : 'c') : y === 1 ? (x % 3 === 0 ? 'b' : 'c') : y === 2 ? (x % 4 === 1 ? 'b' : 'g') : 'f';
      if (y >= 3 && R(x, y) < 0.22) c = 'g';
    }
    if (z === 5) { // junkyard: oily gravel with rust
      c = y === 0 ? (x % 3 === 0 ? 'u' : 't') : y === 1 ? (x % 4 === 2 ? 'f' : 's') : 'f';
      if (y >= 1 && R(x, y) < 0.2) c = 'g';
      if (y >= 2 && R(x, y, 2) < 0.035) c = 'x';
      if (y >= 2 && R(x, y, 3) < 0.07) c = 't';
    }
    p.set(x, y, c);
  }
  // per-variant detail (kept within x 2..13)
  const D = {
    1: [[], [[5, 3, 'k'], [6, 3, 'k'], [7, 4, 'k'], [8, 4, 'n'], [9, 5, 'k']], [[4, 2, 'C'], [5, 2, 'D'], [6, 2, 'C'], [7, 2, 'B'], [5, 3, 'B']], [[9, 3, 'w'], [10, 3, 'w'], [11, 3, 'x']]],
    2: [[], [[6, 1, 'c'], [7, 1, 'd'], [8, 1, 'c'], [7, 2, 'b']], [[9, 2, 'm'], [9, 3, 'k'], [10, 3, 'm'], [10, 4, 'k']], [[4, 0, 'w'], [5, 1, 'w']]],
    3: [[], [[4, 2, 'D'], [5, 2, 'E'], [4, 3, 'D']], [[10, 1, 'C'], [11, 1, 'D'], [12, 1, 'C'], [9, 2, 'C'], [10, 2, 'B'], [11, 2, 'C']], [[3, 3, 'u'], [11, 4, 'u']]],
    4: [[], [[5, 0, 'H'], [5, 1, 'b'], [10, 0, 'z'], [10, 1, 'b']], [[7, 3, 'u'], [8, 3, 'v'], [8, 4, 't'], [7, 4, 's']], [[4, 0, 'e'], [12, 0, 'e'], [8, 0, 'L']]],
    5: [[], [[5, 2, 'u'], [6, 2, 'v'], [5, 3, 't'], [6, 3, 'u']], [[8, 2, 'n'], [9, 2, 'k'], [10, 2, 'n'], [9, 3, 'n'], [8, 1, 'M']], [[3, 3, 'x'], [4, 3, 'y'], [11, 2, 'u'], [12, 3, 'v']]],
  };
  dots(p, D[z][v]);
  return p;
}

// ── deco helpers ──────────────────────────────────────────────────────────────────
function box3(p, x, y, w, h, [lo, mid, hi]) {
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) p.set(x + i, y + j, j === 0 || i === 0 ? hi : j === h - 1 || i === w - 1 ? lo : mid);
  return p;
}
const W_ = (w, h) => new Px(w, h);

// zone 1 — back alley
function trashCan() {
  const p = W_(14, 18);
  box3(p, 2, 6, 10, 11, ['t', 'u', 'v']);
  for (let y = 7; y < 16; y++) for (const x of [4, 7, 10]) p.set(x, y, 't');
  dots(p, [[5, 9, 's'], [6, 10, 's']]);                                   // dent
  box3(p, 1, 4, 12, 2, ['s', 't', 'v']);                                  // lid
  dots(p, [[6, 3, 't'], [7, 3, 't'], [5, 3, 's'], [8, 3, 's']]);          // handle
  tube(p, [[9, 4], [11, 1]], 'w'); dots(p, [[10, 2, 'v'], [12, 1, 'w'], [11, 0, 'v']]); // fish bone peeking
  dots(p, [[2, 2, 'c'], [3, 3, 'd']]);                                    // banana peel-ish litter
  return fin(p);
}
function cartons() {
  const p = W_(20, 14);
  box3(p, 1, 5, 10, 8, ['g', 'h', 'i']); box3(p, 10, 7, 9, 6, ['g', 'h', 'i']); box3(p, 4, 1, 8, 5, ['g', 'h', 'i']);
  for (let y = 5; y < 13; y++) p.set(6, y, 'v');
  for (let y = 1; y < 6; y++) p.set(8, y, 'v');
  dots(p, [[13, 9, 'f'], [14, 9, 'f'], [13, 10, 'f'], [15, 10, 'f'], [14, 11, 'f']]); // stamped paw
  return fin(p);
}
function streetLamp() {
  const p = W_(10, 24);
  for (let y = 6; y < 23; y++) { p.set(4, y, 's'); p.set(5, y, 't'); }
  box3(p, 3, 21, 4, 2, ['n', 's', 't']);
  tube(p, [[5, 5], [7, 3]], 's');
  box3(p, 1, 2, 6, 3, ['s', 't', 'u']);
  dots(p, [[2, 5, 'z'], [3, 5, 'w'], [4, 5, 'z'], [5, 5, 'y']]);
  return fin(p);
}
function puddle() {
  const p = W_(16, 10);
  blob(p, 7, 8, 6, 1.5, ['B', 'C', 'D']);
  dots(p, [[5, 8, 'E'], [9, 7, 'E']]);
  box3(p, 11, 2, 3, 6, ['a', 'b', 'c']); dots(p, [[12, 0, 'b'], [12, 1, 'c'], [12, 4, 'w'], [12, 5, 'w']]);
  return fin(p);
}

// zone 2 — rooftops
function chimney() {
  const p = W_(14, 22);
  box3(p, 3, 9, 8, 12, ['r', 'R', 'o']);
  for (let y = 11; y < 20; y += 3) for (let x = 3; x < 11; x++) p.set(x, y, 'p');
  for (let y = 9; y < 20; y++) if (((y / 3) | 0) % 2) p.set(7, y, 'p'); else p.set(5, y, 'p');
  box3(p, 2, 7, 10, 2, ['s', 't', 'u']);
  disc(p, 7, 4, 2.5, 'v'); disc(p, 10, 2, 1.8, 'u'); disc(p, 6.5, 3.5, 1.2, 'w');
  return fin(p);
}
function antenna() {
  const p = W_(12, 24);
  for (let y = 4; y < 23; y++) p.set(6, y, y % 2 ? 's' : 't');
  for (const [y, w] of [[5, 5], [9, 4], [13, 3]]) for (let i = -w; i <= w; i++) p.set(6 + i, y, Math.abs(i) === w ? 'u' : 't');
  dots(p, [[6, 3, 'R'], [6, 2, 'o']]);
  dots(p, [[4, 22, 's'], [8, 22, 's'], [5, 21, 's'], [7, 21, 's']]);
  return fin(p);
}
function waterTank() {
  const p = W_(22, 20);
  for (const x of [4, 16]) for (let y = 13; y < 19; y++) { p.set(x, y, 's'); p.set(x + 1, y, 't'); }
  tube(p, [[5, 18], [16, 14]], 's'); tube(p, [[5, 14], [16, 18]], 't');
  box3(p, 2, 3, 18, 11, ['B', 'C', 'D']);
  for (let y = 4; y < 13; y += 4) for (let x = 3; x < 19; x++) p.set(x, y, 'B');
  blob(p, 11, 3, 9, 2, ['B', 'C', 'D']);
  dots(p, [[4, 6, 'E'], [4, 7, 'E'], [4, 8, 'D']]);
  return fin(p);
}
function flowerPot() {
  const p = W_(12, 12);
  for (let y = 7; y < 11; y++) for (let x = 3 + (y - 7 >> 1); x < 9 - (y - 7 >> 1); x++) p.set(x, y, x === 3 + (y - 7 >> 1) ? 'o' : 'x');
  box3(p, 2, 6, 8, 2, ['p', 'x', 'o']);
  dots(p, [[5, 5, 'c'], [6, 4, 'b'], [4, 4, 'c'], [7, 3, 'c'], [3, 3, 'd'], [8, 2, 'b'], [5, 3, 'b'], [6, 2, 'H'], [6, 1, 'o'], [5, 2, 'H'], [7, 2, 'H'], [6, 3, 'z']]);
  return fin(p);
}

// zone 3 — fish market
function fishStall() {
  const p = W_(32, 22);
  // awning with stripes
  for (let x = 2; x < 30; x++) for (let y = 2; y < 6; y++) p.set(x, y, ((x >> 2) & 1) ? 'w' : 'R');
  for (let x = 2; x < 30; x++) p.set(x, 6, ((x >> 2) & 1) ? 'v' : 'r');
  for (let x = 2; x < 30; x += 4) { p.set(x + 1, 7, ((x >> 2) & 1) ? 'v' : 'r'); p.set(x + 2, 7, ((x >> 2) & 1) ? 'v' : 'r'); }
  for (const x of [3, 28]) for (let y = 7; y < 21; y++) p.set(x, y, 'g');
  // table + ice + fish
  box3(p, 2, 15, 28, 6, ['f', 'g', 'h']);
  for (let x = 4; x < 28; x++) p.set(x, 14, x % 3 ? 'E' : 'D');
  for (let k = 0; k < 4; k++) { const fx = 5 + k * 6; dots(p, [[fx, 13, 'u'], [fx + 1, 13, 'v'], [fx + 2, 13, 'u'], [fx + 3, 13, 't'], [fx + 4, 12, 't'], [fx + 4, 14, 't'], [fx, 12, 'v'], [fx + 1, 12, 'k']]); }
  dots(p, [[14, 17, 'w'], [15, 17, 'w'], [16, 17, 'w'], [15, 18, 'z']]); // price tag
  return fin(p);
}
function fishCrate() {
  const p = W_(18, 12);
  box3(p, 1, 5, 16, 6, ['f', 'g', 'h']);
  for (let x = 1; x < 17; x++) p.set(x, 8, 'f');
  for (let k = 0; k < 4; k++) dots(p, [[3 + k * 4, 4, 'u'], [4 + k * 4, 4, 'v'], [5 + k * 4, 4, 't'], [3 + k * 4, 3, 'v'], [4 + k * 4, 3, 'k']]);
  dots(p, [[16, 3, 't'], [16, 2, 'u'], [2, 2, 't']]);
  return fin(p);
}
function bucket() {
  const p = W_(12, 12);
  for (let y = 4; y < 11; y++) for (let x = 2 + (y > 8 ? 1 : 0); x < 10 - (y > 8 ? 1 : 0); x++) p.set(x, y, x < 4 ? 'u' : x > 7 ? 's' : 't');
  for (let x = 2; x < 10; x++) p.set(x, 4, 'C');
  dots(p, [[3, 4, 'D'], [4, 4, 'E'], [6, 3, 't'], [6, 2, 'u'], [7, 1, 't'], [5, 1, 'u'], [2, 6, 'v'], [9, 6, 'v']]);
  return fin(p);
}
function lantern() {
  const p = W_(10, 22);
  for (let y = 2; y < 21; y++) p.set(2, y, 'f');
  tube(p, [[2, 3], [7, 3]], 'f');
  blob(p, 6.5, 9, 2.6, 3.5, ['r', 'R', 'o']);
  dots(p, [[6, 5, 'k'], [6, 6, 'y'], [5, 13, 'y'], [6, 13, 'y'], [7, 13, 'y'], [6, 14, 'x'], [6, 9, 'z'], [6, 4, 'f']]);
  dots(p, [[1, 20, 'g'], [3, 20, 'g'], [2, 20, 'g']]);
  return fin(p);
}

// zone 4 — city park
function bench() {
  const p = W_(26, 14);
  for (const y of [3, 5]) box3(p, 2, y, 22, 2, ['g', 'h', 'i']);
  box3(p, 1, 8, 24, 2, ['g', 'h', 'i']);
  for (const x of [3, 21]) { for (let y = 3; y < 13; y++) p.set(x, y, 'n'); p.set(x + 1, 12, 'n'); p.set(x - 1, 12, 'n'); }
  dots(p, [[12, 6, 'f'], [12, 4, 'f']]);
  return fin(p);
}
function tree() {
  const p = W_(24, 24);
  for (let y = 14; y < 23; y++) { p.set(11, y, 'g'); p.set(12, y, 'f'); p.set(10, y, y > 19 ? 'g' : '.'); }
  dots(p, [[9, 22, 'g'], [13, 22, 'f'], [14, 22, 'f']]);
  blob(p, 7, 11, 5, 4.5, ['a', 'b', 'c']); blob(p, 16, 11, 5.5, 4.5, ['a', 'b', 'c']);
  blob(p, 12, 7, 7, 5.5, ['b', 'c', 'd']);
  for (let k = 0; k < 14; k++) p.set(4 + Math.floor(hash(k, 1) * 16), 3 + Math.floor(hash(k, 2) * 12), hash(k, 3) < 0.5 ? 'e' : 'b');
  dots(p, [[6, 9, 'H'], [17, 7, 'H'], [12, 12, 'H']]);
  return fin(p);
}
function parkLamp() {
  const p = W_(8, 24);
  for (let y = 7; y < 22; y++) { p.set(3, y, 'a'); p.set(4, y, 'n'); }
  box3(p, 2, 21, 4, 2, ['n', 'a', 'b']);
  box3(p, 1, 2, 6, 5, ['n', 'z', 'w']); dots(p, [[2, 1, 'a'], [3, 1, 'a'], [4, 1, 'a'], [5, 1, 'a'], [3, 0, 'a'], [4, 0, 'a'], [2, 4, 'y'], [5, 4, 'y']]);
  return fin(p);
}
function flowerBed() {
  const p = W_(20, 12);
  blob(p, 10, 9.5, 8.5, 2, ['f', 'g', 'h']);
  for (let k = 0; k < 8; k++) {
    const x = 3 + k * 2, top = 4 + (k * 7) % 3;
    for (let y = top + 1; y < 9; y++) p.set(x, y, 'b');
    const c = ['H', 'z', 'L', 'o'][k % 4];
    dots(p, [[x, top, c], [x - 1, top + 1, 'c'], [x + 1, top + 2, 'c']]);
  }
  return fin(p);
}

// zone 5 — junkyard
function tires() {
  const p = W_(24, 16);
  const upright = (cx, cy) => { blob(p, cx, cy, 5, 5, ['n', 's', 't']); ring(p, cx, cy, 2.2, 3, 'n'); disc(p, cx, cy, 2.2, 'u'); disc(p, cx, cy, 1, 's'); for (let a = 0; a < 6; a++) p.set(Math.round(cx - 0.5 + Math.cos(a) * 4.3), Math.round(cy - 0.5 + Math.sin(a) * 4.3), a < 3 ? 'n' : 'u'); };
  upright(7, 10.5); upright(17, 10.5);
  blob(p, 12, 4.5, 6, 2.4, ['n', 's', 't']); blob(p, 12, 4.5, 2.6, 1, ['k', 'k', 'k']);
  return fin(p);
}
function brokenTV() {
  const p = W_(16, 16);
  box3(p, 1, 3, 14, 11, ['f', 'g', 'h']);
  box3(p, 3, 5, 8, 7, ['a', 'n', 's']);
  dots(p, [[5, 7, 'D'], [6, 8, 'E'], [7, 7, 'D'], [8, 9, 'D'], [6, 10, 'D']]);
  for (const y of [6, 8, 10]) p.set(13, y, 'u');
  dots(p, [[4, 2, 't'], [3, 1, 't'], [10, 2, 't'], [11, 1, 't'], [3, 14, 'n'], [12, 14, 'n']]);
  return fin(p);
}
function barrel() {
  const p = W_(12, 14);
  box3(p, 2, 2, 8, 11, ['B', 'C', 'D']);
  for (const y of [4, 8, 11]) for (let x = 2; x < 10; x++) p.set(x, y, 'B');
  dots(p, [[3, 3, 'E'], [3, 5, 'E'], [5, 1, 'n'], [6, 1, 'n'], [8, 9, 'x'], [7, 10, 'x'], [9, 6, 'x']]);
  dots(p, [[10, 12, 'n'], [10, 13, 'n'], [11, 13, 'n']]);
  return fin(p);
}
function scrapHeap() {
  const p = W_(30, 20);
  blob(p, 15, 15, 13, 5, ['s', 't', 'u']);
  for (let k = 0; k < 40; k++) { const x = 3 + Math.floor(hash(k, 9) * 24), y = 11 + Math.floor(hash(k, 8) * 8); if (p.get(x, y) !== '.') p.set(x, y, ['x', 'g', 'f', 'u', 'y'][k % 5]); }
  // pipe, gear, spring
  tube(p, [[6, 11], [12, 6]], 'u', 2); dots(p, [[12, 6, 'v'], [13, 6, 'v']]);
  ring(p, 20, 9, 1.4, 3.4, 't'); dots(p, [[20, 5, 't'], [24, 9, 't'], [16, 9, 't'], [20, 13, 's'], [20, 9, '.']]);
  for (let y = 4; y < 10; y++) p.set(26 + (y & 1), y, 'v');
  return fin(p);
}

const DECO = { 1: [trashCan, cartons, streetLamp, puddle], 2: [chimney, antenna, waterTank, flowerPot], 3: [fishStall, fishCrate, bucket, lantern], 4: [bench, tree, parkLamp, flowerBed], 5: [tires, brokenTV, barrel, scrapHeap] };

export default function build() {
  const L = [];
  for (let z = 1; z <= 5; z++) {
    for (let v = 0; v < 4; v++) L.push({ name: `ground_z${z}_${v}`, px: ground(z, v) });
    DECO[z].forEach((f, i) => L.push({ name: `deco_z${z}_${i}`, px: f() }));
  }
  return L;
}
