#!/usr/bin/env node
// Font check (SPEC §8.4, CONTRACT "Font"). Zero dependencies.
//   node tools/verify/fontcheck.mjs [--metrics] [font.ttf ...]
// - Parses the TrueType tables itself (cmap 4/12, head, hhea, OS/2, maxp, loca, glyf, name, fvar).
// - Collects every character of every string literal in crates/game/src/i18n/** (.rs / .ron);
//   always also checks the full Vietnamese alphabet (all tone/vowel marks, both cases), ASCII
//   printable and the UI symbols used in SPEC §7 (· — × … ◆ → etc.).
// - Fails (exit 1) on: missing font/license file, malformed font, any missing glyph, variable font.
// - --metrics prints ascender/descender/lineGap, stacked-diacritic extents (Ặ Ẫ Ỡ …) and the
//   minimum line height that does not clip Vietnamese at 11/13/16 px (× ui_scale 1/1.25/1.5).
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const FONT_DIR = join(ROOT, 'assets', 'fonts');
const I18N_DIR = join(ROOT, 'crates', 'game', 'src', 'i18n');
const LICENSE = join(FONT_DIR, 'LICENSE-ui.txt');
const REQUIRED_FONTS = ['ui.ttf'];
const OPTIONAL_FONTS = ['ui-bold.ttf'];

// ── TrueType parsing ─────────────────────────────────────────────────────────
export function parseFont(buf) {
  const u16 = (o) => buf.readUInt16BE(o), i16 = (o) => buf.readInt16BE(o), u32 = (o) => buf.readUInt32BE(o);
  if (buf.length < 12) throw new Error('file too small');
  const sfnt = u32(0);
  if (sfnt === 0x74746366) throw new Error('TrueType collection (.ttc) not supported');
  if (sfnt === 0x4f54544f) throw new Error('CFF-flavoured OpenType (OTTO); expected TrueType glyf outlines');
  if (sfnt !== 0x00010000 && sfnt !== 0x74727565) throw new Error(`bad sfnt version 0x${sfnt.toString(16)}`);
  const numTables = u16(4), tables = {};
  for (let i = 0; i < numTables; i++) {
    const r = 12 + i * 16;
    const tag = buf.toString('latin1', r, r + 4), off = u32(r + 8), length = u32(r + 12);
    if (off + length > buf.length) throw new Error(`table ${tag} out of bounds`);
    tables[tag] = { off, length };
  }
  for (const t of ['cmap', 'head', 'hhea', 'maxp', 'loca', 'glyf', 'name']) if (!tables[t]) throw new Error(`missing table ${t}`);

  const head = tables.head.off;
  const unitsPerEm = u16(head + 18), indexToLoc = i16(head + 50);
  const numGlyphs = u16(tables.maxp.off + 4);
  const hh = tables.hhea.off;
  const hhea = { ascender: i16(hh + 4), descender: i16(hh + 6), lineGap: i16(hh + 8) };
  let os2 = null;
  if (tables['OS/2']) {
    const o = tables['OS/2'].off;
    os2 = { fsSelection: u16(o + 62), typoAscender: i16(o + 68), typoDescender: i16(o + 70), typoLineGap: i16(o + 72), winAscent: u16(o + 74), winDescent: u16(o + 76) };
    os2.useTypoMetrics = (os2.fsSelection & (1 << 7)) !== 0;
  }

  // cmap: prefer format 12 (full Unicode), fall back to format 4 (BMP)
  const cm = tables.cmap.off, nSub = u16(cm + 2), map = new Map();
  const subs = [];
  for (let i = 0; i < nSub; i++) {
    const r = cm + 4 + i * 8, pid = u16(r), eid = u16(r + 2), so = cm + u32(r + 4);
    subs.push({ pid, eid, so, fmt: u16(so) });
  }
  const pick = subs.find((s) => s.fmt === 12 && (s.pid === 3 && s.eid === 10 || s.pid === 0))
    ?? subs.find((s) => s.fmt === 4 && (s.pid === 3 && s.eid === 1 || s.pid === 0));
  if (!pick) throw new Error(`no Unicode cmap subtable (have ${subs.map((s) => `${s.pid}/${s.eid}:f${s.fmt}`).join(', ')})`);
  if (pick.fmt === 12) {
    const n = u32(pick.so + 12);
    for (let i = 0; i < n; i++) {
      const r = pick.so + 16 + i * 12, s = u32(r), e = u32(r + 4), g = u32(r + 8);
      for (let c = s; c <= e; c++) map.set(c, g + (c - s));
    }
  } else {
    const so = pick.so, segX2 = u16(so + 6), ends = so + 14, starts = ends + segX2 + 2, deltas = starts + segX2, ros = deltas + segX2;
    for (let s = 0; s < segX2 / 2; s++) {
      const end = u16(ends + s * 2), start = u16(starts + s * 2), delta = i16(deltas + s * 2), ro = u16(ros + s * 2);
      for (let c = start; c <= end && c !== 0xffff; c++) {
        let g;
        if (ro === 0) g = (c + delta) & 0xffff;
        else {
          const gi = u16(ros + s * 2 + ro + (c - start) * 2);
          g = gi === 0 ? 0 : (gi + delta) & 0xffff;
        }
        if (g) map.set(c, g);
      }
    }
  }

  const locaAt = (g) => (indexToLoc === 0 ? u16(tables.loca.off + g * 2) * 2 : u32(tables.loca.off + g * 4));
  /** glyph bbox from glyf header (also valid for composites); null for empty glyphs */
  const bbox = (g) => {
    if (g >= numGlyphs) return null;
    const a = locaAt(g), b = locaAt(g + 1);
    if (b <= a) return null;
    const o = tables.glyf.off + a;
    return { xMin: i16(o + 2), yMin: i16(o + 4), xMax: i16(o + 6), yMax: i16(o + 8) };
  };

  // name table (UTF-16BE Windows records preferred)
  const nm = tables.name.off, cnt = u16(nm + 2), strOff = nm + u16(nm + 4), names = {};
  for (let i = 0; i < cnt; i++) {
    const r = nm + 6 + i * 12, pid = u16(r), id = u16(r + 6), ln = u16(r + 8), so = strOff + u16(r + 10);
    if (pid === 3) {
      let s = ''; for (let k = 0; k < ln; k += 2) s += String.fromCharCode(u16(so + k));
      names[id] ??= s;
    } else if (pid === 1 && names[id] === undefined) names[id] = buf.toString('latin1', so, so + ln);
  }
  return { tables, unitsPerEm, numGlyphs, hhea, os2, cmap: map, bbox, names, variable: !!tables.fvar };
}

