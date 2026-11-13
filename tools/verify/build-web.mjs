#!/usr/bin/env node
// Build bản web của crates/game → dist/web/ (SPEC §6 "Bản web", docs/CONTRACT.md hàng "Bản web").
//
//   node tools/verify/build-web.mjs [--profile wasm-release|release|dev] [--fast] [--opt] [--skip-cargo] [--out dist/web]
//
// Các bước: cargo build -p chin-mang --target wasm32-unknown-unknown (CARGO_TARGET_DIR=target/wasm, -j 2)
//   → wasm-bindgen --target web (out-name chin_mang) → [wasm-opt -Oz nếu --opt và có wasm-opt]
//   → copy assets/ → dist/web/assets/ → index.html (tools/verify/web/index.html, __JS__ = chin_mang.js).
import fs from 'node:fs';
import path from 'node:path';
import { ROOT, run, cargoEnv, which, copyDir, fmtMs, flag } from './lib/util.mjs';

const argv = process.argv.slice(2);
const profile = flag(argv, '--profile') || process.env.CM_WASM_PROFILE || 'wasm-release';
const out = path.resolve(ROOT, flag(argv, '--out') || 'dist/web');
// wasm-opt mặc định cho bản release (không --fast); bỏ qua êm nếu không có wasm-opt trong PATH.
const doOpt = argv.includes('--opt') || (!argv.includes('--no-opt') && !(argv.includes('--fast') || process.env.CM_WASM_FAST === '1') && (flag(argv, '--profile') || process.env.CM_WASM_PROFILE || 'wasm-release') !== 'dev');
const skipCargo = argv.includes('--skip-cargo');
// --fast: codegen-units=16, lto=off cho vòng lặp nhanh (không đổi Cargo.toml).
const fast = argv.includes('--fast') || process.env.CM_WASM_FAST === '1';
const jobs = process.env.CM_JOBS || '2';
const TARGET = path.join(ROOT, 'target/wasm');

function die(msg) { console.error(`✘ build-web: ${msg}`); process.exit(1); }
const t0 = Date.now();

if (!skipCargo) {
  const profArgs = profile === 'dev' ? [] : profile === 'release' ? ['--release'] : ['--profile', profile];
  if (profile !== 'dev') {
    // strip=true xóa custom section `target_features` mà wasm-bindgen cần → chỉ strip debuginfo.
    profArgs.push('--config', `profile.${profile}.strip="debuginfo"`);
    if (fast) profArgs.push('--config', `profile.${profile}.codegen-units=16`, '--config', `profile.${profile}.lto=false`);
  }
  console.log(`▶ cargo build -p chin-mang --target wasm32-unknown-unknown ${profArgs.join(' ')} -j ${jobs}`);
  const r = await run('cargo', ['build', '-p', 'chin-mang', '--target', 'wasm32-unknown-unknown', ...profArgs, '-j', jobs],
    { env: cargoEnv({ CARGO_TARGET_DIR: TARGET }) });
  if (r.code) die('cargo build (wasm32) failed');
}
const profDir = profile === 'dev' ? 'debug' : profile;
const wasm = path.join(TARGET, 'wasm32-unknown-unknown', profDir, 'chin-mang.wasm');
if (!fs.existsSync(wasm)) die(`missing ${path.relative(ROOT, wasm)}`);

if (!which('wasm-bindgen')) die('wasm-bindgen not found — cargo install --locked wasm-bindgen-cli --version <phiên bản wasm-bindgen trong Cargo.lock>');
const lockVer = /name = "wasm-bindgen"\nversion = "([^"]+)"/.exec(fs.readFileSync(path.join(ROOT, 'Cargo.lock'), 'utf8'))?.[1];
const cliVer = (await run('wasm-bindgen', ['--version'], { quiet: true })).out.trim().split(' ').pop();
if (lockVer && cliVer !== lockVer) die(`wasm-bindgen CLI ${cliVer} ≠ Cargo.lock ${lockVer} — cargo install --locked wasm-bindgen-cli --version ${lockVer}`);

// Dựng vào thư mục tạm rồi đổi tên: shots.mjs đang phục vụ `out` không bao giờ thấy thư mục dở dang.
const final = out;
const stage = `${final}.tmp-${process.pid}`;
fs.rmSync(stage, { recursive: true, force: true });
fs.mkdirSync(stage, { recursive: true });
console.log(`▶ wasm-bindgen --target web → ${path.relative(ROOT, final)}`);
let r = await run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--remove-name-section', '--remove-producers-section', '--out-dir', stage, '--out-name', 'chin_mang', wasm]);
if (r.code) { fs.rmSync(stage, { recursive: true, force: true }); die('wasm-bindgen failed'); }

const bg = path.join(stage, 'chin_mang_bg.wasm');
if (doOpt) {
  if (which('wasm-opt')) {
    console.log('▶ wasm-opt -Oz');
    const feats = ['--enable-bulk-memory', '--enable-nontrapping-float-to-int', '--enable-sign-ext',
      '--enable-mutable-globals', '--enable-reference-types', '--enable-multivalue'];
    r = await run('wasm-opt', ['-Oz', ...feats, '-o', bg, bg]);
    if (r.code) die('wasm-opt failed');
  } else {
    console.log('… wasm-opt không có trong PATH — bỏ qua tối ưu (không bắt buộc)');
  }
}

const assets = path.join(ROOT, 'assets');
if (fs.existsSync(assets)) copyDir(assets, path.join(stage, 'assets'), (p) => !p.endsWith('.gitkeep'));
const html = fs.readFileSync(path.join(ROOT, 'tools/verify/web/index.html'), 'utf8').replace('__JS__', 'chin_mang.js');
fs.writeFileSync(path.join(stage, 'index.html'), html);

const mb = (fs.statSync(bg).size / 1e6).toFixed(1);
const old = `${final}.old-${process.pid}`;
if (fs.existsSync(final)) fs.renameSync(final, old);
fs.renameSync(stage, final);
fs.rmSync(old, { recursive: true, force: true });
console.log(`✔ build-web: ${path.relative(ROOT, out)} (wasm ${mb} MB, profile ${profile}) in ${fmtMs(Date.now() - t0)}`);
