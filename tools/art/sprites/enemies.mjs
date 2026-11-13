// Enemies (SPEC §2.4). 16×16, facing LEFT (they walk toward the cats), feet on row 14.
// Each enemy = hand-typed body grid + per-frame parts (legs, tail, wings, antennae), outlined last.
// Role chars: '@' eye (open colour per enemy), '%' mouth (closed = body colour, open = dark red).
import { Px } from '../pixel.mjs';
import { G, put, finUnit as fin, dots, recolor, bbox, seat, hash } from './_lib.mjs';

// shared frame tables -------------------------------------------------------------
// walk: phase 0..3 ; attack: 0 windup, 1 strike ; hurt ; death handled generically
const WALK = [0, 1, 2, 3];
const legStep = (ph, i) => [[-1, 0, 1, 0], [0, 1, 0, -1]][i & 1][ph]; // alternating pairs
const lift = (ph, i) => (ph === 1 && (i & 1) === 0) || (ph === 3 && (i & 1) === 1);

function face(p, st, eyeC, mouthC) {
  for (let y = 0; y < p.h; y++) for (let x = 0; x < p.w; x++) {
    const c = p.d[y][x];
    if (c === '@') p.d[y][x] = st.eye === 'hurt' ? 'w' : st.eye === 'dead' ? 'k' : eyeC;
    if (c === '%') p.d[y][x] = st.mouth ? 'r' : mouthC;
  }
  return p;
}

// ── RAT ────────────────────────────────────────────────────────────────────────────
const RAT = G([
  '...vK.......',
  '..vKHt......',
  '.tuuuuttt...',
  'tu@uuuuuttt.',
  'K%uuuuuuutss',
  '.wvuuuuutss.',
  '..vvvtttss..',
]);
const RAT_TAILS = [
  [[1, 0], [2, -1], [2, -2], [3, -3]],
  [[1, 0], [2, 0], [2, -1], [3, -2], [3, -3]],
  [[1, 0], [2, -1], [3, -1], [3, -2], [3, -3]],
  [[1, 0], [2, 0], [3, -1], [3, -2]],
];
function rat(st) {
  const p = new Px(16, 16), x = 1 + (st.dx || 0), y = 6 + (st.dy || 0);
  [3, 5, 8, 10].forEach((lx, i) => {
    const d = st.legs === false ? 0 : legStep(st.ph ?? 0, i), up = st.legs !== false && lift(st.ph ?? 0, i);
    const far = i === 1 || i === 3, c = far ? 's' : 't';
    p.set(x + lx + d, y + 7, c); if (!up) p.set(x + lx + d, 14, far ? 's' : 'K');
  });
  RAT_TAILS[st.tail ?? 0].forEach(([dx, dy], i) => p.set(x + 11 + dx, y + 4 + dy, i < 2 ? 'H' : 'K'));
  put(p, RAT, x, y);
  if (st.mouth) dots(p, [[x + 1, y + 5, 'w'], [x, y + 5, 'r']]);
  return face(p, st, 'R', 'u');
}

