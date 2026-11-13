// Per-sound intent checks for Chín Mạng ("listen proxy"). Each check gets the samples and the
// analyse() result and returns [ok, description]. Thresholds are deliberately loose: they catch a
// sound that is structurally wrong (falling instead of rising, noisy instead of tonal, too long,
// a meow without a pitch glide), not matters of taste.
import { SR, filter } from './sfxr.mjs';
import { pitchTrack, pitchAt, f0Track, fft } from './analyze.mjs';

const st = (a, b) => 12 * Math.log2(b / a);

/**
 * Pitch (strongest partial) in an early vs a late window. Windows are fractions of the duration,
 * or absolute seconds when `abs`. `hp` high-passes first so a low pad/bass does not mask the
 * melody line of a jingle.
 */
function glide(x, a, [e0, e1, l0, l1], { abs = false, hp = 0 } = {}) {
  const y = hp ? filter(filter(x, { type: 'hp', f0: hp }), { type: 'hp', f0: hp }) : x;
  const pt = pitchTrack(y, 0.02), d = abs ? 1 : a.dur;
  return [pitchAt(pt, e0 * d, e1 * d, 0.02), pitchAt(pt, l0 * d, l1 * d, 0.02)];
}
const rising = (min = 1.5, w = [0, 0.25, 0.55, 0.9], o) => (x, a) => { const [p, q] = glide(x, a, w, o); return [q > p * semitone(min), `rising (${p | 0}→${q | 0} Hz)`]; };
const falling = (min = 1.5, w = [0, 0.3, 0.55, 0.95], o) => (x, a) => { const [p, q] = glide(x, a, w, o); return [q < p / semitone(min), `falling (${p | 0}→${q | 0} Hz)`]; };
const semitone = (s) => Math.pow(2, s / 12);
const and = (...cs) => (x, a) => { const r = cs.map((c) => c(x, a)); return [r.every((v) => v[0]), r.map((v) => v[1]).join(', ')]; };
// laptop speakers roll off below ~200 Hz: a frequent sound needs real energy above 250 Hz
const audible = (min = 280) => (x, a) => [a.centroid >= min, `centroid ${a.centroid | 0} ≥ ${min} Hz (small speakers)`];
const durIn = (lo, hi) => (x, a) => [a.dur >= lo && a.dur <= hi, `${a.dur.toFixed(2)} s ∈ [${lo}, ${hi}]`];
const sharp = (max = 0.02) => (x, a) => [a.attack <= max, `attack ${(a.attack * 1000) | 0} ms`];
const dull = (max) => (x, a) => [a.centroid < max, `centroid ${a.centroid | 0} < ${max} Hz`];
// Spectral flatness over 150–4500 Hz (the band every SFX keeps after the 5.5 kHz master low-pass;
// analyse().flatness spans up to 9 kHz and collapses to ~0 on low-passed sounds), floored at
// −60 dB re the frame maximum, energy-weighted over 1024-sample frames. ~0 tone … 1 white noise.
function flatness(x) {
  const N = 1024, k0 = Math.ceil((150 * N) / SR), k1 = Math.floor((4500 * N) / SR);
  let fw = 0, ew = 0;
  for (let off = 0; off < x.length; off += N / 2) {
    const re = new Float64Array(N), im = new Float64Array(N);
    for (let i = 0; i < N && off + i < x.length; i++) re[i] = x[off + i] * (0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (N - 1)));
    fft(re, im);
    let pmax = 0, e = 0; const p = new Float64Array(k1 + 1);
    for (let k = k0; k <= k1; k++) { p[k] = re[k] * re[k] + im[k] * im[k]; pmax = Math.max(pmax, p[k]); e += p[k]; }
    if (e < 1e-9) continue;
    let ls = 0, as = 0; for (let k = k0; k <= k1; k++) { const v = Math.max(p[k], pmax * 1e-6); ls += Math.log(v); as += v; }
    const nb = k1 - k0 + 1; fw += (Math.exp(ls / nb) / (as / nb)) * e; ew += e;
  }
  return ew ? fw / ew : 0;
}
const tonal = (max = 0.08) => (x) => { const f = flatness(x); return [f < max, `tonal (flatness ${f.toFixed(3)})`]; };
/**
 * Noise-like (whoosh, rustle): among audible 10 ms frames in [t0, t1) s, at least `min` have no
 * detectable f0 (band-passed noise has low *flatness* but no periodicity).
 */
