// The built page, served here and driven by a headless browser against Willow's live RPC on the
// keyless path: a Block link on every chain the page lists (latest; on Ethereum five behind its
// tip, as Willow's anchor service trails the newest block by a slot or two), each confirmed by the light client that
// anchors it (Ethereum's for Ethereum and the L2s on it, Sepolia's for the testnets, BNB Smart
// Chain's own), the light-client data asked on that client's route; the same link again in the same
// browser, now from what it kept; and transaction links on chains added after the first seven. The
// page's /v1/rpc/ requests are fetched from UPSTREAM by this script (the page stays same-origin);
// checkpoint.json is UPSTREAM's (its daily sepolia and bnb starts), or with LOCAL_CHECKPOINT=1 the
// build's own, so the starts pinned in app.js are used. Exits 1 on any chain not verified.
//   PLAYWRIGHT=<dir of the playwright package> [UPSTREAM=https://api.willow.tech] [LOCAL_CHECKPOINT=1] [CHAINS=1,56] node live-chains.mjs <dist> [shots dir]
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { extname, join } from 'node:path';

const [dist, shots] = process.argv.slice(2);
const require = createRequire(import.meta.url);
const engine = require(process.env.PLAYWRIGHT || 'playwright')[process.env.BROWSER || 'chromium'];
const UPSTREAM = (process.env.UPSTREAM || 'https://api.willow.tech').replace(/\/$/, '');
const CSP = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' https: http://127.0.0.1:*; img-src 'self' data:; style-src 'self'; worker-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'application/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.txt': 'text/plain', '.woff2': 'font/woff2', '.json': 'application/json' };
const SEPOLIA_CHAINS = new Set([84532, 421614, 11155420, 1301, 763373, 46630]);
const ALL = [1, 8453, 42161, 4663, 10, 130, 57073, 56, 480, 1868, 7777777, 84532, 421614, 11155420, 1301, 763373, 46630];
const chains = process.env.CHAINS ? process.env.CHAINS.split(',').map(Number) : ALL;
const lightClientOf = (id) => (id === 56 ? 'bnb' : SEPOLIA_CHAINS.has(id) ? 'sepolia' : 'ethereum');
const l1Of = (id) => (id === 1 ? 'Ethereum' : id === 56 ? 'BNB Smart Chain' : SEPOLIA_CHAINS.has(id) ? 'Sepolia' : 'Ethereum');

const server = createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (req.method !== 'GET' || !url.pathname.startsWith('/verify/')) return res.writeHead(404).end();
  const name = url.pathname.slice('/verify/'.length) || 'index.html';
  const file = join(dist, name);
  if (name.includes('..') || !existsSync(file)) return res.writeHead(404).end();
  res.writeHead(200, { 'content-type': TYPES[extname(file)] || 'application/octet-stream', 'content-security-policy': CSP, 'cache-control': 'no-cache' }).end(readFileSync(file));
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;
const live = await (await fetch(`${UPSTREAM}/verify/checkpoint.json`, { cache: 'no-store' })).json();
const local = JSON.parse(readFileSync(join(dist, 'checkpoint.json'), 'utf8'));
const starts = process.env.LOCAL_CHECKPOINT ? null : live;
console.log(`upstream ${UPSTREAM}; checkpoint.json: ${starts ? `live (sepolia slot ${live.sepolia && live.sepolia.slot}, bnb ${live.bnb && live.bnb.number})` : 'the build\'s (pinned starts)'}`);

let failures = 0;
const expect = (name, ok, detail) => {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
};
if (shots) mkdirSync(shots, { recursive: true });
const browser = await engine.launch({ headless: true });
console.log(`browser: ${process.env.BROWSER || 'chromium'} ${browser.version()}`);

async function rpcUp(id, method, params) {
  const r = await fetch(`${UPSTREAM}/v1/rpc/${id}/public`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }) });
  return (await r.json()).result;
}

/// A browser context whose /v1/rpc/ requests go to UPSTREAM; `calls` records each light-client request.
async function context() {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const calls = [];
  const errors = [];
  await ctx.route(`${base}/v1/rpc/**`, async (route) => {
    const req = route.request();
    const path = new URL(req.url()).pathname;
    try {
      const body = req.postDataJSON();
      if (body && /^willow_lightClient/.test(body.method)) calls.push({ path, method: body.method, param: body.params && body.params[0] });
    } catch {}
    try {
      const response = await route.fetch({ url: `${UPSTREAM}${path}`, timeout: 60000 });
      await route.fulfill({ response });
    } catch (e) {
      await route.abort();
    }
  });
  if (starts) await ctx.route(`${base}/verify/checkpoint.json`, (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(starts) }));
  ctx.on('page', (p) => {
    p.on('console', (m) => m.type() === 'error' && !/Failed to load resource/.test(m.text()) && errors.push(m.text()));
    p.on('pageerror', (e) => errors.push(String(e)));
  });
  return { ctx, calls, errors };
}

