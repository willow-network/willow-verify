// End-to-end: the built page in a headless browser against mock_rpc.py.
//   [BROWSER=chromium|webkit|firefox] PLAYWRIGHT=<dir of the playwright package> node browser.mjs <dist> <data dir> <leaf key> [screenshot dir]
// ONLY=<section>,<section> runs just those sections.
// Starts the mock itself and stops it at the end; exits 1 on any failed case.
// <data dir>: two Ethereum blocks proven on their own (answer.json with one-block parts, store/
// with their SP1 proofs, tampered.json, forged-pv.json); <leaf key>: the key of the program that
// proved them. Every check passes that key plus another one under Advanced (so the keys in force
// are custom) unless a case says otherwise. Transaction and receipt cases use items/.
import { readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const [dist, data, leafKey, shots] = process.argv.slice(2);
const require = createRequire(import.meta.url);
const engine = require(process.env.PLAYWRIGHT || 'playwright')[process.env.BROWSER || 'chromium'];
// A port nothing answers on yet: a mock left over from another run must not answer for this one.
let PORT;
for (;;) {
  PORT = 8787 + Math.floor(Math.random() * 1000);
  try {
    await fetch(`http://127.0.0.1:${PORT}/`);
  } catch {
    break;
  }
}
const base = `http://127.0.0.1:${PORT}`;
const here = dirname(fileURLToPath(import.meta.url));

const mock = spawn('python3', [join(here, 'mock_rpc.py'), dist, data, String(PORT)], { stdio: 'inherit' });
let mockExited = false;
mock.on('exit', () => (mockExited = true));
for (let i = 0; ; i++) {
  try {
    await fetch(`${base}/verify/`);
    break;
  } catch {
    if (i > 50 || mockExited) throw new Error('mock did not start');
    await new Promise((r) => setTimeout(r, 100));
  }
}

let failures = 0;
const expect = (name, ok, detail) => {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
};

const browser = await engine.launch({ headless: true });
console.log(`browser: ${process.env.BROWSER || 'chromium'} ${browser.version()}`);
const consoleErrors = [];
async function openPage(opts = {}) {
  const ctx = await browser.newContext({ viewport: opts.viewport || { width: 1280, height: 900 }, colorScheme: opts.dark ? 'dark' : 'light' });
  for (const r of opts.routes || (opts.route ? [opts.route] : [])) await ctx.route(...r);
  const page = await ctx.newPage();
  // A fake clock (time still flows) that a case can jump ahead, or set to a time (`clockAt`, ms).
  // Near the light client's 30 days from the saved checkpoint, every page runs at the saved time.
  const at = opts.clockAt ?? (Date.now() > (CHECKPOINT_TIME + 25 * 86400) * 1000 ? ANCHOR_TIME * 1000 : null);
  if (opts.clock || at != null) await page.clock.install(at != null ? { time: at } : undefined);
  page.setDefaultTimeout(15000);
  page.on('console', (m) => {
    if (m.type() !== 'error') return;
    consoleErrors.push(m.text());
    if (process.env.DEBUG) console.log(`      console error: ${m.text().slice(0, 120)}`);
  });
  page.on('pageerror', (e) => consoleErrors.push(String(e)));
  // The JSON-RPC methods the page sends, from its first request.
  page.methods = methodsOf(page);
  await page.goto(`${base}/verify/${opts.query || ''}`);
  if (!opts.noWait && !opts.query) await page.waitForFunction(() => document.getElementById('pinned').textContent.length > 0);
  // The endpoint, the node and the program keys are folded under Advanced: the cases fill them in.
  if (!opts.query) await page.evaluate(() => (document.querySelector('details.advanced').open = true));
  return page;
}

// The page's default nodes per chain (two public nodes from different operators).
const PUBLIC_NODES = {
  1: ['https://ethereum-rpc.publicnode.com', 'https://rpc.mevblocker.io'],
  42161: ['https://arbitrum-one-rpc.publicnode.com', 'https://arb1.arbitrum.io/rpc'],
  130: ['https://unichain-rpc.publicnode.com', 'https://mainnet.unichain.org'],
  8453: ['https://base-rpc.publicnode.com', 'https://mainnet.base.org'],
};
const CORS = { 'access-control-allow-origin': '*', 'access-control-allow-headers': 'content-type', 'access-control-allow-methods': 'POST, OPTIONS' };
/// Routes for chain `chain`'s two public nodes, each answered by the mock's node in `modes[i]`
/// ('down': no answer), so no case reaches a real node.
// An L2's anchors served (the mock serves them only to cases that ask, through its anchorl2 path),
// and the page's clock a minute after the Ethereum block an L2 anchor reads.
const withL2Anchor = (chain, mode = 'anchorl2') => [...publicNodes(chain), [`**/v1/rpc/${chain}/public`, (route) => route.continue({ url: `${base}/v1/rpc/${chain}/${mode}` })]];
const l2Clock = (chain, block) => (Number(anchorFile(`l2-${chain}-${block}.json`).update.data.finalized_header.execution.timestamp) + 60) * 1000;
function publicNodes(chain, modes = ['good', 'good']) {
  return PUBLIC_NODES[chain].map((u, i) => [
    (url) => url.href.replace(/\/$/, '') === u,
    async (route) => {
      if (modes[i] === 'down') return route.abort();
      if (route.request().method() === 'OPTIONS') return route.fulfill({ status: 204, headers: CORS });
      const res = await fetch(`${base}/node/${chain === 1 ? '' : `${chain}/`}${modes[i]}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: route.request().postData() });
      return route.fulfill({ status: res.status, headers: { ...CORS, 'content-type': 'application/json' }, body: await res.text() });
    },
  ]);
}

const idle = () => !document.getElementById('result').hidden && !document.getElementById('run').disabled;
/// How far the page scrolls sideways, and the innermost elements past its right edge.
const overflowOf = (page) =>
  page.evaluate(() => {
    const w = document.documentElement.clientWidth;
    const past = [...document.querySelectorAll('body *')].filter((e) => e.getBoundingClientRect().right > w + 1 && ![...e.children].some((c) => c.getBoundingClientRect().right > w + 1));
    return [document.documentElement.scrollWidth - w, past.slice(0, 3).map((e) => `${e.tagName.toLowerCase()}.${e.className} "${e.textContent.slice(0, 40)}"`).join(', ')];
  });

async function result(page, t) {
  return {
    ms: Date.now() - t,
    tone: await page.getAttribute('#verdict', 'class'),
    title: await page.textContent('#verdictTitle'),
    text: await page.textContent('#verdictText'),
    actions: await page.$$eval('#verdictActions button', (b) => b.map((x) => x.textContent)),
    checks: await page.evaluate(() => [...document.querySelectorAll('#verdictChecks li')].map((li) => `${li.dataset.tone}: ${li.textContent}`)),
    facts: await page.evaluate(() => Object.fromEntries([...document.querySelectorAll('#facts dt')].map((dt) => [dt.textContent, dt.nextElementSibling.textContent]))),
    steps: await page.evaluate(() => [...document.querySelectorAll('#steps li')].map((li) => `${li.dataset.state}: ${li.querySelector('.what').textContent} - ${li.querySelector('.note').textContent}`)),
    logsShown: await page.evaluate(() => !document.getElementById('logsCard').hidden),
    logsTitle: await page.evaluate(() => (document.getElementById('logsTitle') || document.querySelector('#logsCard h2')).textContent),
    logsNote: await page.evaluate(() => document.getElementById('logsMore').textContent),
  };
}

async function fill(page, { rpc = 'good', node = 'good', filter, leaf = CUSTOM_KEYS, other = '', chain = 1, kind = 'logs', item } = {}) {
  if ((await page.inputValue('#chain')) !== String(chain)) await page.selectOption('#chain', String(chain));
  await page.check(`input[name=kind][value=${kind}]`);
  await page.fill('#endpoint', `${base}/v1/rpc/${chain}/${rpc}`);
  await page.fill('#node', node === '' ? '' : `${base}/node/${chain === 1 ? '' : `${chain}/`}${node}`);
  if (filter) await page.fill('#filter', JSON.stringify(filter, null, 2));
  if (item !== undefined) await page.fill('#item', item);
  await page.evaluate(() => (document.querySelector('details.advanced').open = true));
  if (kind === 'logs') await page.fill('#leafKeys', leaf);
  await page.fill('#compare', other);
}

async function check(page, opts = {}) {
  await fill(page, opts);
  const t = Date.now();
  await page.click('#run');
  await page.waitForFunction(idle, null, { timeout: 120000 });
  return result(page, t);
}

/// Runs a check to its verdict, or to a download ask (which then counts as its result).
async function checkOrAsked(page, opts) {
  await fill(page, opts);
  const t = Date.now();
  await page.click('#run');
  // A function, not a string: the page's CSP blocks eval.
  const done = () =>
    (!document.getElementById('result').hidden && !document.getElementById('run').disabled) ||
    [...document.querySelectorAll('#verdictActions button')].some((b) => b.textContent === 'Download and verify');
  await page.waitForFunction(done, null, { timeout: 120000 });
  return result(page, t);
}

/// Starts a check that stops to ask before a large download.
async function checkUntilAsked(page, opts) {
  await fill(page, opts);
  const t = Date.now();
  await page.click('#run');
  await page.waitForSelector('#verdictActions button:text-is("Download and verify")', { timeout: 120000 });
  return result(page, t);
}

async function clickAction(page, label) {
  const t = Date.now();
  await page.click(`#verdictActions button:text-is("${label}")`);
  await page.waitForFunction(idle, null, { timeout: 120000 });
  return result(page, t);
}

let page, r;
// A case that throws (a missing button, a timeout) counts as a failure, and the next one still runs.
async function section(name, fn) {
  if (process.env.ONLY && !process.env.ONLY.split(',').includes(name)) return;
  try {
    await fn();
  } catch (e) {
    expect(`${name}: threw`, false, String(e).split('\n')[0]);
    if (page) await page.context().close().catch(() => {});
    page = await openPage();
  }
}

const hx = (x) => '0x' + x.toString(16);
// The saved blocks (the mock's proof status: proven from SEG_END - 63 to TO, the newest final
// segment ending at SEG_END, the 64-block window before FROM's).
const SAVED = JSON.parse(readFileSync(join(data, 'answer.json'), 'utf8'));
const FROM = parseInt(SAVED.fromBlock, 16);
const TO = parseInt(SAVED.toBlock, 16);
const SEG_END = Math.floor(FROM / 64) * 64 - 1;
const RANGE = { fromBlock: hx(FROM), toBlock: hx(TO) };
const BLOCKS = `blocks ${FROM}–${TO}`;
/// The saved blocks' logs: { block, address, topic0 } from each part's record stream
/// (tx 4 | log 4 | emitter 20 | n 1 | topics 32n | len 4 | data).
const LOGS = SAVED.parts.flatMap((p) => {
  const block = Number(p.proof.split('/')[1].split('-')[0]);
  const r = Buffer.from(p.records.slice(2), 'hex');
  const out = [];
  for (let i = 0; i < r.length; ) {
    const k = r[i + 28];
    out.push({ block, address: '0x' + r.subarray(i + 8, i + 28).toString('hex'), topic0: k ? '0x' + r.subarray(i + 29, i + 61).toString('hex') : null });
    const q = i + 29 + 32 * k;
    i = q + 4 + r.readUInt32BE(q);
  }
  return out;
});
const logsIn = (lo, hi) => LOGS.filter((l) => l.block >= lo && l.block <= hi).length;
const logsText = (k) => `${k.toLocaleString('en-US')} log${k === 1 ? '' : 's'}`;
// The example: Uniswap v4 swaps from Ethereum's PoolManager.
const POOL_MANAGER = '0x000000000004444c5dc75cb358380d2e3de08a90';
const V4_SWAP = '0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f';
const swaps = LOGS.filter((l) => l.address === POOL_MANAGER && l.topic0 === V4_SWAP).length;
// Custom keys in force: the proofs' key, and one more.
const CUSTOM_KEYS = `${leafKey}\n${'22'.repeat(32)}`;
const WRONG_KEY = '33'.repeat(32);
// proofJobs in the proof status of a server that proves blocks on request, and of one that cannot now.
const RUNNER = { pod: 'warm', podSince: 1, paused: false, updatedAt: 1, ageSeconds: 5, stale: false };
const JOBS_UP = { available: true, runner: RUNNER, queued: 0, running: 0 };
const JOBS_DOWN = { available: false, runner: { ...RUNNER, paused: true }, queued: 0, running: 0 };
const NOTHING_PROVEN = { provenRanges: [], firstProvenBlock: null, newestProvenBlock: null, newestProvenNearTip: null, lastSegmentEnd: null };
/// A route that answers Willow's proof status with `chain` (fields of chains["1"]).
const health = (chain) => [
  '**/v1/rpc/health/proofs',
  (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ healthy: false, chains: { 1: { tip: TO + 1, healthy: false, ...chain } } }) }),
];
/// Records the JSON-RPC methods a page sends.
function methodsOf(page) {
  const methods = [];
  page.on('request', (q) => {
    try {
      const b = q.postDataJSON();
      if (b && b.method) methods.push(b.method);
    } catch {}
  });
  return methods;
}
const CUSTOM_OK = new RegExp(`^Verified with custom program keys: ${logsText(logsIn(FROM, TO))} in ${BLOCKS} are complete and correct`);
const FAILED = /^Not verified: Willow's answer failed verification/;
const short = (r) => `${r.title} | ${r.text.slice(0, 200)}`;
// Ethereum's light-client data the mock serves (test/anchor/): when its checkpoint was made, and when
// the anchors were saved (their finality update's signature).
const anchorFile = (f) => JSON.parse(readFileSync(join(here, 'anchor', f), 'utf8'));
const CHECKPOINT_TIME = 1606824023 + anchorFile('checkpoint.json').slot * 12;
const ANCHOR_TIME = 1606824023 + Number(anchorFile(`history-${FROM}.json`).update.data.signature_slot) * 12;

// 0. Checking before the verifier has finished loading still works.
await section('0', async () => {
  page = await openPage({ noWait: true });
  r = await check(page, { filter: RANGE, leaf: WRONG_KEY });
  expect('a check started while the verifier loads runs normally', r.tone.includes('bad') && /vk hash mismatch/.test(r.text), r.text.slice(0, 120));
  r = await check(page, { filter: RANGE });
  expect('the next check after a rejection uses a fresh verifier', CUSTOM_OK.test(r.title), r.title);
  await page.context().close();
});

// 1. The real answer, checked with a custom key: verified, flagged amber in the title.
await section('1', async () => {
  page = await openPage();
  r = await check(page, { filter: RANGE });
  expect('custom program keys show in the verdict title, in amber', r.tone.includes('warn') && CUSTOM_OK.test(r.title), short(r));
  expect('custom keys are explained in the verdict text', /entered under Advanced/.test(r.text), r.text);
  console.log(`      ${r.facts['SP1 proofs verified']} | whole check ${r.ms} ms in the browser | ${r.facts['Downloaded']}`);
  const rows = await page.$$eval('#logs tbody tr', (t) => t.length);
  expect('proven logs are listed', rows === Math.min(logsIn(FROM, TO), 100) && r.logsTitle === 'Proven logs', `${rows} rows, "${r.logsTitle}"`);
  if (shots) await page.screenshot({ path: join(shots, 'verified-light.png'), fullPage: true, caret: 'initial' });
});

// 2. Same page again: proofs that passed come from the session cache.
await section('2', async () => {
  r = await check(page, { filter: RANGE });
  expect('second check reuses downloaded proofs', CUSTOM_OK.test(r.title) && /2 reused/.test(r.facts['Downloaded']), r.facts['Downloaded']);
  console.log(`      second check: ${r.facts['SP1 proofs verified']} | whole check ${r.ms} ms`);
});

// 3. Tampering and lies.
await section('3', async () => {
  const bad = [
    ['altered record is rejected', { rpc: 'tampered' }, FAILED, /records do not match/],
    ['forged public values are rejected', { rpc: 'forged' }, FAILED, /rejected/],
    ['a key of another program rejects the proof', { leaf: WRONG_KEY }, FAILED, /vk hash mismatch/],
    ['eth_getLogs answer missing a log is caught', { rpc: 'lying' }, /differs from the proven logs/, /leaves out or changes 1/],
    ['last block hash not on the node is caught', { node: 'wrong' }, /is not on the chain/, /hash differs/],
  ];
  for (const [name, opts, title, text] of bad) {
    r = await check(page, { filter: RANGE, ...opts });
    expect(name, r.tone.includes('bad') && title.test(r.title) && text.test(r.text), short(r));
    if (r.logsShown) expect(`  ${name}: logs card does not call them proven`, !/proven/i.test(r.logsTitle + r.logsNote), `"${r.logsTitle}" "${r.logsNote}"`);
  }
  r = await check(page, { filter: RANGE, node: 'chain10' });
  expect('a node on another chain gives an incomplete check', r.tone.includes('warn') && /is not an Ethereum node: it answers for chain 10/.test(r.text) && !/^Verified/.test(r.title), short(r));
  expect('an incomplete check does not call the logs proven', !/proven/i.test(r.logsTitle + r.logsNote), `"${r.logsTitle}"`);
  r = await check(page, { filter: RANGE, rpc: 'noproofs' });
  expect('an endpoint without proofs says so', r.tone.includes('bad') && /does not serve proofs/.test(r.text), r.text);
});

// 3b. No matching logs, but the last block is not on the node: nothing is "proven".
await section('3b', async () => {
  r = await check(page, { filter: { ...RANGE, address: '0x' + '00'.repeat(20) }, node: 'wrong' });
  expect('an empty answer that failed verification is not called proven', r.tone.includes('bad') && r.logsShown && !/proven/i.test(r.logsTitle + r.logsNote), `"${r.logsTitle}" "${r.logsNote}"`);
  r = await check(page, { filter: { ...RANGE, address: '0x' + '00'.repeat(20) } });
  expect('an empty verified answer says so', /^Verified with custom program keys: 0 logs/.test(r.title) && /That, too, is proven/.test(r.logsNote), `${r.title} | ${r.logsNote}`);
});

// 4. A proof that fails is not kept: the next check downloads it again.
await section('4', async () => {
  await page.context().close();
  page = await openPage();
  r = await check(page, { filter: RANGE, rpc: 'badproof' });
  expect('a corrupted proof is rejected', FAILED.test(r.title) && /rejected/.test(r.text), short(r));
  r = await check(page, { filter: RANGE });
  expect('a retry after a rejected proof downloads it again', CUSTOM_OK.test(r.title) && !/reused/.test(r.facts['Downloaded']), `${r.title} | ${r.facts['Downloaded']}`);
});

// 5. "latest" past the proven blocks, then the offered retry ends at the newest proven block.
await section('5', async () => {
  r = await check(page, { rpc: 'ahead', filter: { fromBlock: 'latest', toBlock: 'latest' } });
  expect('an unproven latest block is reported', r.tone.includes('warn') && new RegExp(`block ${TO + 1} is not proven yet`).test(r.title), r.title);
  expect('the retry ends at the newest proven block', r.actions[0] === `Verify block ${TO} instead`, r.actions.join(', '));
  r = await clickAction(page, r.actions[0]);
  expect('retry on the last proven block verifies', new RegExp(`^Verified with custom program keys: \\d+ logs? in block ${TO}`).test(r.title), r.title);
  const resolved = JSON.parse(await page.inputValue('#filter'));
  expect('retry wrote concrete block numbers', resolved.fromBlock === hx(TO) && resolved.toBlock === hx(TO), JSON.stringify(resolved));

  r = await check(page, { filter: { fromBlock: hx(FROM), toBlock: hx(TO + 1) } });
  expect('a range past the proofs offers its proven start', r.tone.includes('warn') && r.actions[0] === `Verify blocks ${FROM}–${TO} instead`, `${r.title} | ${r.actions}`);
  r = await clickAction(page, r.actions[0]);
  expect('the proven start verifies', CUSTOM_OK.test(r.title), r.title);
});

// 6. Block tags: numbers and "latest" only, said plainly before anything runs.
await section('6', async () => {
  for (const tag of ['finalized', 'safe', 'earliest']) {
    await fill(page, { filter: { fromBlock: tag, toBlock: 'latest' } });
    await page.evaluate(() => (document.getElementById('result').hidden = true));
    await page.click('#run');
    const msg = await page.textContent('#formError');
    const ran = await page.evaluate(() => !document.getElementById('result').hidden);
    expect(`"${tag}" gets a clear message and no check`, !ran && msg.includes(`"${tag}" is not supported`) && /block numbers and "latest"/.test(msg), msg);
  }
});

// 7. Rate limiting is not "too large"; the server's too-large message is shown.
await section('7', async () => {
  r = await check(page, { filter: RANGE, rpc: 'ratelimited' });
  expect('HTTP 429 reads as a rate limit', /rate limit/i.test(r.title + r.text) && !/too large/i.test(r.title + r.text), short(r));
  r = await check(page, { filter: RANGE, rpc: 'toolarge' });
  expect('-32002 over HTTP 200 reads as too large, with the server message', /too large/i.test(r.title) && r.text.includes('proof would carry more than 10000 records') && !/rate limit/i.test(r.text), short(r));
});

// 8. Blocks not yet in a final segment: a plain explanation and a narrower retry.
await section('8', async () => {
  r = await check(page, { filter: RANGE, rpc: 'tail' });
  expect(
    'the tail limit is explained plainly',
    r.title === `Could not verify: block ${TO} is not in a final segment` && /every log it holds/.test(r.text) && /15 to 30 minutes/.test(r.text) && new RegExp(`newest ends at block ${SEG_END}`).test(r.text),
    short(r),
  );
  expect('the tail limit offers the blocks before it', r.actions[0] === `Verify block ${FROM} instead`, r.actions.join(', '));
  r = await clickAction(page, r.actions[0]);
  expect('the narrower range verifies', new RegExp(`^Verified with custom program keys: [\\d,]+ logs? in block ${FROM} `).test(r.title), r.title);
});

// 8b. No final segment yet (lastSegmentEnd null): no "15 to 30 minutes", no "few minutes".
await section('8b', async () => {
  await page.context().close();
  page = await openPage({ routes: [health({ newestProvenBlock: TO, lastSegmentEnd: null })] });
  r = await check(page, { filter: RANGE, rpc: 'tail' });
  expect('without a final segment the tail message gives no timing', /not in a final segment/.test(r.title) && !/15 to 30 minutes|few minutes/.test(r.text) && new RegExp(`Verify block ${FROM} now, or fewer blocks`).test(r.text), short(r));
  await page.context().close();
  page = await openPage();
});

// 8c. The newest final segment far behind the tip (segments not being made): no timing claim.
await section('8c', async () => {
  await page.context().close();
  page = await openPage({ routes: [health({ newestProvenBlock: TO, lastSegmentEnd: TO - 1000 })] });
  r = await check(page, { filter: RANGE, rpc: 'tail' });
  expect(
    'a final segment far behind the tip gives no timing',
    /not in a final segment/.test(r.title) && !/15 to 30 minutes|few minutes/.test(r.text) && new RegExp(`newest ends at block ${TO - 1000}, 1,001 blocks behind the chain`).test(r.text) && new RegExp(`Verify block ${FROM} now`).test(r.text),
    short(r),
  );
  await page.context().close();
  page = await openPage();
});

// 9. A large download asks first (fresh page: no proofs kept from earlier checks).
await section('9', async () => {
  await page.context().close();
  page = await openPage();
  r = await checkUntilAsked(page, { filter: RANGE, rpc: 'big' });
  expect('more than 20 proofs asks before downloading', /32 proofs/.test(r.title) && /about 80 MB/.test(r.title) && r.actions.join() === 'Download and verify,Cancel', `${short(r)} | ${r.actions}`);
  expect('nothing is downloaded while it asks', /pending|active/.test(r.steps.find((s) => s.includes('Proofs')) || ''), r.steps.join(' / '));
  await page.click('#verdictActions button:text-is("Cancel")');
  await page.waitForFunction(() => !document.getElementById('run').disabled);
  r = await result(page, Date.now());
  expect('cancel stops before the download', /^Could not verify/.test(r.title) && /fewer blocks/.test(r.text), short(r));
  r = await checkUntilAsked(page, { filter: RANGE, rpc: 'big' });
  await page.click('#verdictActions button:text-is("Download and verify")');
  await page.waitForFunction(idle, null, { timeout: 120000 });
  r = await result(page, Date.now());
  expect('continue downloads the proofs', /stored proof \S+ not found/.test(r.text), short(r));
});

// 10. The default and example ranges are the 10 newest blocks of the newest final segment (from
// /v1/rpc/health/proofs): one SP1 proof covers them.
await section('10', async () => {
  await page.context().close();
  page = await openPage();
  await page.waitForFunction(() => /^0x/.test(JSON.parse(document.getElementById('filter').value).fromBlock || ''));
  let f = JSON.parse(await page.inputValue('#filter'));
  expect('the default filter is the newest final segment\'s 10 newest blocks', f.toBlock === hx(SEG_END) && f.fromBlock === hx(SEG_END - 9) && f.topics[0] === V4_SWAP && f.address === POOL_MANAGER, JSON.stringify(f));
  await page.fill('#endpoint', `${base}/v1/rpc/1/good`);
  await page.fill('#filter', '{}');
  await page.click('#example');
  await page.waitForFunction(() => /^0x/.test(JSON.parse(document.getElementById('filter').value).fromBlock || ''));
  f = JSON.parse(await page.inputValue('#filter'));
  expect('the example is the same blocks', f.toBlock === hx(SEG_END) && f.fromBlock === hx(SEG_END - 9), JSON.stringify(f));
});

// 11. Remember only when asked.
await section('11', async () => {
  const stored = await page.evaluate(() => localStorage.getItem('willow-verify:endpoints'));
  expect('endpoints are not stored without the box ticked', stored === null);
  await page.check('#remember');
  const stored2 = await page.evaluate(() => localStorage.getItem('willow-verify:endpoints'));
  expect('endpoints are stored when the box is ticked', !!stored2 && stored2.includes('/v1/rpc/1/good'));
});

// 12. Wording on the page.
await section('12', async () => {
  const lede = (await page.textContent('header.dev-head')).replace(/\s+/g, ' ');
  const about = (await page.textContent('.about')).replace(/\s+/g, ' ');
  expect(
    'the intro claims only the answer and no independent backstop, and How this works says whose code it is',
    /doesn't take the server's word for anything\./.test(lede) &&
      !/backstop|independent|SHA256SUMS/i.test(lede) &&
      /code is served by Willow/.test(about) &&
      /Its source is on GitHub, with a script that rebuilds this page/.test(about) &&
      (await page.getAttribute('.about a[href^="https://github.com/"]', 'href')) === 'https://github.com/willow-network/willow-verify' &&
      /To run the same verification on your own machine, ask Willow for the command-line verifier\./.test(about) && /No node, Willow included\./.test(about),
    `${lede} | ${about.slice(-300)}`,
  );
  const shown = await page.evaluate(() => document.body.innerText);
  expect('no exclamation points on the page', !shown.includes('!'), shown.split('\n').filter((l) => l.includes('!')).join(' | '));
  const coverage = await page.textContent('#coverage');
  expect('coverage lists the compared and the not compared fields', /address, topics, data, block number, transaction index and log index/.test(coverage) && /blockHash, transactionHash, blockTimestamp/.test(coverage) && /removed/.test(coverage), coverage.replace(/\s+/g, ' '));
  const footer = await page.textContent('.about');
  expect('the footer says SP1 proofs', /SP1 proofs/.test(footer) && !/zero-knowledge/i.test(footer), footer.replace(/\s+/g, ' ').slice(0, 120));
  await page.context().close();
});

// 13. Browser failures are not verification failures.
await section('13', async () => {
  const breakWorker = (err) => [
    '**/worker.*.js',
    async (route) => {
      const res = await route.fetch();
      const body = `${await res.text()}\nVerifier.prototype.verify = function () { throw ${err}; };\n`;
      await route.fulfill({ response: res, body });
    },
  ];
  for (const [name, err] of [
    ['out of memory', "new RangeError('WebAssembly.Memory.grow(): Maximum memory size exceeded')"],
    ['a wasm trap', "new WebAssembly.RuntimeError('unreachable')"],
    // Thrown, so the runtime failed, whatever the text says.
    ['an exception with no memory words', "new Error('proof leaf/1-ab rejected')"],
  ]) {
    page = await openPage({ route: breakWorker(err) });
    r = await check(page, { filter: RANGE });
    expect(`${name} reads as a browser failure`, /^Your browser could not finish verifying/.test(r.title) && !/failed verification/.test(r.title + r.text) && r.tone.includes('warn'), short(r));
    await page.context().close();
  }
  page = await openPage({ route: ['**/verifier.*.wasm', (route) => route.fulfill({ status: 404, body: 'gone' })], noWait: true });
  r = await check(page, { filter: RANGE });
  expect('a verifier that cannot load reads as a browser failure', /^Your browser could not finish verifying/.test(r.title), short(r));
  await page.context().close();
});

// 13b. The verdict class comes from the verifier's kind, never from message text: a rejection
// whose proof id (chosen by the server) says "out of memory" is still a failed check, and
// malformed data from Willow is a failed check too.
await section('13b', async () => {
  page = await openPage();
  r = await check(page, { filter: RANGE, rpc: 'oomid' });
  expect('a rejection that mentions "out of memory" is a failed check', FAILED.test(r.title) && /out of memory/.test(r.text) && r.tone.includes('bad'), short(r));
  for (const [name, rpc, what] of [
    ['malformed proof bytes', 'badhex', /malformed bytes/],
    ['an answer without parts', 'noparts', /without parts/],
    ['records that are not hex', 'badrecords', /willow_getLogsProof answer/],
    ['an eth_getLogs answer that is not an array', 'logsnotarray', /not an array/],
  ]) {
    r = await check(page, { filter: RANGE, rpc });
    expect(`${name} is a failed check`, FAILED.test(r.title) && what.test(r.text) && r.tone.includes('bad'), short(r));
  }
  r = await check(page, { filter: RANGE, leaf: 'zz' });
  expect('program keys that do not parse say so, and are not a browser failure', /program keys under Advanced are not valid/.test(r.text) && !/browser|failed verification/i.test(r.title), short(r));
  r = await check(page, { filter: RANGE });
  expect('a check after those still verifies', CUSTOM_OK.test(r.title), r.title);
});

// 14. Cache headers: only content-hashed names are immutable (the mock follows the live server).
await section('14', async () => {
  const files = await (await fetch(`${base}/verify/SHA256SUMS.txt`)).text();
  const hashed = files.split('\n').map((l) => l.split(/\s+/)[1]).filter((x) => x && x !== 'index.html');
  const cc = async (p) => (await fetch(`${base}/verify/${p}`)).headers.get('cache-control');
  expect('hashed files are immutable', (await Promise.all(hashed.map(cc))).every((c) => /immutable/.test(c)), hashed.join(' '));
  expect('index.html and SHA256SUMS.txt revalidate', (await cc('')) === 'no-cache' && (await cc('SHA256SUMS.txt')) === 'no-cache');
  const missing = await fetch(`${base}/verify/app.000000000000.js`);
  expect('a missing hashed name is not cached', missing.status === 404 && !/immutable/.test(missing.headers.get('cache-control') || ''));
});

// 15. A dark-mode browser gets the same white page; phone width renders without overflow.
await section('15', async () => {
  page = await openPage({ dark: true });
  const bg = await page.evaluate(() => [getComputedStyle(document.body).backgroundColor, getComputedStyle(document.querySelector('.card')).backgroundColor]);
  expect('a dark-mode browser still gets the white page', bg.every((c) => c === 'rgb(255, 255, 255)'), bg.join(' | '));
  r = await check(page, { filter: RANGE });
  if (shots) await page.screenshot({ path: join(shots, 'verified-dark.png'), fullPage: true, caret: 'initial' });
  await page.context().close();
  page = await openPage({ viewport: { width: 390, height: 844 } });
  r = await check(page, { filter: RANGE });
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect('phone width has no horizontal page scroll', overflow <= 0, `overflow ${overflow}px`);
  if (shots) await page.screenshot({ path: join(shots, 'verified-phone.png'), fullPage: true, caret: 'initial' });
  r = await check(page, { filter: RANGE, rpc: 'tampered' });
  if (shots) await page.screenshot({ path: join(shots, 'rejected-phone.png'), fullPage: true, caret: 'initial' });
  r = await checkUntilAsked(page, { filter: RANGE, rpc: 'big' });
  if (shots) await page.screenshot({ path: join(shots, 'large-download-phone.png'), fullPage: true, caret: 'initial' });
  await page.click('#verdictActions button:text-is("Cancel")');
  await page.waitForFunction(() => !document.getElementById('run').disabled);
  await page.context().close();
});

// 17. Nothing proven near the tip (newestProvenBlock null): said plainly, no chase of older blocks.
await section('17', async () => {
  page = await openPage({ routes: [health({ newestProvenBlock: null, lastSegmentEnd: FROM - 1 })] });
  r = await check(page, { rpc: 'ahead', filter: { fromBlock: 'latest', toBlock: 'latest' } });
  expect(
    'no proven block near the tip: no retry, try again later',
    r.title === 'Could not verify: Willow is not serving proofs for recent blocks right now' && /Try again later/.test(r.text) && r.actions.length === 0,
    `${short(r)} | ${r.actions}`,
  );
  r = await check(page, { rpc: 'ahead', filter: { fromBlock: hx(FROM), toBlock: hx(TO + 1) } });
  expect('the proven start of a range is still offered', /not serving proofs for recent blocks/.test(r.title) && r.actions.join() === `Verify blocks ${FROM}–${TO} instead`, `${short(r)} | ${r.actions}`);
  await page.fill('#filter', '{}');
  await page.click('#example');
  await page.waitForFunction(() => /^0x/.test(JSON.parse(document.getElementById('filter').value).fromBlock || ''));
  const f = JSON.parse(await page.inputValue('#filter'));
  expect('the example still uses the newest final segment', f.fromBlock === hx(FROM - 10) && f.toBlock === hx(FROM - 1), JSON.stringify(f));
  await page.context().close();
});

// 18. More than 20 proven blocks outside a final segment: the answer itself carries every log of
// those blocks, so the page asks before willow_getLogsProof and offers the blocks in final segments.
await section('18', async () => {
  // 20 blocks in the newest final segment, then 42 proven on their own.
  const wide = { fromBlock: hx(SEG_END - 19), toBlock: hx(TO) };
  const proven = health({ provenRanges: [[SEG_END - 63, TO]], newestProvenBlock: TO, newestProvenNearTip: TO, lastSegmentEnd: SEG_END });
  page = await openPage({ routes: [proven] });
  const methods = methodsOf(page);
  r = await checkUntilAsked(page, { filter: wide, rpc: 'big20' });
  expect(
    'it asks first, with the estimate',
    /^Verifying this needs about \d+ MB to download$/.test(r.title) && /every log of each block/.test(r.text) && new RegExp(`${TO - SEG_END} blocks in this range are not in a final 64-block segment`).test(r.text) && new RegExp(`up to ${TO - SEG_END + 2} proofs`).test(r.text),
    short(r),
  );
  expect('it offers the blocks in final segments', r.actions.join() === `Download and verify,Verify blocks ${SEG_END - 19}–${SEG_END} instead,Cancel`, r.actions.join());
  expect('nothing is fetched from willow_getLogsProof while it asks', !methods.includes('willow_getLogsProof'), methods.join());
  if (shots) await page.screenshot({ path: join(shots, 'tail-ask.png'), fullPage: true, caret: 'initial' });
  await page.click('#verdictActions button:text-is("Cancel")');
  await page.waitForFunction(() => !document.getElementById('run').disabled);
  r = await result(page, Date.now());
  expect('cancel stops before the answer', /^Could not verify: download cancelled/.test(r.title) && !methods.includes('willow_getLogsProof'), short(r));

  await checkUntilAsked(page, { filter: wide, rpc: 'big20' });
  r = await clickAction(page, `Verify blocks ${SEG_END - 19}–${SEG_END} instead`);
  const f = JSON.parse(await page.inputValue('#filter'));
  expect('the narrower range ends at the final segments and runs without asking', f.fromBlock === hx(SEG_END - 19) && f.toBlock === hx(SEG_END) && /stored proof \S+ not found/.test(r.text), `${JSON.stringify(f)} | ${short(r)}`);

  methods.length = 0;
  await checkUntilAsked(page, { filter: wide, rpc: 'big20' });
  r = await clickAction(page, 'Download and verify');
  expect('continue fetches the answer once and asks no second time for proofs within the estimate', methods.filter((m) => m === 'willow_getLogsProof').length === 1 && /stored proof \S+ not found/.test(r.text), `${methods.join()} | ${short(r)}`);

  await checkUntilAsked(page, { filter: wide, rpc: 'big50' });
  await page.click('#verdictActions button:text-is("Download and verify")');
  await page.waitForFunction(() => /proofs, about/.test(document.getElementById('verdictTitle').textContent));
  r = await result(page, Date.now());
  expect('more proofs than the estimate asks again', /^Verifying this needs 52 proofs/.test(r.title), short(r));
  await page.click('#verdictActions button:text-is("Cancel")');
  await page.waitForFunction(() => !document.getElementById('run').disabled);

  r = await check(page, { filter: RANGE });
  expect('20 or fewer blocks outside the segments do not ask', CUSTOM_OK.test(r.title), short(r));
  await page.context().close();

  // A proven range that starts inside a 64-block window: no segment covers its first blocks either.
  page = await openPage({ routes: [health({ provenRanges: [[SEG_END - 40, TO]], newestProvenBlock: TO, lastSegmentEnd: SEG_END })] });
  r = await checkUntilAsked(page, { filter: { fromBlock: hx(SEG_END - 40), toBlock: hx(TO) }, rpc: 'big20' });
  expect('blocks of a window not all proven count as outside a segment', new RegExp(`${TO - SEG_END + 41} blocks in this range are not in a final 64-block segment`).test(r.text) && r.actions.join() === 'Download and verify,Cancel', `${short(r)} | ${r.actions}`);
  await page.context().close();
});

// 18b. The proof status shows a block with no proof: no download ask; Willow is asked, and its
// refusal is shown with the proven part offered.
await section('18b', async () => {
  const wide = { fromBlock: hx(SEG_END - 19), toBlock: hx(TO + 1) };
  page = await openPage({ routes: [health({ provenRanges: [[SEG_END - 63, TO]], newestProvenBlock: TO, newestProvenNearTip: TO, lastSegmentEnd: SEG_END })] });
  let methods = methodsOf(page);
  r = await checkOrAsked(page, { filter: wide, rpc: 'ahead' });
  expect(
    'past the newest proven block: no ask, the proven part offered',
    r.title === `Could not verify: block ${TO + 1} is not proven yet` && r.actions.join() === `Verify blocks ${SEG_END - 19}–${TO} instead` && methods.includes('willow_getLogsProof'),
    `${short(r)} | ${r.actions} | ${methods}`,
  );
  await page.context().close();
  // A status from before proof jobs, with nothing proven near the tip.
  page = await openPage({ routes: [health({ newestProvenBlock: null, lastSegmentEnd: SEG_END })] });
  methods = methodsOf(page);
  r = await checkOrAsked(page, { filter: wide, rpc: 'ahead' });
  expect(
    'nothing proven near the tip: no ask, try the rest later',
    r.title === 'Could not verify: Willow is not serving proofs for recent blocks right now' && r.actions.join() === `Verify blocks ${SEG_END - 19}–${TO} instead` && /try the rest again later/.test(r.text),
    `${short(r)} | ${r.actions} | ${methods}`,
  );
  await page.context().close();
});

// 19. Values nested 300,000 levels deep in what servers send (past JSON.stringify and toString in
// every engine) are Willow's answer failing a check, or a comparison not done: never the browser.
await section('19', async () => {
  page = await openPage();
  const notBrowser = (r) => !/browser/i.test(r.title);
  r = await check(page, { filter: RANGE, rpc: 'tampered+deepproof' });
  expect('a tampered answer with a deeply nested field is a failed check', FAILED.test(r.title) && /records do not match/.test(r.text) && r.tone.includes('bad'), short(r));
  r = await check(page, { filter: RANGE, rpc: 'good+deepproof' });
  expect('a deeply nested field the verifier does not read is skipped', CUSTOM_OK.test(r.title), short(r));
  r = await check(page, { filter: RANGE, rpc: 'lying+deeplogs' });
  expect("Willow's eth_getLogs answer with a deeply nested entry is not verified", /^Not verified: Willow's answer failed verification/.test(r.title) && /recursion limit/.test(r.text) && r.tone.includes('bad'), short(r));
  r = await check(page, { filter: RANGE, other: `${base}/v1/rpc/1/good+deeplogs` });
  expect(
    "another provider's deeply nested answer only goes uncompared",
    CUSTOM_OK.test(r.title) && /answer from 127\.0\.0\.1:\d+ could not be compared/.test(r.text) && /recursion limit/.test(r.text),
    short(r),
  );
  r = await check(page, { filter: RANGE, other: `${base}/v1/rpc/1/good+deeperror` });
  expect("another provider's deeply nested error only goes uncompared", CUSTOM_OK.test(r.title) && /could not be compared/.test(r.text), short(r));
  r = await check(page, { filter: RANGE, rpc: 'good+deeperror' });
  expect(
    "a deeply nested error for Willow's eth_getLogs leaves verification incomplete",
    notBrowser(r) && /but verification is not complete/.test(r.title) && /could not be compared: .*\. The proofs were verified/.test(r.text),
    short(r),
  );
  r = await check(page, { filter: { fromBlock: 'latest', toBlock: 'latest' }, rpc: 'good+deeptip' });
  expect('a deeply nested latest block is not a browser failure', notBrowser(r) && r.tone.includes('bad') && /is not a 0x number/.test(r.text), short(r));
  r = await check(page, { filter: RANGE, node: 'good+deepchain' });
  expect("a deeply nested chain id from the node leaves verification incomplete", notBrowser(r) && /but verification is not complete/.test(r.title) && /is not a 0x number/.test(r.text), short(r));
  r = await check(page, { filter: RANGE });
  expect('a check after those still verifies', CUSTOM_OK.test(r.title), short(r));
  await page.context().close();
});

// 20. The example comes from the proven ranges: blocks of the newest final segment, else of the
// newest proven range; with nothing proven, "latest" when Willow proves blocks on request.
await section('20', async () => {
  page = await openPage({ routes: [health({ provenRanges: [[FROM, TO]], firstProvenBlock: FROM, newestProvenBlock: TO, newestProvenNearTip: null, lastSegmentEnd: null, proofJobs: JOBS_UP })] });
  await page.waitForFunction(() => /^0x/.test(JSON.parse(document.getElementById('filter').value).fromBlock || ''));
  const f = JSON.parse(await page.inputValue('#filter'));
  expect('the default filter is the newest proven range', f.fromBlock === hx(FROM) && f.toBlock === hx(TO), JSON.stringify(f));
  const methods = methodsOf(page);
  r = await check(page, {});
  // The example: the saved blocks' v4 swaps (none is fine: then the example proves there are none).
  const none = new RegExp(`^Verified with custom program keys: ${logsText(swaps)} in blocks ${FROM}–${TO} are complete and correct`);
  expect('the example verifies straight away', none.test(r.title) && !('Proven on request' in r.facts) && !methods.includes('willow_getProofJob'), short(r));
  await page.context().close();
  const exampleWith = async (chain, endpoint = 'good') => {
    page = await openPage({ routes: [health(chain)] });
    await page.fill('#endpoint', `${base}/v1/rpc/1/${endpoint}`);
    await page.fill('#filter', '{}');
    await page.click('#example');
    await page.waitForFunction(() => 'fromBlock' in JSON.parse(document.getElementById('filter').value));
    const out = { f: JSON.parse(await page.inputValue('#filter')), error: await page.textContent('#formError') };
    await page.context().close();
    return out;
  };
  let x = await exampleWith({ provenRanges: [[1000, 2000], [2048, 2103]], firstProvenBlock: 1000, newestProvenBlock: 2103, lastSegmentEnd: 1983, proofJobs: JOBS_UP });
  expect('the example prefers blocks in the newest final segment', x.f.fromBlock === hx(1974) && x.f.toBlock === hx(1983), JSON.stringify(x.f));
  x = await exampleWith({ ...NOTHING_PROVEN, proofJobs: JOBS_UP });
  expect('nothing proven yet, proving on request: the example is "latest"', x.f.fromBlock === 'latest' && x.f.toBlock === 'latest' && !x.error, `${JSON.stringify(x.f)} | ${x.error}`);
  x = await exampleWith({ ...NOTHING_PROVEN, proofJobs: JOBS_UP }, 'public');
  expect('…but not on the keyless path, which proves nothing on request', x.error === 'Willow has no proven blocks to verify right now. Try again later.', x.error);
  x = await exampleWith({ ...NOTHING_PROVEN, proofJobs: JOBS_DOWN });
  expect('nothing proven and no proving on request: said plainly', x.error === 'Willow has no proven blocks to verify right now. Try again later.', x.error);
});

// 21. Blocks with no proof yet: Willow queues a job, the page waits calmly with its progress, and
// runs the check once the job is done.
await section('21', async () => {
  page = await openPage({ routes: [health({ ...NOTHING_PROVEN, proofJobs: JOBS_UP })] });
  const methods = methodsOf(page);
  const count = (m) => methods.filter((x) => x === m).length;
  await fill(page, { filter: RANGE, rpc: 'queue.a' });
  await page.click('#run');
  await page.waitForFunction(() => /^Willow is proving/.test(document.getElementById('verdictTitle').textContent));
  r = await result(page, Date.now());
  expect(
    'a queued proof shows a calm wait',
    r.tone.includes('wait') && r.title === `Willow is proving blocks ${FROM}–${TO} now` && /^Ready in about 4 minutes\. These blocks have no proof yet/.test(r.text) && r.actions.join() === 'Cancel',
    `${short(r)} | ${r.actions}`,
  );
  expect('the wait shows in the progress steps', new RegExp(`^active: Proof from Willow - Waiting for Willow to prove blocks ${FROM}–${TO}\\.`).test(r.steps[1]), r.steps[1]);
  if (shots) await page.screenshot({ path: join(shots, 'waiting.png'), fullPage: true, caret: 'initial' });
  await page.waitForFunction(() => document.getElementById('waitText').textContent === '1 of 2 blocks proven.');
  const bar = await page.evaluate(() => [document.getElementById('waitBar').value, document.getElementById('waitBar').max]);
  expect('the progress line and bar follow willow_getProofJob', bar[0] === 1 && bar[1] === 2, JSON.stringify(bar));
  await page.waitForFunction(idle, null, { timeout: 60000 });
  r = await result(page, Date.now());
  expect('once the job is done, the check runs and verifies', CUSTOM_OK.test(r.title) && new RegExp(`^blocks ${FROM}–${TO}, ready after \\d`).test(r.facts['Proven on request'] || ''), `${short(r)} | ${r.facts['Proven on request']}`);
  expect('it polled the job until done, then asked for the proof once more', count('willow_getProofJob') === 3 && count('willow_getLogsProof') === 2, methods.join());
  await page.context().close();
});

// 22. A failed job: its error, plainly; polling stops; nothing is verified.
await section('22', async () => {
  page = await openPage({ routes: [health({ provenRanges: [[FROM, FROM]], firstProvenBlock: FROM, newestProvenBlock: FROM, lastSegmentEnd: null, proofJobs: JOBS_UP })] });
  const methods = methodsOf(page);
  r = await check(page, { filter: RANGE, rpc: 'queuefail.a' });
  expect(
    'a failed job says so, with its error',
    r.title === `Could not verify: Willow could not prove blocks ${FROM}–${TO}` &&
      /stopped with this error: the prover stopped: out of GPU memory\. Nothing was verified\./.test(r.text) &&
      new RegExp(`Block ${FROM} is already proven`).test(r.text) &&
      r.actions.join() === `Verify block ${FROM} instead` &&
      r.tone.includes('warn') &&
      !r.logsShown,
    `${short(r)} | ${r.actions}`,
  );
  const polls = methods.filter((m) => m === 'willow_getProofJob').length;
  await page.waitForTimeout(4500);
  expect('polling stops when the job fails', methods.filter((m) => m === 'willow_getProofJob').length === polls, `${polls} polls`);
  await page.context().close();
});

// 23. On-demand proving unavailable: said plainly, with proven blocks offered.
await section('23', async () => {
  page = await openPage({ routes: [health({ provenRanges: [[FROM, TO]], firstProvenBlock: FROM, newestProvenBlock: TO, lastSegmentEnd: null, proofJobs: JOBS_DOWN })] });
  r = await check(page, { filter: { fromBlock: hx(FROM), toBlock: hx(TO + 3) }, rpc: 'unavailable' });
  expect(
    'unavailable proving is said plainly',
    r.title === 'Could not verify: Willow cannot prove new blocks right now' &&
      new RegExp(`cannot prove blocks on request right now\\. Blocks ${FROM}–${TO} are already proven`).test(r.text) &&
      r.actions.join() === `Verify blocks ${FROM}–${TO} instead` &&
      r.tone.includes('warn'),
    `${short(r)} | ${r.actions}`,
  );
  r = await clickAction(page, r.actions[0]);
  expect('the proven blocks offered verify', CUSTOM_OK.test(r.title), short(r));
  r = await check(page, { filter: { fromBlock: hx(TO + 2), toBlock: hx(TO + 3) }, rpc: 'unavailable' });
  expect('with nothing proven in the range, the example blocks are offered', r.actions.join() === `Verify blocks ${FROM}–${TO} instead`, `${short(r)} | ${r.actions}`);
  await page.context().close();
});

// 24. Cancel while waiting (at phone width): the wait ends at once, polling stops, and the next
// check runs normally.
await section('24', async () => {
  page = await openPage({ routes: [health({ ...NOTHING_PROVEN, proofJobs: JOBS_UP })], viewport: { width: 390, height: 844 }, dark: true });
  const methods = methodsOf(page);
  await fill(page, { filter: RANGE, rpc: 'queueslow.a' });
  await page.click('#run');
  await page.waitForFunction(() => /blocks proven/.test(document.getElementById('waitText').textContent));
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect('the wait has no horizontal page scroll at phone width', overflow <= 0, `overflow ${overflow}px`);
  if (shots) await page.screenshot({ path: join(shots, 'waiting-phone-dark.png'), fullPage: true, caret: 'initial' });
  await page.click('#verdictActions button:text-is("Cancel")');
  await page.waitForFunction(idle);
  r = await result(page, Date.now());
  expect('cancel ends the wait', r.title === 'Could not verify: you stopped waiting' && new RegExp(`Willow keeps proving blocks ${FROM}–${TO}`).test(r.text) && r.tone.includes('warn'), short(r));
  const polls = methods.filter((m) => m === 'willow_getProofJob').length;
  await page.waitForTimeout(4500);
  expect('polling stops after cancel', methods.filter((m) => m === 'willow_getProofJob').length === polls, `${polls} polls`);
  r = await check(page, { filter: RANGE });
  expect('a check after a cancelled wait runs normally', CUSTOM_OK.test(r.title), short(r));
  await page.context().close();
});

// 25. A job this key cannot read (another key queued it): no progress, but the page asks again
// and finishes once Willow answers.
await section('25', async () => {
  page = await openPage({ routes: [health({ ...NOTHING_PROVEN, proofJobs: JOBS_UP })] });
  const methods = methodsOf(page);
  await fill(page, { filter: RANGE, rpc: 'queueother.a' });
  await page.click('#run');
  await page.waitForFunction(() => /Another request queued this proof/.test(document.getElementById('waitText').textContent));
  await page.waitForFunction(idle, null, { timeout: 60000 });
  r = await result(page, Date.now());
  expect('an unreadable job still ends in a check', CUSTOM_OK.test(r.title) && methods.filter((m) => m === 'willow_getLogsProof').length === 3, `${short(r)} | ${methods}`);
  await page.context().close();
});

// 26. Refusals with proof jobs: limits, sizes, and blocks Willow cannot prove on request.
await section('26', async () => {
  page = await openPage({ routes: [health({ provenRanges: [[FROM, TO]], firstProvenBlock: FROM, newestProvenBlock: TO, lastSegmentEnd: null, proofJobs: JOBS_UP })] });
  r = await check(page, { filter: { fromBlock: hx(FROM), toBlock: hx(TO + 1) }, rpc: 'joblimit' });
  expect(
    "a key's daily limit reads as a limit, not a size",
    r.title === "Could not verify: this key's proving limit is reached" && new RegExp(`daily cap of 20000\\. Blocks ${FROM}–${TO} are already proven`).test(r.text) && r.actions.join() === `Verify blocks ${FROM}–${TO} instead`,
    `${short(r)} | ${r.actions}`,
  );
  r = await check(page, { filter: RANGE, rpc: 'queuelimit' });
  expect('too many blocks to queue: check fewer', r.title === 'Could not verify: too many blocks to prove at once' && /one request can queue at most 2048; narrow the range\. Verify fewer blocks\./.test(r.text), short(r));
  r = await check(page, { filter: RANGE, rpc: 'tailonce' });
  expect('blocks that would stay outside a segment are explained', r.title === 'Could not verify: too many blocks outside a final segment' && /Once proven, 300 blocks of this range would be outside a segment/.test(r.text), short(r));
  r = await check(page, { filter: RANGE, rpc: 'tail' });
  expect(
    'with proof jobs the tail limit gives no segment timing',
    r.title === `Could not verify: block ${TO} is not in a final segment` && !/15 to 30 minutes|few minutes/.test(r.text) && /blocks outside them one at a time/.test(r.text) && r.actions[0] === `Verify block ${FROM} instead`,
    short(r),
  );
  r = await check(page, { filter: { fromBlock: hx(TO + 2), toBlock: hx(TO + 3) } });
  expect('a block past the log store is said plainly', r.title === `Could not verify: Willow does not have block ${TO + 2} yet` && /Try again in a minute/.test(r.text) && !r.actions.length, short(r));
  r = await check(page, { filter: { fromBlock: hx(FROM - 5), toBlock: hx(TO) } });
  expect(
    'an old block Willow cannot prove on request offers the proven part',
    r.title === `Could not verify: Willow has no proof for block ${FROM - 5}` && /cannot prove it on request/.test(r.text) && r.actions.join() === `Verify blocks ${FROM}–${TO} instead`,
    `${short(r)} | ${r.actions}`,
  );
  await page.context().close();
});

// 27. Two hours without the proofs: the page stops waiting and says so (the fake clock jumps ahead).
await section('27', async () => {
  page = await openPage({ routes: [health({ ...NOTHING_PROVEN, proofJobs: JOBS_UP })], clock: true });
  await fill(page, { filter: RANGE, rpc: 'queueslow.b' });
  await page.click('#run');
  await page.waitForFunction(() => /blocks proven/.test(document.getElementById('waitText').textContent));
  await page.clock.fastForward('02:00:05');
  await page.waitForFunction(idle, null, { timeout: 30000 });
  r = await result(page, Date.now());
  expect('after two hours the page stops waiting', r.title === 'Could not verify: the proofs are not ready yet' && /after two hours/.test(r.text) && r.tone.includes('warn'), short(r));
  await page.context().close();
});

const ITEM = (name) => JSON.parse(readFileSync(join(here, 'items', `${name}.json`), 'utf8'));
/// Runs a transaction / receipt / block check to its verdict.
async function checkItem(page, opts) {
  await fill(page, opts);
  const t = Date.now();
  await page.click('#run');
  await page.waitForFunction(idle, null, { timeout: 60000 });
  const out = await result(page, t);
  out.uncovered = await page.textContent('#itemUncovered');
  out.itemShown = await page.evaluate(() => !document.getElementById('itemCard').hidden);
  out.itemTitle = await page.textContent('#itemTitle');
  out.rows = await page.$$eval('#itemTable tbody tr', (rows) => rows.map((tr) => [...tr.cells].map((c) => c.textContent)));
  out.diff = await page.evaluate(() => (document.getElementById('diff').hidden ? '' : document.getElementById('diffBody').textContent));
  return out;
}

// 28. The chain choice: the endpoint's /rpc/<id>/ path, the node per chain (a public node only as a
// placeholder), the example's address, and an endpoint for another chain choosing that chain.
await section('28', async () => {
  page = await openPage();
  await fill(page, { filter: RANGE });
  const ethNode = await page.inputValue('#node');
  await page.selectOption('#chain', '42161');
  expect('choosing a chain rewrites the endpoint path', (await page.inputValue('#endpoint')) === `${base}/v1/rpc/42161/good`, await page.inputValue('#endpoint'));
  const node = await page.evaluate(() => ({ value: document.getElementById('node').value, placeholder: document.getElementById('node').placeholder, label: document.getElementById('nodeLabel').textContent }));
  expect('the node field is per chain, empty to verify against Ethereum', node.value === '' && node.placeholder === 'Empty: verified against Ethereum' && node.label === 'Your Arbitrum One node (optional)', JSON.stringify(node));
  await page.fill('#node', `${base}/node/42161/good`);
  await page.selectOption('#chain', '1');
  expect('each chain keeps its own node', (await page.inputValue('#node')) === ethNode, await page.inputValue('#node'));
  await page.selectOption('#chain', '42161');
  expect('…both ways', (await page.inputValue('#node')) === `${base}/node/42161/good`, await page.inputValue('#node'));
  await page.fill('#filter', '{}');
  await page.click('#example');
  await page.waitForFunction(() => 'fromBlock' in JSON.parse(document.getElementById('filter').value));
  const f = JSON.parse(await page.inputValue('#filter'));
  expect("the example uses the chain's PoolManager", f.address === '0x360e68faccca8ca495c1b759fd9eee466db9fb32' && f.topics[0] === V4_SWAP, JSON.stringify(f));
  await page.fill('#endpoint', `${base}/v1/rpc/130/good`);
  await page.dispatchEvent('#endpoint', 'change');
  expect('an endpoint for another chain chooses it', (await page.inputValue('#chain')) === '130' && (await page.textContent('#nodeLabel')) === 'Your Unichain node (optional)', await page.inputValue('#chain'));
  await page.check('#remember');
  const stored = JSON.parse(await page.evaluate(() => localStorage.getItem('willow-verify:endpoints')));
  expect('remembered: the endpoint, the chain and each chain\'s node', stored.chain === 130 && stored.nodes['42161'] === `${base}/node/42161/good` && stored.nodes['1'] === ethNode && !('130' in stored.nodes), JSON.stringify(stored));
  await page.reload();
  await page.waitForFunction(() => document.getElementById('pinned').textContent.length > 0);
  const back = await page.evaluate(() => [document.getElementById('chain').value, document.getElementById('endpoint').value, document.getElementById('node').value]);
  expect('a later visit restores them', back[0] === '130' && back[1] === `${base}/v1/rpc/130/good` && back[2] === '', back.join(' | '));
  await page.selectOption('#chain', '42161');
  expect("…with each chain's node", (await page.inputValue('#node')) === `${base}/node/42161/good`, await page.inputValue('#node'));
  await page.evaluate(() => localStorage.clear());
  await page.context().close();
  page = await openPage();
  // An endpoint for one chain while another is chosen: said before anything runs.
  await page.selectOption('#chain', '8453');
  // Typed in: leaving the field chooses its chain.
  await page.fill('#endpoint', `${base}/v1/rpc/10/good`);
  await page.focus('#filter');
  expect('leaving an endpoint typed for another chain chooses that chain', (await page.inputValue('#chain')) === '10', await page.inputValue('#chain'));
  // Set without an input event (a password manager, say): refused before anything runs.
  await page.selectOption('#chain', '8453');
  await page.evaluate((u) => (document.getElementById('endpoint').value = u), `${base}/v1/rpc/10/good`);
  await page.click('#run');
  const msg = await page.textContent('#formError');
  expect('an endpoint for another chain than the chosen one is refused', /is for chain 10 \(OP Mainnet\), but Base is chosen/.test(msg), msg);
  await page.context().close();
});

// 29. No node entered on Ethereum: the browser confirms the last block against Ethereum's
// validators itself (the light client from the page's checkpoint, Willow's anchor for the block)
// and asks no public node. Kept committees spare the next visit its downloads. Forged or
// another chain's light-client data fails; a fork the page does not know, a checkpoint too old or
// no anchor leave the block unconfirmed, said plainly, with no fallback to a node.
await section('29', async () => {
  const LC = ['willow_lightClientBootstrap', 'willow_lightClientUpdates'];
  const asked = [];
  const nodes = PUBLIC_NODES[1].map((u) => [(url) => url.href.replace(/\/$/, '') === u, (route) => (asked.push(route.request().url()), route.abort())]);
  page = await openPage({ routes: nodes });
  r = await check(page, { filter: RANGE, node: '' });
  expect(
    "without a node, the browser confirms the last block against Ethereum's validators",
    CUSTOM_OK.test(r.title) && r.text.includes("Your browser verified against Ethereum's validators that the last block is part of Ethereum, and final.") && r.steps.some((x) => x.startsWith(`done: Last block confirmed on Ethereum - Final on Ethereum. Your browser verified the signatures of Ethereum's validators (`) && x.includes(`found block ${TO.toLocaleString('en-US')} with this hash in the history of block`)),
    short(r),
  );
  expect('the light client starts from the checkpoint: its bootstrap, then the periods after it', LC.every((m) => page.methods.includes(m)) && page.methods.includes('willow_getBlockAnchor'), page.methods.join(','));
  expect('no public node is asked on Ethereum', asked.length === 0, asked.join(','));
  page.methods.length = 0;
  r = await check(page, { filter: RANGE, node: '' });
  expect('a second check reuses the verified committees', CUSTOM_OK.test(r.title) && !LC.some((m) => page.methods.includes(m)) && page.methods.includes('willow_getBlockAnchor'), page.methods.join(','));
  await page.reload();
  await page.waitForFunction(() => document.getElementById('pinned').textContent.length > 0);
  await page.evaluate(() => (document.querySelector('details.advanced').open = true));
  page.methods.length = 0;
  r = await check(page, { filter: RANGE, node: '' });
  expect('the next visit starts from the committees it kept, with no download', CUSTOM_OK.test(r.title) && !LC.some((m) => page.methods.includes(m)), page.methods.join(','));
  await page.context().close();

  for (const [mode, name, want] of [
    ['anchorforged', 'a forged signature on the anchor: not verified', /validators' signature over slot \d+ does not verify/],
    ['anchorbadupdate', 'a period update with another committee: not verified', /period \d+'s committee is not under the signed state root/],
  ]) {
    page = await openPage({ routes: nodes });
    r = await check(page, { filter: RANGE, node: '', rpc: mode });
    expect(name, r.tone.includes('bad') && r.title === 'Not verified: the data to confirm the last block failed verification' && want.test(r.text) && r.steps.some((x) => x.startsWith('fail: Last block on Ethereum')), short(r));
    await page.context().close();
  }
  for (const [mode, name, want] of [
    ['anchorgloas', 'an upgrade newer than the page: not confirmed, said plainly', /Ethereum's latest upgrade is newer than this page/],
    ['anchorsepolia', "another chain's validators (Sepolia's, from before this page's start): not confirmed, never green", /cannot be followed from this page's starting point \(signed in period \d+, before this page's starting point\)/],
    ['anchornone', 'no anchor from Willow: not confirmed, no node asked instead', /willow_getBlockAnchor/],
  ]) {
    page = await openPage({ routes: nodes });
    r = await check(page, { filter: RANGE, node: '', rpc: mode });
    expect(name, r.tone.includes('warn') && /^Proofs valid with custom program keys, but verification is not complete/.test(r.title) && /The last block could not be confirmed: /.test(r.text) && want.test(r.text) && asked.length === 0, short(r));
    await page.context().close();
  }
  page = await openPage({ routes: nodes });
  r = await check(page, { filter: RANGE, node: '', rpc: 'anchorwait' });
  expect('a block newer than the light client: asked again, then confirmed', CUSTOM_OK.test(r.title) && page.methods.filter((m) => m === 'willow_getBlockAnchor').length === 2, short(r));
  await page.context().close();
  // 121 days after the checkpoint: too old to start from.
  page = await openPage({ routes: nodes, clockAt: (CHECKPOINT_TIME + 31 * 86400) * 1000 });
  r = await check(page, { filter: RANGE, node: '' });
  expect('a checkpoint more than 30 days old: not confirmed, the page needs an update', r.tone.includes('warn') && /more than 30 days old, too old to start from safely; the page needs an update/.test(r.text), short(r));
  await page.context().close();
});

// 30. A fast chain's refusals: leaves of many blocks, and a leaf the chain has not finished.
await section('30', async () => {
  page = await openPage({ routes: [health({ provenRanges: [[FROM, TO]], firstProvenBlock: FROM, newestProvenBlock: TO, lastSegmentEnd: null, proofJobs: JOBS_UP })] });
  r = await check(page, { filter: RANGE, rpc: 'leaftail' });
  expect(
    "a fast chain's leaf tail is explained in leaves",
    r.title === `Could not verify: block ${TO} is not in a final segment` && /final 2048-block segments, and 1,024-block leaves outside them one at a time/.test(r.text) && /A leaf outside a segment is proven with every log of its blocks/.test(r.text) && r.actions[0] === `Verify block ${FROM} instead`,
    short(r),
  );
  r = await check(page, { filter: RANGE, rpc: 'leafonce' });
  expect('leaves that would stay outside a segment are counted as leaves', r.title === 'Could not verify: too many blocks outside a final segment' && /Once proven, 3 leaves of this range would be outside a segment/.test(r.text), short(r));
  await fill(page, { filter: RANGE, rpc: 'waiting.a' });
  await page.click('#run');
  await page.waitForFunction(() => /^Waiting for Ethereum to make block/.test(document.getElementById('verdictTitle').textContent));
  r = await result(page, Date.now());
  expect('a leaf the chain has not finished: a calm wait', r.tone.includes('wait') && r.title === `Waiting for Ethereum to make block ${TO + 5}` && new RegExp(`Block ${TO} is in a leaf of blocks that ends at block ${TO + 5}`).test(r.text) && r.actions.join() === 'Cancel', short(r));
  await page.waitForFunction(idle, null, { timeout: 30000 });
  r = await result(page, Date.now());
  expect('then Willow is asked again and the check runs', CUSTOM_OK.test(r.title) && new RegExp(`^blocks ${TO}–${TO + 5}, ready after`).test(r.facts['Waited for the chain'] || ''), `${short(r)} | ${JSON.stringify(r.facts)}`);
  await page.context().close();
});

// 31. Transaction and receipt proofs, verified: Arbitrum One's receipt (Nitro's passed-through
// fields named as not covered), Unichain's block receipts, Ethereum's transaction.
await section('31', async () => {
  page = await openPage();
  const arb = ITEM('arb-receipt');
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target });
  expect(
    "an Arbitrum One receipt verifies, Nitro's fields named as not covered",
    r.tone.includes('ok') && new RegExp(`^Verified: the receipt of transaction ${arb.target.slice(0, 8)}…${arb.target.slice(-6)} is correct$`).test(r.title) && /2 fields are not covered by this proof/.test(r.text) && /gasUsedForL1 and timeboosted: the chain's node reports them/.test(r.uncovered),
    `${short(r)} | ${r.uncovered}`,
  );
  const fields = Object.fromEntries(r.rows);
  expect('the proven fields are listed, and the uncovered ones marked', r.itemTitle === 'Proven receipt' && fields.transactionHash === arb.target && fields.blockHash === arb.block.hash && fields.gasUsedForL1 === 'not covered by this proof' && /^\d+ logs?: \[/.test(fields.logs), JSON.stringify(fields).slice(0, 300));
  expect('every step passed', r.steps.every((x) => x.startsWith('done')), r.steps.join(' / '));
  if (shots) await page.screenshot({ path: join(shots, 'receipt-verified.png'), fullPage: true, caret: 'initial' });
  const uni = ITEM('uni-block');
  r = await checkItem(page, { chain: 130, kind: 'blockReceipts', item: uni.target });
  const receipts = uni.answers[0].result.length;
  expect(
    "a Unichain block's receipts verify, with its transactions",
    r.tone.includes('ok') && r.title === `Verified: the ${receipts} receipts of block ${parseInt(uni.block.number, 16)} are complete and correct` && r.rows.length === receipts && r.steps.some((x) => /^done: Willow's eth_getBlockByHash transactions - Matches/.test(x)),
    `${short(r)} | ${r.rows.length} rows`,
  );
  const eth = ITEM('eth-receipt');
  r = await checkItem(page, { kind: 'receipt', item: eth.target });
  expect("a receipt past a block's second transaction leaves its logIndex uncovered, and says why", r.tone.includes('ok') && /logIndex: it counts every earlier log in the block/.test(r.uncovered), `${short(r)} | ${r.uncovered}`);
  const pos = ITEM('eth-receipt-1');
  r = await checkItem(page, { kind: 'receipt', item: pos.target });
  expect('a receipt asked by <block>:<index> verifies', r.tone.includes('ok') && r.facts.Transaction.endsWith('index 1 in the block'), `${short(r)} | ${r.facts.Transaction}`);
  await page.context().close();
});

// 32. A tampered proof fails in the browser; a plain answer that differs, or is null, is caught;
// another provider's difference is only noted.
await section('32', async () => {
  page = await openPage();
  for (const [name, chain, kind, item] of [
    ['a tampered receipt proof', 42161, 'receipt', ITEM('arb-receipt').target],
    ["a tampered block's receipts", 130, 'blockReceipts', ITEM('uni-block').target],
  ]) {
    r = await checkItem(page, { chain, kind, item, rpc: 'tampered' });
    expect(`${name} fails`, r.tone.includes('bad') && FAILED.test(r.title) && /What failed/.test(r.text) && !r.itemShown, short(r));
  }
  const arb = ITEM('arb-receipt');
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, rpc: 'lying' });
  expect(
    "Willow's plain receipt that differs is caught, field by field",
    r.tone.includes('bad') && r.title === "Not verified: Willow's eth_getTransactionReceipt answer differs from the proven one" && /differs: status: proven "0x1", the answer "0x0"/.test(r.diff) && /Receipt in Willow's proof/.test(r.itemTitle),
    `${short(r)} | ${r.diff}`,
  );
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, rpc: 'absent' });
  expect("Willow's plain answer of null is caught", r.tone.includes('bad') && /Willow's plain answer is null/.test(r.text), short(r));
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, other: `${base}/v1/rpc/42161/lying` });
  expect("another provider's difference is noted, and the check still verifies", r.tone.includes('ok') && /differs from the proven one; see Differences/.test(r.text) && r.steps.some((x) => /^fail: eth_getTransactionReceipt answer from/.test(x)), short(r));
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, node: 'wrong' });
  expect('a block hash the node does not have is caught', r.tone.includes('bad') && /is not on the chain/.test(r.title), short(r));
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, node: 'chain10' });
  expect('a node on another chain leaves verification incomplete', r.tone.includes('warn') && /is not an Arbitrum One node: it answers for chain 10/.test(r.text), short(r));
  await page.context().close();
  page = await openPage({ routes: publicNodes(42161, ['down', 'down']) });
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: arb.target, node: '' });
  expect('no node entered and no public node reachable: valid, but not complete', r.tone.includes('warn') && /^Proof valid, but verification is not complete/.test(r.title) && /The block could not be confirmed: arbitrum-one-rpc\.publicnode\.com/.test(r.text), short(r));
  await page.context().close();
});

