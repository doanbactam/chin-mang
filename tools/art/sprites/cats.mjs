// The four cats (SPEC §2.3, §8.1). 16×16, facing right, feet on row 14, outline added last by fin().
// Each cat = role-letter parts (head, body, accessories) + procedural legs/tail, re-posed per frame.
// Roles: a fur, b fur shadow, c fur light, d stripe/point, e eye, m muzzle/belly/paws, K nose (palette).
import { Px } from '../pixel.mjs';
import { G, put, finUnit as fin, dots, recolor, clear } from './_lib.mjs';

// ── per-cat definitions ───────────────────────────────────────────────────────────
// head rows start at y=1, body rows at y=10. Strings are 16 wide.
const CATS = {
  muop: {
    map: { a: 'T', b: 'x', c: 'I', d: 'g', e: 'k', m: 'w', S: 'R', s: 'r', o: 'o' },
    head: [
      '................',
      '......c....c....',
      '.....cKa..cKa...',
      '.....ccdadaab...',
      '.....caaaaaab...',
      '.....caeaaeab...',
      '.....daeaaeab...',
      '.....baamKmab...',
      '......bbmmmb....',
    ],
    body: [
      '..caadadaaam....',
      '..aaadadaab.....',
      '..bbbbbbbbb.....',
    ],
    eyes: [[7, 6], [10, 6]],
    legs: { back: 3, front: 8, paw: 'm', near: ['a', 'b'], far: 'b' },
    tail: { c: ['a', 'd'], tip: 'd', thick: 1 },
    // red scarf: band around the neck + two tails fluttering behind
    acc(p, f) {
      const y = 10 + (f.by || 0) + (f.hy > 0 ? 0 : 0), x = (f.bx || 0);
      dots(p, [[6 + x, y - 1, 'S'], [7 + x, y - 1, 'S'], [8 + x, y - 1, 'S'], [9 + x, y - 1, 'S'], [10 + x, y - 1, 's'],
        [5 + x, y - 1, 'S'], [5 + x, y, 's'], [6 + x, y, 'S']]);
      const fl = f.scarf ?? 0; // 0 hang, 1 flutter, 2 high flutter
      if (fl === 0) dots(p, [[4 + x, y - 1, 'S'], [3 + x, y - 1, 'S'], [3 + x, y, 's'], [2 + x, y, 's']]);
      if (fl === 1) dots(p, [[4 + x, y - 1, 'S'], [3 + x, y - 2, 'S'], [2 + x, y - 2, 's'], [3 + x, y - 1, 's']]);
      if (fl === 2) dots(p, [[4 + x, y - 2, 'S'], [3 + x, y - 2, 'S'], [2 + x, y - 3, 'S'], [1 + x, y - 3, 's'], [4 + x, y - 1, 's']]);
    },
  },
  xiem: {
    map: { a: 'W', b: 'i', c: 'w', d: 'f', D: 'g', e: 'C', m: 'w', S: 'h', s: 'f', B: 'v', Z: 'z', o: 'w' },
    head: [
      '................',
      '......d....d....',
      '.....dKd..dKd...',
      '.....caaaaaab...',
      '.....caaDDaab...',
      '.....caeDDeab...',
      '.....aDeDDeDb...',
      '.....bDDdKdDb...',
      '......bbDdDb....',
    ],
    body: [
      '..caaaaaaaa.....',
      '..aaaaaaaab.....',
      '..bbbbbbbbb.....',
    ],
    eyes: [[7, 6], [10, 6]],
    legs: { back: 3, front: 8, paw: 'd', near: ['d', 'D'], far: 'D', sock: true },
    tail: { c: ['d', 'D'], tip: 'd', thick: 1, long: true },
    // cross-body satchel: strap from shoulder, bag on the hip with a fishbone sticking out
    acc(p, f) {
      const y = 10 + (f.by || 0), x = (f.bx || 0);
      // strap: shoulder (front-top) diagonally down to the hip bag
      dots(p, [[9 + x, y - 1, 's'], [8 + x, y, 's'], [7 + x, y, 's'], [6 + x, y + 1, 's'],
        [2 + x, y, 's'], [3 + x, y, 's'], [4 + x, y, 's'], [5 + x, y, 's'],
        [2 + x, y + 1, 's'], [3 + x, y + 1, 'S'], [4 + x, y + 1, 'S'], [5 + x, y + 1, 'S'], [6 + x, y + 1, 's'],
        [3 + x, y + 2, 'S'], [4 + x, y + 2, 'Z'], [5 + x, y + 2, 'S'], [6 + x, y + 2, 's'],
        [3 + x, y + 3, 's'], [4 + x, y + 3, 'S'], [5 + x, y + 3, 'S'], [6 + x, y + 3, 's'],
        [3 + x, y - 1, 'B'], [3 + x, y - 2, 'B'], [2 + x, y - 2, 'B'], [4 + x, y - 2, 'B']]);
    },
  },
  map: {
    map: { a: 'u', b: 't', c: 'v', d: 's', e: 'k', m: 'w', h: 'd', i: 'c', j: 'b', r: 'a', S: 'g', o: 'w' },
    head: [
      '.......hhii.....',
      '......hiiiij....',
      '.....hiiiijjj...',
      '...rrrrrrrrrrr..',
      '....caaaaaaaab..',
      '....caeaaaeaab..',
      '....caeaaaeaab..',
      '....baaamKmaab..',
      '....SbbmmmmbS...',
    ],
    body: [
      '.caaaaaaaaaam...',
      '.caaaaaaaaaammm.',
      '.bbbbbbbbbbbmm..',
    ],
    headY: 1,
    eyes: [[6, 6], [10, 6]],
    legs: { back: 2, front: 9, paw: 'm', near: ['a', 'b'], far: 'b', wide: true },
    tail: { c: ['a', 'b'], tip: 'b', thick: 2, short: true },
    acc() {},
  },
  mun: {
    map: { a: 'A', b: 'n', c: 'F', d: 'A', e: 'z', m: 'F', h: 'L', i: 'M', j: 'm', r: 'M', S: 'y', o: 'L' },
    head: [
      '...hi...........',
      '....ii..........',
      '....iiii........',
      '....jSSSj.......',
      '.rrrrrrrrrr.c...',
      '.....caaaaacKa..',
      '.....caeaaeaab..',
      '.....baeaaeab...',
      '......bamKmb....',
    ],
    headDy: 1,
    body: [
      '..caaaaaaam.....',
      '..aaaaaaamm.....',
      '..bbbbbbbbb.....',
    ],
    eyes: [[7, 7], [10, 7]],
    legs: { back: 3, front: 8, paw: 'a', near: ['a', 'b'], far: 'b' },
    tail: { c: ['a', 'a'], tip: 'c', thick: 1, curl: true },
    acc() {},
  },
};

