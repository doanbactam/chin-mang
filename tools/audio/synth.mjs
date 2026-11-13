#!/usr/bin/env node
// Chín Mạng audio generator (SPEC §9). Zero dependencies, deterministic.
//   node tools/audio/synth.mjs [--quiet]
// Writes mono 22 050 Hz 16-bit PCM WAVs + audio.json into assets/audio/.
//
// Sound palette (P1 "không làm phiền"): soft, rounded, short. Sine/triangle bodies, filtered
// pulse only as colour, every SFX low-passed (≤ 5.5 kHz), no raw white-noise hiss. Loudness is
// made uniform by gated RMS (−16 dBFS ≈ −16 LUFS for these short mono sounds) with a soft
// limiter at −1 dBFS; "quiet by default" lives in audio.json `gain_db` playback trims.
import { mkdirSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SR, len, tone, mix, encodeWav, noteHz as n, removeDC, filter, oscSample, rng, trimTail, fade } from './sfxr.mjs';
import { renderSong, songLength, chord, perBar } from './tracker.mjs';
import { analyse, gatedRmsDb, highBand, f0Track } from './analyze.mjs';
import { INTENT } from './intent.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
export const OUT_DIR = join(ROOT, 'assets', 'audio');
export const TARGET_RMS_DB = -16; // gated RMS target for every file
export const PEAK_CEIL_DB = -1;

// ═════════════════════════════════════════════════════════════════════════════
// helpers
// ═════════════════════════════════════════════════════════════════════════════
const perc = (decay, a = 0.002, hold = 0) => ({ a, hold, decay });
const semis = (s) => Math.pow(2, s / 12);

/** Piecewise-linear curve over normalised time: pts [[u, v], ...], u ∈ [0,1]. */
function curve(pts) {
  return (u) => {
    if (u <= pts[0][0]) return pts[0][1];
    for (let i = 1; i < pts.length; i++) {
      if (u <= pts[i][0]) {
        const [u0, v0] = pts[i - 1], [u1, v1] = pts[i];
        const k = (u - u0) / (u1 - u0);
        return v0 + (v1 - v0) * (0.5 - 0.5 * Math.cos(Math.PI * k)); // eased segments
      }
    }
    return pts[pts.length - 1][1];
  };
}

/** Constant-0-dB-peak band-pass biquad whose centre/bandwidth follow functions of time. */
function dynBandpass(x, fcFn, bwFn) {
  const out = new Float32Array(x.length);
  let x1 = 0, x2 = 0, y1 = 0, y2 = 0, b0 = 0, b2 = 0, a1 = 0, a2 = 0;
  for (let i = 0; i < x.length; i++) {
    if ((i & 7) === 0) {
      const t = i / SR, fc = Math.min(SR * 0.45, fcFn(t)), q = fc / Math.max(30, bwFn(t));
      const w = (2 * Math.PI * fc) / SR, al = Math.sin(w) / (2 * q), a0 = 1 + al;
      b0 = al / a0; b2 = -al / a0; a1 = (-2 * Math.cos(w)) / a0; a2 = (1 - al) / a0;
    }
    const v = x[i], y = b0 * v + b2 * x2 - a1 * y1 - a2 * y2;
    x2 = x1; x1 = v; y2 = y1; y1 = y; out[i] = y;
  }
  return out;
}

/**
 * Cat voice: glottal-ish source (band-limited saw + a little pulse) with an f0 contour,
 * vibrato and breath, through 3 moving formants (m → i → a → u = "m-i-a-o").
 * o: { dur, f0: [[u,Hz]...], F1/F2/F3: [[u,Hz]...], amp: [[u,0..1]...], vib, breath, seed, purr }
 */
function catVoice(o) {
  const N = len(o.dur), src = new Float32Array(N), r = rng(o.seed ?? 5);
  const f0 = curve(o.f0), amp = curve(o.amp);
  let p = 0, jit = 0;
  for (let i = 0; i < N; i++) {
    const t = i / SR, u = i / N;
    if ((i & 63) === 0) jit = jit * 0.6 + (r() * 2 - 1) * 0.4; // slow random wobble (~0.1 st)
    const vib = o.vib ? Math.sin(2 * Math.PI * o.vib.rate * t) * o.vib.depth * Math.min(1, t / 0.12) : 0;
    const f = f0(u) * semis(vib + jit * 0.12);
    const dt = f / SR;
    let v = 0.75 * oscSample('saw', p, dt) + 0.25 * oscSample('square', p, dt, 0.3);
    if (o.purr) v *= 1 - o.purr.depth * 0.5 * (1 + Math.sin(2 * Math.PI * o.purr.rate * t)) * Math.max(0, 1 - u / o.purr.until);
    v += (r() * 2 - 1) * (o.breath ?? 0.12);
    p += dt; if (p >= 1) p -= 1;
    src[i] = v * amp(u);
  }
  const at = (pts) => { const c = curve(pts); return (t) => c(t / o.dur); };
  const f1 = dynBandpass(src, at(o.F1), (t) => 90 + 0.12 * at(o.F1)(t));
  const f2 = dynBandpass(src, at(o.F2), (t) => 140 + 0.1 * at(o.F2)(t));
  const f3 = dynBandpass(src, at(o.F3 ?? [[0, 3300], [1, 3300]]), () => 380);
  // nasal "m" onset: low-passed source dominates the first ~12 %
  const nasal = filter(src, { type: 'lp', f0: 500 });
  const out = new Float32Array(N);
  for (let i = 0; i < N; i++) {
    const u = i / N, m = Math.max(0, 1 - u / 0.14);
    out[i] = (1 - m) * (1.0 * f1[i] + 0.75 * f2[i] + 0.22 * f3[i]) + m * 0.9 * nasal[i] + 0.1 * src[i] * (1 - m) * 0.3;
  }
  return out;
}