// 33. What Willow refuses, and what cannot be read, said plainly.
await section('33', async () => {
  page = await openPage();
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: '0x' + '11'.repeat(32) });
  expect('a transaction outside the full-block window', r.title === 'Could not verify: Willow does not hold this transaction' && /newest seven days/.test(r.text) && r.tone.includes('warn'), short(r));
  r = await checkItem(page, { chain: 42161, kind: 'blockReceipts', item: '1000' });
  expect('a block outside the full-block window', /^Could not verify: block 1000 is outside the blocks Willow holds in full/.test(r.title), short(r));
  const blk = parseInt(ITEM('arb-block').block.number, 16);
  r = await checkItem(page, { chain: 42161, kind: 'receipt', item: `${blk}:99` });
  expect('an index past the last transaction', r.title === `Could not verify: block ${blk} has no transaction 99` && /Enter an index below/.test(r.text), short(r));
  for (const [kind, item] of [['receipt', '12345'], ['receipt', 'latest:x'], ['blockReceipts', 'finalized'], ['receipt', '']]) {
    await fill(page, { chain: 42161, kind, item });
    await page.evaluate(() => (document.getElementById('result').hidden = true));
    await page.click('#run');
    await page.waitForFunction(() => document.getElementById('formError').textContent.length > 0 && !document.getElementById('run').disabled);
    const msg = await page.textContent('#formError');
    const ran = await page.evaluate(() => !document.getElementById('result').hidden);
    expect(`"${item}" for a ${kind}: a form message, no check`, !ran && msg.length > 10, msg);
  }
  await page.context().close();
});

