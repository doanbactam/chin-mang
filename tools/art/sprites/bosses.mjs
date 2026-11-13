// Bosses (SPEC §2.4). 32×32, facing LEFT, feet on row 30, outline added last.
// Built from shaded blobs (volume lit top-left) + hand-typed detail grids (faces, crown, collar, visor),
// re-posed per frame: walk 4, attack 3, special 3, hurt 1, death 4.
import { Px } from '../pixel.mjs';
import { G, put, finUnit as fin, dots, blob, tube, bbox, clear } from './_lib.mjs';

const GREY = ['t', 'u', 'v', 's'], PINK = ['H', 'K', 'w'], RED = ['r', 'R', 'o', 'm'], GOLD = ['x', 'y', 'z'];
const step = (ph) => [[-1, 1], [0, 0], [1, -1], [0, 1]][ph & 3];
const bob = (ph) => (ph & 1 ? -1 : 0);

// ── RAT KING: fat crowned rat in an ermine-trimmed cape, fork sceptre ───────────────
function ratKing(st) {
  const p = new Px(32, 32), x = st.dx || 0, y = (st.dy || 0) + bob(st.ph ?? 0) + 1, [l, r] = step(st.ph ?? 0);
  // tail (behind everything): long pink curve
  tube(p, [[24 + x, 26 + y], [28 + x, 27 + y], [30 + x, 24 + y - (st.ph & 1)], [29 + x, 20 + y]], (i, k) => (k & 1 ? 'K' : 'H'));
  // cape behind body
  blob(p, 20 + x, 21 + y, 8, 8.5, RED);
  for (let i = 0; i < 4; i++) p.set(24 + x + i * 1, 29 + y, i & 1 ? 'r' : 'm');
  // feet
  blob(p, 12 + x + l, 29.5, 3, 1.6, PINK); blob(p, 19 + x + r, 29.5, 3, 1.6, ['H', 'K', 'K']);
  // body
  blob(p, 15 + x, 21 + y, 8, 8, GREY);
  blob(p, 13 + x, 23 + y, 4.5, 5, ['u', 'v', 'w']); // belly
  // ermine collar
  for (let i = 0; i < 13; i++) { const cx = 10 + x + i, cy = 14 + y + Math.round(Math.abs(i - 6) * 0.25); p.set(cx, cy, 'w'); p.set(cx, cy + 1, i % 3 === 1 ? 'k' : 'w'); }
  // head
  blob(p, 11 + x, 10 + y, 7, 5.5, GREY);
  blob(p, 5 + x, 12 + y, 3.5, 2.5, GREY); // snout
  // ear
  blob(p, 16 + x, 4.5 + y, 3.2, 3.2, ['t', 'u', 'v']); blob(p, 16 + x, 4.8 + y, 1.8, 2, ['H', 'K', 'K']);
  // face details
  const eye = st.eye ?? 'open';
  if (eye === 'open') dots(p, [[7 + x, 8 + y, 'k'], [8 + x, 8 + y, 'R'], [7 + x, 9 + y, 'k'], [8 + x, 9 + y, 'k'], [6 + x, 7 + y, 's'], [7 + x, 7 + y, 's'], [8 + x, 6 + y, 's'], [9 + x, 6 + y, 's']]);
  if (eye === 'hurt') dots(p, [[7 + x, 8 + y, 'k'], [8 + x, 9 + y, 'k'], [9 + x, 8 + y, 'k']]);
  if (eye === 'dead') dots(p, [[7 + x, 8 + y, 'k'], [9 + x, 8 + y, 'k'], [8 + x, 9 + y, 'k'], [7 + x, 10 + y, 'k'], [9 + x, 10 + y, 'k']]);
  dots(p, [[1 + x, 11 + y, 'K'], [2 + x, 11 + y, 'H'], [1 + x, 12 + y, 'H']]); // nose
  dots(p, [[3 + x, 10 + y, 'v'], [4 + x, 9 + y, 'v']]); // whisker glint
  if (st.mouth) dots(p, [[3 + x, 14 + y, 'r'], [4 + x, 14 + y, 'r'], [5 + x, 14 + y, 'r'], [4 + x, 15 + y, 'w'], [5 + x, 15 + y, 'w'], [3 + x, 13 + y, 'k']]);
  else dots(p, [[4 + x, 14 + y, 'w'], [5 + x, 14 + y, 'w'], [3 + x, 13 + y, 's'], [4 + x, 13 + y, 's']]);
  // crown
  const cr = G(['.z..z..z.', '.yz.yz.yz', 'yyyyyyyyy', 'xyRyyCyyx', 'xxxxxxxxx'], {});
  put(p, cr, 7 + x + (st.crownDx || 0), 1 + y + (st.crownDy || 0));
  // arm + fork sceptre
  const a = st.arm ?? 0; // 0 hold, 1 raised, 2 swing, 3 high
  const hand = [[6, 19], [8, 15], [3, 20], [8, 12]][a];
  const top = [[5, 6], [12, 2], [-0, 24], [8, 1]][a];
  const hx = hand[0] + x, hy = hand[1] + y;
  if (a === 2) tube(p, [[hx + 6, hy - 4], [hx - 4, hy + 2]], 'h');
  else tube(p, [[hx, hy + 9], [top[0] + x, top[1] + y + 6]], (i, k) => (k % 3 ? 'h' : 'g'));
  const fork = a === 2 ? G(['zy.', 'yz..', '.yzzz', 'yz..', 'zy.']) : G(['z.z.z', 'y.y.y', 'yyyyy', '.xyx.', '..y..']);
  if (a === 2) put(p, fork, hx - 9, hy - 1); else put(p, fork, top[0] + x - 2, top[1] + y);
  blob(p, hx + 1, hy + 1, 2, 1.8, GREY);
  return p;
}