/** Soft "bell/plink": sine + quiet inharmonic partial, exponential decay. */
const plink = (hz, dur = 0.35, decay = 0.09, bright = 0.25) => mix([
  [tone({ wave: 'sine', dur, f0: hz, env: perc(decay, 0.003) })],
  [tone({ wave: 'sine', dur, f0: hz * 2.76, env: perc(decay * 0.45, 0.002) }), 0, bright],
  [tone({ wave: 'triangle', dur, f0: hz * 2, env: perc(decay * 0.6, 0.002) }), 0, bright * 0.5],
]);

/** Arpeggio of plinks/tones: notes [[name, startSec, gain?]] */
const arp = (notes, voice) => mix(notes.map(([nm, at, g = 1]) => [voice(n(nm)), at, g]));

/** Soft paw/bonk: sine with pitch drop + a muffled noise tap. */
const thump = (f0, f1, decay, tap = 0.5, seed = 1, tapLp = 1400) => mix([
  [tone({ wave: 'sine', dur: decay * 5, f0, f1, slideTime: decay * 2, env: perc(decay, 0.003) })],
  [tone({ wave: 'pnoise', dur: 0.05, f0: 900, filters: [{ type: 'lp', f0: tapLp }], env: perc(0.01, 0.001), seed }), 0, tap],
]);

/** Filtered-noise "fwip" whose band sweeps f0 → f1. */
const swish = (dur, f0, f1, q = 2.5, seed = 9, env = { a: 0.02, d: 0.02, s: 0.8, r: dur * 0.6 }) =>
  tone({ wave: 'noise', dur, filters: [{ type: 'bp', f0, f1, q }, { type: 'lp', f0: 4000 }], env, seed });

