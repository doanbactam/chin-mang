// Icons (SPEC §7, §8.3): HUD/panel icons 12×12, gear items 16×16 per rarity, skills 16×16, talents 12×12,
// soul-shop 12×12, life marks 10×10, achievements 16×16. All outlined; anchor top-left.
import { Px } from '../pixel.mjs';
import { G, fin, dots, blob, disc, ring, tube, bbox, recolor } from './_lib.mjs';
import { GLYPHS } from './font.mjs';

// centre a grid in a w×h canvas and outline it
function ic(rows, w = 12, h = 12, map = {}) {
  const g = typeof rows === 'object' && rows.d ? rows : G(rows, map), b = bbox(g);
  return fin(new Px(w, h).stamp(g.crop(b.x0, b.y0, b.w, b.h), Math.floor((w - b.w) / 2), Math.floor((h - b.h) / 2)));
}

// ── HUD / panel icons ───────────────────────────────────────────────────────────────
const IC = {
  fish: ['...iiih...ih', '.iiiiihh.ihg', 'ikiiihhhhhg.', 'iiihhhhhggg.', '.hhhhgggg.gg', '...ggg....gf'],
  yarn: ['..HHHo..', '.HoHHoH.', 'HHoHHHoH', 'HoHHHoHH', 'HHHHoHHr', '.rHoHHr.', '..rrrr.H', '.......H'],
  soul: ['..E....E.', '..DE..ED.', '.DEEEEEED', '.DEkEEkED', '.DEEEwEED', '..DEEEED.', '...DEED..', '..D.DD.D.', '.....D...'],
  catnip: ['....d.....', '...dec....', '..decbb...', '.decccb...', '.dcccbb.d.', '..ccbb.dec', '...cb.decb', '...b.dccb.', '..b...cbb.', '.b........'],
  bag: ['...gggg...', '..g....g..', '.hhhhhhhh.', 'hiiiiiiiig', 'hhhhzhhhhg', 'hhhgyghhhg', 'hhhhhhhhhg', 'hhhhhhhhgg', '.ggggggg..'],
  settings: ['....uu....', '.u.uvvu.u.', '..uvttvu..', '.uvt..tvt.', 'uvt....ts.', '.ut....ts.', '..tt..tst.', '.t.tsst.s.', '....ss....'],
  close: ['RR....RR', 'oRR..RRr', '.oRRRRr.', '..oRRr..', '..RRRr..', '.RRrRRr.', 'RRr..RRr', 'Rr....rr'],
  lock: ['..xxxx..', '.x....x.', '.x....x.', 'yyyyyyyy', 'yzzzzzyx', 'yzzkkzyx', 'yzzkzzyx', 'yyyyyyxx', '.xxxxxx.'],
  up: ['...dd...', '..dccc..', '.dccccb.', 'dccccccb', '..dcb...', '..dcb...', '..dcb...', '..bbb...'],
  cats: ['.T....T.', 'TKT..TKT', 'TTTTTTTT', 'TkTTTkTx', 'IIwKwIIx', 'TTwwwTTx', '.xxxxxx.'],
  gear: ['....g....', '...yzy...', '..yzzyx..', '.yzzzyyx.', '.yzyyyyx.', 'yzyyyyyxx', 'xxxxxxxxx', '...xkx...'],
  lives: ['.R.....R.', 'RoR...RoR', 'RooRRRRRR', 'RoRRRRRRr', 'RRRRRRRrr', '.RRRRRrr.', '..RRRrr..', '...Rrr...', '....r....'],
  trophy: ['yzzzzzzx', 'yzyyyyyx', 'xyzyyyx.', '.xyyyyx.', '..xyyx..', '...yx...', '..xyyx..', '.xxxxxx.'],
  sort: ['.c....g.', 'ccc...g.', 'cbcb..g.', '.c....g.', '.c....g.', '.c..ghgh', '.c...hhh', '.c....h.'],
  filter: ['vvvvvvvvv', 'uvvvvvvut', '.uvvvvut.', '..uvvut..', '...uvt...', '...uvt...', '...uvt...', '....tt...'],
  check: ['.......dd', '......dcb', '.....dcb.', 'dd..dcb..', 'bcddcb...', '.bccb....', '..bb.....'],
  back: ['...dd...', '..dcc...', '.dccccdd', 'dccccccb', '.bccbbbb', '..bcb...', '...bb...'],
};

