// Pixel digits for floating damage numbers (SPEC §8.3): fd_{0-9,dot,K,M,B,T,plus,minus}, 4×6.
// Glyph body 3×5 in light `w` (the view tints it, e.g. yellow for crits) + 1 px dark drop shadow to the
// right and below so it reads on white, black and busy backgrounds. Anchor top-left; advance = 4 px.
import { Px } from '../pixel.mjs';

export const GLYPHS = {
  0: ['###', '#.#', '#.#', '#.#', '###'],
  1: ['.#.', '##.', '.#.', '.#.', '###'],
  2: ['##.', '..#', '.#.', '#..', '###'],
  3: ['##.', '..#', '.#.', '..#', '##.'],
  4: ['#.#', '#.#', '###', '..#', '..#'],
  5: ['###', '#..', '##.', '..#', '##.'],
  6: ['.##', '#..', '###', '#.#', '###'],
  7: ['###', '..#', '.#.', '.#.', '.#.'],
  8: ['###', '#.#', '.#.', '#.#', '###'],
  9: ['###', '#.#', '###', '..#', '##.'],
  dot: ['...', '...', '...', '...', '.#.'],
  K: ['#.#', '#.#', '##.', '#.#', '#.#'],
  M: ['#.#', '###', '###', '#.#', '#.#'],
  B: ['##.', '#.#', '##.', '#.#', '##.'],
  T: ['###', '.#.', '.#.', '.#.', '.#.'],
  plus: ['...', '.#.', '###', '.#.', '...'],
  minus: ['...', '...', '###', '...', '...'],
};
export function glyph(rows, body = 'w', shade = 'k') {
  const p = new Px(4, 6);
  rows.forEach((r, y) => [...r].forEach((c, x) => { if (c === '#') { if (p.get(x + 1, y + 1) === '.') p.set(x + 1, y + 1, shade); if (p.get(x + 1, y) === '.') p.set(x + 1, y, shade); if (p.get(x, y + 1) === '.') p.set(x, y + 1, shade); } }));
  rows.forEach((r, y) => [...r].forEach((c, x) => { if (c === '#') p.set(x, y, body); }));
  return p;
}
export default function build() {
  return Object.entries(GLYPHS).map(([k, rows]) => ({ name: `fd_${k}`, px: glyph(rows) }));
}