// ═════════════════════════════════════════════════════════════════════════════
// SFX table. gain_db = suggested playback trim (≤ 0) relative to the uniform file loudness;
// frequent sounds sit lower so an idle session stays calm (P1).
// ═════════════════════════════════════════════════════════════════════════════
export const SFX = {
  // ── combat ────────────────────────────────────────────────────────────────
  // soft paw "pok" on a cushion: mid-range body (audible on laptop speakers) + muffled tap
  hit_1: { gain_db: -10, make: () => mix([[thump(460, 230, 0.028, 0.6, 11, 2200)], [tone({ wave: 'triangle', dur: 0.06, f0: 700, f1: 520, env: perc(0.012) }), 0.002, 0.25]]) },
  hit_2: { gain_db: -10, make: () => mix([[thump(400, 200, 0.032, 0.65, 12, 1900)], [tone({ wave: 'triangle', dur: 0.06, f0: 600, f1: 450, env: perc(0.014) }), 0.002, 0.25]]) },
  hit_3: { gain_db: -10, make: () => mix([[thump(520, 260, 0.026, 0.5, 13, 2400)], [tone({ wave: 'triangle', dur: 0.06, f0: 780, f1: 590, env: perc(0.012) }), 0.003, 0.3]]) },
  // crit: firmer thump + a little sparkle ping on top
  crit: { gain_db: -7, make: () => mix([
    [thump(380, 170, 0.045, 0.6, 14, 2200)],
    [plink(n('E6'), 0.3, 0.06, 0.15), 0.012, 0.45],
    [plink(n('B6'), 0.25, 0.05, 0.1), 0.05, 0.3],
  ]) },
  // Mướp: claw swipe — short rising noise "fwip" + tiny paw tap
  cat_attack_muop: { gain_db: -10, make: () => mix([
    [swish(0.12, 900, 2600, 3, 21, { a: 0.03, d: 0.02, s: 0.7, r: 0.06 })],
    [thump(320, 180, 0.02, 0.2, 22), 0.07, 0.6],
  ]) },
  // Xiêm: throws a fish bone — quick rising triangle blip + light whoosh
  cat_attack_xiem: { gain_db: -10, make: () => mix([
    [tone({ wave: 'triangle', dur: 0.1, f0: 520, f1: 1040, env: perc(0.03, 0.003) }), 0, 0.8],
    [swish(0.1, 1500, 3000, 2, 23, { a: 0.01, d: 0.02, s: 0.5, r: 0.06 }), 0.01, 0.5],
  ]) },
  // Mập: belly bonk on the pot helmet — low thump + hollow "tonk"
  cat_attack_map: { gain_db: -10, make: () => mix([
    [thump(220, 100, 0.045, 0.5, 24, 1800), 0, 0.8],
    [tone({ wave: 'triangle', dur: 0.2, f0: 392, env: perc(0.045, 0.002) }), 0.005, 0.7],
    [tone({ wave: 'sine', dur: 0.2, f0: 392 * 2.4, env: perc(0.03, 0.002) }), 0.005, 0.2],
  ]) },
  // Mun: small witch spark — two quick twinkles with vibrato
  cat_attack_mun: { gain_db: -10, make: () => mix([
    [tone({ wave: 'triangle', dur: 0.16, f0: n('A5'), f1: n('E6'), vib: { rate: 22, depth: 0.4 }, env: perc(0.05, 0.004) })],
    [tone({ wave: 'sine', dur: 0.12, f0: n('E6'), env: perc(0.04, 0.003) }), 0.05, 0.45],
  ]) },
  // skills (one per cat, ≤ 1 s)
  skill_muop: { gain_db: -6, make: () => mix([ // three-swipe flurry rising in pitch
    [swish(0.1, 800, 2200, 3, 31), 0], [swish(0.1, 1000, 2600, 3, 32), 0.09], [swish(0.14, 1200, 3200, 3, 33), 0.18],
    [thump(260, 120, 0.04, 0.4, 34), 0.27],
    [plink(n('G5'), 0.3, 0.07, 0.1), 0.28, 0.35],
  ]) },
  skill_xiem: { gain_db: -6, make: () => arp( // bone rain: descending sparkly plinks
    [['E6', 0], ['C6', 0.07, 0.9], ['A5', 0.14, 0.85], ['F5', 0.21, 0.8], ['D5', 0.28, 0.75], ['C5', 0.35, 0.9]],
    (hz) => plink(hz, 0.3, 0.06, 0.2)) },
  skill_map: { gain_db: -6, make: () => mix([ // ground pound: soft boom + pot clang + wobble
    [tone({ wave: 'sine', dur: 0.6, f0: 160, f1: 55, slideTime: 0.25, env: perc(0.12, 0.004) }), 0, 0.7],
    [tone({ wave: 'pnoise', dur: 0.3, f0: 700, filters: [{ type: 'lp', f0: 1300 }], env: perc(0.06, 0.002), seed: 35 }), 0, 0.7],
    [tone({ wave: 'triangle', dur: 0.5, f0: 330, vib: { rate: 9, depth: 0.25 }, env: perc(0.12, 0.002) }), 0.01, 0.6],
    [tone({ wave: 'sine', dur: 0.4, f0: 330 * 2.7, env: perc(0.07, 0.002) }), 0.01, 0.15],
  ]) },
  skill_mun: { gain_db: -6, make: () => mix([ // curse: wobbly descending purple tone + shimmer
    [tone({ wave: 'triangle', dur: 0.75, f0: n('E5'), f1: n('A4'), vib: { rate: 7, depth: 0.6 }, env: { a: 0.04, d: 0.1, s: 0.7, r: 0.35 } })],
    [tone({ wave: 'square', dur: 0.75, f0: n('E5') * 1.006, f1: n('A4') * 1.006, duty: 0.25, vib: { rate: 7, depth: 0.6 }, filters: [{ type: 'lp', f0: 1500 }], env: { a: 0.04, d: 0.1, s: 0.6, r: 0.35 } }), 0, 0.25],
    [arp([['B5', 0.05], ['G5', 0.2], ['E5', 0.35]], (hz) => plink(hz, 0.3, 0.06, 0.1)), 0, 0.35],
  ]) },
  enemy_die: { gain_db: -8, make: () => mix([ // falling "pwoof" + soft puff
    [tone({ wave: 'triangle', dur: 0.22, f0: 700, f1: 220, env: perc(0.06, 0.003) })],
    [tone({ wave: 'noise', dur: 0.2, filters: [{ type: 'lp', f0: 1200, f1: 400 }], env: perc(0.05, 0.01), seed: 41 }), 0.02, 0.6],
  ]) },
  // boss_appear: two low "dun… dun" notes with a soft swell — ominous but still cute
  boss_appear: { gain_db: -4, make: () => mix([
    [tone({ wave: 'triangle', dur: 0.5, f0: n('D3'), env: { a: 0.01, d: 0.15, s: 0.6, r: 0.2 } }), 0, 0.5],
    [tone({ wave: 'triangle', dur: 0.5, f0: n('D4'), env: { a: 0.01, d: 0.15, s: 0.6, r: 0.2 } }), 0, 0.5],
    [tone({ wave: 'triangle', dur: 0.85, f0: n('Ab2'), vib: { rate: 5, depth: 0.2, delay: 0.2 }, env: { a: 0.01, d: 0.2, s: 0.6, r: 0.4 } }), 0.42, 0.5],
    [tone({ wave: 'triangle', dur: 0.85, f0: n('Ab3'), vib: { rate: 5, depth: 0.2, delay: 0.2 }, env: { a: 0.01, d: 0.2, s: 0.6, r: 0.4 } }), 0.42, 0.5],
    [tone({ wave: 'square', dur: 0.5, f0: n('D4'), duty: 0.3, filters: [{ type: 'lp', f0: 1100 }], env: { a: 0.01, d: 0.15, s: 0.5, r: 0.2 } }), 0, 0.3],
    [tone({ wave: 'square', dur: 0.85, f0: n('Ab3'), duty: 0.3, filters: [{ type: 'lp', f0: 1100 }], env: { a: 0.05, d: 0.1, s: 0.6, r: 0.4 } }), 0.42, 0.45],
    [thump(110, 50, 0.08, 0.3, 42), 0, 0.5], [thump(100, 45, 0.1, 0.3, 43), 0.42, 0.5],
  ]) },
  // boss_die: deflating wobble then a happy three-note rise
  boss_die: { gain_db: -4, make: () => mix([
    [tone({ wave: 'triangle', dur: 0.5, f0: 520, f1: 130, vib: { rate: 11, depth: 0.8 }, env: { a: 0.01, d: 0.1, s: 0.7, r: 0.2 } })],
    [tone({ wave: 'noise', dur: 0.35, filters: [{ type: 'lp', f0: 900, f1: 300 }], env: perc(0.09, 0.01), seed: 44 }), 0.3, 0.5],
    [arp([['C5', 0], ['E5', 0.12], ['G5', 0.24], ['C6', 0.36]], (hz) => mix([[plink(hz, 0.5, 0.12, 0.2)], [tone({ wave: 'triangle', dur: 0.3, f0: hz / 2, env: perc(0.08) }), 0, 0.4]])), 0.55],
  ]) },
  // cat_down: little sad falling "mew" (not dramatic)
  cat_down: { gain_db: -5, make: () => catVoice({ dur: 0.6, seed: 51,
    f0: [[0, 620], [0.25, 660], [1, 330]], amp: [[0, 0], [0.12, 1], [0.6, 0.6], [1, 0]],
    F1: [[0, 450], [0.3, 750], [1, 400]], F2: [[0, 2000], [0.3, 1500], [1, 900]], vib: { rate: 6, depth: 0.25 }, breath: 0.08 }) },

  // ── progression / economy ─────────────────────────────────────────────────
  level_up: { gain_db: -5, make: () => arp([['C5', 0], ['E5', 0.08], ['G5', 0.16], ['C6', 0.24]],
    (hz) => mix([[plink(hz, 0.45, 0.11, 0.2)], [tone({ wave: 'square', dur: 0.12, f0: hz, duty: 0.25, filters: [{ type: 'lp', f0: 2500 }], env: perc(0.04) }), 0, 0.25]])) },
  unlock: { gain_db: -4, make: () => mix([ // two chords blooming upward
    [arp([['F5', 0], ['A5', 0.02], ['C6', 0.04]], (hz) => plink(hz, 0.5, 0.14, 0.15))],
    [arp([['G5', 0], ['B5', 0.02], ['D6', 0.04], ['G6', 0.06]], (hz) => plink(hz, 0.7, 0.2, 0.15)), 0.22],
    [tone({ wave: 'triangle', dur: 0.8, f0: n('G3'), env: { a: 0.02, d: 0.2, s: 0.4, r: 0.4 } }), 0.22, 0.4],
  ]) },
  coin: { gain_db: -12, make: () => mix([ // the fish pickup: two rising soft tones B5→E6
    [tone({ wave: 'triangle', dur: 0.2, f0: n('B5'), arp: [[0.06, n('E6') / n('B5')]], env: { a: 0.002, hold: 0.06, decay: 0.045 } })],
    [tone({ wave: 'square', dur: 0.2, f0: n('B5'), duty: 0.25, arp: [[0.06, n('E6') / n('B5')]], filters: [{ type: 'lp', f0: 2800 }], env: { a: 0.002, hold: 0.06, decay: 0.035 } }), 0, 0.2],
  ]) },
  yarn: { gain_db: -12, make: () => mix([ // plush "boing": sine bouncing pitch
    [tone({ wave: 'sine', dur: 0.26, f0: 330, f1: 520, slide: 'lin', vib: { rate: 14, depth: 1.2 }, env: perc(0.07, 0.004) })],
    [tone({ wave: 'triangle', dur: 0.2, f0: 660, f1: 1040, slide: 'lin', env: perc(0.04, 0.003) }), 0, 0.2],
  ]) },
  gear_drop: { gain_db: -8, make: () => mix([ // small clink on the ground
    [plink(1180, 0.3, 0.05, 0.3)], [plink(1590, 0.2, 0.03, 0.2), 0.045, 0.5],
    [thump(260, 150, 0.02, 0.3, 61), 0, 0.4],
  ]) },
  gear_legendary: { gain_db: -3, make: () => mix([ // sparkly rising run + warm chord bed
    [arp([['C6', 0], ['E6', 0.06], ['G6', 0.12], ['C7', 0.18], ['E6', 0.3], ['G6', 0.36], ['C7', 0.42]], (hz) => plink(hz / 2, 0.45, 0.1, 0.3))],
    [tone({ wave: 'triangle', dur: 1.2, f0: n('C4'), env: { a: 0.05, d: 0.3, s: 0.5, r: 0.6 } }), 0.1, 0.35],
    [tone({ wave: 'triangle', dur: 1.2, f0: n('G4'), env: { a: 0.05, d: 0.3, s: 0.5, r: 0.6 } }), 0.1, 0.25],
    [tone({ wave: 'triangle', dur: 1.1, f0: n('E5'), vib: { rate: 5, depth: 0.1 }, env: { a: 0.08, d: 0.3, s: 0.5, r: 0.6 } }), 0.2, 0.25],
  ]) },
  box_land: { gain_db: -6, make: () => mix([ // cardboard thud: hollow box body + papery slap
    [tone({ wave: 'sine', dur: 0.25, f0: 170, f1: 90, slideTime: 0.08, env: perc(0.045, 0.003) }), 0, 0.6],
    [tone({ wave: 'triangle', dur: 0.15, f0: 330, f1: 270, env: perc(0.035, 0.002) }), 0, 0.6],
    [tone({ wave: 'pnoise', dur: 0.12, f0: 900, filters: [{ type: 'bp', f0: 750, q: 0.9 }, { type: 'lp', f0: 1600 }], env: perc(0.025, 0.002), seed: 71 }), 0, 1.2],
    [tone({ wave: 'pnoise', dur: 0.08, f0: 600, filters: [{ type: 'lp', f0: 1200 }], env: perc(0.015, 0.002), seed: 72 }), 0.07, 0.35],
  ]) },
  box_open: { gain_db: -5, make: () => mix([ // two cardboard flaps then "ta-da"
    [swish(0.09, 500, 1400, 1.5, 73, { a: 0.01, d: 0.02, s: 0.6, r: 0.05 })],
    [swish(0.09, 600, 1600, 1.5, 74, { a: 0.01, d: 0.02, s: 0.6, r: 0.05 }), 0.1],
    [arp([['G5', 0], ['C6', 0.09]], (hz) => mix([[plink(hz, 0.4, 0.1, 0.2)], [tone({ wave: 'triangle', dur: 0.3, f0: hz / 2, env: perc(0.08) }), 0, 0.4]])), 0.2],
  ]) },
  catnip: { gain_db: -5, make: () => mix([ // dreamy upward swirl
    [tone({ wave: 'sine', dur: 0.9, f0: n('C5'), f1: n('C6'), vib: { rate: 6, depth: 0.7 }, env: { a: 0.08, d: 0.2, s: 0.7, r: 0.4 } })],
    [tone({ wave: 'sine', dur: 0.9, f0: n('C5') * 1.5, f1: n('C6') * 1.5, vib: { rate: 6.5, depth: 0.7 }, env: { a: 0.12, d: 0.2, s: 0.5, r: 0.4 } }), 0.03, 0.35],
    [arp([['E6', 0.35], ['G6', 0.45], ['C7', 0.55]], (hz) => plink(hz / 2, 0.3, 0.07, 0.15)), 0, 0.3],
  ]) },

  // ── UI ────────────────────────────────────────────────────────────────────
  ui_click: { gain_db: -10, make: () => mix([
    [tone({ wave: 'sine', dur: 0.05, f0: 1250, f1: 900, env: perc(0.012, 0.0015) })],
    [tone({ wave: 'triangle', dur: 0.04, f0: 620, env: perc(0.008, 0.0015) }), 0, 0.3],
  ]) },
  ui_tab: { gain_db: -10, make: () => mix([ // soft upward "pop" (bubble), distinct from the click
    [tone({ wave: 'sine', dur: 0.07, f0: 620, f1: 1180, slideTime: 0.04, env: { a: 0.002, hold: 0.012, decay: 0.014 } })],
    [tone({ wave: 'triangle', dur: 0.05, f0: 620, f1: 1180, slideTime: 0.04, env: perc(0.01, 0.002) }), 0, 0.2],
  ]) },
  ui_error: { gain_db: -8, make: () => mix([ // soft falling double "bup-bup"
    [tone({ wave: 'triangle', dur: 0.12, f0: 330, f1: 290, env: { a: 0.004, d: 0.03, s: 0.7, r: 0.04 } })],
    [tone({ wave: 'triangle', dur: 0.14, f0: 262, f1: 220, env: { a: 0.004, d: 0.03, s: 0.7, r: 0.06 } }), 0.12],
    [tone({ wave: 'square', dur: 0.26, f0: 165, f1: 120, duty: 0.4, filters: [{ type: 'lp', f0: 800 }], env: { a: 0.004, d: 0.05, s: 0.5, r: 0.08 } }), 0, 0.2],
  ]) },

  // ── milestones ────────────────────────────────────────────────────────────
  // prestige "Dùng một mạng": nine soft rising chimes (nine lives) + warm bloom
  prestige: { gain_db: -3, make: () => mix([
    [arp(['C5', 'D5', 'E5', 'G5', 'A5', 'C6', 'D6', 'E6', 'G6'].map((nm, i) => [nm, i * 0.1, 0.7 + i * 0.03]), (hz) => plink(hz, 0.6, 0.15, 0.15))],
    [tone({ wave: 'triangle', dur: 1.6, f0: n('C4'), vib: { rate: 4.5, depth: 0.08 }, env: { a: 0.4, d: 0.3, s: 0.6, r: 0.7 } }), 0.3, 0.22],
    [tone({ wave: 'triangle', dur: 1.5, f0: n('G4'), vib: { rate: 5, depth: 0.08 }, env: { a: 0.45, d: 0.3, s: 0.5, r: 0.7 } }), 0.4, 0.16],
    [tone({ wave: 'sine', dur: 1.3, f0: n('E5'), vib: { rate: 5.5, depth: 0.1 }, env: { a: 0.5, d: 0.3, s: 0.5, r: 0.6 } }), 0.55, 0.16],
  ]) },
  achievement: { gain_db: -4, make: () => mix([
    [arp([['G5', 0], ['C6', 0.1], ['E6', 0.2], ['G6', 0.3]], (hz) => plink(hz / 2, 0.6, 0.14, 0.2))],
    [tone({ wave: 'square', dur: 0.6, f0: n('C5'), duty: 0.25, filters: [{ type: 'lp', f0: 1800 }], env: { a: 0.01, d: 0.15, s: 0.4, r: 0.3 } }), 0.3, 0.25],
    [plink(n('C6'), 0.8, 0.25, 0.25), 0.3, 0.5],
  ]) },
  stage_clear: { gain_db: -5, make: () => arp([['C5', 0], ['E5', 0.1], ['G5', 0.2], ['E5', 0.3, 0.6], ['G5', 0.36]],
    (hz) => mix([[plink(hz, 0.35, 0.09, 0.15)], [tone({ wave: 'triangle', dur: 0.2, f0: hz / 2, env: perc(0.06) }), 0, 0.4]])) },

  // ── meows: "m-i-a-o" formant glides, each with its own personality ─────────
  meow_1: { gain_db: -6, make: () => catVoice({ dur: 0.55, seed: 81, // classic friendly "mi-ao"
    f0: [[0, 470], [0.4, 720], [1, 430]], amp: [[0, 0], [0.12, 0.9], [0.45, 1], [0.8, 0.6], [1, 0]],
    F1: [[0, 380], [0.25, 450], [0.55, 820], [1, 450]], F2: [[0, 2300], [0.25, 2200], [0.55, 1400], [1, 850]],
    vib: { rate: 6, depth: 0.15 } }) },
  meow_2: { gain_db: -6, make: () => catVoice({ dur: 0.34, seed: 82, // "mrrp?" chirp: rolled onset, rising
    f0: [[0, 430], [0.6, 700], [1, 640]], amp: [[0, 0], [0.1, 0.8], [0.6, 1], [1, 0]],
    F1: [[0, 400], [0.5, 600], [1, 750]], F2: [[0, 1800], [0.5, 1900], [1, 1600]],
    purr: { rate: 28, depth: 0.7, until: 0.45 } }) },
  meow_3: { gain_db: -6, make: () => catVoice({ dur: 0.8, seed: 83, // long plaintive "meeeow"
    f0: [[0, 520], [0.35, 780], [0.6, 700], [1, 420]], amp: [[0, 0], [0.1, 0.8], [0.4, 1], [0.85, 0.5], [1, 0]],
    F1: [[0, 350], [0.3, 400], [0.6, 850], [1, 420]], F2: [[0, 2400], [0.3, 2300], [0.6, 1350], [1, 800]],
    vib: { rate: 5.5, depth: 0.25 }, breath: 0.1 }) },
  meow_4: { gain_db: -6, make: () => catVoice({ dur: 0.36, seed: 84, // tiny kitten "mew"
    f0: [[0, 760], [0.35, 1000], [1, 720]], amp: [[0, 0], [0.12, 1], [0.6, 0.8], [1, 0]],
    F1: [[0, 500], [0.4, 900], [1, 600]], F2: [[0, 2500], [0.4, 1900], [1, 1300]], F3: [[0, 3600], [1, 3400]],
    vib: { rate: 7, depth: 0.2 }, breath: 0.08 }) },
};