// ── CROW LORD: huge crow, tattered violet mantle, gold monocle + chain ───────────────
const CROW = ['n', 's', 't', 'k'];
function crowLord(st) {
  const p = new Px(32, 32), x = st.dx || 0, y = (st.dy || 0) + bob(st.ph ?? 0), [l, r] = step(st.ph ?? 0);
  const wing = st.wing ?? 0; // 0 folded, 1 half, 2 spread up
  // legs
  for (const [lx, o] of [[13, l], [18, r]]) { tube(p, [[lx + x, 25 + y], [lx + x + o, 29]], 'x'); dots(p, [[lx + x + o - 1, 30, 'y'], [lx + x + o - 2, 30, 'x'], [lx + x + o + 1, 30, 'y']]); }
  // tail feathers
  for (let i = 0; i < 4; i++) tube(p, [[22 + x, 20 + y], [29 + x, 22 + y + i * 2 - (st.ph & 1)]], i & 1 ? 'n' : 's');
  // back wing (spread)
  if (wing === 2) {
    for (let i = 0; i < 6; i++) tube(p, [[20 + x, 14 + y], [24 + x + i, 3 + y + i * 2]], i & 1 ? 's' : 't', 1);
    blob(p, 22 + x, 10 + y, 4, 6, CROW);
  }
  // mantle
  blob(p, 18 + x, 19 + y, 8.5, 8, ['m', 'M', 'L']);
  for (let i = 0; i < 8; i++) if (i & 1) p.set(11 + x + i * 2, 27 + y, '.'); else p.set(11 + x + i * 2, 27 + y, 'm');
  // body
  blob(p, 15 + x, 18 + y, 7, 7.5, CROW);
  // folded/half wing on top
  if (wing === 0) blob(p, 18 + x, 18 + y, 5, 5, ['n', 's', 't']);
  if (wing === 1) { blob(p, 21 + x, 13 + y, 5, 4, ['n', 's', 't']); for (let i = 0; i < 4; i++) tube(p, [[22 + x, 13 + y], [27 + x, 9 + y + i * 2]], 's'); }
  // head
  blob(p, 10 + x, 9 + y, 5.5, 5.5, CROW);
  // crest feathers
  tube(p, [[11 + x, 4 + y], [13 + x, 1 + y]], 's'); tube(p, [[13 + x, 4 + y], [16 + x, 2 + y]], 't'); tube(p, [[9 + x, 4 + y], [10 + x, 2 + y]], 't');
  // beak
  const open = st.mouth;
  put(p, G(open ? ['..yyy', 'yyyyx', 'y..xx', '...k.', 'yyyxx', '..xx.'] : ['...yy', '.yyyx', 'yyyxx', '..yxx', '...x.']), 1 + x, 8 + y);
  // eye + monocle
  const eye = st.eye ?? 'open';
  dots(p, [[8 + x, 7 + y, 'z'], [9 + x, 7 + y, 'z'], [10 + x, 8 + y, 'z'], [10 + x, 9 + y, 'z'], [9 + x, 10 + y, 'z'], [8 + x, 10 + y, 'z'], [7 + x, 9 + y, 'z'], [7 + x, 8 + y, 'z']]);
  if (eye === 'open') dots(p, [[8 + x, 8 + y, 'R'], [9 + x, 8 + y, 'w'], [8 + x, 9 + y, 'r'], [9 + x, 9 + y, 'R']]);
  else if (eye === 'hurt') dots(p, [[8 + x, 8 + y, 'k'], [9 + x, 9 + y, 'k'], [9 + x, 8 + y, 'E'], [8 + x, 9 + y, 'E']]);
  else dots(p, [[8 + x, 8 + y, 'k'], [9 + x, 9 + y, 'k'], [9 + x, 8 + y, 'k'], [8 + x, 9 + y, 'k']]);
  tube(p, [[9 + x, 11 + y], [11 + x, 15 + y], [14 + x, 16 + y]], (i, k) => (k & 1 ? 'y' : 'z'));
  // gold brooch on mantle
  dots(p, [[14 + x, 16 + y, 'z'], [15 + x, 16 + y, 'y'], [14 + x, 17 + y, 'y'], [15 + x, 17 + y, 'x']]);
  return p;
}

