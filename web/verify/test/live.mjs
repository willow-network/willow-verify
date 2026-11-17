// The built page against a LIVE log RPC, same-origin through mock_rpc.py's proxy mode.
//   WILLOW_UPSTREAM=<log rpc url with key> PLAYWRIGHT=<playwright dir> [CHAIN=<id>] [KIND=logs|transaction|receipt|blockReceipts]
//     [NODE_URL=<the user's node>] [OTHER_URL=<another provider to compare>] [ON_ASK=go] node live.mjs <dist> ['<filter json>' | <transaction or block> | example]
// Prints any wait for proofs made on request, the steps, the verdict and the details; never prints
// the upstream URL.
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const [dist, filterArg = 'example'] = process.argv.slice(2);
const CHAIN = process.env.CHAIN || '1';
const KIND = process.env.KIND || 'logs';
if (!process.env.WILLOW_UPSTREAM) throw new Error('set WILLOW_UPSTREAM');
const require = createRequire(import.meta.url);
const engine = require(process.env.PLAYWRIGHT || 'playwright')[process.env.BROWSER || 'chromium'];
// A port nothing answers on yet: a mock left over from another run must not answer for this one.
let PORT;
for (;;) {
  PORT = 9787 + Math.floor(Math.random() * 1000);
  try {
    await fetch(`http://127.0.0.1:${PORT}/`);
  } catch {
    break;
  }
}
const base = `http://127.0.0.1:${PORT}`;
const here = dirname(fileURLToPath(import.meta.url));
const mock = spawn('python3', [join(here, 'mock_rpc.py'), dist, join(here, 'no-data'), String(PORT)], { stdio: 'inherit' });
let mockExited = false;
mock.on('exit', () => (mockExited = true));
try {
  for (let i = 0; ; i++) {
    try {
      await fetch(`${base}/verify/`);
      break;
    } catch {
      if (i > 50 || mockExited) throw new Error('mock did not start');
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  const browser = await engine.launch({ headless: true });
  const page = await browser.newPage();
  page.on('console', (m) => m.type() === 'error' && console.log(`console: ${m.text()}`));
  await page.goto(`${base}/verify/`);
  await page.selectOption('#chain', CHAIN);
  await page.check(`input[name=kind][value=${KIND}]`);
  await page.fill('#endpoint', `${base}/v1/rpc/${CHAIN}/proxy`);
  await page.fill('#node', process.env.NODE_URL || '');
  if (process.env.OTHER_URL) {
    await page.evaluate(() => (document.querySelector('details.advanced').open = true));
    await page.fill('#compare', process.env.OTHER_URL);
  }
  if (process.env.LEAF_KEYS || process.env.SEGMENT_KEYS) {
    await page.evaluate(() => (document.querySelector('details.advanced').open = true));
    await page.fill('#leafKeys', process.env.LEAF_KEYS || '');
    await page.fill('#segmentKeys', process.env.SEGMENT_KEYS || '');
  }
  if (KIND !== 'logs') {
    if (filterArg === 'example') {
      await page.fill('#item', '');
      await page.click('#itemExample');
      await page.waitForFunction(() => document.getElementById('item').value.length > 0, null, { timeout: 60000 });
    } else {
      await page.fill('#item', filterArg);
    }
    console.log(`chain ${CHAIN}, ${KIND}: ${await page.inputValue('#item')}`);
  } else if (filterArg === 'example') {
    await page.fill('#filter', '{}');
    await page.click('#example');
    await page.waitForFunction(() => /^0x/.test(JSON.parse(document.getElementById('filter').value).fromBlock || ''), null, { timeout: 60000 });
  } else {
    await page.fill('#filter', filterArg);
  }
  if (KIND === 'logs') console.log(`chain ${CHAIN}, filter: ${await page.inputValue('#filter')}`);
  const t = Date.now();
  await page.click('#run');
  // The page may stop to ask before a large download: print the ask, then answer it
  // (ON_ASK=go downloads; anything else cancels). While Willow proves blocks on request, print
  // each new progress line (the page itself stops waiting after two hours).
  let waiting = '';
  for (;;) {
    const state = await page.waitForFunction(
      (last) => {
        if (!document.getElementById('result').hidden && !document.getElementById('run').disabled) return 'done';
        // An ask on screen (once answered, its buttons stay in the hidden card).
        const shown = !document.getElementById('result').hidden;
        if (shown && [...document.querySelectorAll('#verdictActions button')].some((b) => b.textContent === 'Download and verify')) return 'ask';
        if (document.getElementById('result').hidden || !document.getElementById('verdict').classList.contains('wait')) return false;
        const now = `${document.getElementById('verdictTitle').textContent}. ${document.getElementById('waitText').textContent}`;
        return now !== last ? `wait:${now}` : false;
      },
      waiting,
      { timeout: 2.5 * 3600 * 1000 },
    );
    const s = await state.jsonValue();
    if (s === 'done') break;
    if (s.startsWith('wait:')) {
      waiting = s.slice(5);
      console.log(`WAITING: ${waiting}`);
      continue;
    }
    console.log(`\nASKED: ${await page.textContent('#verdictTitle')}\n${await page.textContent('#verdictText')}\n-> ${process.env.ON_ASK === 'go' ? 'Download and verify' : 'Cancel'}`);
    await page.click(`#verdictActions button:text-is("${process.env.ON_ASK === 'go' ? 'Download and verify' : 'Cancel'}")`);
  }
  console.log(`\n${(await page.evaluate(() => [...document.querySelectorAll('#steps li')].map((li) => `[${li.dataset.state}] ${li.querySelector('.what').textContent}: ${li.querySelector('.note').textContent}`))).join('\n')}`);
  console.log(`\nVERDICT (${await page.getAttribute('#verdict', 'class')}): ${await page.textContent('#verdictTitle')}\n${await page.textContent('#verdictText')}`);
  const facts = await page.evaluate(() => [...document.querySelectorAll('#facts dt')].map((dt) => `${dt.textContent}: ${dt.nextElementSibling.textContent}`));
  console.log(`\n${facts.join('\n')}\nwall ${Date.now() - t} ms`);
  if (KIND !== 'logs') {
    const extra = await page.evaluate(() => ({
      uncovered: document.getElementById('itemUncovered').textContent,
      title: document.getElementById('itemTitle').textContent,
      rows: [...document.querySelectorAll('#itemTable tbody tr')].slice(0, 6).map((tr) => [...tr.cells].map((c) => c.textContent.slice(0, 90)).join(' | ')),
      diff: document.getElementById('diff').hidden ? '' : document.getElementById('diffBody').textContent.slice(0, 1500),
    }));
    console.log(`\n${extra.uncovered}\n${extra.title}:\n  ${extra.rows.join('\n  ')}${extra.diff ? `\nDIFFERENCES:\n${extra.diff}` : ''}`);
  }
  await browser.close();
} finally {
  mock.kill();
}
