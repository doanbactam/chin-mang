// Loot drops (8×8) and the carton box (SPEC §2.7). Anchor: bottom-centre.
import { Px } from '../pixel.mjs';
import { G, fin, dots, put } from './_lib.mjs';

const FISH = G([
  '.iii.h',
  'ikiihg',
  'ihhhgg',
  '.ggg.g',
]);
const YARN = G([
  '.HHo.',
  'HoHKo',
  'HHoHH',
  'rHHoH',
  '.rrH.',
]);
// pouch with drawstring; roles: a base, b shadow, c light, t tie, s spark
const POUCH = [
  '..t.t.',
  '..tt..',
  '.caab.',
  'caaaab',
  'caaabb',
  '.bbbb.',
];
const RAR = {
  common: { a: 'u', b: 't', c: 'v', t: 'g' },
  rare: { a: 'C', b: 'B', c: 'D', t: 'w' },
  epic: { a: 'M', b: 'm', c: 'L', t: 'w' },
  legendary: { a: 'y', b: 'x', c: 'z', t: 'w' },
};
function gear(r) {
  const p = new Px(8, 8).stamp(G(POUCH, RAR[r]), 1, 1);
  if (r === 'rare') dots(p, [[2, 5, 'w'], [3, 5, 'w'], [4, 5, 'w'], [5, 5, 'w']]);                     // single band
  if (r === 'epic') dots(p, [[2, 4, 'w'], [3, 4, 'w'], [4, 4, 'w'], [5, 4, 'w'], [2, 6, 'L'], [3, 6, 'w'], [4, 6, 'w'], [5, 6, 'L']]); // double band
  if (r === 'legendary') dots(p, [[2, 5, 'w'], [3, 5, 'w'], [4, 5, 'w'], [5, 5, 'w'], [6, 1, 'w'], [6, 0, 'z'], [6, 2, 'z']]);       // band + sparkle
  return fin(p);
}

// carton box: front face (h/g), top face (i), tape (w/v), stamped paw (g)
const BOX = G([
  '............',
  '.iiiiiwiiii.',
  'iiiiiiviiiih',
  'hhhhhhwhhhhg',
  'hhhhhhvhhhhg',
  'hhghghhhhhhg',
  'hhgggghhhhhg',
  'hhhgghhhhhhg',
  'hhhhhhhhhhgg',
  'gggggggggggg',
]);
function boxClosed() { return fin(new Px(14, 12).stamp(BOX, 1, 1)); }
function boxOpen(i) {
  const p = new Px(14, 14);
  const body = BOX.crop(0, 3, 12, 7);
  p.stamp(body, 1, 6);
  // inside rim
  dots(p, [...Array(10)].map((_, k) => [2 + k, 6, 'f']));
  if (i >= 1) for (let k = 0; k < 8; k++) p.set(3 + k, 5, i === 2 ? (k % 2 ? 'z' : 'y') : 'f');
  // flaps opening: 0 half up, 1 wide, 2 fully open with glow
  const flaps = [
    [G(['iii', 'iih', 'hh.']), 1, 3, G(['iii', 'hii', '.hh']), 10, 3],
    [G(['.i', 'ii', 'ih', 'ih']), 0, 2, G(['i.', 'ii', 'hi', 'hi']), 12, 2],
    [G(['i', 'i', 'h', 'h']), 0, 3, G(['i', 'i', 'h', 'h']), 13 - 1, 3],
  ][i];
  p.stamp(flaps[0], flaps[1], flaps[2]); p.stamp(flaps[3], flaps[4], flaps[5]);
  if (i === 2) dots(p, [[6, 1, 'z'], [7, 0, 'w'], [8, 1, 'z'], [4, 2, 'z'], [10, 2, 'z'], [7, 3, 'z'], [7, 2, 'y']]);
  if (i === 1) dots(p, [[7, 3, 'z'], [6, 4, 'y']]);
  return fin(p);
}

export default function build() {
  const L = [
    { name: 'drop_fish', px: fin(new Px(8, 8).stamp(FISH, 1, 3)) },
    { name: 'drop_yarn', px: fin(new Px(8, 8).stamp(YARN, 1, 2).set(6, 6, 'H').set(6, 5, 'r')) },
  ];
  for (const r of Object.keys(RAR)) L.push({ name: `drop_gear_${r}`, px: gear(r) });
  L.push({ name: 'box_closed', px: boxClosed() });
  for (let i = 0; i < 3; i++) L.push({ name: `box_open_${i}`, px: boxOpen(i) });
  return L;
}