// ── EEL: electric eel rising from a puddle, glowing spots, sparks ───────────────────
const EELC = ['B', 'C', 'D', 'n'];
function eel(st) {
  const p = new Px(32, 32), x = st.dx || 0, ph = st.ph ?? 0, rise = st.rise ?? 0, y = st.dy || 0;
  // puddle
  blob(p, 17 + x, 29.5, 13, 2, ['B', 'C', 'D']);
  dots(p, [[8 + x + ph, 29, 'E'], [22 + x - ph, 30, 'E'], [15 + x, 29, 'E']]);
  // body: S-curve of overlapping discs from the puddle up to the head
  const pts = [];
  for (let i = 0; i <= 12; i++) {
    const t = i / 12;
    pts.push([20 + x + Math.round(5 * Math.sin(t * 3.4 + ph * Math.PI / 2) * (1 - t * 0.4)) - Math.round(t * 6 * (1 + rise * 0.15)), 28 - t * (17 + rise) + y]);
  }
  pts.forEach(([cx, cy], i) => blob(p, cx, cy, 3.4 - i * 0.08, 3.2, EELC));
  // spots along the body
  pts.forEach(([cx, cy], i) => { if (i % 2 === 0 && i < 12) p.set(Math.round(cx), Math.round(cy), (i + ph) % 4 === 0 ? 'z' : 'y'); });
  // dorsal fin (orange) along the back
  pts.slice(2, 11).forEach(([cx, cy], i) => p.set(Math.round(cx + 3), Math.round(cy - 1), i & 1 ? 'x' : 'y'));
  // head
  const [hx, hy] = pts[12];
  blob(p, hx - 2, hy - 1, 5, 3.6, EELC);
  const mouth = st.mouth;
  if (mouth) {
    clear(p, hx - 7, hy, 5, 2);
    dots(p, [[hx - 6, hy, 'r'], [hx - 5, hy, 'r'], [hx - 4, hy, 'r'], [hx - 3, hy, 'r'], [hx - 6, hy + 1, 'w'], [hx - 4, hy + 1, 'w'], [hx - 5, hy - 1, 'w'], [hx - 3, hy - 1, 'w']]);
  } else dots(p, [[hx - 6, hy + 1, 'n'], [hx - 5, hy + 1, 'n'], [hx - 4, hy + 1, 'n']]);
  const eye = st.eye ?? 'open';
  if (eye === 'open') dots(p, [[hx - 3, hy - 3, 'z'], [hx - 2, hy - 3, 'z'], [hx - 3, hy - 2, 'k'], [hx - 2, hy - 2, 'z']]);
  else dots(p, [[hx - 3, hy - 2, 'k'], [hx - 2, hy - 3, 'k'], [hx - 2, hy - 2, 'k']]);
  // whiskers / barbels
  dots(p, [[hx - 7, hy + 2, 'D'], [hx - 8, hy + 3, 'D']]);
  // sparks
  const sp = st.spark ?? 0;
  if (sp) {
    const arcs = sp === 1 ? [[[hx + 3, hy - 4], [hx + 5, hy - 7], [hx + 4, hy - 9]]] :
      [[[hx - 1, hy - 4], [hx - 2, hy - 7], [hx + 1, hy - 9]], [[hx + 4, hy - 2], [hx + 7, hy - 3], [hx + 9, hy - 6]], [[hx - 7, hy - 2], [hx - 9, hy - 5], [hx - 8, hy - 7]], [[pts[6][0] + 4, pts[6][1]], [pts[6][0] + 7, pts[6][1] - 1], [pts[6][0] + 9, pts[6][1] + 1]]];
    for (const a of arcs) tube(p, a, (i, k) => (k & 1 ? 'z' : 'w'));
  }
  return p;
}

