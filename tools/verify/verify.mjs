#!/usr/bin/env node
// Kiểm chứng toàn bộ theo SPEC §10.1 (thứ tự cố định, thoát ≠ 0 nếu bước nào lỗi).
//
//   node tools/verify/verify.mjs [--allow-blocked] [--skip-web] [--only id[,id…]] [--keep-going]
//
//   --allow-blocked  bước bị MÔI TRƯỜNG chặn (thiếu toolchain/target/trình duyệt…) được đánh dấu ⚠ BLOCKED
//                    và không làm fail; vẫn in rõ lý do. Không có cờ này: BLOCKED = lỗi.
//   --skip-web       bỏ build wasm + chụp ảnh (chạy nhanh); in rõ là đã bỏ, không đạt DoD.
//   --keep-going     chạy tiếp các bước sau khi có bước lỗi (mặc định: dừng ở lỗi đầu tiên).
//   --only           chỉ chạy các bước có id trong danh sách (fmt,clippy,test,gates,art,audio,font,stubs,win,wasm,shots).
//
// Log đầy đủ từng bước: target/verify-logs/<id>.log; tóm tắt: target/verify-logs/summary.json.
import fs from 'node:fs';
import path from 'node:path';
import { ROOT, run, cargoEnv, which, fmtMs, flag } from './lib/util.mjs';

const argv = process.argv.slice(2);
const allowBlocked = argv.includes('--allow-blocked');
const skipWeb = argv.includes('--skip-web');
const keepGoing = argv.includes('--keep-going');
const only = flag(argv, '--only')?.split(',');
const LOGS = path.join(ROOT, 'target/verify-logs');
fs.mkdirSync(LOGS, { recursive: true });

const exists = (p) => fs.existsSync(path.join(ROOT, p));
const installedTargets = () => {
  const r = which('rustup') ? (globalThis.__targets ??= run('rustup', ['target', 'list', '--installed'], { quiet: true })) : null;
  return r;
};
const blocked = (reason) => ({ blocked: reason });

/** Mỗi bước: id, tiêu đề, cmd/args/env, pre() → null | {blocked} | {fail}, classify(out) → lý do BLOCKED nếu lỗi do môi trường. */
const STEPS = [
  { id: 'fmt', title: 'cargo fmt --check', cmd: 'cargo', args: ['fmt', '--all', '--', '--check'] },
  { id: 'clippy', title: 'cargo clippy --workspace -D warnings', cmd: 'cargo', args: ['clippy', '--workspace', '-j', '2', '--', '-D', 'warnings'] },
  { id: 'test', title: 'cargo test --workspace', cmd: 'cargo', args: ['test', '--workspace', '-j', '2'] },
  { id: 'gates', title: 'bot balance gates (cm-bot gates)', cmd: 'cargo', args: ['run', '-p', 'cm-bot', '--release', '-j', '2', '--', 'gates'] },
  {
    id: 'art', title: 'art pipeline + lint (build-atlas)', cmd: 'node', args: ['tools/art/build-atlas.mjs'],
    pre: () => (exists('tools/art/build-atlas.mjs') ? null : { fail: 'thiếu tools/art/build-atlas.mjs' }),
  },
  {
    id: 'audio', title: 'audio verify', cmd: 'node', args: ['tools/audio/verify.mjs'],
    pre: () => (exists('tools/audio/verify.mjs') ? null : { fail: 'thiếu tools/audio/verify.mjs' }),
  },
  {
    id: 'font', title: 'font glyph check (fontcheck.mjs)', cmd: 'node', args: ['tools/verify/fontcheck.mjs'],
    pre: () => (exists('tools/verify/fontcheck.mjs') ? null : { fail: 'thiếu tools/verify/fontcheck.mjs' }),
  },
  {
    id: 'stubs', title: 'quét todo!/unimplemented!/TODO (SPEC §0.3, chỉ cảnh báo)', fn: scanStubs, advisory: true,
  },
  {
    id: 'win', title: 'cargo check -p chin-mang --target x86_64-pc-windows-gnu', cmd: 'cargo',
    args: ['check', '-p', 'chin-mang', '--target', 'x86_64-pc-windows-gnu', '-j', '2'],
    env: { CARGO_TARGET_DIR: path.join(ROOT, 'target/win') },
    pre: async () => {
      const r = await installedTargets();
      if (!r || !r.out.includes('x86_64-pc-windows-gnu')) return blocked('chưa cài target: rustup target add x86_64-pc-windows-gnu');
      return null;
    },
    classify: (out) => (/x86_64-w64-mingw32-(gcc|ar|dlltool)|failed to find tool|dlltool/i.test(out)
      ? 'thiếu toolchain mingw-w64 cho build script C (cần x86_64-w64-mingw32-gcc)' : null),
  },
  {
    id: 'wasm', title: 'build web (wasm32 + wasm-bindgen → dist/web)', cmd: 'node', args: ['tools/verify/build-web.mjs'], web: true,
    pre: async () => {
      const r = await installedTargets();
      if (!r || !r.out.includes('wasm32-unknown-unknown')) return blocked('chưa cài target: rustup target add wasm32-unknown-unknown');
      if (!which('wasm-bindgen')) return blocked('thiếu wasm-bindgen CLI (cargo install --locked wasm-bindgen-cli --version <Cargo.lock>)');
      return null;
    },
  },
  {
    id: 'shots', title: 'ảnh chụp Chromium headless (shots.mjs)', cmd: 'node', args: ['tools/verify/shots.mjs'], web: true,
    pre: async () => {
      try {
        const { chromium } = await import('playwright');
        if (!fs.existsSync(chromium.executablePath())) return blocked('chưa tải Chromium: npx playwright install chromium-headless-shell');
      } catch {
        return blocked('chưa cài playwright: npm i (ở gốc repo)');
      }
      return null;
    },
    classify: (out) => (/browserType\.launch|error while loading shared libraries/.test(out) ? 'Chromium không khởi động được (thiếu thư viện hệ thống?)' : null),
  },
];

