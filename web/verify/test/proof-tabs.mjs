// End-to-end: the Block, Account, Call and Fees tabs of the built page in a headless browser,
// against a stand-in server that serves the page (with the live CSP) and the proof methods from
// the block, fee, state and call fixtures (zkvm/logset-guest/blockproof/fixtures,
// zkvm/logset-guest/state-verify/testdata). It holds no anchors for those blocks, so a clean case
// ends "verification is not complete" at the block's confirmation; a tampered proof or a standard
// response that differs ends "Not verified". Exits 1 on any failed case.
//   [BROWSER=chromium|webkit|firefox] PLAYWRIGHT=<dir of the playwright package> node proof-tabs.mjs <dist> [screenshot dir]
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const [dist, shots] = process.argv.slice(2);
const require = createRequire(import.meta.url);
const engine = require(process.env.PLAYWRIGHT || 'playwright')[process.env.BROWSER || 'chromium'];
const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, '..', '..', '..');
const json = (p) => JSON.parse(readFileSync(join(repo, p), 'utf8'));
const W1 = 'zkvm/logset-guest/blockproof/fixtures';
const W2 = 'zkvm/logset-guest/state-verify/testdata';
const BLOCK = json(`${W1}/block/ethereum-26391511.json`);
const FEES = json(`${W1}/fees/ethereum-26391511.fee.json`);
const USDC = json(`${W2}/state/usdc-slots-code.json`);
const CALL = json(`${W2}/call/call-usdc-balance.json`);
const ESTIMATE = json(`${W2}/call/estimate-transfer.json`);
const flip = (h) => h.slice(0, -1) + (h.at(-1) === '0' ? '1' : '0');
const CSP = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' https: http://127.0.0.1:*; img-src 'self' data:; style-src 'self'; worker-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'application/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.txt': 'text/plain', '.woff2': 'font/woff2', '.json': 'application/json' };
const STATE = new Set(['willow_getStateProof', 'willow_getCallProof', 'willow_getEstimateGasProof', 'eth_getBalance', 'eth_getTransactionCount', 'eth_getCode', 'eth_getStorageAt', 'eth_getProof', 'eth_call', 'eth_estimateGas']);

/// One JSON-RPC response, by the path's mode: good, tampered (the proof changed), lying (a standard response changed).
function respond(mode, key, { method, params }) {
  const tampered = mode === 'tampered';
  const lying = mode === 'lying';
  const copy = (x) => structuredClone(x);
  if (key === 'public' && STATE.has(method)) throw { code: -32601, message: `the method ${method} is not served on the public path, use an API key` };
  const node = BLOCK.node.eth_getBlockByNumber;
  switch (method) {
    case 'eth_chainId':
      return '0x1';
    case 'willow_getBlockAnchor':
      throw { code: -32000, message: `not found in this store: no anchor for block ${params[0]}` };
    case 'willow_getBlockProof': {
      const r = copy(BLOCK.result);
      if (tampered) r.transactions[3] = flip(r.transactions[3]);
      return r;
    }
    case 'eth_getBlockByHash': {
      const b = copy(node);
      delete b.totalDifficulty;
      if (!params[1]) b.transactions = b.transactions.map((t) => t.hash);
      if (lying) b.gasUsed = '0x1';
      return b;
    }
    case 'willow_getFeeProof': {
      const r = copy(FEES.result);
      if (tampered) [r.headers[0], r.headers[1]] = [r.headers[1], r.headers[0]];
      return r;
    }
    case 'eth_gasPrice':
    case 'eth_maxPriorityFeePerGas':
    case 'eth_blobBaseFee':
      return FEES.node.responses[method];
    case 'willow_getStateProof': {
      const r = copy(USDC.result);
      if (tampered) r.account.balance = '0x1';
      return r;
    }
    case 'eth_getBalance':
      return lying ? '0x1' : USDC.expect.eth_getBalance;
    case 'eth_getTransactionCount':
    case 'eth_getCode':
      return USDC.expect[method];
    case 'eth_getStorageAt':
      return USDC.expect.eth_getStorageAt[USDC.request.params[1].indexOf(params[1])];
    case 'eth_getProof':
      return USDC.expect.eth_getProof;
    case 'willow_getCallProof': {
      const r = copy(CALL.result);
      if (tampered) r.result = { returned: flip(r.result.returned) };
      return r;
    }
    case 'eth_call':
      return CALL.expect.result;
    case 'willow_getEstimateGasProof':
      return copy(ESTIMATE.result);
    case 'eth_estimateGas':
      return ESTIMATE.expect.result;
    default:
      throw { code: -32601, message: `the method ${method} does not exist/is not available` };
  }
}