// ── ROACH ──────────────────────────────────────────────────────────────────────────
const ROACH = G([
  '....ffgggg..',
  '..ffghiiihg.',
  '.f@fghhhhhgf',
  '%fffgfhhhhgf',
  '.ffggggfggf.',
]);
const ANT = [
  [[[-1, -1], [-2, -1], [-3, -2], [-4, -2]], [[-1, -1], [-2, -2], [-3, -2], [-4, -3]]],
  [[[-1, -1], [-2, -1], [-3, -1], [-4, -2]], [[-1, -1], [-2, -2], [-3, -3], [-4, -3]]],
  [[[-1, 0], [-2, -1], [-3, -1], [-4, -1]], [[-1, -1], [-2, -1], [-3, -2], [-4, -2]]],
  [[[-1, -1], [-2, -2], [-3, -2], [-4, -3]], [[-1, -1], [-2, -2], [-2, -3], [-3, -4]]],
];
function roach(st) {
  const p = new Px(16, 16), x = 3 + (st.dx || 0), y = 9 + (st.dy || 0), ph = st.ph ?? 0;
  // six legs: three pairs, angled outward
  [[3, -1], [6, 0], [9, 1]].forEach(([lx, lean], i) => {
    const d = st.legs === false ? 0 : legStep(ph, i);
    p.set(x + lx + d, y + 5, 'f'); p.set(x + lx + d + lean, 14, 'f');
  });
  [[5, 0], [8, 1]].forEach(([lx], i) => { const d = st.legs === false ? 0 : -legStep(ph, i); p.set(x + lx + d, y + 5, 'g'); });
  const [a1, a2] = ANT[st.ant ?? ph];
  a1.forEach(([dx, dy]) => p.set(x + 2 + dx, y + 2 + dy, 'f'));
  a2.forEach(([dx, dy]) => p.set(x + 3 + dx, y + 1 + dy, 'g'));
  put(p, ROACH, x, y);
  if (st.wings) dots(p, [[x + 6, y - 1, 'i'], [x + 7, y - 2, 'h'], [x + 8, y - 2, 'i'], [x + 9, y - 2, 'h'], [x + 10, y - 1, 'g'], [x + 8, y - 1, 'h'], [x + 9, y - 1, 'h'], [x + 7, y - 1, 'h']]);
  return face(p, st, 'R', 'f');
}

// ── CROW (flies; the view lifts it 10 px) ─────────────────────────────────────────
const CROW = G([
  '...nss......',
  '..nsttn.....',
  '.ns@tsn.....',
  'yyysssssnn..',
  '.%nsssssssnn',
  '...nssssnnnn',
  '....nnnnn..n',
  '.....y.y....',
]);
const WINGS = [
  { g: ['......tu', '....ttsu', '..ttssn.', '.tssnn..'], dy: -3 },          // up
  { g: ['ttttuuu.', 'snnnssst'], dy: 2 },                                    // mid
  { g: ['.tsssn..', '..tssnn.', '...snn..', '....n...'], dy: 3 },            // down
];
function crow(st) {
  const p = new Px(16, 16), x = 2 + (st.dx || 0), y = 6 + (st.dy || 0);
  put(p, CROW, x, y);
  const w = WINGS[st.wing ?? 0];
  put(p, G(w.g), x + 5, y + w.dy, true);
  if (st.dive) { p.set(x + 4, y + 8, 'y'); p.set(x + 6, y + 8, 'y'); }
  if (st.mouth) dots(p, [[x, y + 3, 'y'], [x + 1, y + 4, 'y'], [x + 1, y + 3, 'r']]);
  return face(p, st, 'z', 's');
}

// ── SNAKE ──────────────────────────────────────────────────────────────────────────
function snake(st) {
  const p = new Px(16, 16), ph = st.ph ?? 0, x0 = 3 + (st.dx || 0), lift = st.rear ?? 0;
  // body: tube from neck (x0+2) to tail tip (x 14), sine wave travelling backward
  for (let x = x0 + 2; x <= 14; x++) {
    const t = (x - x0) / 11, amp = 1.2 * (1 - t * 0.3);
    const cy = Math.round(12 + amp * Math.sin(ph * Math.PI / 2 + (x - x0) * 0.85));
    const thick = x > 12 ? 1 : 2;
    for (let j = 0; j < thick; j++) p.set(x, cy - j, j === thick - 1 ? (thick === 1 ? 'c' : 'd') : 'c');
    p.set(x, cy + 1, x > 13 ? '.' : 'e');
    if (thick === 2 && ((x + ph) % 3 === 0)) p.set(x, cy, 'b');
  }
  // neck rising to the head
  const hy = 7 - lift + (st.dy || 0);
  for (let y = hy + 3; y <= 12; y++) { p.set(x0 + 2, y, 'c'); p.set(x0 + 3, y, 'b'); p.set(x0 + 1, y, 'e'); }
  put(p, G(['.dddc.', 'dd@dcb', 'cccccb', '%eeebb']), x0 - 1, hy);
  if (st.tongue) dots(p, [[x0 - 2, hy + 3, 'R'], [x0 - 3, hy + 3, 'R'], [x0 - 4, hy + 2, 'R'], [x0 - 4, hy + 4, 'R']]);
  else if (ph % 2 === 1) p.set(x0 - 2, hy + 3, 'R');
  return face(p, st, 'z', 'e');
}