// 34. The example: a transaction of a recent block, by hash (the block's second); the block itself
// for its receipts. Then the fields for each kind, at phone width in dark mode.
await section('34', async () => {
  page = await openPage({ viewport: { width: 390, height: 844 }, dark: true });
  const arb = ITEM('arb-block');
  await fill(page, { chain: 42161, kind: 'receipt', item: '' });
  await page.click('#itemExample');
  await page.waitForFunction(() => document.getElementById('item').value.length > 0);
  const second = arb.answers[1].result.transactions[1].hash;
  expect('the example is the second transaction of a block 10 behind the newest', (await page.inputValue('#item')) === second, await page.inputValue('#item'));
  await page.check('input[name=kind][value=blockReceipts]');
  await page.click('#itemExample');
  await page.waitForFunction(() => /^\d+$/.test(document.getElementById('item').value));
  expect("the block example is that block", (await page.inputValue('#item')) === String(parseInt(arb.block.number, 16)), await page.inputValue('#item'));
  r = await checkItem(page, { chain: 42161, kind: 'blockReceipts', item: await page.inputValue('#item') });
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect('the block receipts verify at phone width with no horizontal page scroll', r.tone.includes('ok') && overflow <= 0, `${short(r)} | overflow ${overflow}px`);
  if (shots) await page.screenshot({ path: join(shots, 'block-receipts-phone-dark.png'), fullPage: true, caret: 'initial' });
  await page.check('input[name=kind][value=receipt]');
  expect('switching back keeps what was typed for a transaction', (await page.inputValue('#item')) === second, await page.inputValue('#item'));
  const hidden = await page.evaluate(() => [document.getElementById('logsField').hidden, document.getElementById('keysFields').hidden]);
  expect('the filter and the program keys are for log proofs only', hidden.every(Boolean), JSON.stringify(hidden));
  await page.check('input[name=kind][value=logs]');
  r = await check(page, { filter: RANGE });
  expect('a log check after item checks still verifies', CUSTOM_OK.test(r.title), short(r));
  await page.context().close();
});

