// Drives the built page's worker (web/verify-dist/worker.*.js) under Node, message by message as
// app.js's confirmOnEthereum does, on the anchors of the chains added after the first seven
// (test/anchor/chains/, recorded from public endpoints): Soneium, Zora and World Chain through
// Ethereum's light client, the six testnets through Sepolia's (lightClient "sepolia", started from
// sepolia-checkpoint.json) and BNB Smart Chain through its own (lightClient "bnb", from bnb.json's
// start). Each verifies at the level recorded, the light client asking for its data by name; what
// it keeps, restored in a fresh worker, verifies the block again with nothing fetched; another hash
// does not match; a changed header, another start and a stale BNB start fail. Exits 1 on any surprise.
//   node web/verify/test/anchor-chains.mjs <dist dir>
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const [dist] = process.argv.slice(2);
const workerFile = readdirSync(dist).find((f) => /^worker\.[0-9a-f]+\.js$/.test(f));
const here = dirname(fileURLToPath(import.meta.url));
const DIR = join(here, 'anchor', 'chains');
const read = (name) => JSON.parse(readFileSync(join(DIR, name), 'utf8'));
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}

// wasm-bindgen's web glue fetches its module by URL: under Node, a file URL is read from disk.
const netFetch = globalThis.fetch;
globalThis.fetch = async (url, init) => {
  const u = new URL(String(url));
  if (u.protocol !== 'file:') return netFetch(url, init);
  return new Response(readFileSync(fileURLToPath(u)), { headers: { 'content-type': 'application/wasm' } });
};
// The worker responds to each message once, in order; one message is in flight at a time here.
let pending = null;
globalThis.postMessage = (m) => {
  if (m.type === 'checked' || !pending) return;
  const p = pending;
  pending = null;
  p(m);
};
let instances = 0;
/// A fresh worker (the built worker.js imported anew: light clients of its own): send(message) -> its reply.
async function worker() {
  globalThis.onmessage = null;
  await import(`${pathToFileURL(join(dist, workerFile)).href}?instance=${++instances}`);
  const handler = globalThis.onmessage;
  return (msg) => new Promise((resolve) => {
    pending = resolve;
    handler({ data: msg });
  });
}

const ETH = read('checkpoint.json');
const BNB = read('bnb.json');
const STARTS = { sepolia: read('sepolia-checkpoint.json'), bnb: { number: BNB.start.number, hash: BNB.start.hash } };
const SEPOLIA_CHAINS = new Set([84532, 421614, 11155420, 1301, 763373, 46630]);
const lightClientOf = (id) => (id === 56 ? 'bnb' : SEPOLIA_CHAINS.has(id) ? 'sepolia' : null);

/// The data a light client asks for, as Willow's route for it serves it.
function served(lc, kind, from, count) {
  if (kind === 'need-bootstrap') return body(lc === 'bnb' ? BNB.bootstrap : read(lc === 'sepolia' ? 'sepolia-bootstrap.json' : 'bootstrap.json'));
  if (lc === 'bnb') return body(BNB.updates.filter((u) => u.epoch >= from && u.epoch < from + count));
  return body(read(lc === 'sepolia' ? 'sepolia-updates.json' : 'updates.json'));
}

/// confirmOnEthereum's loop: verify; fetch what the named light client asks for; verify again.
/// { r, saved, asked: ['need-bootstrap:sepolia', ...] }.
async function confirm(send, chainId, number, hash, result, now, starts = STARTS) {
  const own = lightClientOf(chainId);
  const extra = own ? { lightClient: own, start: starts[own] } : {};
  const asked = [];
  for (let round = 0; round < 5; round++) {
    const m = await send({ type: 'anchor-verify', chainId, number, hash, response: body(result), checkpoint: ETH, now, ...extra });
    if (m.type === 'error') return { r: { ok: false, kind: m.kind, message: m.message }, asked };
    const r = m.result;
    if (r.ok) return { r, saved: m.saved, asked };
    const lc = r.lightClient ?? null;
    asked.push(`${r.kind}:${lc ?? 'ethereum'}`);
    if (lc !== own) return { r: { ok: false, kind: 'another-light-client', message: String(lc) }, asked };
    const d = await send({ type: r.kind === 'need-bootstrap' ? 'anchor-bootstrap' : 'anchor-updates', response: served(lc, r.kind, r.from, r.count), checkpoint: ETH, now, ...extra });
    if (d.type === 'error') return { r: { ok: false, kind: d.kind, message: d.message }, asked };
  }
  return { r: { ok: false, kind: 'rounds' }, asked };
}

/// One header byte changed (its state root), as anchor-verify's chains_tests do it.
function changedHeader(result) {
  const t = structuredClone(result);
  const headers = t.headers || (t.parlia && t.parlia.headers);
  const h = headers[0];
  const i = h.length - 300;
  headers[0] = h.slice(0, i) + (h[i] === '0' ? '1' : '0') + h.slice(i + 1);
  return t;
}