const noisy = (t0 = 0, t1 = Infinity, min = 0.6) => (x) => {
  const fr = 0.01, W = Math.round(0.02 * SR), f = f0Track(x, fr, 150, 1600, 0.02);
  let gmax = 0; for (const v of x) gmax = Math.max(gmax, Math.abs(v));
  let loud = 0, unp = 0;
  f.forEach((v, i) => {
    const s = Math.round(i * fr * SR); if (s / SR < t0 || s / SR >= t1) return;
    let e = 0; for (let k = s; k < Math.min(x.length, s + W); k++) e = Math.max(e, Math.abs(x[k]));
    if (e >= gmax * 0.15) { loud++; if (!v) unp++; }
  });
  const r = loud ? unp / loud : 0;
  return [loud > 0 && r >= min, `noise-like ${(r * 100) | 0}% of audible frames in ${t0}–${Number.isFinite(t1) ? t1 : 'end'} s`];
};

/** spectral centroid of the window [t0, t1) seconds */
function centroidAt(x, t0, t1) {
  const N = 1024, s = Math.floor(t0 * SR), e = Math.min(x.length, Math.floor(t1 * SR));
  let num = 0, den = 0;
  for (let off = s; off < e; off += N / 2) {
    const re = new Float64Array(N), im = new Float64Array(N);
    for (let i = 0; i < N && off + i < e; i++) re[i] = x[off + i] * (0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (N - 1)));
    fft(re, im);
    for (let k = 1; k < N / 2; k++) { const p = re[k] * re[k] + im[k] * im[k]; num += p * (k * SR) / N; den += p; }
  }
  return den ? num / den : 0;
}

/**
 * Meow: a voiced, pitched cry. f0 (autocorrelation, not the loudest partial) must be in the cat
 * range, rise ≥ `rise` semitones to a peak in the middle of the voiced part, then fall ≥ `fall`;
 * the vowel must "close" (centroid of the tail lower than the brightest middle window).
 */
const meow = ({ rise = 3, fall = 1, lo = 300, hi = 1300 } = {}) => (x, a) => {
  const fr = 0.02, f = f0Track(x, fr);
  const idx = f.map((v, i) => (v ? i : -1)).filter((i) => i >= 0);
  if (idx.length < 5) return [false, `not voiced (${idx.length} pitched frames)`];
  const first = idx[0], last = idx[idx.length - 1], span = last - first + 1;
  const voiced = idx.length / span;
  const med = (arr) => { const s = arr.filter(Boolean).sort((p, q) => p - q); return s[s.length >> 1]; };
  const head = med(f.slice(first, first + Math.max(2, Math.round(span * 0.15))));
  const tail = med(f.slice(last + 1 - Math.max(2, Math.round(span * 0.15)), last + 1));
  let pk = first; for (const i of idx) if (f[i] > f[pk]) pk = i;
  // peak position on a 3-frame median to ignore single-frame glitches
  const smooth = idx.map((i) => [i, med([f[i - 1], f[i], f[i + 1]])]);
  let sp = smooth[0]; for (const s of smooth) if (s[1] > sp[1]) sp = s;
  const pos = (sp[0] - first) / span;
  const all = med(f);
  const d = a.dur, cMid = Math.max(centroidAt(x, d * 0.3, d * 0.5), centroidAt(x, d * 0.15, d * 0.35)), cEnd = centroidAt(x, d * 0.75, d * 0.95);
  const ok = voiced >= 0.7 && all >= lo && all <= hi && st(head, sp[1]) >= rise && st(tail, sp[1]) >= fall && pos >= 0.15 && pos <= 0.8 && cEnd < cMid;
  return [ok, `f0 ${head | 0}→${sp[1] | 0}→${tail | 0} Hz (+${st(head, sp[1]).toFixed(1)}/−${st(tail, sp[1]).toFixed(1)} st, peak at ${(pos * 100) | 0}%, voiced ${(voiced * 100) | 0}%), vowel closes ${cMid | 0}→${cEnd | 0} Hz`];
};