// 35. A link to a transaction: no form, the keyless path and the chain's public nodes, and the
// transaction shown as a receipt from the proven bytes alone (token names from the nodes).
const linkSeal = async (page) => {
  await page.waitForFunction(() => {
    const s = document.querySelector('.seal');
    return (s && !s.classList.contains('checking')) || !document.getElementById('linkIntro').hidden;
  }, null, { timeout: 60000 });
  return page.evaluate(() => {
    const s = document.querySelector('.seal');
    const rows = Object.fromEntries([...document.querySelectorAll('.r-rows dt')].map((dt) => [dt.textContent, dt.nextElementSibling.textContent]));
    return {
      tone: s ? s.className : '',
      title: s ? s.querySelector('.seal-title').textContent : '',
      text: s ? s.querySelector('.seal-text').textContent : '',
      date: s && s.querySelector('.seal-date') ? s.querySelector('.seal-date').textContent : '',
      notes: [...document.querySelectorAll('.how li .note')].map((n) => n.textContent),
      lines: [...document.querySelectorAll('.r-line')].map((l) => l.textContent),
      checks: [...document.querySelectorAll('.r-checks li')].map((li) => `${li.dataset.tone}: ${li.textContent}`),
      rows,
      moves: document.querySelectorAll('.move-list li').length,
      steps: [...document.querySelectorAll('.how li')].map((li) => `${li.dataset.state}: ${li.querySelector('.what').textContent} ${li.querySelector('.note').textContent}`),
      actions: [...document.querySelectorAll('.r-actions > *')].map((b) => b.textContent),
      explorer: document.querySelector('.r-actions a') ? document.querySelector('.r-actions a').href : null,
      extra: [...document.querySelectorAll('.r-extra button')].map((b) => b.textContent),
      groups: [...document.querySelectorAll('.r-group')].filter((g) => !g.hidden).map((g) => g.querySelector('h2').textContent),
      served: document.querySelectorAll('.r-served').length,
      footnotes: document.querySelectorAll('.r-foot').length,
      formShown: getComputedStyle(document.getElementById('form')).display !== 'none',
      intro: document.getElementById('linkIntro').hidden ? '' : document.getElementById('linkIntro').textContent,
    };
  });
};
const linkShort = (x) => `${x.tone} | ${x.title} | ${x.text} | ${x.lines.join(' / ')} | ${x.checks.join(' / ')} | ${x.steps.join(' / ')}`.slice(0, 700);
await section('35', async () => {
  const arb = ITEM('arb-receipt');
  const receipt = arb.answers[0].result;
  const transfers = receipt.logs.filter((l) => l.topics[0] === '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef').length;
  const block = parseInt(arb.block.number, 16).toLocaleString('en-US');
  page = await openPage({ query: `?chain=arbitrum&tx=${arb.target}`, routes: withL2Anchor(42161), clockAt: l2Clock(42161, parseInt(arb.block.number, 16)) });
  const methods = page.methods;
  let x = await linkSeal(page);
  expect('a transaction link verifies on its own, with no form, its seal a title alone', x.tone === 'seal ok' && x.title === 'Transaction verified' && x.text === '' && !x.formShown, linkShort(x));
  expect('…a checklist under the summary: the block it is proven in, settled on Ethereum, and no trust in Willow', x.checks.join(' / ') === `ok: This transaction is in Arbitrum One block ${block} / ok: Settled on Ethereum, can't be reversed / ok: Verified by your browser, no trust in Willow needed`, x.checks.join(' / '));
  const howHere = await page.evaluate(() => { const d = document.querySelector('details.about'); return [d.checkVisibility(), d.open, d.querySelector('summary').textContent.trim()]; });
  expect('…and How this works under the card, folded', howHere[0] && !howHere[1] && howHere[2] === 'How this works', howHere.join(' | '));
  expect('…the date, time and chain under the title, not in the summary, and the time in UTC under More', /^[A-Z][a-z]{2} \d{1,2}, \d{4}, \d{1,2}:\d{2}.* · Arbitrum One$/.test(x.date) && !x.lines.some((l) => l.startsWith('Date')) && / UTC$/.test(x.rows['Time in UTC'] || ''), `${x.date} | ${x.lines.join(' / ')} | ${x.rows['Time in UTC']}`);
  expect('…shown as a receipt from the proven bytes, each fact once', ('Status' in x.rows) === (receipt.status !== '0x1') && !('Block' in x.rows) && !('Time' in x.rows) && !('Settlement' in x.rows) && /^0x[0-9a-f]{4}…[0-9a-f]{4}Copy$/.test(x.rows.From) && /ETH$/.test(x.rows['Network fee'] || '') && /^0x[0-9a-f]{6}…[0-9a-f]{6}Copy$/.test(x.rows['Transaction hash']), JSON.stringify(x.rows).slice(0, 400));
  expect('…every token transfer listed (open, with no settlement above), by its symbol alone, one line a row', (await page.evaluate(() => document.querySelector('details.moves') === null && (document.querySelector('.r-moves') === null || !document.querySelector('.r-moves').hidden))) && x.moves >= transfers && (transfers === 0 || (/\bTKN\b|USDC|USD₮0|WETH|ARB|WBTC/.test(await page.textContent('.move-list')) && !/\(0x/.test(await page.textContent('.move-list')))), `${x.moves} rows, ${transfers} transfers: ${await page.textContent('.move-list').catch(() => '')}`);
  expect('…three steps done, one sentence each, nothing in small type', x.steps.length === 3 && x.steps.every((t) => t.startsWith('done')) && x.notes.every((t) => t === '') && x.steps[0] === 'done: Willow sent the transaction and receipt with proofs. ' && x.steps[1].startsWith(`done: Your browser verified both are in Arbitrum One block ${block}.`) && x.steps[2] === 'done: Your browser verified the block against Ethereum. ' && !x.steps.some((t) => /Settled on Ethereum/.test(t)), x.steps.join(' / '));
  expect('…asking Willow only for the proof, not for its plain answer', methods.includes('willow_getReceiptProof') && !methods.includes('eth_getTransactionReceipt'), methods.join(','));
  expect('…with its link, the explorer and More; the proof and the full checker under More', x.actions.join() === 'Copy link,Arbiscan,More' && x.explorer === `https://arbiscan.io/tx/${receipt.transactionHash}` && x.extra.join() === 'Download the proof,Technical details', `${x.actions.join()} | ${x.explorer} | ${x.extra.join()}`);
  expect('…More in its groups, with no line on the code and no footnotes', x.groups.join() === (x.moves ? 'Transaction,Token movements,How it was verified,Proof' : 'Transaction,How it was verified,Proof') && x.served === 0 && x.footnotes === 0, `${x.groups.join()} | ${x.served}`);
  const placed = await page.evaluate(() => [!!document.querySelector('.r-sum + .r-checks + .r-actions'), document.querySelector('.r-actions button').className]);
  expect('…the checklist under the summary, the buttons under it, the link first', placed[0] && placed[1] === 'primary', placed.join(' | '));
  const shown = () => page.evaluate(() => [document.querySelector('.r-more'), ...document.querySelectorAll('.how .note')].map((n) => n.checkVisibility()));
  const folded = await shown();
  await page.click('.r-toggle');
  const unfolded = await shown();
  expect('…everything else, and how it was verified, folded under More', folded.every((v) => !v) && unfolded.every(Boolean) && (await page.textContent('.r-toggle')) === 'Less', `${folded} → ${unfolded}`);
  await page.click('.r-extra button:text-is("Technical details")');
  const form = await page.evaluate(() => [getComputedStyle(document.getElementById('form')).display !== 'none', document.getElementById('chain').value, document.querySelector('input[name=kind]:checked').value, document.getElementById('item').value, document.getElementById('endpoint').value, location.search]);
  expect('the full checker opens filled in with the same check', form[0] && form[1] === '42161' && form[2] === 'receipt' && form[3] === arb.target.toLowerCase() && form[4].endsWith('/v1/rpc/42161/public') && form[5] === '', form.join(' | '));
  await page.waitForFunction(() => /^Verified/.test(document.getElementById('verdictTitle').textContent), null, { timeout: 30000 }).catch(() => {});
  const ran = await page.evaluate(() => [document.getElementById('result').hidden, document.getElementById('verdictTitle').textContent]);
  expect('…and runs it at once', !ran[0] && /^Verified: the receipt of transaction/.test(ran[1]), ran.join(' | '));
  const top = await page.evaluate(() => Math.round(window.scrollY));
  expect('…opened at the top of the page', top === 0, String(top));
  await page.goBack();
  await page.waitForSelector('.seal.ok, .seal.bad, .seal.warn', { timeout: 30000 });
  const back = await page.evaluate(() => [location.search, document.body.classList.contains('link-mode'), document.querySelector('.seal-title').textContent]);
  expect("…and the browser's Back returns to the link's card", back[0].includes(`tx=${arb.target}`) && back[1] && back[2] === 'Transaction verified', back.join(' | '));
  await page.context().close();

  page = await openPage({ query: `?chain=42161&tx=${arb.target}`, routes: publicNodes(42161, ['good', 'wrong']), viewport: { width: 390, height: 844 }, dark: true });
  x = await linkSeal(page);
  expect('a public node with another block there: not verified', x.tone === 'seal bad' && x.title === 'Not verified' && /arb1\.arbitrum\.io has a different block/.test(x.text), linkShort(x));
  const [overflow, past] = await overflowOf(page);
  expect('…at phone width with no horizontal page scroll', overflow <= 0, `${overflow}px: ${past}`);
  await page.context().close();

  page = await openPage({ query: `?chain=arbitrum&tx=${arb.target}`, routes: publicNodes(42161, ['down', 'down']) });
  x = await linkSeal(page);
  expect('no public node reachable: the proof is valid, the block not confirmed', x.tone === 'seal warn' && x.title === 'Proof valid, block not confirmed', linkShort(x));
  await page.context().close();

  page = await openPage({ query: `?chain=arbitrum&tx=${arb.target}`, routes: [...publicNodes(42161), ['**/v1/rpc/42161/public', (route) => route.continue({ url: `${base}/v1/rpc/42161/tampered` })]] });
  x = await linkSeal(page);
  expect('a tampered proof fails in the link view, and nothing is shown as verified', x.tone === 'seal bad' && x.title === 'Not verified' && /proof failed verification/.test(x.text) && x.lines.length === 0 && !x.checks.some((c) => /no trust in Willow/.test(c)) && x.steps[1].startsWith('fail'), linkShort(x));
  const why = await page.evaluate(() => [document.querySelector('.r-card').classList.contains('open'), document.querySelector('.r-actions button').className]);
  expect('…its steps unfolded to say why, and its link not offered as the main action', why[0] && why[1] === 'secondary', why.join(' | '));
  await page.context().close();

  page = await openPage({ query: `?chain=arbitrum&tx=${arb.target}`, routes: publicNodes(42161, ['good', 'unfinal']) });
  x = await linkSeal(page);
  expect('a block only public nodes confirm: gold, never green, never shown as settled, with the chain\'s own 20 minutes', x.tone === 'seal warn' && x.title === 'Proof valid, block not verified against Ethereum' && x.text === '' && x.checks[1] === "warn: Confirmed by public Arbitrum One nodesArbitrum One treats this block as final about 20 minutes in. This page doesn't verify that." && x.checks.slice(0, 1).every((c) => c.startsWith('ok: ')), linkShort(x));
  expect('…its step says the nodes have it, and why Ethereum was not used', x.steps[2].startsWith('warn: Two public Arbitrum One nodes, run by others, have the block with this hash. Your browser could not verify it against Ethereum: '), x.steps[2]);
  await page.context().close();

  page = await openPage({ query: `?chain=arbitrum&tx=0x${'11'.repeat(32)}`, routes: publicNodes(42161) });
  x = await linkSeal(page);
  expect('a transaction Willow does not hold: said plainly', x.tone === 'seal warn' && x.title === 'Could not verify: Willow does not hold this transaction' && /newest seven days/.test(x.text), linkShort(x));
  await page.context().close();

  for (const [q, want] of [
    ['?chain=solana&tx=0x' + '11'.repeat(32), /names no chain Willow serves/],
    ['?chain=base&tx=0x1234', /not a transaction hash/],
    ['?chain=base&from=10&to=5', /first block is after its last/],
    ['?chain=base&from=1&to=2&address=0xabc', /address is not a list of addresses/],
  ]) {
    page = await openPage({ query: q });
    x = await linkSeal(page);
    expect(`a link that cannot be read (${q.slice(0, 24)}…) says so`, /^This link cannot be read/.test(x.intro) && want.test(x.intro) && !x.formShown, x.intro);
    await page.context().close();
  }
});

// 37. A swap router's fill is the settlement: what was paid, what was received and the fee, from
// the proven OrderFilled event (the mock's nodes name every token TKN, with 6 decimals).
await section('37', async () => {
  const c = ITEM('base-swap-receipt');
  const fill = c.answers[0].result.logs.find((l) => l.topics[0] === '0xfb6c56e214cadb4768595d6c38ec36861d571a297ad429eb15de3b9f2f7fe09f');
  const word = (i) => BigInt('0x' + fill.data.slice(2 + 64 * i, 66 + 64 * i));
  const six = (x) => {
    const s = x.toString().padStart(7, '0');
    const frac = s.slice(-6).replace(/0+$/, '');
    return `${BigInt(s.slice(0, -6)).toLocaleString('en-US')}${frac ? `.${frac}` : ''}`;
  };
  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453), clockAt: l2Clock(8453, Number(c.proof.result.blockNumber)) });
  const x = await linkSeal(page);
  const settle = x.lines;
  expect('a router fill shows as a verified swap', x.tone === 'seal ok' && x.title === 'Swap verified', linkShort(x));
  const tokenOut = '0x' + fill.data.slice(2 + 64 + 24, 2 + 128);
  expect('…paid and received from the event, one line each: a token by its name, an unlisted one\'s address under More', settle[0] === `Paid${six(word(2))} USDC` && settle[1] === `Received${six(word(3))} TKN` && (x.rows['TKN token'] || '').startsWith(`${tokenOut.slice(0, 6)}…${tokenOut.slice(-4)}`), `${settle.join(' | ')} | ${x.rows['TKN token']}`);
  await page.setViewportSize({ width: 390, height: 844 });
  const [overflow, past] = await overflowOf(page);
  expect('…at phone width with no horizontal page scroll', overflow <= 0, `${overflow}px: ${past}`);
  expect('…the fee in the summary, one line, under what was received', settle[2] === `Fee${six(word(4))} USDC` && settle.length === 3, settle.join(' | '));
  const sent = c.answers[0].result;
  const trader = `0x${fill.topics[1].slice(26)}`;
  expect('…the trader and the router under More, with From and To only where they add to them', x.groups[0] === 'Trade' && x.rows.Trader.startsWith(`0x${fill.topics[1].slice(26, 30)}`) && x.rows.Router.startsWith(fill.address.slice(0, 6).toLowerCase()) && ('From' in x.rows) === (sent.from.toLowerCase() !== trader) && ('To' in x.rows) === (sent.to.toLowerCase() !== trader && sent.to.toLowerCase() !== fill.address.toLowerCase()), JSON.stringify(x.rows).slice(0, 400));
  expect('…and Basescan beside Copy link', x.actions.join() === 'Copy link,Basescan,More' && x.explorer === `https://basescan.org/tx/${sent.transactionHash}`, `${x.actions.join()} | ${x.explorer}`);
  const route = await page.evaluate(() => [document.querySelector('details.moves') === null, document.querySelector('.r-moves h2').textContent, document.querySelector('.r-moves').hidden, document.querySelectorAll('.r-moves .move-list li').length]);
  expect("…the route's transfers shown in a group of their own, not folded", route[0] && route[1] === 'Token movements' && !route[2] && route[3] > 0, route.join(' | '));
  await page.context().close();
  // A reverted transaction moved nothing: no settlement, no value, only the fee.
  const rev = ITEM('base-reverted-receipt');
  page = await openPage({ query: `?chain=base&tx=${rev.target}`, routes: withL2Anchor(8453), clockAt: l2Clock(8453, Number(rev.proof.result.blockNumber)) });
  const y = await linkSeal(page);
  const paid = y.lines.filter((l) => /^(Paid|Received)/.test(l)).length;
  expect('a reverted transaction is verified as failed, with nothing shown as paid', y.tone === 'seal ok' && y.title === 'Transaction verified' && /^StatusFailed/.test(y.lines[0]) && paid === 0 && !('Value' in y.rows) && /^Failed/.test(y.rows.Status) && /ETH/.test(y.rows['Network fee']), `${linkShort(y)} | ${JSON.stringify(y.rows)}`);
  await page.context().close();
});

