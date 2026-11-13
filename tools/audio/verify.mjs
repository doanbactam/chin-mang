#!/usr/bin/env node
// Verifies generated audio against SPEC §9. Exit code 1 on any failure.
//   node tools/audio/verify.mjs
// Checks: every SPEC §9 name present in audio.json and on disk (and nothing unlisted), manifest
// schema {name, file, kind, gain_db}, WAV validity (RIFF/PCM 16-bit/22 050 Hz; SFX mono), duration
// per sound class, no clipping (peak ≤ −0.9 dBFS, no full-scale samples), loudness uniform
// (gated RMS −16 dBFS ± 2 ≈ −16 LUFS ± 2), no sustained energy above 6 kHz, click-free SFX edges,
// music: ≥ 60 s loop on a bar boundary with a seam as smooth as the interior, and the per-sound
// "listen proxy" intent checks (tools/audio/intent.mjs: pitch glides, meow f0 contour, …).
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SR, decodeWav } from './sfxr.mjs';
import { analyse, gatedRmsDb, highBand } from './analyze.mjs';
import { INTENT } from './intent.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const DIR = process.env.CM_AUDIO_DIR ?? join(ROOT, 'assets', 'audio'); // override only for self-tests

// SPEC §9 list
const SFX = ['hit_1', 'hit_2', 'hit_3', 'crit',
  ...['muop', 'xiem', 'map', 'mun'].map((c) => `cat_attack_${c}`), ...['muop', 'xiem', 'map', 'mun'].map((c) => `skill_${c}`),
  'enemy_die', 'boss_appear', 'boss_die', 'cat_down', 'level_up', 'unlock', 'coin', 'yarn', 'gear_drop', 'gear_legendary',
  'box_land', 'box_open', 'catnip', 'ui_click', 'ui_tab', 'ui_error', 'prestige', 'achievement', 'stage_clear',
  'meow_1', 'meow_2', 'meow_3', 'meow_4'];
const MUSIC = ['music_calm'];

// duration limits by class (seconds). "Mọi âm ngắn" — frequent sounds are the shortest.
const CLASS = [
  [/^ui_(click|tab)$/, 0.01, 0.15, 'ui'],
  [/^(hit_\d|crit|cat_attack_\w+|coin|yarn|enemy_die|gear_drop|box_land|ui_error)$/, 0.04, 0.4, 'frequent'],
  [/^(prestige|boss_appear|boss_die|achievement|gear_legendary)$/, 0.5, 2.5, 'jingle'],
  [/.*/, 0.15, 1.2, 'event'],
];
const classOf = (name) => CLASS.find(([re]) => re.test(name));
const RMS_TARGET = -16, RMS_TOL = 2;     // gated RMS (dBFS) ≈ LUFS for short mono material
const PEAK_MAX_DB = -0.9;
const HF = { hz: 6000, totalMax: 0.02, runRatio: 0.2, runMax: 0.06 };
const MUSIC_MIN = 60, MUSIC_MAX = 180;

const errors = [];
const fail = (name, msg) => errors.push(`${name}: ${msg}`);

const manPath = join(DIR, 'audio.json');
if (!existsSync(manPath)) { console.error('assets/audio/audio.json missing — run `node tools/audio/synth.mjs`'); process.exit(1); }
let man;
try { man = JSON.parse(readFileSync(manPath, 'utf8')); } catch (e) { console.error(`audio.json: ${e.message}`); process.exit(1); }
const list = Array.isArray(man.sounds) ? man.sounds : [];
if (!list.length) fail('audio.json', 'no "sounds" array');

const byName = new Map();
for (const e of list) {
  if (typeof e?.name !== 'string' || typeof e.file !== 'string') { fail('audio.json', `bad entry ${JSON.stringify(e)}`); continue; }
  if (byName.has(e.name)) fail(e.name, 'duplicate entry');
  byName.set(e.name, e);
  if (!['sfx', 'music'].includes(e.kind)) fail(e.name, `kind "${e.kind}" not sfx|music`);
  if (!(typeof e.gain_db === 'number' && e.gain_db <= 0 && e.gain_db >= -24)) fail(e.name, `gain_db ${e.gain_db} not in [-24, 0]`);
}
for (const n of SFX) if (!byName.has(n)) fail(n, 'missing from audio.json'); else if (byName.get(n).kind !== 'sfx') fail(n, 'kind must be sfx');
for (const n of MUSIC) if (!byName.has(n)) fail(n, 'missing from audio.json'); else if (byName.get(n).kind !== 'music') fail(n, 'kind must be music');
for (const n of byName.keys()) if (!SFX.includes(n) && !MUSIC.includes(n)) fail(n, 'not in the SPEC §9 list');
const onDisk = existsSync(DIR) ? readdirSync(DIR).filter((f) => /\.(wav|ogg)$/.test(f)) : [];
for (const f of onDisk) if (![...byName.values()].some((e) => e.file === f)) fail(f, 'audio file not listed in audio.json');

