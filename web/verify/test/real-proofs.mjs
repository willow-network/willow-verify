// Runs the built wasm verifier (web/verify-dist) under Node on real SP1 proofs.
//   node web/verify/test/real-proofs.mjs <dist dir> <data dir> <leaf key hex>
// <data dir> holds answer.json (a willow_getLogsProof result of one-block parts) and store/ (their
// SP1 proofs), as test/save_live_data.py saves them; <leaf key> is the key of the program that
// proved them. Positive and negative cases; exits 1 on any surprise.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [dist, data, leafKey] = process.argv.slice(2);
if (!dist || !data || !leafKey) {
  console.error('usage: node real-proofs.mjs <dist dir> <data dir> <leaf key hex>');
  process.exit(2);
}
const files = readdirSync(dist);
const glue = files.find((f) => /^verifier\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^verifier\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, Verifier } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });

const answer = JSON.parse(readFileSync(join(data, 'answer.json'), 'utf8'));
// Every log of the saved blocks: the answer's one-block parts carry each block's whole record
// stream (tx 4 | log 4 | emitter 20 | n 1 | topics 32n | len 4 | data), and the filter has none.
const LOGS = answer.parts.reduce((k, p) => {
  const r = Buffer.from(p.records.slice(2), 'hex');
  for (let i = 0; i < r.length; k++) {
    const q = i + 29 + 32 * r[i + 28];
    i = q + 4 + r.readUInt32BE(q);
  }
  return k;
}, 0);
const filter = { fromBlock: answer.fromBlock, toBlock: answer.toBlock };
const proofFile = (id) => new Uint8Array(readFileSync(join(data, 'store', `${id}.proof`)));

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}

/// A JSON-RPC response body carrying `result` (JSON text, or a value to serialise), as the page
/// hands it over: verify and compare read the body as the server sent it.
const body = (result) => `{"jsonrpc":"2.0","id":1,"result":${typeof result === 'string' ? result : JSON.stringify(result)}}`;

// The verifier never throws a verdict: every method returns JSON with `ok`, and `kind` and
// `message` when it is false. A throw here would be the runtime failing, and counts as a failure.
// `ans` is the result (a value, or JSON text); `response` a whole response body instead.
function check({ ans = answer, response = body(ans), flt = filter, leaf = leafKey, segment = '', proofs = proofFile } = {}) {
  const v = new Verifier(1, segment, leaf);
  const times = [];
  let t0 = 0;
  const get = (id) => {
    t0 = performance.now();
    return proofs(id);
  };
  const done = (id, err) => times.push({ id, ms: performance.now() - t0, err });
  let r;
  try {
    r = JSON.parse(v.verify(response, JSON.stringify(flt), get, done));
  } catch (e) {
    return { threw: String(e.message || e), error: `threw: ${e.message || e}`, times };
  }
  if (!r.ok) return { error: r.message, kind: r.kind, times };
  return { v, times, report: r.report, logs: r.logs };
}
const compare = (v, plain) => JSON.parse(v.compare(body(plain)));

const clone = (x) => JSON.parse(JSON.stringify(x));
function flipHex(s, at) {
  const i = s.startsWith('0x') ? 2 + at : at;
  const c = s[i] === '0' ? '1' : '0';
  return s.slice(0, i) + c + s.slice(i + 1);
}

// Positive: the real answer verifies to its logs, and every proof is timed.
const ok = check();
expect('real answer verifies', !ok.error && ok.report.logs === LOGS, ok.error || `${ok.report.logs} logs`);
for (const t of ok.times) console.log(`      SP1 check ${t.id}: ${t.ms.toFixed(1)} ms${t.err ? ` error ${t.err}` : ''}`);
const again = check();
for (const t of again.times) console.log(`      SP1 check again ${t.id}: ${t.ms.toFixed(1)} ms`);

if (ok.v) {
  const plain = ok.logs.map((l) => ({ ...l, blockHash: '0x' + '00'.repeat(32), transactionHash: '0x' + '11'.repeat(32), removed: false }));
  const same = compare(ok.v, plain).result;
  expect('matching eth_getLogs answer compares exact', same.exact && same.matched === LOGS, `${same.matched} matched`);
  const dropped = compare(ok.v, plain.slice(1)).result;
  expect('eth_getLogs answer missing a log is caught', !dropped.exact && dropped.missing.length === 1);
  const altered = clone(plain);
  altered[2].data = altered[2].data === '0x' ? '0x00' : flipHex(altered[2].data, 0);
  const alt = compare(ok.v, altered).result;
  expect('eth_getLogs answer with altered data is caught', !alt.exact && alt.missing.length === 1 && alt.unproven.length === 1);
  const extra = compare(ok.v, [...plain, { ...plain[0], logIndex: '0x63' }]).result;
  expect('eth_getLogs answer with an extra log is caught', !extra.exact && extra.unproven.length === 1);
  const notArray = compare(ok.v, { logs: plain });
  expect('an eth_getLogs answer that is not an array is malformed-input', !notArray.ok && notArray.kind === 'malformed-input', JSON.stringify(notArray));
}
const fresh = JSON.parse(new Verifier(1, '', leafKey).compare(body([])));
expect('compare before any verified answer is internal (a page fault)', !fresh.ok && fresh.kind === 'internal', JSON.stringify(fresh));

// "address": [] matches any address, as in eth_getLogs (and on Willow's server).
const anyAddr = check({ flt: { ...filter, address: [] } });
expect('"address": [] matches every log', !anyAddr.error && anyAddr.report.logs === ok.report.logs, anyAddr.error || `${anyAddr.report.logs} logs`);

