// Runs the built block and fee verifier (web/verify-dist/items.*.wasm) under Node, as the worker
// and @willow-network/rpc call it, on the block, fee and fee-history proofs in
// zkvm/logset-guest/blockproof/fixtures/ (all seven chains, with publicnode's own responses).
// Clean and tampered cases; every tampered proof must fail with its fault. Exits 1 on any surprise.
//   node web/verify/test/blocks.mjs <dist dir>
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const [dist] = process.argv.slice(2);
const files = readdirSync(dist);
const glue = files.find((f) => /^items\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^items\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, ItemVerifier } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });
const here = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(here, '..', '..', '..', 'zkvm', 'logset-guest', 'blockproof', 'fixtures');
const NITRO = new Set([42161, 4663]);
const OP_STACK = new Set([8453, 10, 130, 57073]);

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });
const load = (dir) =>
  readdirSync(join(FIXTURES, dir))
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((name) => ({ name: name.replace(/\.json$/, ''), f: JSON.parse(readFileSync(join(FIXTURES, dir, name), 'utf8')) }));
/// One nibble `fromEnd` characters from the end flipped.
const flip = (h, fromEnd = 1) => {
  const i = h.length - fromEnd;
  return h.slice(0, i) + (h[i] === '0' ? '1' : '0') + h.slice(i + 1);
};
const swap = (a, i, j) => ([a[i], a[j]] = [a[j], a[i]]);
const hexNum = (h) => parseInt(h, 16);

/// A request of `kind`, then `proof` (changed by `change`) verified.
function run(chainId, kind, target, proof, change) {
  const v = new ItemVerifier(chainId);
  const req = JSON.parse(v.request(kind, target));
  const p = structuredClone(proof);
  if (change) change(p);
  const t = performance.now();
  const r = JSON.parse(v.verify(body(p)));
  return { v, req, r, ms: performance.now() - t };
}

/// A top-level RLP list's items as hex (no prefix), and the list encoded again from items.
function rlpItems(hex) {
  const b = Buffer.from(hex.slice(2), 'hex');
  const head = (at) => {
    const x = b[at];
    if (x < 0x80) return [at, 1];
    if (x < 0xb8) return [at + 1, x - 0x80];
    if (x < 0xc0) return [at + 1 + (x - 0xb7), b.readUIntBE(at + 1, x - 0xb7)];
    if (x < 0xf8) return [at + 1, x - 0xc0];
    return [at + 1 + (x - 0xf7), b.readUIntBE(at + 1, x - 0xf7)];
  };
  const [start, len] = head(0);
  const out = [];
  for (let at = start; at < start + len; ) {
    const [s, l] = head(at);
    out.push(b.subarray(at, s + l).toString('hex'));
    at = s + l;
  }
  return out;
}
function rlpList(items) {
  const payload = Buffer.from(items.join(''), 'hex');
  const n = payload.length;
  const digits = n.toString(16);
  const len = Buffer.from(digits.length % 2 ? `0${digits}` : digits, 'hex');
  const prefix = n < 56 ? Buffer.from([0xc0 + n]) : Buffer.concat([Buffer.from([0xf7 + len.length]), len]);
  return `0x${Buffer.concat([prefix, payload]).toString('hex')}`;
}

// ---------- block proofs ----------

