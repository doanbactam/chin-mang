#!/usr/bin/env node
// Chụp ảnh bắt buộc SPEC §10.2 từ bản web (dist/web) bằng Chromium headless (WebGL2/SwiftShader).
//
//   node tools/verify/shots.mjs [--only name[,name…]] [--dist dist/web] [--out docs/shots] [--timeout 120]
//
// Mỗi ảnh: mở `index.html?shot=<tên>&bg=…&lang=…&scale=…`, chờ window.__CM_READY === true,
// chụp 1280×600 → docs/shots/<tên>.png. Thất bại nếu: console.error / pageerror / request lỗi / HTTP ≥ 400 /
// timeout / canvas trống (so ảnh có canvas với ảnh chỉ có nền: < 0.2% pixel khác ⇒ trống).
import fs from 'node:fs';
import path from 'node:path';
import { ROOT, fmtMs, flag } from './lib/util.mjs';
import { serveStatic } from './lib/serve.mjs';
import { launchBrowser, newPage } from './lib/browser.mjs';
import { decodePng } from './lib/png.mjs';

/** SPEC §10.2 — tên → tham số query. Mặc định lang=vi, scale=2, bg=busy. */
export const SHOTS = [
  ['strip-z1', { bg: 'busy' }],
  ['strip-z3-boss', { bg: 'busy' }],
  ['strip-z5', { bg: 'busy' }],
  ['strip-white', { bg: 'white' }],
  ['strip-black', { bg: 'black' }],
  ['panel-cats', {}],
  ['panel-cat-detail', {}],
  ['panel-gear', {}],
  ['panel-souls', {}],
  ['panel-achievements', {}],
  ['panel-settings', {}],
  ['offline-summary', {}],
  ['tutorial-bubble', {}],
  ['box', {}],
  ['vi-diacritics', { lang: 'vi' }],
  ['en', { lang: 'en' }],
  ['scale1', { scale: '1' }],
  ['scale3', { scale: '3' }],
].map(([name, p]) => ({ name, params: { shot: name, bg: 'busy', lang: 'vi', scale: '2', ...p } }));

const argv = process.argv.slice(2);
const only = flag(argv, '--only')?.split(',').map((s) => s.trim()).filter(Boolean);
const dist = path.resolve(ROOT, flag(argv, '--dist') || 'dist/web');
const outDir = path.resolve(ROOT, flag(argv, '--out') || 'docs/shots');
const timeoutS = Number(flag(argv, '--timeout') || 120);

const list = only ? SHOTS.filter((s) => only.includes(s.name)) : SHOTS;
if (only) {
  const unknown = only.filter((n) => !SHOTS.some((s) => s.name === n));
  if (unknown.length) { console.error(`✘ shots: không có ảnh tên ${unknown.join(', ')} (xem SPEC §10.2)`); process.exit(2); }
}
if (!fs.existsSync(path.join(dist, 'index.html'))) {
  console.error(`✘ shots: thiếu ${path.relative(ROOT, dist)}/index.html — chạy node tools/verify/build-web.mjs trước`);
  process.exit(2);
}
fs.mkdirSync(outDir, { recursive: true });

function diffRatio(a, b) {
  const A = decodePng(a); const B = decodePng(b);
  let n = 0;
  for (let i = 0; i < A.rgba.length; i += 4) {
    if (Math.abs(A.rgba[i] - B.rgba[i]) + Math.abs(A.rgba[i + 1] - B.rgba[i + 1]) + Math.abs(A.rgba[i + 2] - B.rgba[i + 2]) > 24) n++;
  }
  return n / (A.w * A.h);
}

const srv = await serveStatic(dist);
const browser = await launchBrowser();
let failures = 0;
const t00 = Date.now();
for (const s of list) {
  const t0 = Date.now();
  const { ctx, page, errors, logs } = await newPage(browser);
  const qs = new URLSearchParams(s.params).toString();
  try {
    await page.goto(`${srv.url}index.html?${qs}`, { waitUntil: 'load', timeout: 60_000 });
    await page.waitForFunction(() => window.__CM_READY === true || (window.__CM_ERRORS || []).length > 0, null,
      { timeout: timeoutS * 1000, polling: 250 });
    const ready = await page.evaluate(() => window.__CM_READY === true);
    if (!ready) errors.push('lỗi trước khi sẵn sàng (xem console)');
  } catch (e) {
    errors.push(`timeout ${timeoutS} s chờ window.__CM_READY (${String(e.message).split('\n')[0]})`);
  }
  let file = path.join(outDir, `${s.name}.png`);
  let blank = null;
  let shot = null;
  try {
    shot = await page.screenshot();
    await page.evaluate(() => { const c = document.getElementById('bevy'); if (c) c.style.visibility = 'hidden'; });
    const bgOnly = await page.screenshot();
    blank = diffRatio(shot, bgOnly);
    if (blank < 0.002) errors.push(`canvas gần như trống (${(blank * 100).toFixed(3)}% pixel khác nền)`);
  } catch (e) {
    errors.push(`screenshot failed: ${e.message}`);
  }
  const inPage = await page.evaluate(() => window.__CM_ERRORS || []).catch(() => []);
  for (const m of inPage) if (!errors.some((e) => e.includes(m))) errors.push(`__CM_ERRORS: ${m}`);
  const ok = errors.length === 0;
  if (!ok) failures++;
  // Ảnh lỗi không ghi đè ảnh tốt trong docs/shots: để ở target/verify-logs/.
  if (!ok) file = path.join(ROOT, 'target/verify-logs', `shot-fail-${s.name}.png`);
  if (shot) { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, shot); }
  const cov = blank == null ? '' : ` · nội dung ${(blank * 100).toFixed(1)}%`;
  console.log(`${ok ? '✔' : '✘'} ${s.name.padEnd(20)} ${fmtMs(Date.now() - t0).padStart(7)}${cov} → ${path.relative(ROOT, file)}`);
  if (!ok) {
    for (const e of errors.slice(0, 15)) console.log(`    ${e}`);
    const tail = logs.slice(-10);
    if (tail.length) console.log('    console (cuối):\n' + tail.map((l) => '      ' + l.slice(0, 300)).join('\n'));
  }
  await ctx.close();
}
await browser.close();
await srv.close();
if (srv.misses.length) console.log(`  404: ${[...new Set(srv.misses)].join(', ')}`);
console.log(`${failures ? '✘' : '✔'} shots: ${list.length - failures}/${list.length} ok in ${fmtMs(Date.now() - t00)}`);
process.exit(failures ? 1 : 0);