// ═════════════════════════════════════════════════════════════════════════════
// mastering
// ═════════════════════════════════════════════════════════════════════════════
const dbToLin = (d) => Math.pow(10, d / 20);

/** Soft limiter: transparent below ~−7 dBFS, smoothly saturates toward the ceiling. */
function limit(x, ceilDb = PEAK_CEIL_DB) {
  const c = dbToLin(ceilDb) * 0.995, knee = c * 0.45;
  for (let i = 0; i < x.length; i++) {
    const v = x[i], a = Math.abs(v);
    if (a > knee) x[i] = Math.sign(v) * (knee + (c - knee) * Math.tanh((a - knee) / (c - knee)));
  }
  return x;
}

/** Scale to the gated-RMS target, then soft-limit; iterate because limiting lowers RMS. */
function loudness(x, target = TARGET_RMS_DB, maxDriveDb = 6) {
  let pk = 0; for (const v of x) pk = Math.max(pk, Math.abs(v));
  const base = Float32Array.from(x);
  let g = dbToLin(target - gatedRmsDb(base)), y = x;
  const gMax = dbToLin(PEAK_CEIL_DB + maxDriveDb) / pk; // never drive peaks more than +6 dB into the limiter
  for (let it = 0; it < 6; it++) {
    g = Math.min(g, gMax);
    y = limit(base.map((v) => v * g));
    const err = target - gatedRmsDb(y);
    if (Math.abs(err) < 0.1) break;
    g *= dbToLin(err);
  }
  return y;
}

