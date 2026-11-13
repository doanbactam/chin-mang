// Server tĩnh tối thiểu (không phụ thuộc ngoài) để phục vụ dist/web cho Chromium headless.
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.ogg': 'audio/ogg',
  '.wav': 'audio/wav',
  '.css': 'text/css; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.ron': 'text/plain; charset=utf-8',
};

/** Phục vụ `root` trên 127.0.0.1 (cổng ngẫu nhiên nếu port=0). Trả về { url, close, misses }. */
export async function serveStatic(root, port = 0) {
  root = path.resolve(root);
  const misses = [];
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    let p = path.normalize(path.join(root, decodeURIComponent(u.pathname)));
    if (!p.startsWith(root)) { res.writeHead(403).end(); return; }
    if (p.endsWith(path.sep)) p = path.join(p, 'index.html');
    fs.stat(p, (err, st) => {
      if (err || !st.isFile()) {
        // Bevy thử .meta trước khi tải asset nếu meta_check != Never — không coi là lỗi.
        if (!p.endsWith('.meta')) misses.push(u.pathname);
        res.writeHead(404, { 'content-type': 'text/plain' }).end('not found');
        return;
      }
      res.writeHead(200, {
        'content-type': MIME[path.extname(p).toLowerCase()] || 'application/octet-stream',
        'content-length': st.size,
        'cache-control': 'no-store',
      });
      fs.createReadStream(p).pipe(res);
    });
  });
  await new Promise((ok) => server.listen(port, '127.0.0.1', ok));
  const { port: actual } = server.address();
  return {
    url: `http://127.0.0.1:${actual}/`,
    misses,
    close: () => new Promise((ok) => server.close(ok)),
  };
}

// Cho phép chạy tay: node tools/verify/lib/serve.mjs dist/web 8080
if (import.meta.url === `file://${process.argv[1]}`) {
  const s = await serveStatic(process.argv[2] || 'dist/web', Number(process.argv[3] || 8080));
  console.log(`serving ${process.argv[2] || 'dist/web'} at ${s.url}`);
}