export const INTENT = {
  hit_1: and(durIn(0.05, 0.25), sharp(), dull(1800), audible()),
  hit_2: and(durIn(0.05, 0.25), sharp(), dull(1800), audible()),
  hit_3: and(durIn(0.05, 0.25), sharp(), dull(1800), audible()),
  crit: and(durIn(0.1, 0.45), sharp(), (x, a) => [a.centroid > 500, `has sparkle (centroid ${a.centroid | 0} Hz)`]),
  cat_attack_muop: and(durIn(0.08, 0.3), noisy(0, 0.06)),
  cat_attack_xiem: and(durIn(0.06, 0.25), rising(3)),
  cat_attack_map: and(durIn(0.08, 0.35), sharp(), dull(1500), audible(250)),
  cat_attack_mun: and(durIn(0.08, 0.3), tonal()),
  skill_muop: and(durIn(0.3, 1.0), noisy(0, 0.25)),
  skill_xiem: and(durIn(0.3, 1.0), falling(5), tonal()),
  skill_map: and(durIn(0.3, 1.0), sharp(0.03), dull(1000), audible(220)),
  skill_mun: and(durIn(0.5, 1.0), falling(3)),
  enemy_die: and(durIn(0.1, 0.4), falling(3, [0, 0.03, 0.08, 0.13], { abs: true })),
  boss_appear: and(durIn(0.8, 1.6), dull(1000), audible(220), (x, a) => [a.sustained > 0.3, `sustained ${(a.sustained * 100) | 0}%`]),
  boss_die: and(durIn(0.8, 1.6), falling(4, [0, 0.12, 0.3, 0.45], { abs: true }), rising(7, [0.55, 0.65, 0.9, 1.0], { abs: true, hp: 400 })),
  cat_down: and(durIn(0.3, 0.9), falling(4), meow({ rise: 0, fall: 4 })),
  level_up: and(durIn(0.3, 0.8), rising(7), tonal()),
  unlock: and(durIn(0.5, 1.2), rising(2), tonal()),
  coin: and(durIn(0.1, 0.25), rising(3), tonal()),
  yarn: and(durIn(0.1, 0.35), tonal()),
  gear_drop: and(durIn(0.1, 0.4), sharp(), tonal(0.12)),
  gear_legendary: and(durIn(0.8, 1.8), tonal()),
  box_land: and(durIn(0.08, 0.35), sharp(), dull(1000), audible()),
  box_open: and(durIn(0.3, 0.8), noisy(0, 0.18), rising(4, [0.2, 0.28, 0.3, 0.4], { abs: true, hp: 400 })),
  catnip: and(durIn(0.6, 1.2), rising(4)),
  ui_click: and(durIn(0.01, 0.08), sharp(0.01)),
  ui_tab: and(durIn(0.02, 0.12), sharp(0.01), rising(3, [0, 0.015, 0.035, 0.05], { abs: true })),
  ui_error: and(durIn(0.1, 0.35), falling(2), dull(1200)),
  prestige: and(durIn(1.2, 2.5), rising(12, [0, 0.08, 0.8, 0.95], { abs: true, hp: 400 }), tonal()),
  achievement: and(durIn(0.6, 1.5), rising(7, [0, 0.08, 0.3, 0.45], { abs: true, hp: 400 }), tonal()),
  stage_clear: and(durIn(0.3, 0.8), rising(4), tonal()),
  meow_1: and(durIn(0.35, 0.8), meow()),
  meow_2: and(durIn(0.2, 0.5), meow({ rise: 3, fall: 0.5 })),
  meow_3: and(durIn(0.5, 1.0), meow()),
  meow_4: and(durIn(0.2, 0.5), meow({ rise: 3, fall: 1, hi: 1500 })),
};