// ── MASTIFF: massive guard dog, spiked collar, jowls, scar ──────────────────────────
const TAN = ['g', 'h', 'i', 'f'], DARK = ['f', 'g', 'h'];
function mastiff(st) {
  const p = new Px(32, 32), x = (st.dx || 0) + 1, y = (st.dy || 0) + bob(st.ph ?? 0), ph = st.ph ?? 0;
  const [l, r] = step(ph), hd = st.headUp || 0;
  // back legs + tail
  tube(p, [[27 + x, 18 + y], [Math.min(29, 28 + x), 15 + y - (ph & 1)], [Math.min(30, 29 + x), 13 + y]], 'g');
  for (const [lx, o, c] of [[22, -r, 'g'], [25, r, 'f']]) { tube(p, [[lx + x, 24 + y], [lx + x + o, 28]], c, 3); dots(p, [[lx + x + o - 1, 30, 'f'], [lx + x + o, 30, 'f'], [lx + x + o + 1, 30, 'f'], [lx + x + o + 2, 30, 'f']]); }
  for (const [lx, o, c] of [[9, l, 'g'], [13, -l, 'h']]) { tube(p, [[lx + x, 24 + y], [lx + x + o, 28]], c, 3); dots(p, [[lx + x + o - 1, 30, 'g'], [lx + x + o, 30, 'i'], [lx + x + o + 1, 30, 'i'], [lx + x + o + 2, 30, 'g']]); }
  // body
  blob(p, 19 + x, 20 + y, 10.5, 7, TAN);
  blob(p, 21 + x, 16 + y, 6, 3, ['g', 'g', 'h']); // saddle
  // head
  const hy = 13 + y - hd;
  blob(p, 9 + x, hy, 7.5, 6.5, TAN);
  blob(p, 5 + x, hy + 3, 4.5, 3.4, ['g', 'h', 'i']); // muzzle/jowls
  blob(p, 4 + x, hy + 2, 3.5, 2, DARK); // dark mask
  dots(p, [[1 + x, hy + 1, 'k'], [2 + x, hy + 1, 'k'], [1 + x, hy + 2, 'n'], [2 + x, hy, 's']]); // nose
  // ear
  blob(p, 13 + x, hy - 3, 2.5, 3.5, DARK);
  // eye + brow + scar
  const eye = st.eye ?? 'open';
  if (eye === 'open') dots(p, [[7 + x, hy - 1, 'R'], [8 + x, hy - 1, 'k'], [6 + x, hy - 2, 'f'], [7 + x, hy - 2, 'f'], [8 + x, hy - 3, 'f']]);
  else dots(p, [[6 + x, hy - 1, 'k'], [7 + x, hy - 1, 'k'], [8 + x, hy - 2, 'k']]);
  dots(p, [[10 + x, hy - 3, 'o'], [10 + x, hy - 2, 'o'], [11 + x, hy - 1, 'o']]);
  // mouth / teeth
  if (st.mouth) {
    blob(p, 4 + x, hy + 5, 3.2, 2, ['r', 'r', 'R']);
    dots(p, [[2 + x, hy + 4, 'w'], [5 + x, hy + 4, 'w'], [3 + x, hy + 7, 'w'], [6 + x, hy + 7, 'w'], [4 + x, hy + 6, 'H']]);
  } else dots(p, [[2 + x, hy + 5, 'f'], [3 + x, hy + 5, 'f'], [4 + x, hy + 5, 'f'], [3 + x, hy + 6, 'w']]);
  // spiked collar
  for (let i = 0; i < 9; i++) {
    const cx = 11 + x + i, cy = hy + 5 + Math.round(i * 0.3);
    p.set(cx, cy, 'R'); p.set(cx, cy + 1, 'r');
    if (i % 2 === 0) p.set(cx, cy - 1, 'v');
  }
  dots(p, [[15 + x, hy + 8, 'z'], [15 + x, hy + 9, 'y']]); // tag
  return p;
}