// ── tail poses: lists of [dx,dy] from the tail root (rear of the body, top row) ──────
const TAILS = {
  up: [[0, 0], [-1, -1], [-1, -2], [-1, -3], [0, -4]],
  upR: [[0, 0], [-1, -1], [-1, -2], [-1, -3], [-1, -4], [0, -5]],
  swish: [[0, 0], [-1, -1], [-1, -2], [0, -3], [0, -4], [1, -5]],
  flick: [[0, 0], [-1, -1], [-1, -2], [-1, -3], [-1, -4], [-1, -5]],
  low: [[0, 0], [-1, 0], [-1, -1], [-1, -2], [0, -3]],
  bristle: [[0, 0], [-1, -1], [-1, -2], [-1, -3], [-1, -4], [-1, -5], [0, -6]],
  droop: [[0, 0], [-1, 1], [-1, 2]],
};

function drawTail(p, def, pose, rx, ry) {
  let pts = TAILS[pose];
  const t = def.tail;
  if (t.short) pts = pts.slice(0, 4);
  if (t.long && pose !== 'droop') pts = [...pts, [pts.at(-1)[0] + 1, pts.at(-1)[1] - 1]];
  if (t.curl && pose !== 'droop') pts = [...pts, [pts.at(-1)[0] + 1, pts.at(-1)[1]]];
  pts.forEach(([dx, dy], i) => {
    const last = i === pts.length - 1, c = last ? t.tip : t.c[i & 1];
    p.set(Math.max(1, rx + dx), ry + dy, c);
    if (t.thick > 1 && i < pts.length - 1) p.set(Math.max(1, rx + dx) + 1, ry + dy, 'b');
  });
}

