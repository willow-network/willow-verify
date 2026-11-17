// Runs the built transaction and receipt verifier (web/verify-dist/items.*.wasm) under Node on the
// proofs saved in test/items/, as the worker calls it. Positive and tampered cases; exits 1 on any
// surprise.
//   node web/verify/test/items.mjs <dist dir>
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const [dist] = process.argv.slice(2);
const files = readdirSync(dist);
const glue = files.find((f) => /^items\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^items\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, ItemVerifier } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });
const here = dirname(fileURLToPath(import.meta.url));
const ITEMS = join(here, 'items');

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });
const flip = (h, at) => {
  const i = 2 + (at % (h.length - 2));
  return h.slice(0, i) + (h[i] === '0' ? '1' : '0') + h.slice(i + 1);
};

/// The case's request made, then its proof (changed by `change`) verified.
function run(c, change) {
  const v = new ItemVerifier(c.chainId);
  const req = JSON.parse(v.request(c.kind, c.target));
  const proof = structuredClone(c.proof.result);
  if (change) change(proof);
  const t = performance.now();
  const r = JSON.parse(v.verify(body(proof)));
  return { v, req, r, ms: performance.now() - t };
}

for (const file of readdirSync(ITEMS).filter((f) => f.endsWith('.json')).sort()) {
  const name = file.replace(/\.json$/, '');
  const c = JSON.parse(readFileSync(join(ITEMS, file), 'utf8'));
  const { v, req, r, ms } = run(c);
  expect(`${name}: the request is the saved one`, req.ok && req.method === c.proof.method && JSON.stringify(req.params) === JSON.stringify(c.proof.params), JSON.stringify(req));
  expect(`${name}: verifies`, r.ok && r.report.block.hash === c.block.hash, `${r.ok ? `${ms.toFixed(1)} ms, uncovered ${JSON.stringify(r.report.unproven)}` : r.message}`);
  c.answers.forEach((a, i) => {
    const out = JSON.parse(v.compare(i === 0 ? 'answer' : 'transactions', body(a.result)));
    expect(`${name}: Willow's ${a.method} matches the proven one`, out.ok && out.result.exact, out.ok ? `${out.result.matched} values` : out.message);
  });
  const tamper = c.proof.result.receipts
    ? (p) => (p.receipts[p.receipts.length - 1] = flip(p.receipts[p.receipts.length - 1], 20))
    : (p) => {
        const items = p.receiptProofs || p.transactionProofs;
        const it = items[items.length - 1];
        it.proof[0] = flip(it.proof[0], 40);
      };
  const bad = run(c, tamper).r;
  expect(`${name}: a tampered proof is rejected`, !bad.ok && bad.kind === 'rejected', bad.message);
  const header = run(c, (p) => (p.header = flip(p.header, 100))).r;
  expect(`${name}: a tampered header is rejected`, !header.ok && header.kind === 'rejected', header.message);
}

// What was typed, checked before anything is asked.
const v = new ItemVerifier(1);
for (const [kind, text] of [['transaction', '12345'], ['receipt', 'latest:x'], ['blockReceipts', 'finalized']]) {
  const r = JSON.parse(v.request(kind, text));
  expect(`"${text}" for a ${kind} is an invalid request`, !r.ok && r.kind === 'invalid-request', r.message);
}
// Another chain's proof.
const uni = JSON.parse(readFileSync(join(ITEMS, 'uni-receipt.json'), 'utf8'));
const other = new ItemVerifier(1);
other.request('receipt', uni.target);
const r = JSON.parse(other.verify(body(uni.proof.result)));
expect("another chain's proof is rejected", !r.ok && r.kind === 'rejected', r.message);

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