async function scanStubs(log) {
  const bad = [];
  const re = /\btodo!\s*\(|\bunimplemented!\s*\(|\bTODO\b|\bFIXME\b/;
  const walk = (dir, exts) => {
    if (!fs.existsSync(dir)) return;
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) { if (!['node_modules', 'target'].includes(e.name)) walk(p, exts); continue; }
      if (!exts.some((x) => p.endsWith(x))) continue;
      fs.readFileSync(p, 'utf8').split('\n').forEach((line, i) => {
        if (re.test(line)) bad.push(`${path.relative(ROOT, p)}:${i + 1}: ${line.trim().slice(0, 120)}`);
      });
    }
  };
  walk(path.join(ROOT, 'crates'), ['.rs', '.ron']);
  walk(path.join(ROOT, 'tools/art'), ['.mjs']);
  walk(path.join(ROOT, 'tools/audio'), ['.mjs']);
  log.write(bad.join('\n') + '\n');
  if (bad.length) console.log(bad.slice(0, 20).map((l) => '    ' + l).join('\n') + (bad.length > 20 ? `\n    … +${bad.length - 20}` : ''));
  return { code: bad.length ? 1 : 0, out: bad.join('\n') };
}

const results = [];
let failed = false;
const t00 = Date.now();
console.log(`verify · ${new Date().toISOString()}${allowBlocked ? ' · --allow-blocked' : ''}${skipWeb ? ' · --skip-web' : ''}`);
for (const s of STEPS) {
  if (only && !only.includes(s.id)) continue;
  const label = `${s.id.padEnd(6)} ${s.title}`;
  if (failed && !keepGoing) { results.push({ id: s.id, status: 'not-run' }); console.log(`·  ${label} — chưa chạy (bước trước lỗi)`); continue; }
  if (skipWeb && s.web) { results.push({ id: s.id, status: 'skipped' }); console.log(`–  ${label} — BỎ QUA (--skip-web)`); continue; }
  const t0 = Date.now();
  const logFile = path.join(LOGS, `${s.id}.log`);
  const log = fs.createWriteStream(logFile);
  let status; let reason = '';
  const pre = s.pre ? await s.pre() : null;
  if (pre?.blocked || pre?.fail) {
    reason = pre.blocked || pre.fail;
    status = pre.blocked ? 'blocked' : 'fail';
    log.write(reason + '\n');
  } else {
    let r;
    if (s.fn) r = await s.fn(log);
    else {
      log.write(`$ ${s.cmd} ${s.args.join(' ')}\n`);
      r = await run(s.cmd, s.args, { env: cargoEnv(s.env || {}), quiet: true, tee: log });
    }
    if (r.code === 0) status = 'ok';
    else {
      const why = s.classify?.(r.out);
      status = why ? 'blocked' : 'fail';
      reason = why || `exit ${r.code}`;
      if (!s.fn) {
        const tail = r.out.trim().split('\n').slice(-25).map((l) => '    ' + l).join('\n');
        console.log(tail);
      }
    }
  }
  log.end();
  const ms = Date.now() - t0;
  if (s.advisory && status === 'fail') { status = 'warn'; reason = `${reason} (chỉ cảnh báo)`; }
  const mark = { ok: '✔', fail: '✘', warn: '⚠', blocked: allowBlocked ? '⚠' : '✘' }[status];
  const extra = status === 'ok' ? '' : status === 'blocked' ? ` — BLOCKED: ${reason}` : ` — ${reason}`;
  console.log(`${mark}  ${label} (${fmtMs(ms)})${extra}  [log: ${path.relative(ROOT, logFile)}]`);
  results.push({ id: s.id, title: s.title, status, reason, ms });
  if (status === 'fail' || (status === 'blocked' && !allowBlocked)) failed = true;
}
fs.writeFileSync(path.join(LOGS, 'summary.json'), JSON.stringify({ at: new Date().toISOString(), allowBlocked, skipWeb, results }, null, 2));
const n = (st) => results.filter((r) => r.status === st).length;
console.log(`\n${failed ? '✘ FAIL' : '✔ PASS'} · ok ${n('ok')} · fail ${n('fail')} · warn ${n('warn')} · blocked ${n('blocked')} · skipped ${n('skipped')} · not-run ${n('not-run')} · ${fmtMs(Date.now() - t00)}`);
if (!failed && (n('blocked') || n('skipped'))) console.log('  (chú ý: có bước BLOCKED/BỎ QUA — chưa đạt DoD §12.1 nếu chưa ghi rõ trong docs/VERIFY.md)');
process.exit(failed ? 1 : 0);