// ── OWL ────────────────────────────────────────────────────────────────────────────
const OWL = G([
  '.f.......f.',
  '.gf.....fg.',
  '.ghhhhhhhg.',
  'gwiiiiiiiwg',
  'gizzihizzig',
  'gi@zihi@zig',
  'giiiiyiiiig',
  'ghhhhyhhhhg',
  'ghihhhhihhg',
  '.ghhihhhhg.',
  '..ggggggg..',
]);
function owl(st) {
  const p = new Px(16, 16), x = 2 + (st.dx || 0), y = 3 + (st.dy || 0);
  const fl = st.flap ?? 0;
  // wings
  for (let j = 0; j < 5; j++) { p.set(x - 1 - (fl > 0 && j < 3 ? 1 : 0), y + 4 + j - fl, 'f'); p.set(x + 11 + (fl > 0 && j < 3 ? 1 : 0), y + 4 + j - fl, 'f'); }
  put(p, OWL, x, y);
  // feet
  if (!st.air) dots(p, [[x + 3, 14, 'y'], [x + 4, 14, 'x'], [x + 6, 14, 'y'], [x + 7, 14, 'x']]);
  else dots(p, [[x + 3, y + 11, 'y'], [x + 7, y + 11, 'y']]);
  if (st.mouth) dots(p, [[x + 5, y + 7, 'r']]);
  if (st.feather) dots(p, [[x - 1, y + 2, 'i'], [x - 2, y + 2, 'h'], [x - 2, y + 1, 'i'], [x, y + 1, 'h']]);
  return face(p, st, 'k', 'y');
}

// ── DOG ────────────────────────────────────────────────────────────────────────────
const DOG = G([
  '..ff..........',
  '.fggf.........',
  '.fhhhh........',
  '.fh@hhh.......',
  'hhhhhhhgi.....',
  'kiiihhhhffgggh',
  '.%iihhhhhgfhhg',
  '..wiihhhhhhhgg',
  '....ghhhgghhg.',
]);
const DOG_TAIL = [[[0, 0], [1, -1], [1, -2]], [[0, 0], [1, -1], [2, -2]], [[0, 0], [1, 0], [2, -1]], [[0, 0], [1, -1], [2, -1]]];
function dog(st) {
  const p = new Px(16, 16), x = 0 + (st.dx || 0) + 1, y = 4 + (st.dy || 0), ph = st.ph ?? 0;
  [[4, 'g', 'h'], [6, 'f', 'g'], [9, 'g', 'h'], [11, 'f', 'g']].forEach(([lx, far, near], i) => {
    const d = st.legs === false ? 0 : legStep(ph, i), up = st.legs !== false && lift(ph, i), isFar = i === 1 || i === 3;
    for (let yy = y + 9; yy <= (up ? 13 : 14); yy++) p.set(x + lx + d, yy, yy === 14 || (up && yy === 13) ? 'f' : isFar ? far : near);
    if (!isFar) for (let yy = y + 9; yy <= 13; yy++) p.set(x + lx + d + 1, yy, 'g');
  });
  (DOG_TAIL[st.tail ?? ph]).forEach(([dx, dy], i) => p.set(Math.min(14, x + 12 + dx), y + 4 + dy, i === 2 ? 'i' : 'h'));
  put(p, DOG, x, y);
  if (st.mouth) dots(p, [[x, y + 6, 'r'], [x + 1, y + 6, 'r'], [x, y + 7, 'w'], [x + 1, y + 7, 'w']]);
  if (st.growl) dots(p, [[x + 2, y + 2, 'k'], [x + 3, y + 2, 'k']]);
  return face(p, st, 'R', 'i');
}