const blocks = load('block');
expect('block fixtures of all seven chains', new Set(blocks.map((b) => b.f.chainId)).size === 7);
for (const { name, f } of blocks) {
  const r0 = f.result;
  const node = f.node.eth_getBlockByNumber;
  const light = { ...node, transactions: node.transactions.map((t) => (typeof t === 'string' ? t : t.hash)) };
  for (const target of [r0.blockHash, String(hexNum(r0.blockNumber))]) {
    const { v, req, r, ms } = run(f.chainId, 'block', target, r0);
    const byHash = target === r0.blockHash;
    expect(`${name} (by ${byHash ? 'hash' : 'number'}): the request`, req.ok && req.method === 'willow_getBlockProof' && JSON.stringify(req.params) === JSON.stringify([byHash ? r0.blockHash.toLowerCase() : r0.blockNumber]), JSON.stringify(req));
    if (!r.ok) {
      expect(`${name}: verifies`, false, r.message);
      continue;
    }
    const b = r.report.block;
    const unproven = name.includes('uncles-unproven');
    expect(
      `${name}: verifies, naming the block to anchor and its parent`,
      b.hash === r0.blockHash.toLowerCase() && b.parentHash === node.parentHash.toLowerCase() && b.timestamp === hexNum(node.timestamp) && b.number === r0.blockNumber && r.report.transactions === node.transactions.length,
      `${ms.toFixed(1)} ms, ${r.report.transactions} transactions`,
    );
    expect(`${name}: unproven fields`, isDeepStrictEqual(r.report.unproven, unproven ? ['size', 'uncles'] : []), JSON.stringify(r.report.unproven));
    const c = JSON.parse(v.compare('answer', body(light)));
    expect(`${name}: proven eth_getBlockByNumber(n, false) is publicnode's`, c.ok && c.result.exact && c.result.unproven.every((k) => k === 'totalDifficulty' || (unproven && ['size', 'uncles'].includes(k))), c.ok ? `${c.result.matched} values, not covered ${JSON.stringify(c.result.unproven)}` : c.message);
    const lazy = JSON.parse(v.block(false));
    expect(`${name}: block(false) is the proven response`, lazy.ok && isDeepStrictEqual(lazy.proven, r.proven));
    const t = performance.now();
    const full = JSON.parse(v.block(true));
    const fullMs = performance.now() - t;
    const ct = JSON.parse(v.compare('transactions', body(node)));
    expect(`${name}: block(true), senders recovered, is publicnode's full block`, full.ok && full.proven.transactions.length === node.transactions.length && ct.ok && ct.result.exact, full.ok ? `${fullMs.toFixed(1)} ms; ${ct.ok ? `${ct.result.matched} values, not covered ${JSON.stringify([...new Set(ct.result.unproven.map((k) => k.replace(/\[\d+\]/g, '[]')))])}` : ct.message}` : full.message);
  }
}

// Each tampered block proof fails with its fault.
const block = (prefix) => blocks.find((b) => b.name.startsWith(prefix)).f;
const named = (name) => blocks.find((b) => b.name === name).f;
const tamper = (label, f, change, fault, target = f.result.blockHash, chainId = f.chainId) => {
  const { v, r } = run(chainId, 'block', target, f.result, change);
  expect(`block: ${label} fails as ${fault}`, !r.ok && r.kind === 'rejected' && r.fault === fault, r.ok ? 'verified' : `${r.kind}/${r.fault}: ${r.message}`);
  const after = JSON.parse(v.block(true));
  if (!after.ok && after.kind !== 'internal') expect(`block: ${label}: nothing rendered after a failure`, false, after.kind);
};
const eth = block('ethereum-26');
tamper('a byte of the header', eth, (p) => (p.header = flip(p.header, 10)), 'header');
tamper('blockHash', eth, (p) => (p.blockHash = flip(p.blockHash)), 'header');
tamper('blockNumber', eth, (p) => (p.blockNumber = `0x${(hexNum(p.blockNumber) + 1).toString(16)}`), 'header');
tamper('chainId', eth, (p) => (p.chainId = '0xa'), 'wrong-chain');
tamper('a transaction changed', eth, (p) => (p.transactions[3] = flip(p.transactions[3], 40)), 'transactions');
tamper('a transaction dropped', eth, (p) => p.transactions.splice(5, 1), 'transactions');
tamper('transactions reordered', eth, (p) => swap(p.transactions, 1, 2), 'transactions');
tamper('no transactions', eth, (p) => delete p.transactions, 'missing');
tamper('a withdrawal amount', eth, (p) => (p.withdrawals = flip(p.withdrawals, 1)), 'withdrawals');
tamper('a withdrawal dropped', eth, (p) => (p.withdrawals = rlpList(rlpItems(p.withdrawals).slice(1))), 'withdrawals');
tamper('withdrawals reordered', eth, (p) => {
  const w = rlpItems(p.withdrawals);
  swap(w, 0, 1);
  p.withdrawals = rlpList(w);
}, 'withdrawals');
tamper('no withdrawals', eth, (p) => delete p.withdrawals, 'missing');
tamper('uncles left unproven where there are none', eth, (p) => (p.unproven = ['size', 'uncles']), 'uncles');
tamper('another block for the hash asked', eth, (p) => Object.assign(p, structuredClone(block('ethereum-17').result)), 'wrong-block');
tamper('another block for the number asked', eth, () => {}, 'wrong-block', String(hexNum(eth.result.blockNumber) + 1));
tamper("another chain's block", block('base-'), (p) => Object.assign(p, structuredClone(block('ink-').result)), 'wrong-chain');
const pre = named('ethereum-1500000');
tamper('an uncle changed', pre, (p) => (p.uncles[0] = flip(p.uncles[0], 50)), 'uncles');
tamper('the uncle dropped', pre, (p) => delete p.uncles, 'uncles');
tamper('withdrawals before Shanghai', pre, (p) => (p.withdrawals = '0xc0'), 'withdrawals');
const op = block('op-');
tamper('an OP-stack block listing withdrawals', op, (p) => (p.withdrawals = eth.result.withdrawals), 'withdrawals');
tamper("a deposit's receipt", op, (p) => (p.receiptProofs.at(-1).value = flip(p.receiptProofs.at(-1).value, 20)), 'receipts');
{
  const { r } = run(1, 'block', eth.result.blockHash, eth.result);
  const v = new ItemVerifier(1);
  v.request('block', eth.result.blockHash);
  const bad = JSON.parse(v.verify('<html>502</html>'));
  expect('block: an unreadable response is malformed-input, without a fault', r.ok && !bad.ok && bad.kind === 'malformed-input' && bad.fault === undefined, JSON.stringify(bad));
  const none = JSON.parse(v.block(false));
  expect('block: block() without a verified block proof is internal', !none.ok && none.kind === 'internal');
}
for (const t of ['finalized', '0x', '-1', '']) {
  const r = JSON.parse(new ItemVerifier(1).request('block', t));
  expect(`block: "${t}" is an invalid request`, !r.ok && r.kind === 'invalid-request', r.message);
}