/** One-shot chain: DC → gentle 2×biquad low-pass (5.5 kHz) → trim → fades → loudness. */
export function masterSfx(buf) {
  let b = removeDC(Float32Array.from(buf));
  b = filter(filter(b, { type: 'lp', f0: 5500, q: 0.6 }), { type: 'lp', f0: 5500, q: 0.6 });
  b = trimTail(b, -55, 0.012);
  fade(b, 0.003, 0.015);
  b = loudness(b);
  fade(b, 0.001, 0.004); // guarantee exact-zero edges after limiting
  return b;
}

// ═════════════════════════════════════════════════════════════════════════════
// music_calm: lofi chiptune, 72 BPM, 20 bars (66.7 s), A A' B B' A'' — loops seamlessly
// ═════════════════════════════════════════════════════════════════════════════
const PROG = ['Fmaj7', 'Em7', 'Dm7', 'Cmaj7', 'Fmaj7', 'Em7', 'Dm7', 'Cmaj7',
  'Bbmaj7', 'Am7', 'Gm7', 'C7', 'Bbmaj7', 'Am7', 'Gm7', 'C7',
  'Fmaj7', 'Em7', 'Dm7', 'Cmaj7'];
const MELODY = [
  'E5 - - C5 - A4 - .', 'G4 - B4 - D5 - - .', 'F5 - E5 - D5 - A4 -', 'E5 - - - - - . .',
  'E5 - - G5 - E5 C5 -', 'D5 - B4 - G4 - - .', 'A4 - C5 - D5 - F5 -', 'E5 - - - D5 - C5 -',
  'D5 - - F5 - A5 - .', 'G5 - E5 - C5 - - .', 'Bb4 - D5 - F5 - E5 -', 'E5 - - - - - . .',
  'F5 - D5 - Bb4 - D5 -', 'C5 - - E5 - - G5 -', 'F5 - E5 - D5 - Bb4 -', 'C5 - - - - - . .',
  '. . A4 - C5 - E5 -', 'D5 - - - B4 - . .', '. . F4 - A4 - C5 -', 'B4 - - - G4 - . .',
];
const ROOT_OF = (sym) => /^([A-G][b#]?)/.exec(sym)[1];

export const MUSIC = {
  music_calm: { gain_db: -8, song: {
    bpm: 72, beatsPerBar: 4, stepsPerBeat: 2, bars: 20,
    channels: [
      { // lead: soft triangle "flute" with late vibrato and a dotted-eighth echo
        wave: 'triangle', gain: 0.28, pattern: MELODY.join(' | '),
        inst: { a: 0.02, d: 0.25, s: 0.55, r: 0.18, gate: 0.85, vib: { delay: 0.25, rate: 5, depth: 0.12 } },
        echo: { delay: 0.625, fb: 0.3, wet: 0.28 },
      },
      { // keys: broken 7th chords in 8ths, narrow pulse, dark low-pass (lofi)
        wave: 'square', gain: 0.12, lowpass: 1600,
        pattern: perBar(PROG, (c) => { const t = chord(c, 4); const x = t[3] ?? t[0]; return [t[0], t[1], t[2], x, t[1], t[2], x, t[2]]; }),
        inst: { a: 0.01, d: 0.3, s: 0.3, r: 0.12, gate: 0.75, duty: 0.25 },
        echo: { delay: 0.4167, fb: 0.25, wet: 0.2 },
      },
      { // bass: triangle, root on 1 (held), fifth on 3, approach note on the "and" of 4
        wave: 'triangle', gain: 0.2,
        pattern: perBar(PROG, (c) => { const t = chord(c, 2); return [t[0], '-', '-', '.', t[2], '-', `${ROOT_OF(c)}3`, '.']; }),
        inst: { a: 0.01, d: 0.2, s: 0.7, r: 0.08, gate: 0.9 },
      },
      { // drums: lazy kick/rim, very soft hats; the whole mix is low-passed afterwards
        wave: 'noise', gain: 0.24, levels: { k: 1, s: 0.45, h: 0.18, o: 0.15, t: 0.6 }, lowpass: 3000,
        pattern: perBar(PROG, (c, b) => (b % 4 === 3 ? 'k . h . s . t t' : b % 2 ? 'k . h k s . h .' : 'k . h . s . h .').split(' ')),
      },
    ],
  } },
};

/** Circular 2-pole low-pass: filter the loop twice and keep the 2nd pass so state wraps seamlessly. */
function circularLp(x, hz, q = 0.707) {
  const L = x.length, two = new Float32Array(2 * L);
  two.set(x, 0); two.set(x, L);
  return filter(two, { type: 'lp', f0: hz, q }).slice(L);
}

export function renderMusic(def) {
  let x = renderSong(def.song);
  x = circularLp(circularLp(x, 3600), 3600); // warm lofi top end, no hiss above 6 kHz
  removeDC(x);
  // loudness on a periodic signal: constant gain + memoryless limiter keep the loop seamless
  return loudness(x, TARGET_RMS_DB - 1, 3);
}

// ═════════════════════════════════════════════════════════════════════════════
// build
// ═════════════════════════════════════════════════════════════════════════════
const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  const quiet = process.argv.includes('--quiet');
  mkdirSync(OUT_DIR, { recursive: true });
  for (const f of readdirSync(OUT_DIR)) if (f.endsWith('.wav')) unlinkSync(join(OUT_DIR, f));
  const manifest = { sample_rate: SR, target_rms_dbfs: TARGET_RMS_DB, peak_ceiling_dbfs: PEAK_CEIL_DB, sounds: [] };
  const fmt = (v, d = 1) => (Number.isFinite(v) ? v.toFixed(d) : '-');
  for (const [name, def] of Object.entries(SFX)) {
    const x = masterSfx(def.make());
    writeFileSync(join(OUT_DIR, `${name}.wav`), encodeWav(x));
    manifest.sounds.push({ name, file: `${name}.wav`, kind: 'sfx', gain_db: def.gain_db, duration: +(x.length / SR).toFixed(4) });
    if (!quiet) {
      const a = analyse(x), hb = highBand(x), chk = INTENT[name]?.(x, a);
      const f0 = f0Track(x).filter(Boolean);
      console.log(`${name.padEnd(16)} ${fmt(a.dur, 3)} s  rms ${fmt(gatedRmsDb(x))} dB  pk ${fmt(a.peakDb, 2)}  cen ${a.centroid | 0} Hz  >6k ${(hb.total * 100).toFixed(2)}%` +
        (name.startsWith('meow') || name === 'cat_down' ? `  f0 ${f0.map((v) => v | 0).join(' ')}` : '') + (chk ? `  ${chk[0] ? 'OK ' : 'BAD'} ${chk[1]}` : ''));
    }
  }
  for (const [name, def] of Object.entries(MUSIC)) {
    const x = renderMusic(def), s = def.song;
    writeFileSync(join(OUT_DIR, `${name}.wav`), encodeWav(x));
    manifest.sounds.push({ name, file: `${name}.wav`, kind: 'music', gain_db: def.gain_db, duration: +(x.length / SR).toFixed(4),
      loop: true, loop_start: 0, loop_end: x.length, bpm: s.bpm, bars: s.bars, beats_per_bar: s.beatsPerBar });
    if (!quiet) console.log(`${name.padEnd(16)} ${(x.length / SR).toFixed(2)} s  rms ${gatedRmsDb(x).toFixed(1)} dB  pk ${analyse(x).peakDb.toFixed(2)}  >6k ${(highBand(x).total * 100).toFixed(3)}%  (${songLength(s)} samples, ${s.bars} bars @ ${s.bpm} BPM)`);
  }
  writeFileSync(join(OUT_DIR, 'audio.json'), JSON.stringify(manifest, null, 2) + '\n');
  console.log(`wrote ${manifest.sounds.length} sounds → ${OUT_DIR}`);
}
