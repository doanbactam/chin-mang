// Tiện ích chung cho tools/verify: chạy lệnh, môi trường cargo, đo thời gian.
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');

/** Môi trường cho cargo: thêm ~/.cargo/bin và sysroot cục bộ (~/.local/sysroot, xem docs/DECISIONS.md) nếu có. */
export function cargoEnv(extra = {}) {
  const env = { ...process.env };
  const home = os.homedir();
  const sys = path.join(home, '.local/sysroot');
  const add = (k, v) => { env[k] = env[k] ? `${v}:${env[k]}` : v; };
  add('PATH', path.join(home, '.cargo/bin'));
  if (fs.existsSync(sys)) {
    const lib = path.join(sys, 'usr/lib/x86_64-linux-gnu');
    add('PATH', path.join(sys, 'usr/bin'));
    add('LD_LIBRARY_PATH', lib);
    add('PKG_CONFIG_PATH', path.join(lib, 'pkgconfig'));
    add('LIBRARY_PATH', lib);
  }
  const local = path.join(home, '.local/bin');
  if (fs.existsSync(local)) add('PATH', local);
  return { ...env, ...extra };
}

/** Chạy lệnh, stream ra stdout/stderr (hoặc nuốt nếu quiet), trả { code, out }. */
export function run(cmd, args, { cwd = ROOT, env = cargoEnv(), quiet = false, tee = null } = {}) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    const on = (s) => (d) => {
      out += d;
      if (tee) tee.write(d);
      if (!quiet) s.write(d);
    };
    child.stdout.on('data', on(process.stdout));
    child.stderr.on('data', on(process.stderr));
    child.on('error', (e) => resolve({ code: 127, out: out + String(e) }));
    child.on('close', (code) => resolve({ code: code ?? 1, out }));
  });
}

export function which(bin) {
  const r = spawnSync('sh', ['-c', `command -v ${bin}`], { env: cargoEnv(), encoding: 'utf8' });
  return r.status === 0 ? r.stdout.trim() : null;
}

export function copyDir(src, dst, filter = () => true) {
  fs.mkdirSync(dst, { recursive: true });
  for (const e of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, e.name); const d = path.join(dst, e.name);
    if (!filter(s, e)) continue;
    if (e.isDirectory()) copyDir(s, d, filter); else fs.copyFileSync(s, d);
  }
}

export const fmtMs = (ms) => (ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`);

/** Đọc giá trị của cờ `--name value`. */
export function flag(argv, name) {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
}