// 38. Navigation: a link's page is the card alone, its header the logo, which links to Willow in a new
// tab, with no footer; the card's "Technical details" leads to the form. A link
// with an empty tx opens the form's Transaction tab on its chain. The tabs are Logs, Transaction and
// Block.
await section('38', async () => {
  const c = ITEM('base-swap-receipt');
  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453), clockAt: l2Clock(8453, Number(c.proof.result.blockNumber)) });
  await linkSeal(page);
  const chrome = await page.evaluate(() => [[...document.querySelectorAll('.site-head a')].map((a) => `${a.className}=${a.getAttribute('href')} (${a.target})`).join(', '), document.querySelectorAll('.site-foot').length, getComputedStyle(document.querySelector('.r-extra')).justifyContent]);
  expect('a link\'s header is the logo alone, linking to Willow in a new tab, and there is no footer', chrome[0] === 'brand=https://willow.tech (_blank)' && chrome[1] === 0, chrome.join(' | '));
  expect('…and the proof\'s two buttons side by side, centered', chrome[2] === 'center', chrome[2]);
  await page.context().close();
  page = await openPage({ query: '?chain=base&tx=' });
  await page.waitForSelector('#form');
  const box = await page.evaluate(() => [location.search, document.getElementById('chain').value, document.querySelector('input[name=kind]:checked').value, document.getElementById('itemLabel').textContent, document.body.classList.contains('link-mode'), document.querySelectorAll('#logsField .hint, #itemField .hint').length, [...document.querySelectorAll('input[name=kind]')].map((i) => `${i.value}:${i.parentElement.textContent.trim()}`).join(', ')]);
  expect('an empty tx opens the form on its chain\'s Transaction tab, with no notes under the field and the tabs Logs, Transaction and Block', box[0] === '' && box[1] === '8453' && box[2] === 'receipt' && box[3] === 'Transaction' && !box[4] && box[5] === 0 && box[6] === 'logs:Logs, receipt:Transaction, blockReceipts:Block', box.join(' | '));
  await page.context().close();
  page = await openPage({ query: '?tx=' });
  await page.waitForSelector('#form');
  const eth = await page.evaluate(() => [document.getElementById('chain').value, document.querySelector('input[name=kind]:checked').value]);
  expect('…on Ethereum when it names no chain', eth.join(' | ') === '1 | receipt', eth.join(' | '));
  await page.context().close();
});