// ── character sets ───────────────────────────────────────────────────────────
const VI_LOWER = 'aàáảãạăằắẳẵặâầấẩẫậbcdđeèéẻẽẹêềếểễệghiìíỉĩịklmnoòóỏõọôồốổỗộơờớởỡợpqrstuùúủũụưừứửữựvxyỳýỷỹỵfjwz';
const BASELINE = [
  ...Array.from({ length: 95 }, (_, i) => String.fromCharCode(32 + i)), // ASCII printable
  ...VI_LOWER, ...VI_LOWER.toUpperCase(),
  ...'·—–×…◆→←↑↓✓•°±%«»“”‘’', // SPEC §7 UI symbols + typographic punctuation
];

function walk(dir) {
  const out = [];
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (/\.(rs|ron)$/.test(f)) out.push(p);
  }
  return out;
}

/** Every character inside "…" / r"…" / r#"…"# string literals (comments skipped). */
export function stringChars(src) {
  const chars = new Set();
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    if (c === '/' && src[i + 1] === '/') { while (i < n && src[i] !== '\n') i++; continue; }
    if (c === '/' && src[i + 1] === '*') { const e = src.indexOf('*/', i + 2); i = e < 0 ? n : e + 2; continue; }
    if (c === "'" ) { // char literal or lifetime
      const m = /^'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'/u.exec(src.slice(i, i + 16));
      if (m) { const v = m[1].startsWith('\\u{') ? String.fromCodePoint(parseInt(m[1].slice(3, -1), 16)) : m[1].length > 1 && m[1][0] === '\\' ? null : m[1]; if (v) chars.add(v); i += m[0].length; continue; }
      i++; continue;
    }
    const raw = /^r(#*)"/.exec(src.slice(i, i + 12));
    if (raw && (i === 0 || !/[A-Za-z0-9_]/.test(src[i - 1]))) {
      const close = '"' + raw[1], s = i + raw[0].length, e = src.indexOf(close, s);
      for (const ch of src.slice(s, e < 0 ? n : e)) if (ch !== '\n' && ch !== '\r') chars.add(ch);
      i = e < 0 ? n : e + close.length; continue;
    }
    if (c === '"') {
      i++;
      while (i < n && src[i] !== '"') {
        if (src[i] === '\\') {
          const nx = src[i + 1];
          if (nx === 'u') { const m = /^\\u\{([0-9a-fA-F]+)\}/.exec(src.slice(i, i + 12)); if (m) { chars.add(String.fromCodePoint(parseInt(m[1], 16))); i += m[0].length; continue; } }
          if (nx === '\n') { i += 2; while (i < n && /\s/.test(src[i])) i++; continue; }
          if (nx === '"' || nx === '\\' || nx === "'") chars.add(nx);
          i += 2; continue; // \n \t \r \0 are control chars, not glyphs
        }
        const cp = src.codePointAt(i), ch = String.fromCodePoint(cp);
        if (ch !== '\n' && ch !== '\r') chars.add(ch);
        i += ch.length;
      }
      i++; continue;
    }
    i++;
  }
  // `{name}` format arguments contribute braces that are never rendered; keep them anyway
  // (fonts have them) — harmless and stricter.
  return chars;
}

