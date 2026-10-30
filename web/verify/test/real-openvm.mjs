// Runs the built wasm verifiers (web/verify-dist) under Node on real OpenVM proofs: the log
// verifier, which checks the answer and picks each proof's keys, and the OpenVM module it calls
// for each OpenVM proof, wired as worker.js wires them.
//   node web/verify/test/real-openvm.mjs <dist dir> [runs]
// Data: test/logs/eth-26069632 (an Ethereum leaf and the L1 over its 64-block window) and
// test/logs/base-52969856 (a Base L1 over two 64-block leaves), from the 10/28 GPU run under the
// minted programs (README.txt in each). Ethereum's cutover is pinned at 26,361,856 since 10/29,
// after these blocks, so the Ethereum answers must be refused at the pin; the dist must be the test
// build (build.sh with CHECKPOINT=test), whose log verifier takes a test cutover
// (Verifier.setTestCutover), and under one at their first block they run in full, as Base's does
// under no cutover. Each <name>.json is {filter, logs, result}. The L1s carry
// their children's proofs as deferrals. Positive and negative cases, and verify times; exits 1 on
// any surprise.
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';

const [dist, runsArg] = process.argv.slice(2);
if (!dist) {
  console.error('usage: node real-openvm.mjs <dist dir> [runs]');
  process.exit(2);
}
const RUNS = Number(runsArg || 5);
const here = dirname(fileURLToPath(import.meta.url));
const files = readdirSync(dist);
const find = (re) => files.find((f) => re.test(f));
const load = async (stem) => {
  const glue = await import(pathToFileURL(join(dist, find(new RegExp(`^${stem}\\.[0-9a-f]+\\.js$`)))).href);
  const wasm = readFileSync(join(dist, find(new RegExp(`^${stem}\\.[0-9a-f]+\\.wasm$`))));
  const t = performance.now();
  glue.initSync({ module: wasm });
  return { glue, ms: performance.now() - t, bytes: wasm.length };
};
const logs = await load('verifier');
const ovm = await load('openvm');
const { Verifier } = logs.glue;
const { OpenVmVerifier } = ovm.glue;
let t = performance.now();
const ov = new OpenVmVerifier();
const keysMs = performance.now() - t;
console.log(`modules: log verifier ${logs.bytes} B (instantiated in ${logs.ms.toFixed(1)} ms), OpenVM ${ovm.bytes} B (${ovm.ms.toFixed(1)} ms; pinned keys loaded in ${keysMs.toFixed(1)} ms)`);

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}

const body = (result) => `{"jsonrpc":"2.0","id":1,"result":${JSON.stringify(result)}}`;
const clone = (x) => JSON.parse(JSON.stringify(x));
const hex = (b) => Buffer.from(b).toString('hex');
const flipHex = (s, at) => {
  const i = 2 + at;
  return s.slice(0, i) + (s[i] === '0' ? '1' : '0') + s.slice(i + 1);
};

// The two modules trust the same OpenVM programs (both read pins/program-keys.txt).
const ovKeys = JSON.parse(ov.keys());
const pinned = JSON.parse(new Verifier(1, '', '').keys()).pinned.openvm;
expect('the OpenVM module verifies under the log verifier\'s pinned OpenVM keys', ovKeys.ok && ovKeys.leaf === pinned.leaf[0] && ovKeys.l1 === pinned.segment[0], JSON.stringify(ovKeys));
// Only the test build's log verifier takes a test cutover; build.sh refuses any other build with it.
const testBuild = typeof Verifier.prototype.setTestCutover === 'function';
expect('the dist is the test build (CHECKPOINT=test), whose log verifier takes a test cutover', testBuild);

const CASES = [
  { name: 'Ethereum leaf (block 26069632)', file: 'eth-26069632/block.json', kind: 'leaf' },
  { name: 'Ethereum L1 (26069632-26069695, 64 one-block leaves deferred)', file: 'eth-26069632/segment.json', kind: 'l1' },
  { name: 'Base L1 (52969856-52969983, two 64-block leaves deferred)', file: 'base-52969856/segment.json', kind: 'l1' },
];