// 36. A link to a log range: the full checker's log check, without the form or the plain
// comparison, each proven log's transaction linked to its own check.
await section('36', async () => {
  page = await openPage({ query: `?chain=ethereum&from=${FROM}&to=${TO}`, routes: publicNodes(1) });
  const methods = page.methods;
  await page.waitForFunction(idle, null, { timeout: 120000 });
  r = await result(page, Date.now());
  const all = logsIn(FROM, TO);
  expect("a log link verifies with the published keys, final on Ethereum by its validators", r.tone.includes('ok') && r.title === `${all.toLocaleString('en-US')} event${all === 1 ? '' : 's'} verified` && r.text === '' && r.checks.join(' / ') === `ok: None missing in these 2 blocks / ok: Finalized on Ethereum, can't be reversedLast block: ${TO.toLocaleString('en-US')} / ok: Verified by your browser, no trust in Willow needed`, `${short(r)} | ${r.checks.join(' / ')}`);
  expect('…how it was verified, a sentence a step, with no step for the fixed blocks', r.steps.length === 4 && r.steps.every((x) => x.startsWith('done: ')) && r.steps[0].startsWith('done: Willow sent a proof for every block in the range') && r.steps[3] === 'done: Your browser verified the last block against Ethereum. - ' && r.steps.every((x) => x.endsWith(' - ')), r.steps.join(' / '));
  const hiddenNow = () => page.evaluate(() => ['logsCard', 'progress', 'linkProof', 'details'].map((id) => document.getElementById(id).checkVisibility()));
  const before = await hiddenNow();
  await page.click('#verdictActions .r-toggle');
  const after = await hiddenNow();
  const proofButtons = await page.$$eval('#linkProofActions button', (b) => b.map((x) => x.textContent));
  expect('…the list, the steps and the proof folded under More, the facts in the steps', before.every((v) => !v) && after.slice(0, 3).every(Boolean) && !after[3] && proofButtons.join() === 'Technical details' && (await page.textContent('#verdictActions .r-toggle')) === 'Less', `${before} → ${after} | ${proofButtons}`);
  expect('…with no plain eth_getLogs comparison', !methods.includes('eth_getLogs') && !r.steps.some((x) => /eth_getLogs answer/.test(x)), methods.join(','));
  const link = await page.$eval('#logs tbody tr td:nth-child(2) a', (a) => a.getAttribute('href'));
  expect("…each log's transaction linked to its own check", new RegExp(`\\?chain=ethereum&tx=${FROM}%3A\\d+$`).test(link), link);
  const intro = await page.textContent('#linkIntro');
  const formShown = await page.evaluate(() => getComputedStyle(document.getElementById('form')).display !== 'none');
  expect('…under a heading that says what is checked, with no form', /Every event in 2 blocks/.test(intro) && !formShown, intro);
  expect('…and its link to copy, and More', r.actions.join() === 'Copy link,More', r.actions.join());
  await page.context().close();
  // A narrower range offered in the link view is a new link, named by its own blocks.
  page = await openPage({ query: `?chain=ethereum&from=${FROM}&to=${TO}`, routes: [...publicNodes(1), ['**/v1/rpc/1/public', (route) => route.continue({ url: `${base}/v1/rpc/1/tail` })]] });
  await page.waitForFunction(idle, null, { timeout: 120000 });
  r = await result(page, Date.now());
  await Promise.all([page.waitForURL(new RegExp(`to=${FROM}$`)), page.click(`#verdictActions button:text-is("Verify block ${FROM} instead")`)]);
  await page.waitForFunction(idle, null, { timeout: 120000 });
  r = await result(page, Date.now());
  const narrowed = await page.textContent('#linkIntro');
  expect('a narrower range in the link view opens its own link, heading and all', r.tone.includes('ok') && /Every event in 1 block/.test(narrowed) && new URL(page.url()).searchParams.get('from') === String(FROM), `${short(r)} | ${narrowed}`);
  await page.context().close();
  // The full checker offers a link once a check verifies.
  page = await openPage({ routes: publicNodes(1) });
  r = await check(page, { filter: RANGE, node: '', leaf: '' });
  expect('a verified check in the full checker offers its link', r.tone.includes('ok') && r.actions.join() === 'Copy link', short(r));
  await page.context().close();
});