const server = createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (req.method === 'GET' && url.pathname.startsWith('/verify/')) {
    const name = url.pathname.slice('/verify/'.length) || 'index.html';
    const file = join(dist, name);
    if (name.includes('..') || !existsSync(file)) return res.writeHead(404).end();
    return res.writeHead(200, { 'content-type': TYPES[extname(file)] || 'application/octet-stream', 'content-security-policy': CSP, 'cache-control': 'no-cache' }).end(readFileSync(file));
  }
  const m = /^\/v1\/rpc\/(\d+)\/([a-z]+)$/.exec(url.pathname);
  if (req.method !== 'POST' || !m) return res.writeHead(404).end();
  let body = '';
  req.on('data', (c) => (body += c));
  req.on('end', () => {
    const it = JSON.parse(body);
    let out;
    try {
      out = { jsonrpc: '2.0', id: it.id, result: respond(m[2], m[2], it) };
    } catch (e) {
      out = { jsonrpc: '2.0', id: it.id, error: e };
    }
    res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(out));
  });
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;

let failures = 0;
const expect = (name, ok, detail) => {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures += 1;
};
if (shots) mkdirSync(shots, { recursive: true });
const browser = await engine.launch({ headless: true });
console.log(`browser: ${process.env.BROWSER || 'chromium'} ${browser.version()}`);
const errors = [];