// ── VACUUM TITAN: upright giant vacuum robot, hose arm, dust bag, visor ──────────────
const STEEL = ['t', 'u', 'v', 's'];
function titan(st) {
  const p = new Px(32, 32), x = st.dx || 0, y = (st.dy || 0) + bob(st.ph ?? 0), ph = st.ph ?? 0;
  // dust bag on the back
  blob(p, 24 + x, 15 + y, 5, 7, ['r', 'R', 'o']);
  dots(p, [[24 + x, 12 + y, 'z'], [25 + x, 13 + y, 'z'], [24 + x, 14 + y, 'y']]);
  // treads
  for (let i = 0; i < 18; i++) { const c = (i + ph) % 3 === 0 ? 'k' : 's'; p.set(8 + x + i, 29, c); p.set(8 + x + i, 30, (i + ph) % 3 === 1 ? 'n' : 's'); }
  for (const wx of [10, 16, 22]) blob(p, wx + x + 0.5, 28.5, 2, 2, ['s', 't', 'u']);
  // body (rounded box) with hazard stripe
  for (let yy = 8; yy <= 26; yy++) for (let xx = 8; xx <= 23; xx++) {
    const corner = (yy === 8 || yy === 26) && (xx === 8 || xx === 23);
    if (corner) continue;
    let c = 'u';
    if (xx <= 9 || yy <= 9) c = 'v';
    if (xx >= 22 || yy >= 25) c = 't';
    if (yy === 22 || yy === 23) c = (xx + yy) % 4 < 2 ? 'y' : 'k';
    p.set(xx + x, yy + y, c);
  }
  // head dome
  blob(p, 15.5 + x, 7 + y, 7, 4.5, STEEL);
  // visor
  const eye = st.eye ?? 'open', vy = 6 + y;
  for (let i = 0; i < 10; i++) { p.set(9 + x + i, vy, 'k'); p.set(9 + x + i, vy + 1, 'n'); }
  if (eye === 'open') dots(p, [[10 + x + (st.look || 0), vy, 'R'], [11 + x + (st.look || 0), vy, 'o'], [12 + x + (st.look || 0), vy, 'R'], [11 + x + (st.look || 0), vy + 1, 'R']]);
  else if (eye === 'hurt') dots(p, [[10 + x, vy, 'z'], [12 + x, vy, 'z'], [11 + x, vy + 1, 'z']]);
  else dots(p, [[10 + x, vy, 's'], [11 + x, vy, 't'], [12 + x, vy, 's']]);
  // antenna + light
  tube(p, [[18 + x, 3 + y], [20 + x, 0 + y + 1]], 's');
  p.set(20 + x, 1 + y, ph & 1 ? 'R' : 'o');
  // grille + gauge
  for (let i = 0; i < 4; i++) dots(p, [[11 + x + i * 2, 13 + y, 's'], [11 + x + i * 2, 15 + y, 's'], [11 + x + i * 2, 17 + y, 's']]);
  dots(p, [[18 + x, 13 + y, 'D'], [19 + x, 13 + y, 'C'], [18 + x, 14 + y, 'C'], [19 + x, 14 + y, 'B']]);
  // hose arm with nozzle
  const arm = st.arm ?? 0; // 0 down, 1 forward, 2 raised, 3 suck (forward, wide nozzle)
  const end = [[4, 24], [2, 16], [5, 4], [3, 17]][arm];
  const ctrl = [[5, 18], [4, 14], [3, 10], [4, 15]][arm];
  tube(p, [[9 + x, 14 + y], [ctrl[0] + x, ctrl[1] + y], [end[0] + x + 1, end[1] + y]], (i, k) => (k % 2 ? 's' : 't'), 2);
  const nz = arm === 3 ? G(['ttk', 'uvk', 'uuk', 'uvk', 'ttk']) : G(['tk', 'vk', 'vk', 'tk']);
  put(p, nz, end[0] + x - 1, end[1] + y - (arm === 3 ? 2 : 1));
  if (st.mouth) dots(p, [[13 + x, 10 + y, 'k'], [14 + x, 10 + y, 'k'], [15 + x, 10 + y, 'k'], [16 + x, 10 + y, 'k']]);
  if (arm === 3) for (const [a, b] of [[-1, 0], [-3, -2], [-3, 2], [-5, -1], [-5, 1]]) p.set(end[0] + x + a - 1, end[1] + y + b, 'v');
  return p;
}