// 39. An Ethereum transaction two weeks old: its block final on Ethereum through the beacon state
// (Willow's daily anchor), verified in the browser, with no node asked about the block. Forged
// anchor data is "Not verified"; an upgrade the page does not know leaves the block unconfirmed.
await section('39', async () => {
  const eth = ITEM('eth-receipt');
  const tx = eth.proof.params[0];
  const block = Number(eth.proof.result.blockNumber);
  page = await openPage({ query: `?chain=ethereum&tx=${tx}`, routes: publicNodes(1) });
  let x = await linkSeal(page);
  expect(
    'an older Ethereum transaction: final on Ethereum, verified in the browser',
    x.tone === 'seal ok' && x.text === '' && x.checks[1] === "ok: Finalized on Ethereum, can't be reversed" && x.steps.length === 3 && x.steps.every((t) => t.startsWith('done')) && !('Finality' in x.rows),
    linkShort(x),
  );
  expect("…its step says what the browser verified it against, in one sentence", x.steps[2] === 'done: Your browser verified the block against Ethereum. ' && x.notes.every((t) => t === ''), x.steps[2]);
  expect('…and Etherscan beside Copy link', x.actions.join() === 'Copy link,Etherscan,More', x.actions.join());
  expect('…from the beacon state, with no node asked about the block', page.methods.includes('willow_getBlockAnchor') && !page.methods.includes('eth_getBlockByNumber'), page.methods.join(','));
  await page.context().close();

  page = await openPage({ query: `?chain=ethereum&tx=${tx}`, routes: [...publicNodes(1), ['**/v1/rpc/1/public', (route) => route.continue({ url: `${base}/v1/rpc/1/anchorforged` })]] });
  x = await linkSeal(page);
  expect('a forged anchor: not verified, said plainly', x.tone === 'seal bad' && x.title === 'Not verified' && x.text.startsWith(`Willow's proof is valid, but Willow's data to confirm block ${block.toLocaleString('en-US')} on Ethereum failed verification:`) && x.steps[2].startsWith('fail:'), linkShort(x));
  await page.context().close();

  page = await openPage({ query: `?chain=ethereum&tx=${tx}`, routes: [...publicNodes(1), ['**/v1/rpc/1/public', (route) => route.continue({ url: `${base}/v1/rpc/1/anchorgloas` })]] });
  x = await linkSeal(page);
  expect('an upgrade newer than the page: proof valid, block not confirmed, in gold', x.tone === 'seal warn' && x.title === 'Proof valid, block not confirmed' && /Ethereum's latest upgrade is newer than this page/.test(x.text) && x.checks[1] === 'warn: Block not confirmed', linkShort(x));
  await page.context().close();
});

// 40. A Base transaction a week old: its block final on Ethereum through Base's dispute game (read
// with Ethereum's contract code over state the light client verified), its output root and Base's
// own history; no node asked. With no anchor from Willow the public nodes still answer, labelled;
// a forged anchor is "Not verified". The page runs at the time the anchor was saved.
await section('40', async () => {
  const c = ITEM('base-swap-receipt');
  const block = Number(c.proof.result.blockNumber);
  const at = l2Clock(8453, block);
  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453), clockAt: at });
  let x = await linkSeal(page);
  expect(
    'a Base transaction a week old: final on Ethereum through its dispute game, verified in the browser',
    x.tone === 'seal ok' && x.title === 'Swap verified' && x.text === '' && x.checks[1] === "ok: Settled on Ethereum, can't be reversed" && !x.checks.some((c) => /final/.test(c)),
    linkShort(x),
  );
  expect('…its step says it was verified against Base\'s claim on Ethereum, in one sentence', x.steps[2] === 'done: Your browser verified the block against Ethereum. ', x.steps[2]);
  // Public nodes are asked only for unknown tokens' names (eth_call), never about the block.
  expect('…and asks no public node about the block', !page.methods.includes('eth_getBlockByNumber'), page.methods.join(','));
  await page.context().close();

  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453, 'anchornone'), clockAt: at });
  x = await linkSeal(page);
  expect(
    'no anchor from Willow: the public nodes answer, in gold, labelled as not verified against Ethereum',
    x.tone === 'seal warn' && x.title === 'Proof valid, block not verified against Ethereum' && x.text === '' && /could not verify it against Ethereum: /.test(x.steps[2]) && x.checks[1] === "warn: Confirmed by public Base nodesBase treats this block as final about 20 minutes in. This page doesn't verify that.",
    linkShort(x),
  );
  await page.context().close();

  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453, 'anchorforged'), clockAt: at });
  x = await linkSeal(page);
  expect('a forged Base anchor: not verified, and no node asked instead', x.tone === 'seal bad' && x.title === 'Not verified' && /failed verification/.test(x.text), linkShort(x));
  await page.context().close();

  page = await openPage({ query: `?chain=base&tx=${c.target}`, routes: withL2Anchor(8453), clockAt: at + 2 * 3600 * 1000 });
  x = await linkSeal(page);
  expect('Ethereum state from Willow two hours old: not used, the public nodes answer, in gold', x.tone === 'seal warn' && x.checks[1].startsWith('warn: Confirmed by public Base nodes') && /minutes old/.test(x.steps[2]), linkShort(x));
  await page.context().close();
});