// legs: each [dx, lift] for backNear, backFar, frontNear, frontFar
function drawLegs(p, def, legs, bodyBottom, bx) {
  const L = def.legs, w = L.wide ? 3 : 2;
  const one = (x, dx, lift, far) => {
    const top = bodyBottom + 1, bot = lift ? 13 : 14;
    for (let y = top; y <= bot; y++) {
      for (let i = 0; i < (far ? 2 : w); i++) {
        const xx = x + dx + i + bx + (far ? 1 : 0);
        if (far) { p.set(xx, y, y === bot ? 'b' : L.far); continue; }
        let c = i === w - 1 ? L.near[1] : L.near[0];
        if (y === bot) c = L.paw;
        else if (L.sock && y >= 13) c = L.near[0];
        p.set(xx, y, c);
      }
    }
  };
  const [bn, bf, fn, ff] = legs;
  one(L.back, bf[0], bf[1], true); one(L.front, ff[0], ff[1], true);
  one(L.back, bn[0], bn[1], false); one(L.front, fn[0], fn[1], false);
}

function eyesTo(p, def, mode, hx, hy) {
  for (const [x0, y0] of def.eyes) {
    const x = x0 + hx, y = y0 + hy + (def.headDy || 0);
    if (mode === 'open') continue;
    const fur = def.map.a;
    if (mode === 'closed') { p.set(x, y, fur === 'W' ? 'd' : fur); p.set(x, y + 1, 'k'); p.set(x - 1, y + 1, 'k'); }
    if (mode === 'hurt') { p.set(x, y, 'k'); p.set(x - 1, y, fur); p.set(x, y + 1, 'k'); p.set(x - 1, y + 1, 'k'); }
    if (mode === 'wide') { p.set(x, y, 'w'); p.set(x, y + 1, def.map.e); }
  }
}

const STAND = [[0, 0], [0, 0], [0, 0], [0, 0]];
// pose: hx,hy head offset; bx,by body offset; legs; tail pose; eyes; paw; scarf; extra(p)
function pose(id, f = {}) {
  const def = CATS[id], p = new Px(16, 16), bx = f.bx || 0, by = f.by || 0, hx = f.hx ?? bx, hy = (f.hy ?? by);
  const body = G(def.body), bodyBottom = 12 + by;
  drawLegs(p, def, f.legs ?? STAND, bodyBottom, bx);
  // far legs were drawn first; tail behind body
  drawTail(p, def, f.tail ?? 'up', 2 + bx - (def.bodyX ? 1 : 0), 10 + by);
  put(p, body, bx, 10 + by);
  if (f.legsFront) drawLegs(p, def, f.legs ?? STAND, bodyBottom, bx);
  const head = G(def.head);
  put(p, head, hx, 1 + hy + (def.headY || 0));
  def.acc(p, { ...f, bx, by });
  eyesTo(p, def, f.eyes ?? 'open', hx, hy - (def.headDy ? 1 : 0) + (def.headY || 0));
  if (f.mouth) { const [x0, y0] = def.eyes[1]; p.set(x0 + hx, y0 + 2 + hy + (def.headY || 0), 'O'); }
  if (f.extra) f.extra(p, def, { bx, by, hx, hy });
  return p;
}
const out = (id, px) => fin(recolor(px, { O: 'r', ...CATS[id].map }));

// ── animation tables ─────────────────────────────────────────────────────────────
const WALK = [
  { legs: [[1, 0], [-1, 0], [-1, 0], [1, 0]], tail: 'up', scarf: 1 },
  { legs: [[0, 0], [0, 1], [0, 1], [0, 0]], by: -1, tail: 'swish', scarf: 2 },
  { legs: [[-1, 0], [1, 0], [1, 0], [-1, 0]], tail: 'upR', scarf: 1 },
  { legs: [[0, 1], [0, 0], [0, 0], [0, 1]], by: -1, tail: 'flick', scarf: 2 },
];