// ── TICK ───────────────────────────────────────────────────────────────────────────
const TICK = G([
  '.....rrrrr...',
  '...rRRoRRRr..',
  '..rRoRRRRRRr.',
  '.frRRRHRRRRRr',
  'f@fRRRRRRRRrr',
  '%ffrRRRRRRrr.',
  '..f.rrrrrrr..',
]);
function tick(st) {
  const p = new Px(16, 16), x = 1 + (st.dx || 0), y = 7 + (st.dy || 0), ph = st.ph ?? 0;
  [[3, -1], [5, 0], [8, 0], [10, 1]].forEach(([lx, lean], i) => {
    const d = st.legs === false ? 0 : legStep(ph, i);
    p.set(x + lx + d, y + 7, 'f'); p.set(x + lx + d + lean, y + 8 > 14 ? 14 : y + 8, 'n');
  });
  put(p, TICK, x, y);
  if (st.mouth) dots(p, [[x - 0, y + 5, 'r'], [x + 0, y + 6, 'f']]);
  if (st.fat) dots(p, [[x + 12, y + 2, 'r'], [x + 12, y + 3, 'R'], [x + 6, y - 1, 'r']]);
  return face(p, st, 'z', 'f');
}

// ── VACUUM ROBOT ───────────────────────────────────────────────────────────────────
const VAC = G([
  '.....CD.......',
  '...uvvvvvvu...',
  '.tuvvvvvvvvut.',
  'tkkttttttttts.',
  's@k@tttttttts.',
  'tsssssssssssst',
  '.ssssssssssss.',
]);
function vacuum(st) {
  const p = new Px(16, 16), x = 1 + (st.dx || 0), y = 6 + (st.dy || 0), ph = st.ph ?? 0;
  put(p, VAC, x, y);
  // wheels
  for (const wx of [3, 10]) dots(p, [[x + wx, y + 7, 'k'], [x + wx + 1, y + 7, 'n'], [x + wx, 14, 'n'], [x + wx + 1, 14, 'k'], [x + wx + (ph & 1), y + 7, 's']]);
  // spinning side brush at the front
  const B = [[[-1, 7], [0, 8], [1, 7]], [[-1, 8], [0, 7], [1, 8]], [[-1, 7], [0, 7], [0, 8]], [[0, 8], [1, 7], [-1, 8]]][ph];
  B.forEach(([dx, dy]) => p.set(x + 1 + dx, Math.min(14, y + dy), 'y'));
  p.set(x + 1, Math.min(14, y + 7), 'x');
  // status LED on top blinks
  p.set(x + 8, y + 1, ph & 1 ? 'R' : 'o');
  if (st.mouth) dots(p, [[x + 1, y + 5, 'k'], [x + 2, y + 5, 'R'], [x + 3, y + 5, 'k']]);
  if (st.dust) dots(p, [[x - 0, y + 3, 'u'], [x - 0, y + 1, 'v']]);
  return face(p, st, 'R', 'k');
}

