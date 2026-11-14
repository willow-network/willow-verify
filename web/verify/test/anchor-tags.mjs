// Runs the built block confirmation (web/verify-dist/anchor.*.wasm) under Node on the newest and
// finalized blocks of all seven chains (test/anchor/tags/, `willow_getBlockAnchor ["latest"]` and
// `["finalized"]`), as @willow-network/rpc anchors its head: the light client caught up from the
// saved checkpoint, then each response verified at the time it was saved. Sequencer-signed blocks
// carry `anchorTime`. Exits 1 on any surprise.
//   node web/verify/test/anchor-tags.mjs <dist dir>
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const [dist] = process.argv.slice(2);
const files = readdirSync(dist);
const glue = files.find((f) => /^anchor\.[0-9a-f]+\.js$/.test(f));
const wasm = files.find((f) => /^anchor\.[0-9a-f]+\.wasm$/.test(f));
const { initSync, Anchor } = await import(pathToFileURL(join(dist, glue)).href);
initSync({ module: readFileSync(join(dist, wasm)) });
const here = dirname(fileURLToPath(import.meta.url));
const TAGS = join(here, 'anchor', 'tags');
const tag = (name) => JSON.parse(readFileSync(join(TAGS, `${name}.json`), 'utf8'));

let failures = 0;
function expect(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
}
const body = (result) => JSON.stringify({ jsonrpc: '2.0', id: 1, result });
const flip = (h) => h.slice(0, -1) + (h.at(-1) === '0' ? '1' : '0');
const checkpoint = tag('checkpoint');
const ZERO = `0x${'00'.repeat(32)}`;

/// A light client caught up for a response saved at `now`, as the client and page do it.
function caughtUp(chainId, response, now) {
  const a = new Anchor(checkpoint.root, checkpoint.slot);
  for (let round = 0; round < 4; round++) {
    const n = parseInt(JSON.parse(response).result.blockNumber, 16);
    const r = JSON.parse(a.verify(chainId, n, ZERO, response, now));
    if (r.ok || (r.kind !== 'need-bootstrap' && r.kind !== 'need-updates')) return { a, r, n };
    if (r.kind === 'need-bootstrap') JSON.parse(a.bootstrap(body(tag('bootstrap')), now));
    else JSON.parse(a.updates(body(tag('updates'))));
  }
  return { a, r: { ok: false, kind: 'rounds' }, n: 0 };
}

const cases = readdirSync(TAGS)
  .filter((f) => /^(latest|finalized)-\d+\.json$/.test(f))
  .sort();
expect('newest blocks of all seven chains, and Ethereum finalized', cases.length === 8, cases.join(' '));
for (const file of cases) {
  const c = JSON.parse(readFileSync(join(TAGS, file), 'utf8'));
  const response = typeof c.response === 'string' ? c.response : JSON.stringify(c.response);
  const t = performance.now();
  const { a, r, n } = caughtUp(c.chainId, response, c.savedAt);
  const ms = performance.now() - t;
  const e = c.expect;
  expect(
    `${file}: anchored by ${e.path}, ${e.level}`,
    r.ok && r.path === e.path && r.level === e.level && r.hash === e.hash && r.matches === false,
    r.ok ? `block ${n}, ${ms.toFixed(0)} ms` : `${r.kind}: ${r.message}`,
  );
  if (e.anchorTime !== undefined) expect(`${file}: anchorTime is the signed block's time`, r.ok && r.anchorTime === e.anchorTime, `${r.anchorTime} vs ${e.anchorTime}`);
  if (!r.ok) continue;
  // The proven hash: matches when passed in; another block's number is refused.
  const again = JSON.parse(a.verify(c.chainId, n, e.hash, response, c.savedAt));
  expect(`${file}: the proven hash matches`, again.ok && again.matches === true);
  const other = JSON.parse(a.verify(c.chainId, n + 1, e.hash, response, c.savedAt));
  expect(`${file}: the response for another block is refused`, !other.ok && other.kind === 'rejected', `${other.kind}: ${other.message}`);
  if (e.level === 'signed') {
    const later = JSON.parse(a.verify(c.chainId, n, e.hash, response, c.savedAt + (c.chainId === 1 ? 1200 : 4 * 3600)));
    expect(`${file}: replayed later, a signed head is stale`, !later.ok && later.kind === 'stale-head', `${later.kind}: ${later.message}`);
  }
}
{
  const c = tag('latest-8453');
  const response = JSON.parse(typeof c.response === 'string' ? c.response : JSON.stringify(c.response));
  const s = JSON.stringify(response);
  // One nibble of the sequencer's signature, wherever the response carries it.
  const m = /"(?:signature|sig)"\s*:\s*"(0x[0-9a-fA-F]+)"/.exec(s);
  if (m) {
    const forged = s.replace(m[1], flip(m[1]));
    const { r } = caughtUp(8453, forged, c.savedAt);
    expect("latest-8453: a sequencer signature changed is rejected", !r.ok && (r.kind === 'rejected' || r.kind === 'malformed-input'), `${r.kind}: ${r.message}`);
  } else console.log('      (no signature field found in latest-8453 to change)');
}

console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
