// Runs the built state module's trace proofs (web/verify-dist/state.*.wasm, `StateVerifier` kind
// `trace`) under Node, as the worker and @willow-network/rpc call it, on the proofs in
// zkvm/logset-guest/trace-verify/testdata/ (Ethereum and Unichain mainnet, recorded beside the
// trace methods' responses; calls/: Ethereum calls' traces). Each verifies to that response, and each tampered proof fails with
// its kind. With a second argument, every `*.json` there ({params, result, expect}, as the log
// RPC's live test dumps them) is verified and timed too. Exits 1 on any surprise.
//   node web/verify/test/traces.mjs <dist dir> [dump dir]
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const [dist, dumps] = process.argv.slice(2);
const files = readdirSync(dist);
const glue = files.find((f) => /^state\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^state\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, StateVerifier } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });
const here = dirname(fileURLToPath(import.meta.url));
const DATA = join(here, '..', '..', '..', 'zkvm', 'logset-guest', 'trace-verify', 'testdata');

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });

/// `params` asked, `result` verified: the verdict and how long verify took.
function run(chainId, params, result) {
  const v = new StateVerifier(chainId);
  const req = JSON.parse(v.request('trace', JSON.stringify(params)));
  const text = body(result);
  const t = performance.now();
  const r = JSON.parse(v.verify(text));
  return { v, req, r, ms: performance.now() - t, bytes: text.length };
}

for (const file of readdirSync(DATA).filter((f) => f.endsWith('.json')).sort()) {
  const f = JSON.parse(readFileSync(join(DATA, file), 'utf8'));
  const chainId = parseInt(f.block.chainId, 16);
  for (const c of f.cases) {
    const pick = (field) => c.witness[field].map((i) => f.pool[field][i]);
    const proof = { ...f.block, ancestors: c.ancestors, witness: { nodes: pick('nodes'), codes: pick('codes') }, algorithm: 'willow-trace-v1' };
    const [method, params] = c.params;
    const name = `${file} ${method} ${JSON.stringify(params).slice(0, 90)}`;
    const { v, req, r, ms, bytes } = run(chainId, c.params, proof);
    expect(`${name}: asks willow_getTraceProof`, req.ok && req.method === 'willow_getTraceProof' && isDeepStrictEqual(req.params, c.params));
    expect(`${name}: verifies to the trace method's response`, r.ok && isDeepStrictEqual(r.proven.result, c.expect), r.ok ? `${ms.toFixed(1)} ms, ${bytes} bytes` : `${r.kind}: ${r.message}`);
    if (!r.ok) continue;
    expect(`${name}: compares as exact`, JSON.parse(v.compare(method, body(c.expect))).result.exact === true);
    for (const [what, change, kind] of [
      ['no nodes', (p) => (p.witness.nodes = []), 'missing-read'],
      ['a transaction left out', (p) => p.transactions.pop(), 'transactions'],
      ['another parent', (p) => (p.parent = p.header), 'parent'],
    ]) {
      const bent = structuredClone(proof);
      change(bent);
      const out = run(chainId, c.params, bent).r;
      expect(`${name}: ${what} fails as ${kind}`, !out.ok && out.kind === kind, out.ok ? 'verified' : out.kind);
    }
  }
}

// Calls' traces (debug_traceCall, trace_call on Ethereum): each a call proof's block and witness,
// the call run in the module as eth_call runs it, under the caller's overrides, traced.
const CALLS = join(DATA, 'calls');
for (const file of readdirSync(CALLS).filter((f) => f.endsWith('.json')).sort()) {
  const f = JSON.parse(readFileSync(join(CALLS, file), 'utf8'));
  for (const c of f.cases) {
    const [method, params] = c.params;
    const name = `${file} ${method} ${JSON.stringify(params).slice(0, 90)}`;
    const { v, req, r, ms, bytes } = run(1, c.params, c.result);
    expect(`${name}: asks willow_getTraceProof`, req.ok && req.method === 'willow_getTraceProof' && isDeepStrictEqual(req.params, c.params));
    expect(`${name}: verifies to the trace method's response`, r.ok && isDeepStrictEqual(r.proven.result, c.expect), r.ok ? `${ms.toFixed(1)} ms, ${bytes} bytes` : `${r.kind}: ${r.message}`);
    if (!r.ok) continue;
    expect(`${name}: compares as exact`, JSON.parse(v.compare(method, body(c.expect))).result.exact === true);
    for (const [what, change, kind] of [
      ['an account left out', (p) => p.witness.accounts.shift(), 'missing-read'],
      ['another chain', (p) => (p.chainId = '0x5'), 'wrong-chain'],
      ['another algorithm', (p) => (p.algorithm = 'willow-trace-v2'), 'algorithm'],
    ]) {
      const bent = structuredClone(c.result);
      change(bent);
      const out = run(1, c.params, bent).r;
      expect(`${name}: ${what} fails as ${kind}`, !out.ok && out.kind === kind, out.ok ? 'verified' : out.kind);
    }
  }
}

if (dumps) {
  for (const file of readdirSync(dumps).filter((f) => f.endsWith('.json')).sort()) {
    const d = JSON.parse(readFileSync(join(dumps, file), 'utf8'));
    const chainId = parseInt(d.result.chainId, 16);
    const runs = [];
    let r;
    for (let i = 0; i < 3; i++) {
      const x = run(chainId, d.params, d.result);
      r = x.r;
      runs.push(x.ms);
    }
    const same = r.ok && (d.expect === null || isDeepStrictEqual(r.proven.result, d.expect));
    expect(`${file}: verifies in wasm`, same, r.ok ? `MEASURE wasm verify ${runs.map((x) => x.toFixed(0)).join(', ')} ms` : `${r.kind}: ${r.message}`);
  }
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