async function blockLink(c, id, name) {
  const page = await c.ctx.newPage();
  page.setDefaultTimeout(180000);
  const t0 = Date.now();
  const at = id === 1 ? String(parseInt(await rpcUp(1, 'eth_blockNumber', []), 16) - 5) : 'latest';
  await page.goto(`${base}/verify/?chain=${id}&block=${at}`);
  await page.waitForFunction(() => !document.getElementById('result').hidden && !document.getElementById('run').disabled);
  const o = {
    tone: (await page.getAttribute('#verdict', 'class')).replace('verdict ', ''),
    title: await page.textContent('#verdictTitle'),
    text: await page.textContent('#verdictText'),
    anchor: await page.$eval('#steps li:last-child', (li) => ({ state: li.dataset.state, what: li.querySelector('.what').textContent, note: li.querySelector('.note').textContent })),
    secs: ((Date.now() - t0) / 1000).toFixed(1),
  };
  if (shots) await page.screenshot({ path: join(shots, `${name}.png`), fullPage: true });
  await page.close();
  return o;
}

/// The root (or BNB hash) a bootstrap must be asked for: the build's checkpoint on Ethereum; the live
/// starts when the page reads them (they are never older than the ones pinned in app.js); unchecked with LOCAL_CHECKPOINT.
const want = (id) => (lightClientOf(id) === 'ethereum' ? local.root : !starts ? null : lightClientOf(id) === 'bnb' ? live.bnb.hash : live.sepolia.root);
for (const id of chains) {
  const c = await context();
  const o = await blockLink(c, id, `block-${id}`);
  const lc = lightClientOf(id);
  const route = lc === 'ethereum' ? '/v1/rpc/1/public' : `/v1/rpc/${id}/public`;
  const boot = c.calls.filter((x) => x.method === 'willow_lightClientBootstrap');
  const rightRoute = c.calls.every((x) => x.path === route);
  const rightRoot = boot.every((x) => !want(id) || x.param === want(id));
  expect(
    `chain ${id}: Block link verified, confirmed by ${lc === 'ethereum' ? "Ethereum's" : lc === 'sepolia' ? "Sepolia's" : "BNB Smart Chain's"} light client`,
    o.tone === 'ok' && /^Verified: block \d+$/.test(o.title) && o.anchor.state === 'done' && o.anchor.note.includes(l1Of(id)) && boot.length === 1 && rightRoute && rightRoot,
    `${o.secs} s; ${o.tone}: ${o.title} | ${o.anchor.what}: ${o.anchor.note.slice(0, 220)} | light client: ${c.calls.map((x) => `${x.method.replace('willow_lightClient', '')}@${x.path.split('/')[3]}`).join(', ')}`,
  );
  // The same link again in this browser: the light client restores what it kept; no bootstrap.
  if (id === 1 || id === 56 || id === 84532 || id === 1868) {
    const before = c.calls.length;
    const again = await blockLink(c, id, `block-${id}-again`);
    const fetched = c.calls.slice(before).filter((x) => x.method === 'willow_lightClientBootstrap');
    expect(`chain ${id}: again in the same browser, from the kept light client, no bootstrap`, again.tone === 'ok' && fetched.length === 0, `${again.secs} s; ${again.tone}: ${again.title}; bootstraps ${fetched.length}`);
  }
  expect(`chain ${id}: no console errors`, c.errors.length === 0, c.errors.slice(0, 2).join(' | '));
  await c.ctx.close();
}

// Transaction links: the receipt card on chains added after the first seven.
for (const [id, slug] of (process.env.CHAINS ? [] : [[1868, 'soneium'], [84532, 'base-sepolia'], [56, 'bnb'], [46630, 'robinhood-testnet']])) {
  const tip = parseInt(await rpcUp(id, 'eth_blockNumber', []), 16);
  const block = await rpcUp(id, 'eth_getBlockByNumber', [`0x${(tip - 10).toString(16)}`, false]);
  const tx = block.transactions[Math.min(1, block.transactions.length - 1)];
  const c = await context();
  const page = await c.ctx.newPage();
  page.setDefaultTimeout(180000);
  const t0 = Date.now();
  await page.goto(`${base}/verify/?chain=${slug}&tx=${tx}`);
  await page.waitForFunction(() => !document.querySelector('.r-card') || !document.querySelector('.r-card').classList.contains('checking'));
  const seal = await page.$eval('.seal', (s) => ({ cls: s.className, title: s.querySelector('.seal-title').textContent, text: s.querySelector('.seal-text').textContent }));
  const checks = await page.$$eval('.r-checks li', (l) => l.map((li) => `${li.dataset.tone}: ${li.textContent}`));
  if (shots) await page.screenshot({ path: join(shots, `tx-${slug}.png`), fullPage: true });
  expect(`${slug} transaction link: verified, its block confirmed against ${l1Of(id)}`, /\bok\b/.test(seal.cls) && checks.length === 3 && checks.every((x) => x.startsWith('ok')), `${((Date.now() - t0) / 1000).toFixed(1)} s; ${seal.title} | ${checks.join(' | ')}`);
  expect(`${slug} transaction link: no console errors`, c.errors.length === 0, c.errors.slice(0, 2).join(' | '));
  await c.ctx.close();
}

await browser.close();
server.close();
console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