// ---------- fee proofs ----------

const fees = load('fees');
for (const { name, f } of fees.filter((x) => x.name.endsWith('.fee'))) {
  const r0 = f.result;
  const { v, req, r, ms } = run(f.chainId, 'fees', r0.blockHash, r0);
  expect(`${name}: the request`, req.ok && req.method === 'willow_getFeeProof' && JSON.stringify(req.params) === JSON.stringify([r0.blockHash.toLowerCase()]), JSON.stringify(req));
  expect(`${name}: verifies as willow-fee-v1`, r.ok && isDeepStrictEqual(r.proven, f.verified) && r.report.formula === 'willow-fee-v1' && r.report.block.hash === r0.blockHash.toLowerCase() && /^0x[0-9a-f]{64}$/.test(r.report.block.parentHash), r.ok ? `${ms.toFixed(1)} ms: gasPrice ${r.proven.eth_gasPrice}, tip ${r.proven.eth_maxPriorityFeePerGas}, blob ${r.proven.eth_blobBaseFee}` : r.message);
  if (!r.ok) continue;
  const notes = [];
  for (const [m, theirs] of Object.entries(f.node.responses)) {
    const c = JSON.parse(v.compare(m, body(theirs)));
    if (!c.ok) notes.push(`${m}: ${c.message}`);
    else notes.push(`${m} ${c.result.exact ? 'equal' : `differs (${c.result.differ[0]})`}`);
    // op-geth's tip at the same head is the formula's (design 1.3.6).
    if (m === 'eth_maxPriorityFeePerGas' && OP_STACK.has(f.chainId)) expect(`${name}: eth_maxPriorityFeePerGas equals the node's at the same head`, c.ok && c.result.exact, JSON.stringify(c.result || c));
  }
  console.log(`      publicnode at block ${f.node.head}: ${notes.join('; ')}`);
}
const ethFee = fees.find((x) => x.name === 'ethereum-26391511.fee').f;
const feeTamper = (label, change, fault, chainId = 1) => {
  const { r } = run(chainId, 'fees', ethFee.result.blockHash, ethFee.result, change);
  expect(`fees: ${label} fails as ${fault}`, !r.ok && r.kind === 'rejected' && r.fault === fault, r.ok ? 'verified' : `${r.kind}/${r.fault}: ${r.message}`);
};
feeTamper('another formula', (p) => (p.formula = 'willow-fee-v2'), 'unknown-formula');
feeTamper('chainId', (p) => (p.chainId = '0xa'), 'wrong-chain');
feeTamper('blockHash', (p) => (p.blockHash = flip(p.blockHash)), 'header');
feeTamper('a byte of the newest header', (p) => (p.headers[3] = flip(p.headers[3], 10)), 'header');
feeTamper('two headers swapped', (p) => swap(p.headers, 0, 1), 'broken-link');
feeTamper('a header dropped', (p) => p.headers.shift(), 'wrong-range');
feeTamper('a body for another block', (p) => swap(p.bodies, 0, 1), 'body');
feeTamper('a body dropped', (p) => p.bodies.pop(), 'missing');
feeTamper('a transaction of a body', (p) => (p.bodies[0].transactions[0] = flip(p.bodies[0].transactions[0], 6)), 'transactions');
{
  const op = fees.find((x) => x.name === 'op-159400794.fee').f;
  const { r } = run(10, 'fees', op.result.blockHash, op.result, (p) => (p.bodies[0].receipts[0] = flip(p.bodies[0].receipts[0], 6)));
  expect("fees: an OP-stack receipt's gas fails as receipts", !r.ok && r.fault === 'receipts', r.ok ? 'verified' : `${r.fault}: ${r.message}`);
}

