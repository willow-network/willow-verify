// Runs the built state, call and estimate verifier (web/verify-dist/state.*.wasm) under Node, as
// the worker and @willow-network/rpc call it, on the proofs in
// zkvm/logset-guest/state-verify/testdata/ (recorded with geth's own responses). Clean and tampered
// cases; every tampered proof must fail with its kind. Exits 1 on any surprise.
//   node web/verify/test/state.mjs <dist dir>
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const [dist] = process.argv.slice(2);
const files = readdirSync(dist);
const glue = files.find((f) => /^state\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^state\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, StateVerifier } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });
const here = dirname(fileURLToPath(import.meta.url));
const DATA = join(here, '..', '..', '..', 'zkvm', 'logset-guest', 'state-verify', 'testdata');

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });
const response = (r) => JSON.stringify({ jsonrpc: '2.0', id: 1, ...r });
const load = (dir) =>
  readdirSync(join(DATA, dir))
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((name) => ({ name: name.replace(/\.json$/, ''), f: JSON.parse(readFileSync(join(DATA, dir, name), 'utf8')) }));
const chainOf = (name) => (name.startsWith('base-') ? 8453 : 1);
const kindOf = (name) => (name.startsWith('call-') ? 'call' : name.startsWith('estimate-') ? 'estimate' : 'state');
const flip = (h) => h.slice(0, -1) + (h.at(-1) === '0' ? '1' : '0');
const USDC = '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48';
const HISTORY = '0x0000f90827f1c53a10cb7a02335b175320002935';

/// The fixture's request made, then its result (changed by `change`) verified.
function run(name, f, change) {
  const v = new StateVerifier(chainOf(name));
  const req = JSON.parse(v.request(kindOf(name), JSON.stringify(f.request.params)));
  const result = structuredClone(f.result);
  if (change) change(result);
  const t = performance.now();
  const r = JSON.parse(v.verify(body(result)));
  return { v, req, r, ms: performance.now() - t };
}
const account = (r, address) => r.witness.accounts.find((a) => a.address.toLowerCase() === address);

// ---------- state proofs ----------