// ── gear items 16×16 (shape + rarity colours + rarity mark) ─────────────────────────
const RAMP = { common: ['t', 'u', 'v'], rare: ['B', 'C', 'D'], epic: ['m', 'M', 'L'], legendary: ['x', 'y', 'z'] };
const ITEM = {
  collar: [
    '..............',
    '....aabbbb....',
    '..abb....bba..',
    '.ab........ba.',
    'ab..........ba',
    'ab..........ba',
    '.ab........ba.',
    '..abbc..cbba..',
    '....abvvba....',
    '......vu......',
    '.....uzyu.....',
    '.....uyyx.....',
    '......xx......',
  ],
  bell: [
    '......k.......',
    '.....uvu......',
    '....bbcbb.....',
    '...bcccbba....',
    '..bcccbbbba...',
    '..bccbbbbba...',
    '..bcbbbbbba...',
    '.bccbbbbbbba..',
    '.bbbbbbbbbba..',
    'aaaaaaaaaaaaa.',
    '....aakkaa....',
    '......ka......',
  ],
  claws: [
    '.c.....c.....c',
    '.cb....cb....b',
    '.cb...ccb...cb',
    'ccb...cb...ccb',
    'cbb..ccb...cb.',
    'cb...cbb..ccb.',
    'cb..ccb...cbb.',
    'bb..cbb..ccb..',
    'hhhhhhhhhhhhg.',
    'iiiiiiiiiiihg.',
    'hhhhhhhhhhhgg.',
  ],
};
function item(kind, r) {
  const [lo, mid, hi] = RAMP[r];
  const map = kind === 'collar' ? { a: lo, b: mid, c: hi } : kind === 'bell' ? { a: lo, b: mid, c: hi } : { b: mid, c: hi };
  const p = new Px(16, 16).stamp(G(ITEM[kind], map), 1, kind === 'claws' ? 3 : 2);
  // rarity mark in the top-left corner: rare 1 pip, epic 2 pips, legendary star
  if (r === 'rare') dots(p, [[1, 1, 'w']]);
  if (r === 'epic') dots(p, [[1, 1, 'w'], [3, 1, 'w']]);
  if (r === 'legendary') dots(p, [[2, 0, 'w'], [1, 1, 'w'], [2, 1, 'z'], [3, 1, 'w'], [2, 2, 'w']]);
  return fin(p);
}

// ── skills 16×16 (round badge + symbol) ─────────────────────────────────────────────
function badge(ramp) { const p = new Px(16, 16); blob(p, 8, 8, 7, 7, ramp); ring(p, 8, 8, 6.2, 7.2, ramp[0]); return p; }
function skill(id) {
  if (id === 'muop') { // Cào Liên Hoàn: three claw slashes
    const p = badge(['x', 'T', 'I']);
    for (let k = 0; k < 3; k++) tube(p, [[4 + k * 3, 3], [5 + k * 3, 7], [4 + k * 3, 12]], (i, s) => (s < 2 ? 'w' : 'E'));
    return fin(p);
  }
  if (id === 'xiem') { // Mưa Xương Cá: bones falling
    const p = badge(['B', 'C', 'D']);
    for (const [x, y] of [[4, 3], [8, 5], [11, 2]]) dots(p, [[x, y, 'w'], [x - 1, y + 1, 'v'], [x + 1, y + 1, 'v'], [x, y + 1, 'w'], [x, y + 2, 'w'], [x - 1, y + 3, 'v'], [x + 1, y + 3, 'v'], [x, y + 3, 'w'], [x, y + 4, 'k']]);
    for (let x = 3; x < 13; x++) p.set(x, 12, x % 2 ? 'E' : 'D');
    return fin(p);
  }
  if (id === 'map') { // Ngồi Đè: helmet slamming down + impact
    const p = badge(['s', 't', 'u']);
    p.stamp(G(['...dddc...', '..dccccb..', '.dccccccb.', 'aaaaaaaaaa']), 3, 4);
    for (let x = 3; x < 13; x++) p.set(x, 11, 'z');
    dots(p, [[2, 10, 'y'], [13, 10, 'y'], [4, 12, 'y'], [11, 12, 'y'], [7, 2, 'w'], [8, 2, 'w'], [7, 1, 'v']]);
    return fin(p);
  }
  // mun: Lời Nguyền Mèo Đen — glowing cursed eye
  const p = badge(['m', 'M', 'L']);
  blob(p, 8, 8, 5, 3, ['z', 'z', 'w']); disc(p, 8, 8, 2, 'A'); dots(p, [[8, 7, 'k'], [8, 8, 'k'], [8, 9, 'k'], [7, 7, 'w']]);
  dots(p, [[3, 4, 'L'], [12, 4, 'L'], [4, 12, 'L'], [12, 12, 'w']]);
  return fin(p);
}