const L2 = readdirSync(DIR).filter((f) => /^l2-\d+-[a-z]+\.json$/.test(f)).sort();
const seen = new Set();
for (const file of L2) {
  const f = read(file);
  const lc = lightClientOf(f.chainId);
  seen.add(f.chainId);
  const send = await worker();
  const { r, saved, asked } = await confirm(send, f.chainId, f.block, f.hash, f.result, f.savedAt);
  expect(`${file}: verified ${f.expect.level} by ${f.expect.path}, through ${lc ?? 'Ethereum'}'s light client`, r.ok && r.matches === true && r.level === f.expect.level && r.path === f.expect.path && asked.every((a) => a.endsWith(`:${lc ?? 'ethereum'}`)) && asked.some((a) => a.startsWith('need-bootstrap')), r.ok ? asked.join(' ') : `${r.kind}: ${r.message}`);
  if (!r.ok) continue;
  const other = await confirm(send, f.chainId, f.block, `0x${'ab'.repeat(32)}`, f.result, f.savedAt);
  expect(`${file}: another hash does not match`, other.r.ok && other.r.matches === false && other.asked.length === 0);
  const bent = await confirm(send, f.chainId, f.block, f.hash, changedHeader(f.result), f.savedAt);
  expect(`${file}: a changed header fails`, !bent.r.ok && (bent.r.kind === 'rejected' || bent.r.kind === 'malformed-input'), `${bent.r.kind}: ${bent.r.message}`);
  // What the page keeps, restored in a fresh worker (the next visit), verifies the block with nothing fetched.
  const again = await worker();
  const restored = await again({ type: 'anchor-restore', saved, checkpoint: ETH, now: f.savedAt, ...(lc ? { lightClient: lc, start: STARTS[lc] } : {}) });
  const after = await confirm(again, f.chainId, f.block, f.hash, f.result, f.savedAt);
  expect(`${file}: the kept light client restores and verifies again with nothing fetched`, restored.type === 'anchor-done' && after.r.ok && after.r.matches === true && after.asked.length === 0, `${restored.type} ${after.asked.join(' ')} ${after.r.kind ?? ''}`);
}
expect('Soneium, Zora, World Chain and the six testnets have recorded anchors', [1868, 7777777, 480, ...SEPOLIA_CHAINS].every((id) => seen.has(id)), [...seen].join(' '));

// Sepolia from another start: the recorded bootstrap is not for it.
{
  const f = read(L2.find((x) => x.startsWith('l2-84532-')));
  const send = await worker();
  const starts = { ...STARTS, sepolia: { root: '0x5d5b94c9c82d9f56409699948380d58de837f39a170e0de81c547642d3b58415', slot: 11577216 } };
  const { r } = await confirm(send, f.chainId, f.block, f.hash, f.result, f.savedAt, starts);
  expect("Sepolia from another start: the recorded bootstrap is refused", !r.ok && (r.kind === 'rejected' || r.kind === 'malformed-input'), `${r.kind}: ${r.message}`);
}

// BNB Smart Chain, its own validators followed from the recorded start.
for (const a of BNB.anchors) {
  const send = await worker();
  const { r, saved, asked } = await confirm(send, 56, a.block, a.hash, a.result, a.savedAt);
  expect(`BNB block ${a.block}: verified ${a.expect.level} by its validators' votes`, r.ok && r.matches === true && r.level === a.expect.level && r.path === 'parlia' && asked.length >= 2 && asked.every((x) => x.endsWith(':bnb')), r.ok ? `${r.signers} of ${r.of}; ${asked.join(' ')}` : `${r.kind}: ${r.message}`);
  if (!r.ok) continue;
  const bent = await confirm(send, 56, a.block, a.hash, changedHeader(a.result), a.savedAt);
  expect(`BNB block ${a.block}: a changed header fails`, !bent.r.ok, `${bent.r.kind}`);
  const again = await worker();
  const restored = await again({ type: 'anchor-restore', saved, checkpoint: ETH, now: a.savedAt, lightClient: 'bnb', start: STARTS.bnb });
  const after = await confirm(again, 56, a.block, a.hash, a.result, a.savedAt);
  expect(`BNB block ${a.block}: the kept walk restores and verifies again with nothing fetched`, restored.type === 'anchor-done' && after.r.ok && after.asked.length === 0, `${restored.type} ${after.asked.join(' ')}`);
}
{
  const a = BNB.anchors[0];
  const send = await worker();
  const { r } = await confirm(send, 56, a.block, a.hash, a.result, a.savedAt, { ...STARTS, bnb: { number: 133547000, hash: '0x5256b501fc40848cc63455f3022f45332df90d2776e322470ff2432108015014' } });
  expect('BNB from another start: the recorded bootstrap is refused', !r.ok && (r.kind === 'rejected' || r.kind === 'malformed-input'), `${r.kind}: ${r.message}`);
  const late = await confirm(await worker(), 56, a.block, a.hash, a.result, a.savedAt + 8 * 86400);
  expect('BNB from a start over 7 days old: refused as a stale starting point', !late.r.ok && late.r.kind === 'stale-checkpoint', `${late.r.kind}: ${late.r.message}`);
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