// ── main ─────────────────────────────────────────────────────────────────────
const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  const args = process.argv.slice(2);
  const wantMetrics = args.includes('--metrics');
  const explicit = args.filter((a) => !a.startsWith('--'));
  const errors = [], warnings = [];

  if (!existsSync(LICENSE)) errors.push(`missing ${relative(ROOT, LICENSE)}`);
  else {
    const lic = readFileSync(LICENSE, 'utf8');
    if (!/SIL OPEN FONT LICENSE|Open Font License|CC0|Creative Commons Zero/i.test(lic)) errors.push('LICENSE-ui.txt is neither OFL nor CC0');
  }

  // which characters must be present
  const needed = new Map(); // char → source
  for (const ch of BASELINE) needed.set(ch, 'baseline');
  let i18nFiles = [];
  if (existsSync(I18N_DIR)) i18nFiles = walk(I18N_DIR);
  if (!i18nFiles.length) warnings.push(`no i18n sources under ${relative(ROOT, I18N_DIR)}/ — checked baseline Vietnamese + SPEC §7 symbol set only`);
  for (const f of i18nFiles) for (const ch of stringChars(readFileSync(f, 'utf8'))) if (!needed.has(ch) || needed.get(ch) === 'baseline') needed.set(ch, relative(ROOT, f));
  const fromI18n = [...needed.values()].filter((v) => v !== 'baseline').length;

  const fonts = explicit.length ? explicit : [...REQUIRED_FONTS, ...OPTIONAL_FONTS.filter((f) => existsSync(join(FONT_DIR, f)))];
  for (const f of explicit.length ? [] : REQUIRED_FONTS) if (!existsSync(join(FONT_DIR, f))) errors.push(`missing assets/fonts/${f}`);

  for (const f of fonts) {
    const p = explicit.length ? f : join(FONT_DIR, f);
    if (!existsSync(p)) continue;
    let font;
    try { font = parseFont(readFileSync(p)); } catch (e) { errors.push(`${f}: malformed font: ${e.message}`); continue; }
    if (font.variable) errors.push(`${f}: variable font (fvar present) — ship static instances`);
    const missing = [];
    for (const [ch, src] of needed) {
      const cp = ch.codePointAt(0);
      if (cp < 0x20 || /\s/.test(ch) && cp !== 0x20) continue; // whitespace other than space is layout-only
      if (!font.cmap.has(cp)) missing.push(`U+${cp.toString(16).toUpperCase().padStart(4, '0')} '${ch}' (${src})`);
    }
    console.log(`${f}: ${font.names[4] ?? '?'} — ${font.names[5] ?? '?'}; ${font.numGlyphs} glyphs, ${font.cmap.size} mapped code points; checked ${needed.size} chars (${fromI18n} from ${i18nFiles.length} i18n file(s))`);
    if (missing.length) errors.push(`${f}: ${missing.length} missing glyph(s):\n    ${missing.join('\n    ')}`);
    if (wantMetrics) printMetrics(f, font);
  }

  for (const w of warnings) console.warn(`WARN: ${w}`);
  if (errors.length) { console.error(`\nFONT CHECK FAILED (${errors.length}):\n  ${errors.join('\n  ')}`); process.exit(1); }
  console.log('FONT CHECK OK');
}