for (const c of CASES) {
  const data = JSON.parse(readFileSync(join(here, 'logs', c.file), 'utf8'));
  const store = join(here, 'logs', dirname(c.file), 'store');
  const answer = data.result;
  const chainId = Number(answer.chainId);
  const proofFile = (id) => new Uint8Array(readFileSync(join(store, `${id}.proof`)));
  // The chain's first OpenVM block for a run: 'pinned' (the verifier's own), or a test cutover (a
  // block number, or null for none).
  let cut = 'pinned';
  // As worker.js: the log verifier calls the OpenVM module for each OpenVM proof.
  const run = ({ ans = answer, proofs = proofFile, openvm = (k, p, v) => ov.check(k, p, v), seg = '', leaf = '', cutover = cut } = {}) => {
    const v = new Verifier(chainId, seg, leaf);
    if (cutover !== 'pinned') v.setTestCutover(cutover === null ? undefined : BigInt(cutover));
    const checks = [];
    let t0 = 0;
    const get = (id) => {
      t0 = performance.now();
      return proofs(id);
    };
    const done = (id, err, system) => checks.push({ id, ms: performance.now() - t0, err, system });
    try {
      const r = JSON.parse(v.verify(body(ans), JSON.stringify(data.filter), get, done, openvm));
      return { r, checks };
    } catch (e) {
      return { threw: String(e.message || e), checks };
    } finally {
      v.free();
    }
  };
  const neg = (name, opts, want) => {
    const { r, threw } = run(opts);
    expect(`${c.file}: ${name}`, !threw && r && !r.ok && r.kind === 'rejected' && (!want || want.test(r.message)), threw || (r.ok ? `verified ${r.report.logs} logs` : `${r.kind}: ${r.message}`));
  };

  console.log(`-- ${c.name}`);
  const kv = new Verifier(chainId, '', '');
  const pinnedFrom = JSON.parse(kv.keys()).openvmFrom;
  kv.free();
  const first = Number(answer.fromBlock);
  const last = Number(answer.toBlock);
  const verifies = (name, opts) => {
    const { r, threw } = run(opts);
    expect(`${c.file}: ${name}`, !threw && r && r.ok && r.report.logs === data.logs, threw || (r.ok ? `${r.report.logs} logs` : `${r.kind}: ${r.message}`));
  };
  const wrongSystem = (name, opts) => neg(name, opts, /is proven with OpenVM, but this chain uses SP1 there/);
  if (pinnedFrom != null && last < Number(pinnedFrom)) {
    // A fixture from before its chain's pinned cutover (Ethereum's: block 26,361,856, pinned
    // 10/29; these are 10/28 test proofs) has OpenVM parts where the chain uses SP1: the page
    // refuses it.
    wrongSystem(`refused at the pinned cutover (OpenVM from block ${Number(pinnedFrom)})`, { cutover: 'pinned' });
    if (!testBuild) {
      const [part] = answer.parts;
      const e = ov.check(Buffer.from(c.kind === 'leaf' ? ovKeys.leaf : ovKeys.l1, 'hex'), proofFile(part.proof), Buffer.from(part.publicValues.slice(2), 'hex'));
      expect(`${c.file}: the OpenVM module alone verifies the proof under its minted key`, e === undefined, e);
      continue;
    }
    // Under a test cutover the rule is the same: refused with the cutover just past the answer's
    // last block, verified with it at or below the first. Everything below runs at the first.
    wrongSystem(`refused under a test cutover at block ${last + 1}`, { cutover: last + 1 });
    verifies(`verifies under a test cutover at block ${first - 1}`, { cutover: first - 1 });
    cut = first;
    console.log(`      the rest under a test cutover at block ${first}, its first`);
  }
  const ok = run();
  expect(`${c.file}: the real answer verifies to its ${data.logs} logs`, ok.r && ok.r.ok && ok.r.report.logs === data.logs, ok.threw || (ok.r.ok ? `${ok.r.report.logs} logs` : ok.r.message));
  if (ok.r && ok.r.ok) {
    const rep = ok.r.report;
    const cutover = cut === 'pinned' ? pinnedFrom : `0x${cut.toString(16)}`;
    expect(`${c.file}: the report counts one OpenVM proof from the first block, under cutover ${cutover}`, rep.proofs.openvm === 1 && rep.proofs.sp1 === 0 && rep.openvmFrom === answer.fromBlock && rep.cutover === cutover, JSON.stringify({ proofs: rep.proofs, openvmFrom: rep.openvmFrom, cutover: rep.cutover }));
    expect(`${c.file}: the check names its system`, ok.checks.length === 1 && ok.checks[0].system === 'openvm', JSON.stringify(ok.checks.map((x) => x.system)));
  }
  // Verify times: the whole answer through both modules, and the OpenVM proof check alone.
  const [part] = answer.parts;
  const proof = proofFile(part.proof);
  const pv = Buffer.from(part.publicValues.slice(2), 'hex');
  const key = Buffer.from(c.kind === 'leaf' ? ovKeys.leaf : ovKeys.l1, 'hex');
  const alone = [];
  const whole = [];
  for (let i = 0; i < RUNS; i++) {
    t = performance.now();
    const e = ov.check(key, proof, pv);
    alone.push(performance.now() - t);
    if (e !== undefined) expect(`${c.file}: check alone, run ${i}`, false, e);
    t = performance.now();
    run();
    whole.push(performance.now() - t);
  }
  const fmt = (xs) => xs.map((x) => x.toFixed(1)).join(' / ');
  console.log(`      ${proof.length} B proof. OpenVM check alone, ${RUNS} runs: ${fmt(alone)} ms; whole answer: ${fmt(whole)} ms`);

  // Negative: every one must be a returned 'rejected', never a throw.
  neg('without the OpenVM module, an OpenVM proof fails', { openvm: null }, /no OpenVM module/);
  neg('an OpenVM check that throws fails the proof', { openvm: () => { throw new Error('boom'); } }, /OpenVM check: boom/);
  const raw = Buffer.from(zlib.zstdDecompressSync(proof.subarray(4)));
  const rezip = (b) => new Uint8Array(Buffer.concat([Buffer.from('OVM2'), zlib.zstdCompressSync(b)]));
  for (const at of [raw.length >> 3, raw.length >> 1, raw.length - 40]) {
    const bad = Buffer.from(raw);
    bad[at] ^= 1;
    neg(`proof byte ${at} of ${raw.length} changed`, { proofs: () => rezip(bad) });
  }
  let a = clone(answer);
  a.parts[0].publicValues = flipHex(a.parts[0].publicValues, 2 * 200);
  neg('a changed public value byte (the end block hash)', { ans: a });
  // An OpenVM proof dressed as SP1 goes to SP1's verifier, under SP1's keys, and fails there,
  // even with OpenVM's keys entered as SP1 keys under Advanced. With no cutover, so that a cutover
  // at the first block does not refuse it as the wrong system first.
  const asSp1 = () => new Uint8Array(Buffer.concat([Buffer.from([1, 0, 0, 0]), proof.subarray(4)]));
  const asSp1Cut = testBuild ? null : 'pinned';
  neg('an OpenVM proof with an SP1 tag', { proofs: asSp1, cutover: asSp1Cut });
  neg('an OpenVM proof with an SP1 tag, OpenVM keys entered as SP1 keys', { proofs: asSp1, seg: ovKeys.l1, leaf: ovKeys.leaf, cutover: asSp1Cut });
  neg('bytes of neither system', { proofs: () => new Uint8Array(Buffer.concat([Buffer.from('OVM3'), proof.subarray(4)])) }, /neither an SP1 nor an OpenVM proof/);
  // The other kind's real proof in its place: another program, so another key.
  const otherFile = c.kind === 'leaf' ? join(here, 'logs/eth-26069632/store/l1/26069632-26069695.proof') : join(here, 'logs/eth-26069632/store/leaf/26069632-9c515fec4200c98aa049fd273312827d683afa5af0da67a785e336a3acdd2063.proof');
  neg(`the real ${c.kind === 'leaf' ? 'L1' : 'leaf'} proof swapped in`, { proofs: () => new Uint8Array(readFileSync(otherFile)) });
}

// The OpenVM module on its own: a key that is not minted, a short key, SP1 bytes.
const leafKey = Buffer.from(ovKeys.leaf, 'hex');
const someProof = new Uint8Array(readFileSync(join(here, 'logs/eth-26069632/store/leaf/26069632-9c515fec4200c98aa049fd273312827d683afa5af0da67a785e336a3acdd2063.proof')));
expect('a key that is not minted is refused', /^no OpenVM program/.test(ov.check(new Uint8Array(32).fill(7), someProof, new Uint8Array(0)) || ''));
expect('a key that is not 32 bytes is refused', ov.check(leafKey.subarray(0, 31), someProof, new Uint8Array(0)) === 'a program key is 32 bytes');
expect('SP1 bytes are refused', ov.check(leafKey, new Uint8Array([1, 0, 0, 0, 9]), new Uint8Array(0)) === 'not an OpenVM proof');

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