// Negative: every tampering must fail, as a returned 'rejected' (not a throw).
const neg = (name, opts, want, kind = 'rejected') => {
  const r = check(opts);
  expect(name, !r.threw && r.kind === kind && (!want || want.test(r.error)), r.error ? `${r.kind}: ${r.error}` : `verified ${r.report.logs} logs`);
};

let a = clone(answer);
a.parts[1].records = flipHex(a.parts[1].records, a.parts[1].records.length - 3);
neg('altered record byte', { ans: a }, /records do not match/);

a = clone(answer);
a.parts[0].publicValues = flipHex(a.parts[0].publicValues, 384);
neg('forged public values (end block hash)', { ans: a }, /rejected/);

a = clone(answer);
const pvLen = a.parts[0].publicValues.length - 2;
a.parts[0].publicValues = flipHex(a.parts[0].publicValues, pvLen - 1);
neg('forged public values (log count)', { ans: a });

const published = check({ leaf: '' });
expect('the published keys verify the proofs', !published.error && published.report.logs === LOGS, published.error || `${published.report.logs} logs`);
neg('wrong program key (random)', { leaf: 'ab'.repeat(32) }, /rejected/);

neg(
  'altered proof byte',
  {
    proofs: (id) => {
      const b = proofFile(id);
      b[b.length >> 1] ^= 1;
      return b;
    },
  },
  /rejected/,
);
neg('proof of the other block swapped in', { proofs: (id) => proofFile(answer.parts[(answer.parts.findIndex((p) => p.proof === id) + 1) % 2].proof) }, /rejected/);

a = clone(answer);
a.parts = [a.parts[0]];
neg('missing block part', { ans: a }, /not the requested|no proof covers/);

a = clone(answer);
a.parts = [a.parts[1]];
a.fromBlock = filter.fromBlock;
neg('first block dropped', { ans: a }, /no proof covers/);

neg('answer for a different range', { flt: { ...filter, toBlock: '0x' + (parseInt(filter.toBlock, 16) + 1).toString(16) } }, /not the requested/);

a = clone(answer);
a.chainId = '0xa';
neg('wrong chain', { ans: a }, /chain/);

// The kind decides, never the text: a server-chosen proof id that says "out of memory" in a
// rejected answer is still 'rejected'.
a = clone(answer);
const OOM = ' out of memory: Array buffer allocation failed';
for (const p of a.parts) p.proof += OOM;
neg('a rejection whose text says "out of memory" is rejected', { ans: a, proofs: (id) => {
  const b = proofFile(id.slice(0, -OOM.length));
  b[b.length >> 1] ^= 1;
  return b;
} }, /out of memory/);

// Data that cannot be read is malformed-input; the page's own filter and keys have kinds of their own.
neg('an answer that is not JSON', { response: '<html>502 Bad Gateway</html>' }, /willow_getLogsProof answer/, 'malformed-input');
neg('a response without a result', { response: '{"jsonrpc":"2.0","id":1}' }, /missing field `result`/, 'malformed-input');
a = clone(answer);
delete a.parts;
neg('an answer without parts', { ans: a }, /parts/, 'malformed-input');
a = clone(answer);
a.parts[0].records = '0xzz';
neg('records that are not hex', { ans: a }, null, 'malformed-input');
neg('a filter with an unresolved block tag', { flt: { ...filter, toBlock: 'latest' } }, /resolved/, 'invalid-filter');
neg('program keys that do not parse', { leaf: 'zz' }, null, 'invalid-keys');
const badKeys = JSON.parse(new Verifier(1, '', 'zz').keys());
expect('keys() reports keys that do not parse', !badKeys.ok && badKeys.kind === 'invalid-keys', JSON.stringify(badKeys));
const goodKeys = JSON.parse(new Verifier(1, '', '').keys());
expect('keys() reports the published keys', goodKeys.ok && goodKeys.leaf[0] === goodKeys.pinned.leaf, JSON.stringify(goodKeys));

// Nesting 300,000 levels deep (past JSON.stringify in Firefox and Safari): skipped where the
// verifier reads nothing, malformed-input where it reads, and never a throw.
const DEEP = '['.repeat(300000) + ']'.repeat(300000);
const withX = (result) => body(`{"x":${DEEP},${JSON.stringify(result).slice(1)}`);
const deepOk = check({ response: withX(answer) });
expect('a deeply nested field the verifier does not read is skipped', !deepOk.error && deepOk.report.logs === LOGS, deepOk.error || `${deepOk.report.logs} logs`);
const deepId = check({ response: `{"jsonrpc":"2.0","id":${DEEP},"result":${JSON.stringify(answer)}}` });
expect('a deeply nested JSON-RPC id is skipped', !deepId.error && deepId.report.logs === LOGS, deepId.error);
a = clone(answer);
a.parts[1].records = flipHex(a.parts[1].records, a.parts[1].records.length - 3);
neg('an altered record with a deeply nested field is rejected', { response: withX(a) }, /records do not match/);
const inPart = JSON.stringify(answer).replace('"parts":[{', `"parts":[{"x":${DEEP},`);
neg('a deeply nested field inside a part is malformed-input', { response: body(inPart) }, /recursion limit/, 'malformed-input');
if (deepOk.v) {
  const plain = deepOk.logs.map((l) => ({ ...l, blockHash: '0x' + '00'.repeat(32), transactionHash: '0x' + '11'.repeat(32), removed: false }));
  const deepEntry = JSON.parse(deepOk.v.compare(body(`[${DEEP},${JSON.stringify(plain).slice(1)}`)));
  expect('an eth_getLogs answer with a deeply nested entry is malformed-input', !deepEntry.ok && deepEntry.kind === 'malformed-input' && /recursion limit/.test(deepEntry.message), JSON.stringify(deepEntry).slice(0, 200));
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