// a front paw (2×2) at (x,y) with optional claw tips
function paw(p, x, y, claws = false, c = 'a') {
  dots(p, [[x, y, c], [x + 1, y, c], [x, y + 1, 'b'], [x + 1, y + 1, 'm']]);
  if (claws) dots(p, [[x + 2, y, 'w'], [x + 2, y + 1, 'w']]);
}

const ATTACK = {
  muop: [ // claw swipe
    { hx: -1, legs: [[-1, 0], [0, 0], [0, 0], [0, 0]], tail: 'bristle', scarf: 0, extra: (p) => paw(p, 11, 5) },
    { bx: 1, hx: 1, hy: 1, legs: [[-1, 0], [0, 0], [1, 0], [2, 0]], tail: 'flick', scarf: 2, extra: (p) => paw(p, 12, 10, true) },
    { legs: [[0, 0], [0, 0], [1, 0], [0, 0]], tail: 'swish', scarf: 1, extra: (p) => paw(p, 11, 12) },
  ],
  xiem: [ // throw a fishbone
    { hx: -1, legs: [[-1, 0], [0, 0], [0, 0], [0, 0]], tail: 'flick', extra: (p) => { paw(p, 4, 6); dots(p, [[3, 4, 'v'], [3, 5, 'w'], [4, 5, 'v'], [2, 5, 'v']]); } },
    { hx: 1, bx: 0, legs: [[-1, 0], [0, 0], [1, 0], [1, 0]], tail: 'swish', extra: (p) => { paw(p, 12, 8); dots(p, [[14, 6, 'w'], [14, 7, 'v']]); } },
    { legs: [[0, 0], [0, 0], [1, 0], [0, 0]], tail: 'upR', extra: (p) => paw(p, 11, 11) },
  ],
  map: [ // belly bump
    { bx: -1, hx: -1, hy: 1, legs: [[0, 0], [0, 0], [-1, 0], [0, 0]], tail: 'low' },
    { bx: 1, hx: 1, hy: -1, legs: [[-1, 0], [-1, 0], [1, 0], [1, 0]], tail: 'up', eyes: 'closed', mouth: true },
    { legs: [[0, 0], [0, 0], [0, 0], [0, 0]], hy: 1, tail: 'swish' },
  ],
  mun: [ // raise paw, cast curse
    { hy: 1, legs: STAND, tail: 'up', extra: (p) => { paw(p, 11, 8, false, 'a'); dots(p, [[13, 7, 'L']]); } },
    { hx: 1, legs: [[-1, 0], [0, 0], [1, 0], [1, 0]], tail: 'flick', eyes: 'wide', extra: (p) => { paw(p, 12, 9, false, 'a'); dots(p, [[14, 8, 'L'], [14, 9, 'M'], [13, 7, 'w']]); } },
    { legs: STAND, tail: 'swish', extra: (p) => paw(p, 10, 12, false, 'a') },
  ],
};
const SKILL = {
  muop: [ // Cào Liên Hoàn: crouch, leap with both paws, double swipe
    { by: 1, hy: 2, legs: [[-1, 0], [0, 0], [0, 0], [0, 0]], tail: 'bristle', scarf: 0, eyes: 'hurt' },
    { by: -2, hy: -1, legs: [[-1, 1], [0, 1], [1, 1], [1, 1]], tail: 'flick', scarf: 2, extra: (p) => { paw(p, 12, 5, true); paw(p, 12, 9, true); } },
    { bx: 1, hx: 1, legs: [[-1, 0], [0, 0], [1, 0], [2, 0]], tail: 'swish', scarf: 2, extra: (p) => paw(p, 12, 11, true) },
  ],
  xiem: [ // Mưa Xương Cá: reach into satchel, toss a fan of bones upward
    { hy: 1, hx: -1, legs: STAND, tail: 'low', extra: (p) => paw(p, 5, 11) },
    { hy: -1, legs: [[0, 0], [0, 0], [0, 1], [0, 0]], tail: 'up', eyes: 'closed', extra: (p) => { paw(p, 11, 4); dots(p, [[13, 1, 'w'], [12, 2, 'v'], [14, 2, 'v'], [10, 1, 'v'], [11, 2, 'w']]); } },
    { legs: STAND, tail: 'upR', extra: (p) => { paw(p, 12, 6); dots(p, [[14, 3, 'w'], [13, 2, 'v']]); } },
  ],
  map: [ // Ngồi Đè: jump up, slam down, shockwave squash
    { by: 1, hy: 2, legs: [[0, 0], [0, 0], [0, 0], [0, 0]], tail: 'low', eyes: 'hurt' },
    { by: -2, hy: 0, legs: [[0, 1], [0, 1], [0, 1], [0, 1]], tail: 'up', eyes: 'wide', extra: (p) => dots(p, [[3, 14, 'u'], [12, 14, 'u'], [7, 14, 'v']]) },
    { by: 1, hy: 2, legs: [[-1, 0], [-1, 0], [1, 0], [1, 0]], tail: 'low', eyes: 'closed', mouth: true, extra: (p) => dots(p, [[0, 13, 'u'], [15 - 1, 13, 'u']]) },
  ],
  mun: [ // Lời Nguyền: paws up, circle of purple, point
    { hy: 1, legs: STAND, tail: 'flick', eyes: 'closed', extra: (p) => { paw(p, 11, 7); dots(p, [[13, 5, 'M'], [12, 4, 'L']]); } },
    { hy: -1, by: -1, legs: [[0, 1], [0, 0], [0, 0], [0, 1]], tail: 'bristle', eyes: 'wide', extra: (p) => { paw(p, 12, 4); dots(p, [[14, 2, 'L'], [13, 1, 'w'], [14, 1, 'M'], [12, 2, 'M']]); } },
    { hx: 1, legs: [[-1, 0], [0, 0], [1, 0], [1, 0]], tail: 'swish', eyes: 'wide', extra: (p) => { paw(p, 12, 9); dots(p, [[14, 8, 'M'], [14, 7, 'L']]); } },
  ],
};