// ---------- fee-history proofs ----------

for (const { name, f } of fees.filter((x) => x.name.endsWith('.history'))) {
  const r0 = f.result;
  const target = JSON.stringify(f.request.params);
  const { v, req, r, ms } = run(f.chainId, 'feeHistory', target, r0);
  expect(`${name}: the request`, req.ok && req.method === 'willow_getFeeHistoryProof' && isDeepStrictEqual(req.params, f.request.params), JSON.stringify(req));
  expect(`${name}: verifies as geth's eth_feeHistory`, r.ok && isDeepStrictEqual(r.proven, f.verified) && r.report.block.hash === r0.blockHash.toLowerCase(), r.ok ? `${ms.toFixed(1)} ms, ${r.proven.gasUsedRatio.length} blocks` : r.message);
  if (!r.ok) continue;
  const c = JSON.parse(v.compare('answer', body(f.node.eth_feeHistory)));
  if (NITRO.has(f.chainId)) console.log(`      Nitro's own eth_feeHistory (differences expected, design 1.3.7): ${c.ok ? `${c.result.differCount} differ, e.g. ${c.result.differ[0] || 'none'}` : c.message}`);
  else expect(`${name}: publicnode's eth_feeHistory is the proven one`, c.ok && c.result.exact, c.ok ? `${c.result.matched} values${c.result.differ.length ? `; ${c.result.differ[0]}` : ''}` : c.message);
}
const base4 = fees.find((x) => x.name === 'base-56049971-4.history').f;
const historyTamper = (label, change, fault) => {
  const { r } = run(8453, 'feeHistory', JSON.stringify(base4.request.params), base4.result, change);
  expect(`fee history: ${label} fails as ${fault}`, !r.ok && r.kind === 'rejected' && r.fault === fault, r.ok ? 'verified' : `${r.kind}/${r.fault}: ${r.message}`);
};
historyTamper('two headers swapped', (p) => swap(p.headers, 1, 2), 'broken-link');
historyTamper("a receipt's cumulative gas", (p) => (p.bodies[0].receipts[0] = flip(p.bodies[0].receipts[0], 6)), 'receipts');
historyTamper('another formula', (p) => (p.formula = 'x'), 'unknown-formula');
historyTamper('chainId', (p) => (p.chainId = '0x1'), 'wrong-chain');
historyTamper('a header dropped', (p) => {
  p.headers.shift();
  p.bodies.shift();
}, 'wrong-range');
for (const [t, why] of [
  ['["0x0", "latest", []]', 'a count of 0'],
  ['["0x20", "latest", [10, 50]]', 'over 16 blocks with rewards'],
  ['["0x4", "latest", [50, 10]]', 'percentiles not increasing'],
  ['["0x4", "latest", [101]]', 'a percentile over 100'],
  ['["0x4", 5, []]', 'a newest block that is not a string'],
  ['{"blockCount": 4}', 'not a list'],
]) {
  const r = JSON.parse(new ItemVerifier(8453).request('feeHistory', t));
  expect(`fee history: ${why} is an invalid request`, !r.ok && r.kind === 'invalid-request', r.message);
}
{
  const r = JSON.parse(new ItemVerifier(42161).request('feeHistory', '["0x20", "0x10", [50]]'));
  expect('fee history: Nitro takes over 16 blocks with rewards (zeros, no bodies)', r.ok && isDeepStrictEqual(r.params, ['0x20', '0x10', [50]]), JSON.stringify(r));
  const t = JSON.parse(new ItemVerifier(1).request('feeHistory', '[5000, "latest"]'));
  expect('fee history: a count over 1,024 is truncated, as geth does', t.ok && isDeepStrictEqual(t.params, ['0x400', 'latest', []]), JSON.stringify(t));
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