// 41. An Arbitrum One transaction two weeks old: its block final on Ethereum through the Rollup's
// assertion (read with the contract's own code over state the light client verified), the asserted
// block's header and Arbitrum's own history; a forged anchor is "Not verified".
await section('41', async () => {
  const c = ITEM('arb-receipt');
  const block = parseInt(c.block.number, 16);
  const at = l2Clock(42161, block);
  page = await openPage({ query: `?chain=arbitrum&tx=${c.target}`, routes: withL2Anchor(42161), clockAt: at });
  let x = await linkSeal(page);
  expect(
    'an Arbitrum One transaction: final on Ethereum through its assertion, verified in the browser',
    x.tone === 'seal ok' && x.text === '' && x.checks[1] === "ok: Settled on Ethereum, can't be reversed" && x.steps[2] === 'done: Your browser verified the block against Ethereum. ',
    linkShort(x),
  );
  expect('…and asks no public node about the block', !page.methods.includes('eth_getBlockByNumber'), page.methods.join(','));
  await page.context().close();

  page = await openPage({ query: `?chain=arbitrum&tx=${c.target}`, routes: withL2Anchor(42161, 'anchorforged'), clockAt: at });
  x = await linkSeal(page);
  expect('a forged Arbitrum One anchor: not verified', x.tone === 'seal bad' && x.title === 'Not verified' && /failed verification/.test(x.text), linkShort(x));
  await page.context().close();
});

// 16. NETWORK=1: a real public node answers from the browser (CORS and the CSP allow it) and
// confirms the saved blocks, which are mainnet's.
await section('16', async () => {
  if (process.env.NETWORK) {
    page = await openPage();
    await fill(page, { filter: RANGE });
    await page.fill('#node', 'https://ethereum-rpc.publicnode.com');
    await page.click('#run');
    await page.waitForFunction(idle, null, { timeout: 120000 });
    const title = await page.textContent('#verdictTitle');
    expect('publicnode is reachable from the page and confirms the last block', CUSTOM_OK.test(title), title);
    await page.context().close();
  }
});

// Expected: the 404s and 429s the cases ask for, and (with screenshots, in WebKit) the CSP
// refusing the style Playwright injects while it takes a screenshot.
// Also the public nodes' requests that cases abort ('down'), which browsers log as CORS failures.
const nodeHosts = Object.values(PUBLIC_NODES).flat().map((u) => new URL(u).host);
const unexpected = consoleErrors.filter((e) => !/Failed to load resource|status of 429/.test(e) && !(shots && /^Refused to apply a stylesheet/.test(e)) && !nodeHosts.some((h) => e.includes(h)));
expect('no console errors', unexpected.length === 0, unexpected.slice(0, 3).join(' | '));

await browser.close();
mock.kill();
console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