const states = load('state');
for (const { name, f } of states) {
  const { v, req, r, ms } = run(name, f);
  expect(`${name}: the request is the proof method's own params`, req.ok && req.method === 'willow_getStateProof' && isDeepStrictEqual(req.params, f.request.params), JSON.stringify(req).slice(0, 200));
  expect(`${name}: verifies, deriving the standard responses`, r.ok && isDeepStrictEqual(r.proven, f.expect) && r.report.block.hash === f.result.blockHash.toLowerCase() && /^0x[0-9a-f]{64}$/.test(r.report.block.parentHash), r.ok ? `${ms.toFixed(1)} ms` : `${r.kind}: ${r.message}`);
  if (!r.ok) continue;
  const methods = ['eth_getBalance', 'eth_getTransactionCount', 'eth_getProof', ...(f.expect.eth_getCode === null ? [] : ['eth_getCode'])];
  const same = methods.every((m) => JSON.parse(v.compare(m, body(f.expect[m]))).result.exact) && f.expect.eth_getStorageAt.every((s, i) => JSON.parse(v.compare(`eth_getStorageAt:${i}`, body(s))).result.exact);
  expect(`${name}: the same responses compare as exact`, same);
  const other = JSON.parse(v.compare('eth_getBalance', body('0x123456789')));
  expect(`${name}: another balance differs`, other.ok && !other.result.exact && other.result.differ.length === 1, other.ok ? other.result.differ[0] : other.message);
}
const state = (name) => states.find((s) => s.name === name);
const stateTamper = (label, s, change, kind) => {
  const { r } = run(s.name, s.f, change);
  expect(`state: ${label} fails as ${kind}`, !r.ok && r.kind === kind, r.ok ? 'verified' : `${r.kind}: ${r.message}`);
};
const usdc = state('usdc-slots-code');
const eoa = state('eoa');
stateTamper('a byte of the header', eoa, (r) => (r.header = flip(r.header)), 'header');
stateTamper('blockHash', eoa, (r) => (r.blockHash = flip(r.blockHash)), 'header');
stateTamper('blockNumber', eoa, (r) => (r.blockNumber = '0x1'), 'header');
stateTamper('chainId', eoa, (r) => (r.chainId = '0x2'), 'wrong-chain');
stateTamper('another block for the one asked', eoa, (r) => {
  for (const k of ['blockNumber', 'blockHash', 'header']) r[k] = state('base-usdc').f.result[k];
}, 'wrong-block');
stateTamper('the balance', eoa, (r) => (r.account.balance = '0x1'), 'account');
stateTamper('a lower nonce', eoa, (r) => (r.account.nonce = '0x0'), 'account');
stateTamper('an account proof node', eoa, (r) => (r.account.accountProof[1] = flip(r.account.accountProof[1])), 'account');
stateTamper('a proof of another account', eoa, (r) => (r.account = structuredClone(usdc.f.result.account)), 'account');
stateTamper('an absent account given a balance', state('absent'), (r) => (r.account.balance = '0x1'), 'account');
stateTamper('a slot value', usdc, (r) => (r.account.storageProof[0].value = '0x1'), 'slot');
stateTamper('the proof of another slot', usdc, (r) => (r.account.storageProof[0].key = '0x2'), 'slot');
stateTamper('a slot dropped', usdc, (r) => r.account.storageProof.pop(), 'slot');
stateTamper('a storage proof node', usdc, (r) => (r.account.storageProof[0].proof[1] = flip(r.account.storageProof[0].proof[1])), 'slot');
stateTamper('code swapped (same length, other hash)', usdc, (r) => (r.code = flip(r.code)), 'code');
stateTamper('code left out', usdc, (r) => delete r.code, 'code');
stateTamper('a result that is not one', usdc, (r) => {
  for (const k of Object.keys(r)) delete r[k];
}, 'malformed');
{
  const v = new StateVerifier(1);
  v.request('state', JSON.stringify(eoa.f.request.params));
  const r = JSON.parse(v.verify('<html>502</html>'));
  expect('state: an unreadable response is malformed', !r.ok && r.kind === 'malformed', r.message);
}

// ---------- calls and estimates ----------