// ── lying poses (shared role templates, per-cat accessories) ─────────────────────
const SLEEP = [
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '.......c...c....',
  '......cKa.cKa...',
  '...cccaaadadab..',
  '..caaaaaaaaaab..',
  '.caaaaaeeaeeab..',
  '.caaaaaamKmmab..',
  '.baaaabbbmmmmb..',
  '..bdadadadadab..',
  '................',
];
const FLAT = [
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '................',
  '........c...c...',
  '.......cKa.cKa..',
  '..ccccaaadadaab.',
  '.caaaaaaeeaeeab.',
  '.aaaaaaaamKmaab.',
  '.bmmbbbbbmmmmbb.',
  '................',
];
function lying(id, tpl, variant) {
  const def = CATS[id];
  let p = G(tpl);
  // per-cat face/markings on the shared templates
  if (id === 'xiem') { // dark mask, ears and tail
    p = recolor(p, { d: 'd' });
    for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
      const c = p.get(x, y);
      if (c === 'K' && y < 11) p.set(x, y, 'K');
    }
  }
  if (id !== 'muop') p = recolor(p, { d: id === 'xiem' ? 'd' : 'a' });
  if (id === 'xiem') { const ey = tpl === SLEEP ? 8 : 10; for (let y = 0; y <= ey; y++) for (let x = 6; x < 16; x++) if (p.get(x, y) === 'c' || p.get(x, y) === 'a') p.set(x, y, 'd'); }
  if (id === 'xiem') { // point colour on face area around eyes / tail
    for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
      const c = p.get(x, y);
      if (tpl === SLEEP && x >= 7 && x <= 12 && y >= 11 && y <= 13 && c === 'a') p.set(x, y, 'd');
      if (tpl === FLAT && x >= 8 && x <= 13 && y >= 12 && y <= 13 && c === 'a') p.set(x, y, 'd');
      if (tpl === SLEEP && y === 14 && c !== '.') p.set(x, y, x & 1 ? 'd' : 'D');
    }
  }
  if (id === 'map') { // wider, rounder: duplicate a body column; helmet over the head
    const w = new Px(16, 16);
    for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) w.d[y][x] = p.get(x <= 3 ? x + 1 : x, y);
    p = w; p.d.forEach((r) => (r[0] = '.'));
    if (tpl === SLEEP) {
      clear(p, 6, 7, 8, 2);
      put(p, G(['..hhii..', '.hiiiij.', 'rrrrrrrr'], {}), 6, 7);
      clear(p, 0, 10, 1, 4);
    } else {
      // helmet knocked off, lying upside-down behind
      clear(p, 7, 9, 7, 2);
      dots(p, [[8, 10, 'b'], [12, 10, 'b']]);
      put(p, G(['rrrrr', '.jiih', '..jj.']), 1, 8);
    }
  }
  if (id === 'mun') { // little witch hat
    if (tpl === SLEEP) put(p, G(['....hi', '...ii.', '..iiii', '.jSSSj', 'rrrrrrr']), 4, 4);
    else put(p, G(['hi....', '.ii...', '.iiii.', 'jSSSj.', 'rrrrrr']), 2, 6);
  }
  if (id === 'muop') { // scarf
    if (tpl === SLEEP) dots(p, [[6, 13, 'S'], [7, 13, 'S'], [8, 13, 's'], [5, 12, 'S'], [5, 13, 's']]);
    else dots(p, [[6, 13, 'S'], [6, 12, 'S'], [6, 14, 's'], [5, 14, 'S'], [4, 14, 's']]);
  }
  if (id === 'xiem') { // satchel
    if (tpl === SLEEP) dots(p, [[3, 10, 'S'], [4, 10, 'S'], [3, 11, 's'], [4, 11, 'S'], [5, 9, 's'], [6, 9, 's'], [4, 9, 'B']]);
    else dots(p, [[2, 12, 'S'], [3, 12, 'S'], [2, 13, 's'], [3, 13, 's'], [4, 11, 's'], [5, 11, 's'], [1, 11, 'B']]);
  }
  if (id === 'mun') { // yellow eyes stay shut but black fur keeps a sheen
    for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) if (p.get(x, y) === 'e') p.set(x, y, 'k');
  } else for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) if (p.get(x, y) === 'e') p.set(x, y, 'k');
  if (variant === 'breathe') { // rise the back by 1 px
    const q = p.clone();
    for (let x = 1; x <= 8; x++) {
      let top = -1; for (let y = 0; y < 15; y++) if (q.get(x, y) !== '.') { top = y; break; }
      if (top > 0) { p.set(x, top - 1, q.get(x, top)); p.set(x, top, q.get(x, top + 1) === '.' ? q.get(x, top) : 'a'); }
    }
  }
  if (variant === 'z0') dots(p, [[12, 3, 'w'], [13, 3, 'w'], [13, 4, 'w'], [12, 5, 'w'], [13, 5, 'w']].map(([x, y, c]) => [x, y, c]));
  if (variant === 'z1') dots(p, [[13, 1, 'w'], [14, 1, 'w'], [14, 2, 'w'], [13, 3, 'w'], [14, 3, 'w'], [11, 5, 'w']]);
  return p;
}

