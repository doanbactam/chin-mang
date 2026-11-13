// Mở Chromium headless (Playwright) với WebGL2 qua SwiftShader — chạy được trên máy không GPU.
import { chromium } from 'playwright';

export const CHROMIUM_ARGS = [
  '--use-angle=swiftshader',
  '--enable-unsafe-swiftshader',
  '--ignore-gpu-blocklist',
  '--enable-webgl',
  '--disable-background-timer-throttling',
  '--disable-renderer-backgrounding',
  '--disable-backgrounding-occluded-windows',
];

/** headless shell mặc định; CM_CHROMIUM_NEW=1 dùng chế độ --headless=new (channel "chromium"). */
export async function launchBrowser() {
  const opts = { headless: true, args: CHROMIUM_ARGS };
  if (process.env.CM_CHROMIUM_NEW === '1') opts.channel = 'chromium';
  return chromium.launch(opts);
}

/** Trang 1280×600, DPR 1, nền trong suốt; gom console.error / pageerror / request lỗi. */
export async function newPage(browser) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 600 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const errors = [];
  const logs = [];
  const benign = (t) => /Using exceptions for control flow/.test(t);
  page.on('console', (m) => {
    const t = m.text();
    logs.push(`[${m.type()}] ${t}`);
    const src = m.location()?.url || '';
    // 404 của file .meta (Bevy dò meta khi meta_check != Never) không phải lỗi game.
    if (m.type() === 'error' && !benign(t) && !src.endsWith('.meta')) errors.push(`console.error: ${t}${src ? ` (${src})` : ''}`);
  });
  page.on('pageerror', (e) => { if (!benign(String(e))) errors.push(`pageerror: ${e.message}`); });
  page.on('requestfailed', (r) => errors.push(`requestfailed: ${r.url()} ${r.failure()?.errorText}`));
  page.on('response', (r) => {
    if (r.status() >= 400 && !r.url().endsWith('.meta')) errors.push(`http ${r.status()}: ${r.url()}`);
  });
  return { ctx, page, errors, logs };
}