async function open(query = '', viewport = { width: 1280, height: 900 }) {
  const ctx = await browser.newContext({ viewport });
  const page = await ctx.newPage();
  page.setDefaultTimeout(30000);
  page.on('console', (m) => m.type() === 'error' && !/Failed to load resource/.test(m.text()) && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${base}/verify/${query}`);
  return page;
}
const done = (page) => page.waitForFunction(() => !document.getElementById('result').hidden && !document.getElementById('run').disabled);
async function outcome(page) {
  return {
    tone: (await page.getAttribute('#verdict', 'class')).replace('verdict ', ''),
    title: await page.textContent('#verdictTitle'),
    text: await page.textContent('#verdictText'),
    steps: await page.$$eval('#steps li', (l) => l.map((li) => `${li.dataset.state}: ${li.querySelector('.what').textContent} - ${li.querySelector('.note').textContent}`)),
    table: await page.$$eval('#itemTable tbody tr', (r) => r.map((tr) => tr.textContent)),
    tableTitle: await page.textContent('#itemTitle'),
    coverage: await page.textContent('#proofCoverageText'),
  };
}
async function run(kind, { mode = 'good', fill, name }) {
  const page = await open();
  await page.check(`input[name=kind][value=${kind}]`);
  await page.evaluate(() => (document.querySelector('details.advanced').open = true));
  await page.fill('#endpoint', `${base}/v1/rpc/1/${mode}`);
  await fill(page);
  await page.click('#run');
  await done(page);
  const o = await outcome(page);
  if (shots) await page.screenshot({ path: join(shots, `${name}.png`), fullPage: true });
  await page.context().close();
  return o;
}
const stepOf = (o, what) => o.steps.find((s) => s.includes(what)) || '';
const notComplete = (o) => o.tone === 'warn' && o.title === 'Proof valid, but verification is not complete' && /no anchor for block/.test(o.text);

// ---------- Block ----------
const n = parseInt(BLOCK.result.blockNumber, 16);
const typeBlock = (p) => p.fill('#item', String(n));
{
  const o = await run('block', { fill: typeBlock, name: 'block' });
  expect('Block: the proof verifies; Willow\'s eth_getBlockByHash, both forms, are the proven block; no anchor here', notComplete(o) && stepOf(o, 'Verifying the block').startsWith('done') && o.steps.filter((s) => s.includes("Willow's eth_getBlockByHash") && s.startsWith('done')).length === 2, `${o.tone}: ${o.title} | ${o.steps.join(' | ')}`);
  expect('Block: the proven fields are listed', o.table.some((r) => r.startsWith('hash') && r.includes(BLOCK.result.blockHash)) && o.table.some((r) => r.startsWith('transactions')) && /every transaction/.test(o.coverage), `${o.tableTitle}: ${o.table.length} rows`);
}
{
  const o = await run('block', { mode: 'tampered', fill: typeBlock, name: 'block-tampered' });
  expect('Block: a changed transaction fails verification', o.tone === 'bad' && o.title === "Not verified: Willow's proof failed verification" && /\(transactions\)/.test(o.text) && stepOf(o, 'Verifying the block').startsWith('fail'), `${o.tone}: ${o.title}: ${o.text.slice(0, 160)}`);
}
{
  const o = await run('block', { mode: 'lying', fill: typeBlock, name: 'block-lying' });
  expect("Block: Willow's standard response with another gasUsed is called out", o.tone === 'bad' && /Willow's eth_getBlockByHash.* response differs/.test(o.title), `${o.tone}: ${o.title}`);
}

// ---------- Account ----------
const [address, slots] = USDC.request.params;
const typeAccount = (p) => p.fill('#address', address).then(() => p.fill('#slots', slots.join(', '))).then(() => p.fill('#at', USDC.result.blockHash));
{
  const page = await open();
  await page.check('input[name=kind][value=account]');
  await page.fill('#address', address);
  await page.click('#run');
  const err = await page.textContent('#formError');
  expect('Account: on the keyless path the page asks for an API key, sending nothing', /only for an API key/.test(err) && (await page.isHidden('#progress')), err);
  await page.context().close();
}
{
  const o = await run('account', { fill: typeAccount, name: 'account' });
  const want = BigInt(USDC.expect.eth_getBalance).toLocaleString('en-US');
  expect("Account: the proof verifies, and Willow's five standard responses at that block are the proven ones", notComplete(o) && stepOf(o, 'Verifying the account').startsWith('done') && o.steps.filter((s) => s.startsWith('done: Willow')).length === 3 + slots.length + 1, o.steps.join(' | '));
  expect('Account: balance, nonce, code and slots are listed', o.table.some((r) => r.startsWith('eth_getBalance') && r.includes(want)) && o.table.filter((r) => r.startsWith('slot ')).length === slots.length && o.table.some((r) => r.startsWith('eth_getCode') && r.includes('bytes')), o.table.slice(0, 3).join(' | '));
}
{
  const o = await run('account', { mode: 'tampered', fill: typeAccount, name: 'account-tampered' });
  expect('Account: a changed balance fails verification', o.tone === 'bad' && /\(account\)/.test(o.text), `${o.tone}: ${o.title}: ${o.text.slice(0, 160)}`);
}
{
  const o = await run('account', { mode: 'lying', fill: typeAccount, name: 'account-lying' });
  expect("Account: Willow's standard eth_getBalance with another value is called out", o.tone === 'bad' && /Willow's eth_getBalance response differs/.test(o.title), `${o.tone}: ${o.title}`);
}

// ---------- Call and estimate ----------
const typeCall = (call, estimate) => async (p) => {
  await p.fill('#call', JSON.stringify(call));
  if (estimate) await p.check('#estimate');
  await p.fill('#at', CALL.result.blockHash);
};
{
  const o = await run('call', { fill: typeCall(CALL.request.params[0]), name: 'call' });
  expect("Call: the browser's own run is the proven result; Willow's eth_call is the same", notComplete(o) && stepOf(o, 'Running the call').startsWith('done') && stepOf(o, "Willow's eth_call").startsWith('done') && o.table.some((r) => r.includes(CALL.expect.result)), `${o.title} | ${o.steps.join(' | ')}`);
}
{
  const o = await run('call', { mode: 'tampered', fill: typeCall(CALL.request.params[0]), name: 'call-tampered' });
  expect('Call: another result fails verification', o.tone === 'bad' && /\(outcome\)/.test(o.text), `${o.tone}: ${o.title}: ${o.text.slice(0, 160)}`);
}
{
  const o = await run('call', { fill: typeCall(ESTIMATE.request.params[0], true), name: 'estimate' });
  expect("Estimate: geth's search, run in the browser, finds the proven gas", notComplete(o) && stepOf(o, 'Estimating its gas').startsWith('done') && o.table.some((r) => r.includes('21,000 gas')), `${o.title} | ${o.table.join(' | ')}`);
}

// ---------- Fees ----------
{
  const o = await run('fees', { fill: (p) => p.fill('#at', FEES.result.blockHash), name: 'fees' });
  expect('Fees: the four blocks verify and the formula gives the fees', notComplete(o) && stepOf(o, 'computing the fees').startsWith('done') && o.table.some((r) => r.startsWith('eth_gasPrice') && r.includes(BigInt(FEES.verified.eth_gasPrice).toLocaleString('en-US'))), `${o.title} | ${o.table.join(' | ')}`);
}
{
  const o = await run('fees', { mode: 'tampered', fill: (p) => p.fill('#at', FEES.result.blockHash), name: 'fees-tampered' });
  expect('Fees: two headers swapped fail verification', o.tone === 'bad' && /\(broken-link\)/.test(o.text), `${o.tone}: ${o.title}: ${o.text.slice(0, 160)}`);
}

// ---------- Links ----------
{
  const page = await open(`?chain=ethereum&block=${n}`);
  await done(page);
  const o = await outcome(page);
  expect('Link ?chain=ethereum&block=N: the Block tab opens and verifies at once', notComplete(o) && (await page.isChecked('input[name=kind][value=block]')) && (await page.inputValue('#item')) === String(n), `${o.tone}: ${o.title}`);
  await page.context().close();
}
{
  const call = Buffer.from(JSON.stringify(CALL.request.params[0])).toString('base64url');
  const page = await open(`?chain=ethereum&call=${call}&block=${CALL.result.blockHash}`);
  await page.waitForFunction(() => document.getElementById('formError').textContent.length > 0);
  const filled = JSON.parse(await page.inputValue('#call'));
  expect('Link ?chain=&call=&block=: the Call tab is filled in, and asks for an API key', (await page.isChecked('input[name=kind][value=call]')) && filled.to === CALL.request.params[0].to && /API key/.test(await page.textContent('#formError')));
  await page.context().close();
}
{
  const page = await open(`?chain=ethereum&address=${address}&slots=${slots.join(',')}&block=${n}`);
  await page.waitForFunction(() => document.getElementById('formError').textContent.length > 0);
  expect('Link ?chain=&address=&slots=&block=: the Account tab is filled in', (await page.isChecked('input[name=kind][value=account]')) && (await page.inputValue('#address')) === address && (await page.inputValue('#slots')).split(', ').length === slots.length && (await page.inputValue('#at')) === String(n));
  await page.context().close();
}
{
  const page = await open('?chain=ethereum&address=0x12');
  await page.waitForFunction(() => !document.getElementById('linkIntro').hidden);
  expect('Link with a bad address: says so', /address is not an account address/.test(await page.textContent('#linkIntro')));
  await page.context().close();
}

// ---------- Phone width ----------
for (const kind of ['block', 'account', 'call', 'fees']) {
  const page = await open('', { width: 360, height: 780 });
  await page.check(`input[name=kind][value=${kind}]`);
  const over = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  if (shots && kind === 'call') await page.screenshot({ path: join(shots, 'phone-call.png'), fullPage: true });
  expect(`Phone width: the ${kind} tab does not scroll sideways`, over <= 0, `${over}px`);
  await page.context().close();
}

expect('No console errors', errors.length === 0, errors.slice(0, 3).join(' | '));
await browser.close();
server.close();
console.log(failures ? `${failures} FAILED` : 'all passed');
process.exit(failures ? 1 : 0);