const DEFS = {
  rat: { fn: rat, walk: (ph) => ({ ph, tail: ph, dy: ph & 1 ? -1 : 0 }), atk: [{ dx: 1, tail: 1, legs: false }, { dx: -1, mouth: true, tail: 3, dy: 1 }], hurt: { dx: 1, dy: -1, eye: 'hurt', tail: 2, legs: false } },
  roach: { fn: roach, walk: (ph) => ({ ph }), atk: [{ dx: 1, ant: 2, legs: false, wings: true }, { dx: -1, mouth: true, ant: 1 }], hurt: { dx: 1, dy: -1, eye: 'hurt', ant: 3, legs: false, wings: true } },
  crow: { fn: crow, walk: (ph) => ({ wing: [0, 1, 2, 1][ph], dy: [0, 1, 1, 0][ph], ph }), atk: [{ wing: 0, dx: 1, dy: 0 }, { wing: 2, dx: -1, dy: 2, mouth: true, dive: true }], hurt: { wing: 0, dx: 1, eye: 'hurt' } },
  snake: { fn: snake, walk: (ph) => ({ ph }), atk: [{ ph: 1, dx: 1, rear: 2 }, { ph: 2, dx: -2, rear: -1, mouth: true, tongue: true }], hurt: { ph: 3, dx: 1, rear: 1, eye: 'hurt' } },
  owl: { fn: owl, walk: (ph) => ({ dy: [0, -1, -2, -1][ph], flap: [0, 1, 1, 0][ph], air: ph === 2 }), atk: [{ flap: 1, dy: -1, dx: 0 }, { flap: 0, mouth: true, feather: true, dx: 1 }], hurt: { flap: 1, dx: 1, eye: 'hurt' } },
  dog: { fn: dog, walk: (ph) => ({ ph, dy: ph & 1 ? -1 : 0 }), atk: [{ dx: 1, legs: false, growl: true, tail: 0 }, { dx: -1, mouth: true, growl: true, tail: 2 }], hurt: { dx: 1, eye: 'hurt', legs: false, tail: 3 } },
  tick: { fn: tick, walk: (ph) => ({ ph, dy: ph & 1 ? -1 : 0 }), atk: [{ dx: 1, legs: false, fat: false }, { dx: -1, mouth: true, fat: true }], hurt: { dx: 1, dy: -1, eye: 'hurt', legs: false } },
  vacuum: { fn: vacuum, walk: (ph) => ({ ph, dy: 0 }), atk: [{ ph: 1, dx: 1, dust: true }, { ph: 2, dx: -1, mouth: true }], hurt: { ph: 0, dx: 1, dy: -1, eye: 'hurt' } },
};

// generic death: knocked up → on its back → flattened puff
function death(fn, i, base) {
  const raw = fn({ ...base, eye: 'dead', legs: false, mouth: true });
  if (i === 0) return raw.shift(1, -2);
  const b = bbox(raw), flipped = new Px(16, 16).stamp(raw.crop(b.x0, b.y0, b.w, b.h), 0, 0);
  let up = new Px(16, 16); for (let y = 0; y < b.h; y++) for (let x = 0; x < b.w; x++) up.d[b.h - 1 - y][x] = flipped.d[y][x];
  up = seat(up, 16, 16, 14, 0);
  if (i === 1) return up;
  // squash to ~half height, with pale dust puffs at the sides
  const sq = new Px(16, 16), ub = bbox(up), keep = [];
  for (let y = ub.y0; y <= ub.y1; y++) if ((y - ub.y0) % 2 === 0 || y === ub.y1) keep.push(y);
  keep.forEach((sy, k) => { for (let x = 0; x < 16; x++) sq.d[14 - (keep.length - 1 - k)][x] = up.d[sy][x]; });
  const sb = bbox(sq);
  dots(sq, [[sb.x0 - 1, 14, 'v'], [sb.x0 - 1, 13, 'w'], [sb.x1 + 1, 14, 'v'], [sb.x1 + 1, 13, 'u']]);
  return sq;
}

export default function build() {
  const list = [];
  for (const [id, d] of Object.entries(DEFS)) {
    const add = (a, i, px) => list.push({ name: `en_${id}_${a}_${i}`, px: fin(px) });
    WALK.forEach((ph) => add('walk', ph, d.fn(d.walk(ph))));
    d.atk.forEach((st, i) => add('attack', i, d.fn(st)));
    add('hurt', 0, d.fn(d.hurt));
    for (let i = 0; i < 3; i++) add('death', i, death(d.fn, i, d.walk(0)));
  }
  return list;
}
