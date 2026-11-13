#!/usr/bin/env node
// Probe pipeline web: build cm-probe → wasm-bindgen → phục vụ → Chromium headless (SwiftShader) → PNG.
// Dùng: node tools/verify/wasm-probe/run.mjs [--no-build]
// Kết quả: tools/verify/wasm-probe/probe-{busy,black}.png
import fs from 'node:fs';
import path from 'node:path';
import { ROOT, run, cargoEnv, which } from '../lib/util.mjs';
import { encodePng } from '../lib/png.mjs';
import { serveStatic } from '../lib/serve.mjs';
import { launchBrowser, newPage } from '../lib/browser.mjs';

const HERE = path.join(ROOT, 'tools/verify/wasm-probe');
const TARGET = path.join(ROOT, 'target/wasm');
const OUT = path.join(TARGET, 'probe-web');
const noBuild = process.argv.includes('--no-build');

function die(msg) { console.error('✘ ' + msg); process.exit(1); }

if (!noBuild) {
  const r = await run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-j', '2'], {
    cwd: HERE, env: cargoEnv({ CARGO_TARGET_DIR: TARGET }),
  });
  if (r.code) die('cargo build (wasm) failed');
}
const wasm = path.join(TARGET, 'wasm32-unknown-unknown/release/cm-probe.wasm');
if (!fs.existsSync(wasm)) die('missing ' + wasm);
if (!which('wasm-bindgen')) die('wasm-bindgen not found (cargo install --locked wasm-bindgen-cli --version <Cargo.lock>)');
fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(path.join(OUT, 'assets/fonts'), { recursive: true });
let r = await run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--remove-name-section', '--remove-producers-section', '--out-dir', OUT, '--out-name', 'cm_probe', wasm]);
if (r.code) die('wasm-bindgen failed');

// Sprite mèo 16×16 sinh bằng code (đủ để thấy pixel sắc/nhòe).
const PAL = { '.': [0, 0, 0, 0], k: [26, 20, 30, 255], o: [232, 140, 48, 255], w: [250, 240, 220, 255], r: [200, 40, 50, 255], g: [80, 200, 90, 255] };
const ART = [
  '................',
  '..k.........k...',
  '.kok.......kok..',
  '.kook.....kook..',
  '.kooookkkooook..',
  '.koooooooooook..',
  'kooowkooowkoook.',
  'koooggooooggook.',
  'kooowwoookwwook.',
  '.koooowkwooook..',
  '..kkrrrrrrrkk...',
  '...koooooook....',
  '..koowoooowok...',
  '..koowoooowok...',
  '..kkk.kkk.kkk...',
  '................',
];
const px = new Uint8Array(16 * 16 * 4);
ART.forEach((row, y) => [...row].forEach((c, x) => px.set(PAL[c], (y * 16 + x) * 4)));
fs.writeFileSync(path.join(OUT, 'assets/cat.png'), encodePng(16, 16, px));

const fontCandidates = [
  path.join(ROOT, 'assets/fonts/ui.ttf'),
  '/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf',
  '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  '/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf',
];
const font = fontCandidates.find((f) => fs.existsSync(f));
if (!font) die('no TTF font found');
fs.copyFileSync(font, path.join(OUT, 'assets/fonts/probe.ttf'));
fs.writeFileSync(path.join(OUT, 'index.html'),
  fs.readFileSync(path.join(ROOT, 'tools/verify/web/index.html'), 'utf8').replace('__JS__', 'cm_probe.js'));
console.log(`probe dist: ${OUT} (font ${font}, wasm ${(fs.statSync(path.join(OUT, 'cm_probe_bg.wasm')).size / 1e6).toFixed(1)} MB)`);

const srv = await serveStatic(OUT);
const browser = await launchBrowser();
let failed = false;
for (const bg of ['busy', 'black']) {
  const { ctx, page, errors, logs } = await newPage(browser);
  const t0 = Date.now();
  await page.goto(`${srv.url}?bg=${bg}`);
  try {
    await page.waitForFunction(() => window.__CM_READY === true, null, { timeout: 90_000 });
  } catch {
    errors.push('timeout waiting for __CM_READY (90 s)');
  }
  const file = path.join(HERE, `probe-${bg}.png`);
  await page.screenshot({ path: file });
  console.log(`${errors.length ? '✘' : '✔'} probe bg=${bg} ${Date.now() - t0} ms → ${path.relative(ROOT, file)}`);
  if (errors.length) { failed = true; console.log(errors.join('\n')); console.log(logs.slice(-30).join('\n')); }
  await ctx.close();
}
await browser.close();
await srv.close();
process.exit(failed ? 1 : 0);