const p999 = (arr) => arr[Math.floor(arr.length * 0.999)];
let total = 0;
const rows = [];
for (const [name, e] of byName) {
  const p = join(DIR, e.file);
  if (!existsSync(p)) { fail(name, `file ${e.file} missing`); continue; }
  if (!e.file.endsWith('.wav')) { fail(name, `${e.file}: only PCM .wav is produced/verified`); continue; }
  const buf = readFileSync(p);
  total += buf.length;
  let w;
  try { w = decodeWav(buf); } catch (err) { fail(name, `bad WAV: ${err.message}`); continue; }
  if (w.sr !== SR) fail(name, `sample rate ${w.sr} ≠ ${SR}`);
  if (e.kind === 'sfx' && w.channels !== 1) fail(name, `SFX must be mono (channels=${w.channels})`);
  if (w.channels > 2) fail(name, `channels=${w.channels}`);
  // fold to mono for analysis
  const ch = w.channels, n = w.samples.length / ch, x = new Float32Array(n);
  for (let i = 0; i < n; i++) { let s = 0; for (let c = 0; c < ch; c++) s += w.samples[i * ch + c]; x[i] = s / ch; }
  if (!n) { fail(name, 'empty'); continue; }
  const dur = n / SR;
  if (e.duration !== undefined && Math.abs(e.duration - dur) > 0.001) fail(name, `duration ${dur.toFixed(4)} ≠ manifest ${e.duration}`);

  let full = 0; for (const r of w.raw) if (r >= 32767 || r <= -32767) full++;
  if (full) fail(name, `${full} clipped (full-scale) samples`);
  const a = analyse(x);
  if (a.nan) fail(name, `${a.nan} NaN samples`);
  if (a.peakDb > PEAK_MAX_DB) fail(name, `peak ${a.peakDb.toFixed(2)} dBFS > ${PEAK_MAX_DB}`);
  const rms = gatedRmsDb(x);
  if (Math.abs(rms - RMS_TARGET) > RMS_TOL) fail(name, `gated RMS ${rms.toFixed(1)} dBFS outside ${RMS_TARGET} ± ${RMS_TOL}`);
  const hb = highBand(x, HF.hz, HF.runRatio);
  if (hb.total > HF.totalMax) fail(name, `${(hb.total * 100).toFixed(1)}% of energy above ${HF.hz} Hz (max ${HF.totalMax * 100}%)`);
  if (hb.longestRun > HF.runMax) fail(name, `harsh: ${(hb.longestRun * 1000) | 0} ms run with > ${HF.runRatio * 100}% energy above ${HF.hz} Hz`);

  let extra = '';
  if (e.kind === 'sfx') {
    const [, lo, hi, cls] = classOf(name);
    if (dur < lo || dur > hi) fail(name, `${cls} SFX duration ${dur.toFixed(3)} s outside [${lo}, ${hi}]`);
    if (Math.abs(x[0]) > 0.002 || Math.abs(x[n - 1]) > 0.002) fail(name, `edge not silent (first ${x[0].toFixed(4)}, last ${x[n - 1].toFixed(4)}) → click`);
    const check = INTENT[name];
    if (!check) fail(name, 'no intent check defined');
    else { const [ok, why] = check(x, a); extra = `${ok ? 'OK ' : 'BAD'} ${why}`; if (!ok) fail(name, `intent: ${why}`); }
  } else {
    if (dur < MUSIC_MIN || dur > MUSIC_MAX) fail(name, `music duration ${dur.toFixed(2)} s outside [${MUSIC_MIN}, ${MUSIC_MAX}]`);
    if (e.loop !== true) fail(name, 'music must have loop: true');
    const expect = Math.round((e.bars * e.beats_per_bar * 60 * SR) / e.bpm);
    if (n !== expect || e.loop_start !== 0 || e.loop_end !== n) fail(name, `loop not on a bar boundary: ${n} samples, expected ${expect} (${e.bars} bars @ ${e.bpm} BPM)`);
    // seam continuity: jump and slope-prediction error across end→start vs. interior distribution
    const d = [], e2 = [];
    for (let i = 2; i < n; i++) { d.push(Math.abs(x[i] - x[i - 1])); e2.push(Math.abs(x[i] - (2 * x[i - 1] - x[i - 2]))); }
    d.sort((p, q) => p - q); e2.sort((p, q) => p - q);
    const jump = Math.abs(x[0] - x[n - 1]);
    const pred = Math.max(Math.abs(x[0] - (2 * x[n - 1] - x[n - 2])), Math.abs(x[1] - (2 * x[0] - x[n - 1])));
    if (jump > p999(d)) fail(name, `loop seam jump ${jump.toFixed(4)} > interior p99.9 ${p999(d).toFixed(4)} → click`);
    if (pred > p999(e2)) fail(name, `loop seam slope break ${pred.toFixed(4)} > interior p99.9 ${p999(e2).toFixed(4)}`);
    extra = `loop ${e.bars} bars @ ${e.bpm} BPM, seam Δ ${jump.toFixed(4)} (interior p99.9 ${p999(d).toFixed(4)})`;
  }
  rows.push(`${name.padEnd(16)}${e.kind.padEnd(6)}${dur.toFixed(3).padStart(7)} s  rms${rms.toFixed(1).padStart(6)}  pk${a.peakDb.toFixed(1).padStart(6)}  >6k${(hb.total * 100).toFixed(2).padStart(5)}%  ${String(e.gain_db).padStart(3)} dB  ${extra}`);
}

console.log(rows.join('\n'));
console.log(`\n${rows.length} files, ${(total / 1e6).toFixed(2)} MB`);
if (errors.length) {
  console.error(`\nAUDIO VERIFY FAILED (${errors.length}):\n  ${errors.join('\n  ')}`);
  process.exit(1);
}
console.log('AUDIO VERIFY OK');