// ── frame tables ────────────────────────────────────────────────────────────────────
const DEFS = {
  rat_king: { fn: ratKing,
    attack: [{ arm: 1, dx: 1 }, { arm: 2, dx: -1, mouth: true }, { arm: 0, dx: 0, mouth: true }],
    special: [{ arm: 3, dy: -1, mouth: true }, { arm: 3, dy: -1, mouth: true, crownDx: -1, ph: 1 }, { arm: 1, mouth: true }],
    hurt: { dx: 2, eye: 'hurt', mouth: true, crownDx: 1, crownDy: 1 } },
  crow_lord: { fn: crowLord,
    attack: [{ wing: 1, dx: 1, dy: -1 }, { wing: 2, dx: -2, mouth: true }, { wing: 1, dx: -1 }],
    special: [{ wing: 2, dy: -1 }, { wing: 2, dy: -2, mouth: true }, { wing: 1, dx: -2, dy: 1, mouth: true }],
    hurt: { wing: 1, dx: 2, eye: 'hurt', mouth: true } },
  eel: { fn: eel,
    attack: [{ ph: 1, rise: -2, dx: 1 }, { ph: 2, rise: 2, dx: -2, mouth: true }, { ph: 3, rise: 0, mouth: true }],
    special: [{ ph: 0, rise: 1, spark: 1 }, { ph: 1, rise: 2, spark: 2, mouth: true }, { ph: 2, rise: 1, spark: 2 }],
    hurt: { ph: 1, rise: -1, dx: 2, eye: 'hurt', mouth: true } },
  mastiff: { fn: mastiff,
    attack: [{ dx: 0, headUp: 1 }, { dx: -1, mouth: true, headUp: -1 }, { dx: 0, mouth: true }],
    special: [{ headUp: 2, dx: 1 }, { headUp: 3, mouth: true, dy: -1 }, { headUp: 2, mouth: true }],
    hurt: { dx: 1, eye: 'hurt', headUp: 1 } },
  vacuum_titan: { fn: titan,
    attack: [{ arm: 2, dx: 1 }, { arm: 1, dx: -1, mouth: true }, { arm: 0 }],
    special: [{ arm: 3, look: 0 }, { arm: 3, ph: 1, mouth: true, dx: 1 }, { arm: 3, ph: 2, mouth: true, look: -1 }],
    hurt: { dx: 2, eye: 'hurt', arm: 2 } },
};

// death: stagger, sink with x-eyes, slump, collapse with dust
function death(fn, i) {
  const st = { eye: i ? 'dead' : 'hurt', mouth: true };
  const base = fn(st);
  if (i === 0) return fn({ ...st, dx: 1, dy: -1 });
  const keepRatio = [1, 0.8, 0.6, 0.42][i], b = bbox(base), rows = [];
  for (let k = 0; k < Math.round(b.h * keepRatio); k++) rows.push(b.y0 + Math.floor((k * b.h) / Math.round(b.h * keepRatio)));
  const out = new Px(32, 32), shift = i === 3 ? 1 : 0;
  rows.forEach((sy, k) => { for (let x = 0; x < 32; x++) { const c = base.get(x - shift, sy); if (c !== '.') out.set(x, 30 - (rows.length - 1 - k), c); } });
  if (i >= 2) { const ob = bbox(out); dots(out, [[ob.x0 - 1, 30, 'v'], [ob.x0 - 2, 29, 'u'], [ob.x1 + 1, 30, 'v'], [ob.x1 + 2, 29, 'u'], [ob.x0 + 2, ob.y0 - 2, 'z'], [ob.x1 - 3, ob.y0 - 3, 'z']]); }
  return out;
}

export default function build() {
  const list = [];
  for (const [id, d] of Object.entries(DEFS)) {
    const add = (a, i, px) => list.push({ name: `boss_${id}_${a}_${i}`, px: fin(px) });
    for (let ph = 0; ph < 4; ph++) add('walk', ph, d.fn({ ph }));
    d.attack.forEach((s, i) => add('attack', i, d.fn(s)));
    d.special.forEach((s, i) => add('special', i, d.fn(s)));
    add('hurt', 0, d.fn(d.hurt));
    for (let i = 0; i < 4; i++) add('death', i, death(d.fn, i));
  }
  return list;
}