// ── portraits: big head, 16×16 bust ──────────────────────────────────────────────
const PORTRAIT = [
  '................',
  '..c.........c...',
  '..cKc......cKb..',
  '..cKKcdadacKKb..',
  '..caaadadaaaab..',
  '.caaaaaaaaaaaab.',
  '.caaeeaaaaeeaab.',
  '.caaeeaaaaeeaab.',
  '.daaaaammaaaaad.',
  '.baaaammKmmaaab.',
  '..baaammmmaaab..',
  '...bbaaaaaabb...',
  '..ccaaaaaaaaab..',
  '.caaaaaaaaaaaab.',
  '.caaaaaammaaaab.',
  '................',
];
function portrait(id) {
  let p = G(PORTRAIT);
  const def = CATS[id];
  if (id !== 'muop') p = recolor(p, { d: 'a' });
  if (id === 'xiem') {
    for (const [x, y] of [[4, 5], [5, 5], [10, 5], [11, 5], [3, 6], [6, 6], [7, 6], [8, 6], [9, 6], [12, 6], [3, 7], [6, 7], [7, 7], [8, 7], [9, 7], [12, 7], [4, 8], [5, 8], [10, 8], [11, 8], [5, 9], [10, 9], [6, 10], [9, 10]]) p.set(x, y, 'd');
    p = recolor(p, { m: 'D' });
    dots(p, [[2, 1, 'd'], [12, 1, 'd'], [2, 2, 'd'], [4, 2, 'd'], [11, 2, 'd'], [13, 2, 'D'], [7, 9, 'D'], [8, 9, 'K'], [6, 9, 'D'], [9, 9, 'D']]);
    dots(p, [[8, 12, 'S'], [7, 13, 'S'], [6, 14, 'S'], [9, 11, 's'], [3, 13, 'S'], [4, 14, 's']]); // strap
    dots(p, [[7, 9, 'D'], [8, 9, 'K'], [7, 10, 'D'], [8, 10, 'D']]);
  }
  if (id === 'muop') dots(p, [[3, 12, 'S'], [4, 12, 'S'], [5, 12, 'S'], [6, 12, 'S'], [7, 12, 'S'], [8, 12, 'S'], [9, 12, 'S'], [10, 12, 'S'], [11, 12, 's'], [12, 12, 's'], [2, 13, 'S'], [3, 13, 's'], [10, 13, 'S'], [11, 13, 'S'], [10, 14, 's']]);
  if (id === 'map') {
    clear(p, 0, 0, 16, 4);
    put(p, G(['....hhhiii....', '..hhiiiiiijj..', '.hiiiiiiiiijj.', 'rrrrrrrrrrrrrr']), 1, 1);
    dots(p, [[1, 4, 'r'], [14, 4, 'r'], [2, 10, 'S'], [13, 10, 'S'], [2, 11, 'S'], [13, 11, 'S']]);
    for (let x = 1; x < 15; x++) if (p.get(x, 4) === 'a' || p.get(x, 4) === 'd') p.set(x, 4, 'b');
  }
  if (id === 'mun') {
    clear(p, 3, 0, 9, 4);
    put(p, G(['.......hi', '......ii.', '....iiii.', '...jSSSSj', 'rrrrrrrrrrrr']), 2, 0);
    dots(p, [[1, 4, 'r'], [13, 4, 'c'], [13, 3, 'K'], [13, 2, 'c'], [2, 2, 'c'], [2, 3, 'K']]);
  }
  // eyes: 2×2 with a glint
  for (const [x, y] of [[4, 6], [10, 6]]) {
    const e = def.map.e === 'k' ? 'k' : 'e';
    dots(p, [[x, y, e], [x + 1, y, 'w'], [x, y + 1, e], [x + 1, y + 1, e]]);
  }
  return p;
}