// ── talents 12×12 ───────────────────────────────────────────────────────────────────
const TAL = {
  power: ['.w.w.w..', '.v.v.v..', 'RRoRoRR.', 'RoooooRr', 'RooKooRr', 'RoKKKoRr', '.RRRRRr.'],
  vigor: ['.R..R..', 'RoRRoRR', 'RowRRRr', 'RRRRRRr', '.RRRRr.', '..RRr..', '...r...'],
  frenzy: ['....zy', '...zy.', '..zy..', '.zzzzy', '...zy.', '..zy..', '.zy...', 'zy....'],
  pierce: ['.......D', '......DE', 'v.v..DE.', 'vvvvwE..', 'kwvwvv..', 'vvvvv...', 'v.v.....'],
  thorns: ['.v..v..v.', '.t..t..t.', 'tutututut', 'uuuuuuuut', '.uuuuuut.', '.uuuuuut.', '..uuuut..', '...utt...'],
  hex: ['..LMMM...', '.LMMMMm..', 'LMzzzMMm.', 'MzzkkzMm.', 'MzzkkzMm.', '.MzzzMm..', '..MMMm...'],
};
// ── soul shop 12×12 ─────────────────────────────────────────────────────────────────
const SOUL = {
  claw_sharpen: ['.w....w...', '.vw...vw..', '..vw...vw.', '..vv...vv.', '...v....v.', 'ssttttttt.', 'stuuuuuut.', '.ssssssss.'],
  thick_fur: ['..vvv.....', '.vwwwv.vv.', 'vwwwwwvwwv', 'vwwuwwwwwv', 'uvwwwwuwwu', '.uuvwwwvu.', '..tuuuut..'],
  fat_wallet: ['..ggg....', '.g...g...', 'hhhhhhhh.', 'hiiiiiihg', 'hhhzyhhhg', 'hhhyxhhhg', 'hhhhhhhgg', '.ggggggg.'],
  long_nap: ['...zz.....', '..zz....ww', '.zz......w', '.zz.....w.', '.zz...ww..', '.zzz......', '..zzzz.y..', '....zzy...'],
  auto_catnip: ['...yyyy...', '.yy....y..', 'y...d..yyy', 'y..dec..y.', 'y.decb....', 'y.dcb....y', '..cb.....y', '.y......y.', '..yyyyyy..'],
  head_start: ['k.........', 'kRRRo.....', 'kRRoRR....', 'kRRRRo....', 'k.........', 'k..v.v.v..', 'k.........', 'kk........'],
  lucky_paw: ['.y.y.y..', 'yzyzyzy.', '.y.y.y..', '........', '.yyyyy..', 'yzzzzzy.', 'yzzzzzyx', '.yyyyxx.'],
};
// ── achievements 16×16 ──────────────────────────────────────────────────────────────
const ACH = {
  zone: ['.....g.......', '....ggRRRRo..', '....gRRRRRRo.', '....gRRwRRRo.', '....gRRRRRo..', '....g........', '....g........', '...ggg.......', '.bbbbbbbbc...', 'bcccccccccb..', 'bdddcccdcccb.'],
  boss: ['.z...z...z.', '.yz.yzy.zy.', '.yyyyyyyyy.', '.yRyyCyyRy.', '.xxxxxxxxx.', '..uuuuuuu..', '.uvvuuuvvu.', '.ukvuuukvu.', '.uuuuKuuuu.', '..uwuuwuu..', '...uuuuu...'],
  cat: ['.T.......T.', 'TKT.....TKT', 'TKKTTTTTKKT', 'TTTgTgTTTTx', 'TTTTTTTTTTx', 'TkkTTTTkkTx', 'TkkTTTTkkTx', 'IIIwwKwwIIx', '.IIwwwwwIx.', '..xxxxxxx..'],
  level: ['....d....', '...dcc...', '..dcccb..', '.dcccccb.', '...dcb...', '.z.dcb.z.', 'zwzdcbzwz', '.z.dcb.z.', '...bbb...'],
  loot: ['..zzzzzzzz..', '.zyyyyyyyyx.', 'zyggggggggyx', 'yghhhhhhhhgx', 'yyyyyzzyyyyx', 'yhhhhzxhhhgx', 'yhhhhhhhhhgx', 'yhhhhhhhhhgx', 'xxxxxxxxxxxx'],
  lives: ['.E......E.', 'EDE....EDE', 'DEEDDDDEED', 'DEEEEEEEED', 'DEkEEEEkED', 'DEEEwwEEED', '.DEEEEEED.', '..DEEEED..', '..D.DD.D..', '.D...D..D.'],
  box: ['.iiiiiwiiii.', 'iiiiiiviiiih', 'hhhhhhwhhhhg', 'hhhhhhvhhhhg', 'hhghghhhhhhg', 'hhgggghhhhhg', 'hhhgghhhhhhg', 'gggggggggggg'],
  wealth: ['....ii.......', '..iiihhg.....', 'iikihhgg.ii..', 'hhhhhhgiiihg.', '.gghhgikihhgg', '....iihhhhgg.', '..iikihgggg..', 'ihhhhhhgg....', '.ggggggg.....'],
};