const calls = load('call');
for (const { name, f } of calls) {
  const { v, req, r, ms } = run(name, f);
  const method = kindOf(name) === 'call' ? 'eth_call' : 'eth_estimateGas';
  expect(`${name}: the request`, req.ok && req.method === f.request.method && isDeepStrictEqual(req.params, f.request.params), JSON.stringify(req).slice(0, 200));
  expect(
    `${name}: verifies, its own run coming to geth's ${method}`,
    r.ok && isDeepStrictEqual(r.proven, f.expect) && (kindOf(name) !== 'estimate' || r.report.algorithm === 'geth-v1.17.10-estimate'),
    r.ok ? `${ms.toFixed(0)} ms, ${f.result.witness.accounts.length} accounts: ${JSON.stringify(r.proven).slice(0, 90)}` : `${r.kind}: ${r.message}`,
  );
  if (!r.ok) continue;
  const c = JSON.parse(v.compare(method, response(f.reference[method])));
  expect(`${name}: ${f.reference.clientVersion.split('/')[1]}'s own response is the proven one`, c.ok && c.result.exact, c.ok ? c.result.differ.join('; ') : c.message);
}
const call = (name) => calls.find((c) => c.name === name);
const callTamper = (label, c, change, kind) => {
  const { r } = run(c.name, c.f, change);
  expect(`${kindOf(c.name)}: ${label} fails as ${kind}`, !r.ok && r.kind === kind, r.ok ? 'verified' : `${r.kind}: ${r.message}`);
};
const balance = call('call-usdc-balance');
callTamper('an account dropped from the witness', balance, (r) => (r.witness.accounts = r.witness.accounts.filter((a) => a.address.toLowerCase() !== USDC)), 'missing-read');
callTamper('a slot dropped', balance, (r) => account(r, USDC).storageProof.pop(), 'missing-read');
callTamper('a slot value', balance, (r) => (account(r, USDC).storageProof[0].value = '0x1234'), 'slot');
callTamper('a shared proof node', balance, (r) => (r.witness.nodes[0] = flip(r.witness.nodes[0])), 'account');
callTamper('code swapped (same length, other hash)', balance, (r) => {
  for (const k of Object.keys(r.witness.codes)) r.witness.codes[k] = flip(r.witness.codes[k]);
}, 'code');
callTamper('code left out', balance, (r) => (r.witness.codes = {}), 'code');
callTamper('a different result', balance, (r) => (r.result = { returned: flip(r.result.returned) }), 'outcome');
callTamper('a revert for a result', balance, (r) => (r.result = { reverted: '0x' }), 'outcome');
callTamper('a byte of the header', balance, (r) => (r.header = flip(r.header)), 'header');
callTamper('chainId', balance, (r) => (r.chainId = '0x2'), 'wrong-chain');
const hashes = call('call-blockhash');
callTamper('the history account with another code hash', hashes, (r) => (account(r, HISTORY).codeHash = `0x${'11'.repeat(32)}`), 'history');
callTamper('a history slot dropped', hashes, (r) => account(r, HISTORY).storageProof.pop(), 'missing-read');
const transfer = call('estimate-transfer');
const gas = parseInt(transfer.f.result.estimate.gas, 16);
callTamper('another algorithm', transfer, (r) => (r.algorithm = 'geth-v1.16.0-estimate'), 'algorithm');
callTamper('the estimate + 1', transfer, (r) => (r.estimate = { gas: `0x${(gas + 1).toString(16)}` }), 'estimate');
callTamper('the estimate - 1', transfer, (r) => (r.estimate = { gas: `0x${(gas - 1).toString(16)}` }), 'estimate');
callTamper('an account dropped from the witness', call('estimate-uniswap-swap'), (r) => r.witness.accounts.pop(), 'missing-read');
{
  // An account of another proof at the same block is extra and unrelated: allowed, ignored.
  const weth = call('estimate-weth-deposit').f.result.witness;
  const extra = structuredClone(weth.accounts.find((a) => a.address.toLowerCase() === '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'));
  const expand = (list) => list.map((n) => (/^#\d+$/.test(n) ? weth.nodes[Number(n.slice(1))] : n));
  extra.accountProof = expand(extra.accountProof);
  for (const s of extra.storageProof) s.proof = expand(s.proof);
  const { r } = run(balance.name, balance.f, (p) => {
    p.witness.accounts.push(extra);
    p.witness.codes[extra.codeHash.toLowerCase()] = weth.codes[extra.codeHash.toLowerCase()];
  });
  expect('call: an extra, unrelated account in the witness is ignored', r.ok && isDeepStrictEqual(r.proven, balance.f.expect), r.ok ? '' : `${r.kind}: ${r.message}`);
}

// ---------- requests ----------

const a = '0x28c6c06298d514db089934071355e5743bf21d60';
for (const [kind, t, why] of [
  ['state', ['0x12', [], 'latest'], 'an address that is not one'],
  ['state', [a, Array(257).fill('0x1'), 'latest'], 'over 256 storage keys'],
  ['state', [a, [], 'yesterday'], 'a block that is not one'],
  ['call', [{ to: a }, 'latest', { [a]: { balance: '0x1' } }], 'a state override'],
  ['estimate', [{ to: a, gasPrice: '0x1', maxFeePerGas: '0x1' }, 'latest'], 'both gas prices'],
]) {
  const r = JSON.parse(new StateVerifier(1).request(kind, JSON.stringify(t)));
  expect(`${kind}: ${why} is an invalid request`, !r.ok && r.kind === 'invalid-request', r.message);
}
{
  const r = JSON.parse(new StateVerifier(8453).request('call', JSON.stringify([{ to: a }, 'latest'])));
  expect('call: a call on an L2 is unsupported (the client sends it as it is)', !r.ok && r.kind === 'unsupported', r.message);
  const v = new StateVerifier(1);
  expect('verify before any request is internal', JSON.parse(v.verify(body({}))).kind === 'internal');
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