export default function build() {
  const list = [];
  for (const id of Object.keys(CATS)) {
    const add = (a, i, px) => list.push({ name: `cat_${id}_${a}_${i}`, px: out(id, px) });
    add('idle', 0, pose(id, { tail: 'up', scarf: 0 }));
    add('idle', 1, pose(id, { hy: 1, tail: 'swish', scarf: 0, eyes: 'closed' }));
    WALK.forEach((f, i) => add('walk', i, pose(id, f)));
    ATTACK[id].forEach((f, i) => add('attack', i, pose(id, f)));
    add('hurt', 0, pose(id, { hx: -1, hy: 1, bx: -1, legs: [[0, 0], [1, 0], [0, 0], [-1, 0]], tail: 'bristle', eyes: 'hurt', mouth: true, scarf: 2 }));
    add('death', 0, pose(id, { hx: -1, hy: 2, by: 1, legs: [[0, 0], [1, 0], [0, 0], [1, 0]], tail: 'droop', eyes: 'hurt', mouth: true }));
    add('death', 1, pose(id, { hx: -1, hy: 3, by: 1, bx: -1, legs: [[-1, 0], [1, 0], [1, 0], [2, 0]], tail: 'droop', eyes: 'closed', mouth: true }));
    add('death', 2, lying(id, FLAT));
    add('sleep', 0, lying(id, SLEEP, 'z0'));
    add('sleep', 1, lying(id, SLEEP, 'z1'));
    SKILL[id].forEach((f, i) => add('skill', i, pose(id, f)));
    list.push({ name: `portrait_${id}`, px: fin(recolor(portrait(id), CATS[id].map)) });
  }
  return list;
}