function printMetrics(name, font) {
  const upm = font.unitsPerEm, { hhea, os2 } = font;
  console.log(`  unitsPerEm ${upm}; hhea asc ${hhea.ascender} desc ${hhea.descender} gap ${hhea.lineGap}` +
    (os2 ? `; OS/2 typo ${os2.typoAscender}/${os2.typoDescender}/${os2.typoLineGap} win ${os2.winAscent}/${os2.winDescent} USE_TYPO_METRICS=${os2.useTypoMetrics}` : ''));
  const ext = (s) => {
    let yMax = -1e9, yMin = 1e9, top = '', bot = '';
    for (const ch of s) {
      const b = font.bbox(font.cmap.get(ch.codePointAt(0)) ?? 0);
      if (!b) continue;
      if (b.yMax > yMax) { yMax = b.yMax; top = ch; }
      if (b.yMin < yMin) { yMin = b.yMin; bot = ch; }
    }
    return { yMax, yMin, top, bot };
  };
  for (const ch of 'ẶẪỠẤỆỘ') {
    const b = font.bbox(font.cmap.get(ch.codePointAt(0)));
    console.log(`  ${ch}: yMax ${b.yMax} (${(b.yMax / upm).toFixed(3)} em)  yMin ${b.yMin} (${(b.yMin / upm).toFixed(3)} em)`);
  }
  // Ascent/descent the layout engine uses (parley: OS/2 typo when USE_TYPO_METRICS, else hhea).
  const asc = os2?.useTypoMetrics ? os2.typoAscender : hhea.ascender;
  const desc = -(os2?.useTypoMetrics ? os2.typoDescender : hhea.descender);
  const vi = ext(VI_LOWER + VI_LOWER.toUpperCase());
  const br = ext('gjpqy|()[]{}');
  console.log(`  Vietnamese ink: top ${vi.yMax} ('${vi.top}', ${(vi.yMax / upm).toFixed(3)} em) … bottom ${vi.yMin} ('${vi.bot}', ${(vi.yMin / upm).toFixed(3)} em); brackets/descenders bottom ${br.yMin} ('${br.bot}')`);
  console.log(`  layout ascent ${asc} + descent ${desc} = ${((asc + desc) / upm).toFixed(3)} em; diacritic overshoot above ascent ${vi.yMax - asc} units (${((vi.yMax - asc) / upm).toFixed(3)} em)`);
  // parley 0.11 (bevy_text 0.20) splits extra leading in half, giving the *top* the floored half
  // (line_break.rs, Chrome-compatible). The top of the tallest stacked capital must fit inside
  // ascent + floor(leading/2); the bottom inside descent + the rest.
  console.log('  min LineHeight::Px so stacked diacritics are not clipped (Bevy default RelativeToFont(1.2) is too small):');
  for (const px of [11, 13, 16]) {
    const row = [1, 1.25, 1.5].map((s) => {
      const size = px * s, k = size / upm;
      const a = Math.round(asc * k), d = Math.round(desc * k);
      const needTop = Math.max(0, Math.ceil(vi.yMax * k) - a), needBot = Math.max(0, Math.ceil(-Math.min(vi.yMin, br.yMin) * k) - d);
      const lh = a + d + Math.max(2 * needTop, needBot);
      return `${+size.toFixed(2)}px → ${lh}px (${(lh / size).toFixed(2)}×)`;
    });
    console.log(`  ${String(px).padStart(2)} px: ${row.join(' | ')}`);
  }
}