// life marks: gold-rimmed soul coin with the life number in pixel digits
function life(n) {
  const p = new Px(10, 10);
  blob(p, 5, 5, 4, 4, ['B', 'C', 'D']); ring(p, 5, 5, 3.3, 4.2, n === 9 ? 'z' : 'y');
  GLYPHS[n].forEach((r, y) => [...r].forEach((c, x) => { if (c === '#') p.set(4 + x - 1 + 1, 2 + y + 0, 'w'); }));
  return fin(p);
}

function cog() {
  const p = new Px(12, 12);
  for (const [x, y] of [[5, 1], [1, 5], [9, 5], [5, 9]]) dots(p, [[x, y, 'u'], [x + 1, y, 't'], [x, y + 1, 'u'], [x + 1, y + 1, 't']]);
  for (const [x, y] of [[2, 2], [9, 2], [2, 9], [9, 9]]) p.set(x, y, 't');
  blob(p, 6, 6, 3.6, 3.6, ['s', 't', 'v']);
  disc(p, 6, 6, 1.3, '.');
  return fin(p);
}
export default function build() {
  const L = [];
  for (const [k, rows] of Object.entries(IC)) L.push({ name: `ic_${k}`, px: k === 'settings' ? cog() : ic(rows) });
  for (const k of ['collar', 'bell', 'claws']) for (const r of Object.keys(RAMP)) L.push({ name: `item_${k}_${r}`, px: item(k, r) });
  for (const c of ['muop', 'xiem', 'map', 'mun']) L.push({ name: `skill_${c}`, px: skill(c) });
  for (const [k, rows] of Object.entries(TAL)) L.push({ name: `talent_${k}`, px: ic(rows) });
  for (const [k, rows] of Object.entries(SOUL)) L.push({ name: `soul_${k}`, px: ic(rows) });
  for (let i = 1; i <= 9; i++) L.push({ name: `life_${i}`, px: life(i) });
  for (const [k, rows] of Object.entries(ACH)) L.push({ name: `ach_${k}`, px: ic(rows, 16, 16) });
  return L;
}
