// The page: fetches an answer and its proofs, hands them to the verifier in a Web Worker, then
// compares the plain answers and confirms the block hash against Ethereum's validators (or on a
// node). Every pass/fail decision is made in the wasm. Two flows: log proofs (SP1) and transaction /
// receipt proofs (Merkle-Patricia proofs against the block hash); both end at a block that the
// light client confirms (TRUSTLESS-ANCHOR.md).

// Served at https://api.willow.tech/verify/, so the same origin is Willow's API.
const ORIGIN = /^https?:$/.test(location.protocol) ? location.origin : 'https://api.willow.tech';
// The chains Willow's RPC serves, in the docs' order. `slugs`: names a link may use; `nodes`: two
// public nodes from different operators (allowing browser requests) that confirm the block when the
// user enters none; `explorer`, `explorerName`: a block explorer, for a link to the transaction; `blockMb`: a
// typical block's log records as hex, in MB, for the whole-block download estimate (Ethereum about
// 120 KB of records; the others from the chains' measured composition); `poolManager`: Uniswap v4's
// PoolManager, the example's address; `finalDays`: about how long an L2 block takes to be final on
// Ethereum (its claim's challenge period); `claimEvery`: how often its claims go on Ethereum.
// Every chain's gas is paid in ETH.
const CHAINS = [
  { id: 1, name: 'Ethereum', a: 'an', slugs: ['ethereum', 'eth', 'mainnet'], nodes: ['https://ethereum-rpc.publicnode.com', 'https://rpc.mevblocker.io'], explorer: 'https://etherscan.io', explorerName: 'Etherscan', blockMb: 0.25, poolManager: '0x000000000004444c5dc75cb358380d2e3de08a90' },
  { id: 8453, name: 'Base', a: 'a', slugs: ['base'], nodes: ['https://base-rpc.publicnode.com', 'https://mainnet.base.org'], explorer: 'https://basescan.org', explorerName: 'Basescan', finalDays: 5, claimEvery: '20 minutes', blockMb: 0.32, poolManager: '0x498581ff718922c3f8e6a244956af099b2652b2b' },
  { id: 42161, name: 'Arbitrum One', a: 'an', slugs: ['arbitrum', 'arbitrum-one', 'arb'], nodes: ['https://arbitrum-one-rpc.publicnode.com', 'https://arb1.arbitrum.io/rpc'], explorer: 'https://arbiscan.io', explorerName: 'Arbiscan', finalDays: 6.4, claimEvery: 'hour', blockMb: 0.009, poolManager: '0x360e68faccca8ca495c1b759fd9eee466db9fb32' },
  { id: 4663, name: 'Robinhood Chain', a: 'a', slugs: ['robinhood', 'robinhood-chain'], nodes: ['https://robinhood-rpc.publicnode.com', 'https://rpc.mainnet.chain.robinhood.com'], explorer: null, finalDays: 6.4, claimEvery: '30 minutes', blockMb: 0.019, poolManager: '0x8366a39cc670b4001a1121b8f6a443a643e40951' },
  { id: 10, name: 'OP Mainnet', a: 'an', slugs: ['optimism', 'op', 'op-mainnet'], nodes: ['https://optimism-rpc.publicnode.com', 'https://mainnet.optimism.io'], explorer: 'https://optimistic.etherscan.io', explorerName: 'OP Etherscan', finalDays: 7, claimEvery: 'hour', blockMb: 0.14, poolManager: '0x9a13f98cb987694c9f086b1f5eb990eea8264ec3' },
  { id: 130, name: 'Unichain', a: 'a', slugs: ['unichain'], nodes: ['https://unichain-rpc.publicnode.com', 'https://mainnet.unichain.org'], explorer: 'https://uniscan.xyz', explorerName: 'Uniscan', finalDays: 7, claimEvery: 'hour', blockMb: 0.004, poolManager: '0x1f98400000000000000000000000000000000004' },
  { id: 57073, name: 'Ink', a: 'an', slugs: ['ink'], nodes: ['https://ink-rpc.publicnode.com', 'https://rpc-gel.inkonchain.com'], explorer: 'https://explorer.inkonchain.com', explorerName: 'Ink Explorer', finalDays: 7, claimEvery: '12 hours', blockMb: 0.016, poolManager: '0x360e68faccca8ca495c1b759fd9eee466db9fb32' },
];
// Willow's keyless path: proofs of blocks already proven, nothing proven on request.
const PUBLIC_KEY = 'public';
// Days of blocks Willow holds in full, the ones it proves transactions and receipts from.
const FULL_DAYS = 'seven';
// A block a node does not have yet (seconds old): ask again this many times, this far apart.
const NODE_RETRIES = 3;
const NODE_RETRY_MS = 2000;
// Uniswap v4 Swap events, from the chain's PoolManager.
const SWAP_TOPIC = '0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f';
const EXAMPLE_BLOCKS = 10;
const STORE_KEY = 'willow-verify:endpoints';
const DOWNLOADS_AT_ONCE = 4;
const SHOW_LOGS = 100;
// Above this many proofs to download, ask first. A compressed SP1 proof is about
// 1.27 MB, sent as hex: about 2.5 MB on the wire.
const LARGE_DOWNLOAD = 20;
const PROOF_WIRE_MB = 2.5;
// A leaf not yet in a final segment comes whole in the willow_getLogsProof answer: every log of
// each of its blocks (one block on Ethereum), up to 16 MB of records per answer (about 32 MB as
// hex). Above this much of it (about 20 Ethereum blocks, or one leaf on most fast chains), ask first.
const TAIL_ASK_MB = 5;
const TAIL_ANSWER_MAX_MB = 32;
const HEALTH_TIMEOUT_MS = 8000;
// Without the proof status's geometry: Ethereum's, one-block leaves and 64-block segments (each
// starting at a multiple of 64). Without proof jobs, segments count as current while the newest
// ends at most SEGMENT_MAX_LAG blocks behind the tip (as the server's proof health does).
const LEAF_BLOCKS = 1;
const SEGMENT_BLOCKS = 64;
const SEGMENT_MAX_LAG = 512;
// HTTP 429 from Willow: wait this long before each retry.
const RATE_LIMIT_WAITS_MS = [1000, 2000];
// While Willow proves blocks on request: poll the jobs every retryAfterSeconds within these bounds,
// ask willow_getLogsProof again at most this often when a job cannot be read, stop after two hours.
const POLL_MIN_S = 2;
const POLL_MAX_S = 15;
const REASK_MAX_S = 60;
const WAIT_MAX_MS = 2 * 60 * 60 * 1000;
// Jobs polled for progress, polls in a row that may fail, and the job id format.
const JOBS_POLLED = 8;
const POLL_FAILURES_MAX = 8;
const JOB_ID = /^j-\d{1,20}-[0-9a-f]{8}$/;
// Receipts listed for a block's receipts.
const SHOW_RECEIPTS = 100;

const $ = (id) => document.getElementById(id);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let worker = null;
let lastLogs = null;
let lastRange = null;
let lastVerified = false;
// The last transaction / receipt check: { name, json } for the download.
let lastItems = null;
// Nodes typed per chain this session (and remembered, when asked).
const nodes = {};

// ---------- small helpers ----------

class CheckError extends Error {
  constructor(message, extra = {}) {
    super(message);
    Object.assign(this, extra);
  }
}

function hostOf(url) {
  try {
    return new URL(url).host;
  } catch {
    return 'the endpoint';
  }
}

function httpUrl(s) {
  try {
    const u = new URL(s);
    return u.protocol === 'https:' || u.protocol === 'http:';
  } catch {
    return false;
  }
}

const HEX = new Int8Array(128).fill(-1);
for (let i = 0; i < 16; i++) {
  HEX['0123456789abcdef'.charCodeAt(i)] = i;
  HEX['0123456789ABCDEF'.charCodeAt(i)] = i;
}

function hexToBytes(s) {
  let i = s.startsWith('0x') ? 2 : 0;
  if ((s.length - i) % 2) throw new CheckError('odd-length hex');
  const out = new Uint8Array((s.length - i) / 2);
  for (let j = 0; j < out.length; j++, i += 2) {
    const a = s.charCodeAt(i), b = s.charCodeAt(i + 1);
    const hi = a < 128 ? HEX[a] : -1, lo = b < 128 ? HEX[b] : -1;
    if (hi < 0 || lo < 0) throw new CheckError('bad hex');
    out[j] = (hi << 4) | lo;
  }
  return out;
}

const n = (x) => Number(x).toLocaleString('en-US');
const hexNum = (h) => parseInt(h, 16);

/// A 0x number from a server's answer, or a CheckError. Never converts anything else to text: a
/// deeply nested array would throw a stack overflow in the browser.
function quantity(x, what) {
  const v = typeof x === 'string' && /^0x[0-9a-fA-F]{1,13}$/.test(x) ? hexNum(x) : NaN;
  if (!Number.isSafeInteger(v)) throw new CheckError(`${what} is not a 0x number.`);
  return v;
}
const toHex = (x) => '0x' + x.toString(16);

function bytes(b) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function secs(ms) {
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;
}

const elapsed = (ms) => (ms < 60000 ? secs(ms) : `${Math.round(ms / 60000)} min`);

/// `s` ending in a full stop, for a message joined to others in one paragraph.
const sentence = (s) => (/[.?]$/.test(s) ? s : `${s}.`);
const cap = (s) => s.charAt(0).toUpperCase() + s.slice(1);
const clamp = (x, lo, hi) => Math.min(Math.max(x, lo), hi);

function blocksText(from, to) {
  return from === to ? `block ${from}` : `blocks ${from}–${to}`;
}

/// "Blocks 5–9 are" or "Block 5 is", for a sentence.
const blocksAre = (r) => `${cap(blocksText(r.from, r.to))} ${r.from === r.to ? 'is' : 'are'}`;

function minutesText(ms) {
  const m = Math.max(1, Math.round(ms / 60000));
  return m === 1 ? '1 minute' : `${m} minutes`;
}

function el(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') e.className = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v);
  }
  for (const k of kids) e.append(k);
  return e;
}

// ---------- chains ----------

const chainOf = (id) => CHAINS.find((c) => c.id === id) || null;
/// The page opened from a link: no form, the keyless endpoint and the chain's public nodes.
const linkMode = () => document.body.classList.contains('link-mode');
/// The chain chosen on the page.
const chosen = () => chainOf(Number($('chain').value)) || CHAINS[0];
const RPC_PATH = /^(.*\/rpc\/)(\d+)(\/.*)?$/;

/// The chain id an endpoint URL names (its /rpc/<id>/ path), or null.
function endpointChain(url) {
  try {
    const m = RPC_PATH.exec(new URL(url).pathname);
    return m ? Number(m[2]) : null;
  } catch {
    return null;
  }
}

/// `url` naming chain `id` in its /rpc/<id>/ path; unchanged when it names no chain.
function withChain(url, id) {
  try {
    const u = new URL(url);
    const m = RPC_PATH.exec(u.pathname);
    if (!m || !/^https?:$/.test(u.protocol)) return url;
    u.pathname = `${m[1]}${id}${m[3] || '/'}`;
    return u.toString();
  } catch {
    return url;
  }
}

const defaultEndpoint = (ch) => `${ORIGIN}/v1/rpc/${ch.id}/${PUBLIC_KEY}`;
/// Whether `url` is Willow's keyless path, which proves nothing on request.
const isPublic = (url) => {
  try {
    return new URL(url).pathname.replace(/\/+$/, '').endsWith(`/rpc/${endpointChain(url)}/${PUBLIC_KEY}`);
  } catch {
    return false;
  }
};

// ---------- JSON-RPC ----------

/// The browser itself failed (out of memory: RangeError in Chrome and Safari, InternalError in
/// Firefox; a wasm trap), told by the error's type alone, never by its text. The worker reports
/// its own failures as kind 'internal'.
function isRuntimeFailure(e) {
  return (
    e instanceof RangeError ||
    (typeof WebAssembly !== 'undefined' && e instanceof WebAssembly.RuntimeError) ||
    (typeof InternalError !== 'undefined' && e instanceof InternalError)
  );
}

async function rpc(url, method, params) {
  for (let attempt = 0; ; attempt++) {
    let res;
    try {
      res = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
        cache: 'no-store',
        credentials: 'omit',
        referrerPolicy: 'no-referrer',
      });
    } catch (e) {
      throw new CheckError(
        `${hostOf(url)} could not be reached. If it is on another site, it may not accept requests from browsers.`,
      );
    }
    if (res.status === 429 && attempt < RATE_LIMIT_WAITS_MS.length) {
      await res.text().catch(() => {});
      const after = Number(res.headers.get('retry-after'));
      await sleep(after > 0 && after <= 10 ? after * 1000 : RATE_LIMIT_WAITS_MS[attempt]);
      continue;
    }
    const text = await res.text();
    let body = null;
    try {
      body = JSON.parse(text);
    } catch {}
    if (typeof body !== 'object' || Array.isArray(body)) body = null;
    const msg = body && body.error ? errorText(body.error) : '';
    // Rate limiting is HTTP 429 (its JSON-RPC code, -32005, is shared with "too large").
    if (res.status === 429) {
      throw new CheckError(`${hostOf(url)} answered HTTP 429 (${msg || 'too many requests'}). It is limiting requests for the moment; wait a minute, then try again.`, {
        title: 'Could not verify: rate limit reached',
        tone: 'warn',
        rateLimited: true,
      });
    }
    if (res.status === 401 || res.status === 403) {
      throw new CheckError(`${hostOf(url)} did not accept the request (HTTP ${res.status}${msg ? `: ${msg}` : ''}). If it takes an API key in its URL, make sure the key is right.`);
    }
    if (!body) throw new CheckError(`${hostOf(url)} answered ${method} with HTTP ${res.status}, not JSON-RPC.`);
    if (body.error) {
      // rpcData is only ever read field by field, never printed.
      throw new CheckError(`${method}: ${msg}`, { rpcMessage: msg, rpcCode: body.error.code, rpcData: body.error.data, httpStatus: res.status });
    }
    if (!('result' in body)) throw new CheckError(`${hostOf(url)} sent no result for ${method}.`);
    // `text` goes to the verifier as sent: re-serialising server data here could overflow the stack.
    return { result: body.result, size: text.length, text };
  }
}

/// A JSON-RPC error's message. Never throws: an error that is not a string can nest too deeply to print.
function errorText(error) {
  if (typeof error.message === 'string') return error.message;
  try {
    const s = JSON.stringify(error);
    return s.length > 300 ? `${s.slice(0, 300)}…` : s;
  } catch {
    return 'an error the page could not print';
  }
}

/// Willow's public status page `what` (proofs, or '' for the log store's) on `endpoint`'s host.
function healthUrl(endpoint, what = 'proofs') {
  try {
    const u = new URL(endpoint);
    const m = /^(.*\/rpc)\/\d+(?:\/[^/]*)?\/?$/.exec(u.pathname);
    if (m && (u.protocol === 'https:' || u.protocol === 'http:')) return `${u.origin}${m[1]}/health${what ? `/${what}` : ''}`;
  } catch {}
  return null;
}

/// Chain `id`'s entry of a public status page (no key), or null when it cannot be read.
async function healthOf(endpoint, what, id) {
  const url = healthUrl(endpoint, what);
  if (!url) return null;
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), HEALTH_TIMEOUT_MS);
  try {
    const res = await fetch(url, { cache: 'no-store', credentials: 'omit', referrerPolicy: 'no-referrer', signal: ctl.signal });
    // 503 (not healthy) still carries the report.
    const c = ((await res.json()).chains || {})[String(id)];
    return c && typeof c === 'object' && !Array.isArray(c) ? c : null;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

/// Willow's public proof status for chain `ch` on `endpoint`'s host (GET /v1/rpc/health/proofs, no
/// key), or null when it cannot be read. `ranges`: proven [from, to] block ranges, oldest first,
/// complete from block `knownFrom`; `segmentEnd`: where the newest final segment ends; `nearTip`:
/// the newest proven block near the tip; `leaf`, `window`: blocks per leaf and per final segment;
/// `jobs`: null, or { available } when blocks with no proof yet are proven on request. Unknown
/// numbers are null.
async function proofStatus(endpoint, ch) {
  const c = await healthOf(endpoint, 'proofs', ch.id);
  if (!c) return null;
  try {
    const num = (x) => (Number.isSafeInteger(x) && x >= 0 ? x : null);
    const newest = num(c.newestProvenBlock);
    // Before proof jobs the status listed no ranges, and newestProvenBlock was the newest proven block near the tip.
    const listed = Array.isArray(c.provenRanges);
    const ranges = listed ? rangesOf(c.provenRanges) : newest != null ? [[0, newest]] : [];
    const first = num(c.firstProvenBlock);
    const jobs = c.proofJobs && typeof c.proofJobs === 'object' ? { available: c.proofJobs.available === true } : null;
    // Powers of two, a segment of 1 to 64 leaves (as the server checks its geometry).
    const pow2 = (x) => Number.isSafeInteger(x) && x > 0 && (x & (x - 1)) === 0;
    const geometry = pow2(c.leafBlocks) && pow2(c.windowBlocks) && c.windowBlocks >= c.leafBlocks && c.windowBlocks / c.leafBlocks <= 64;
    return {
      tip: num(c.tip),
      nearTip: listed ? num(c.newestProvenNearTip) : newest,
      segmentEnd: num(c.lastSegmentEnd),
      ranges,
      // The status lists only the newest ranges; below the oldest listed, it cannot tell.
      knownFrom: first != null && ranges.length && first < ranges[0][0] ? ranges[0][0] : 0,
      leaf: geometry ? c.leafBlocks : LEAF_BLOCKS,
      window: geometry ? c.windowBlocks : SEGMENT_BLOCKS,
      jobs,
    };
  } catch {
    return null;
  }
}

/// The [from, to] block number pairs in `list`, sorted; anything else is left out.
function rangesOf(list) {
  const ok = (x) => Number.isSafeInteger(x) && x >= 0;
  return list
    .filter((r) => Array.isArray(r) && r.length === 2 && ok(r[0]) && ok(r[1]) && r[0] <= r[1])
    .map(([a, z]) => [a, z])
    .sort((x, y) => x[0] - y[0]);
}

/// The first block of [from, to] the proof status shows no proof for, or null.
function firstUnproven(from, to, head) {
  if (!head) return null;
  let b = Math.max(from, head.knownFrom);
  for (const [a, z] of head.ranges) {
    if (b > to || a > b) break;
    if (z >= b) b = z + 1;
  }
  return b <= to ? b : null;
}

/// The newest stretch of [from, to] the proof status shows proven, when that is not all of it.
function provenPart(from, to, head) {
  for (const [a, z] of head ? [...head.ranges].reverse() : []) {
    const lo = Math.max(a, from);
    const hi = Math.min(z, to);
    if (lo <= hi) return lo === from && hi === to ? null : { from: lo, to: hi };
  }
  return null;
}

/// The example's blocks: the 10 newest of Willow's newest final segment (one SP1 proof covers
/// them), else of its newest proven range; null when the status shows nothing proven.
function exampleRange(head) {
  if (!head) return null;
  const end = head.segmentEnd;
  if (end != null) {
    const r = head.ranges.find(([a, z]) => a <= end && end <= z);
    return { from: Math.max(r ? r[0] : 0, end - EXAMPLE_BLOCKS + 1), to: end };
  }
  const last = head.ranges[head.ranges.length - 1];
  return last ? { from: Math.max(last[0], last[1] - EXAMPLE_BLOCKS + 1), to: last[1] } : null;
}

/// The blocks of [from, to] the proof status shows proven but not in a final segment: the
/// willow_getLogsProof answer carries each of their leaves whole (every log of the leaf's blocks; a
/// leaf is one block on Ethereum), and each leaf needs its own SP1 proof. A block past the newest
/// segment's end, or whose segment window is not all proven, is in no segment. null when there are
/// none, or the status is unknown.
function tailEstimate(from, to, head, ch) {
  if (!head) return null;
  const W = head.window;
  const S = head.leaf;
  let blocks = 0;
  let leaves = 0;
  let inSegments = 0;
  let first = null;
  const leavesOf = (x, y) => (x > y ? 0 : Math.floor(y / S) - Math.floor(x / S) + 1);
  for (const [a, z] of head.ranges) {
    const lo = Math.max(a, from);
    const hi = Math.min(z, to);
    if (lo > hi) continue;
    const sLo = Math.max(lo, Math.ceil(a / W) * W);
    const sHi = Math.min(hi, Math.floor((z + 1) / W) * W - 1, head.segmentEnd ?? -1);
    const seg = sLo <= sHi ? sHi - sLo + 1 : 0;
    const whole = hi - lo + 1 - seg;
    if (whole && first == null) first = seg && lo === sLo ? sHi + 1 : lo;
    blocks += whole;
    leaves += seg ? leavesOf(lo, sLo - 1) + leavesOf(sHi + 1, hi) : leavesOf(lo, hi);
    inSegments += seg;
  }
  if (!blocks) return null;
  const segments = inSegments ? Math.ceil(inSegments / W) + 1 : 0;
  const answerMb = Math.min(leaves * S * ch.blockMb, TAIL_ANSWER_MAX_MB);
  const proofs = leaves + segments;
  return { first, blocks, leaves, leaf: S, window: W, segmentEnd: head.segmentEnd, proofs, answerMb, totalMb: answerMb + proofs * PROOF_WIRE_MB };
}

/// Blocks per leaf and per final segment: the proof status's, else Ethereum's.
const geometryOf = (head) => (head ? { leaf: head.leaf, window: head.window } : { leaf: LEAF_BLOCKS, window: SEGMENT_BLOCKS });
const leafWord = (g) => (g.leaf === 1 ? 'block' : `${n(g.leaf)}-block leaf`);

/// How many proofs a range needs, for the download messages.
const proofsPer = (g) => `one per ${g.window}-block segment, and one per ${leafWord(g)} not yet in a segment`;

// ---------- the filter ----------

/// The filter as typed, with fromBlock/toBlock checked: block numbers or "latest" (the default).
function readFilter() {
  let f;
  try {
    f = JSON.parse($('filter').value);
  } catch (e) {
    throw new CheckError(`The filter is not valid JSON: ${e.message}`);
  }
  if (!f || typeof f !== 'object' || Array.isArray(f)) throw new CheckError('The filter must be a JSON object.');
  if ('blockHash' in f) throw new CheckError('Use fromBlock and toBlock instead of blockHash.');
  for (const k of ['fromBlock', 'toBlock']) {
    const v = k in f ? f[k] : 'latest';
    if (v === 'latest') continue;
    if (v === 'safe' || v === 'finalized') {
      throw new CheckError(`${k}: "${v}" is not supported. Willow's log RPC answers block numbers and "latest"; enter a block number instead.`);
    }
    if (v === 'earliest' || v === 'pending') {
      throw new CheckError(`${k}: "${v}" is not supported here. This page takes block numbers and "latest"; enter a block number instead.`);
    }
    const ok =
      (typeof v === 'string' && /^0x[0-9a-fA-F]{1,13}$/.test(v) && Number.isSafeInteger(hexNum(v))) ||
      (typeof v === 'number' && Number.isSafeInteger(v) && v >= 0);
    if (!ok) throw new CheckError(`${k} must be a block number such as "0x18db868", or "latest".`);
  }
  return f;
}

/// Turns fromBlock/toBlock into 0x numbers, asking the endpoint once for "latest".
async function resolveRange(f, endpoint) {
  const out = { ...f };
  let tip = null;
  const nums = {};
  for (const k of ['fromBlock', 'toBlock']) {
    const v = k in f ? f[k] : 'latest';
    let num;
    if (v === 'latest') {
      if (tip === null) tip = quantity((await rpc(endpoint, 'eth_blockNumber', [])).result, `The latest block from ${hostOf(endpoint)}`);
      num = tip;
    } else {
      num = typeof v === 'number' ? v : hexNum(v);
    }
    nums[k] = num;
    out[k] = toHex(num);
  }
  if (nums.fromBlock > nums.toBlock) throw new CheckError('fromBlock is after toBlock.');
  return { filter: out, from: nums.fromBlock, to: nums.toBlock, tip };
}

// ---------- progress steps ----------

const steps = {};

function setupSteps(list) {
  const ol = $('steps');
  ol.replaceChildren();
  for (const k of Object.keys(steps)) delete steps[k];
  for (const [key, what] of list) {
    const note = el('div', { class: 'note' });
    const li = el('li', { 'data-state': 'pending' }, el('span', { class: 'mark', 'aria-hidden': 'true' }), el('div', {}, el('div', { class: 'what' }, what), note));
    steps[key] = { li, note };
    ol.append(li);
  }
  $('progress').hidden = false;
}

function skipStep(s) {
  s.li.dataset.state = 'skip';
  s.note.textContent = 'Not run';
}

function step(key, state, note) {
  const s = steps[key];
  if (!s) return;
  if (state) s.li.dataset.state = state;
  if (note !== undefined) s.note.textContent = note;
}

function relabel(key, text) {
  const s = steps[key];
  if (s) s.li.querySelector('.what').textContent = text;
}

// ---------- the worker ----------

/// The verifier's worker, started on first use. Requests are answered in order, so each answer
/// settles the oldest open request; `checked` messages report progress on it.
function spawnWorker() {
  if (worker) return worker;
  const w = new Worker(new URL('./__WORKER__', import.meta.url), { type: 'module' });
  const wk = { w, queue: [], keys: null, ready: null };
  worker = wk;
  w.onmessage = (e) => worker === wk && settle(wk, e.data);
  w.onerror = (e) => {
    e.preventDefault();
    failAll(wk, e.message || 'the verifier could not start in this browser');
  };
  w.onmessageerror = () => worker === wk && settle(wk, { type: 'error', step: 'load', kind: 'internal', message: 'a message from the verifier could not be read' });
  return wk;
}

function settle(wk, m) {
  const head = wk.queue[0];
  if (!head) return;
  if (m.type === 'checked') return head.onProgress && head.onProgress(m);
  wk.queue.shift();
  if (m.type !== 'error') return head.resolve(m);
  // After a failed check or a crash, start clean: kept proofs and wasm memory go with it.
  if (m.step === 'verify' || m.kind === 'internal') stopWorker(wk.w);
  // The worker's kind decides; the message is only shown (it can carry server text).
  if (m.kind === 'invalid-keys') head.reject(new CheckError(`The program keys under Advanced are not valid: ${m.message}`, { stage: m.step, kind: m.kind }));
  else head.reject(new CheckError(m.message, { stage: m.step, kind: m.kind }));
}

/// The worker could not run: every open request fails as the browser's failure.
function failAll(wk, message) {
  const open = wk.queue.splice(0);
  if (worker === wk) stopWorker(wk.w);
  for (const q of open) q.reject(new CheckError(message, { stage: 'load', kind: 'internal' }));
}

/// The log verifier for chain `chainId` with these keys: started (or restarted) as needed.
function startWorker(chainId, segmentKeys, leafKeys) {
  const want = `${chainId}\n${segmentKeys}\n${leafKeys}`;
  if (worker && worker.keys === want && worker.ready) return worker.ready;
  // Other keys or another chain: a fresh worker, so nothing kept carries over.
  if (worker && worker.keys !== null) stopWorker(worker.w);
  const wk = spawnWorker();
  wk.keys = want;
  wk.ready = ask({ type: 'init', chainId, segmentKeys, leafKeys }).then(
    (m) => {
      $('pinned').textContent = `blocks ${m.keys.pinned.leaf}, segments ${m.keys.pinned.segment}`;
      return m.keys;
    },
    (e) => {
      stopWorker(wk.w);
      throw e;
    },
  );
  return wk.ready;
}

function stopWorker(w) {
  w.terminate();
  if (worker && worker.w === w) {
    const open = worker.queue.splice(0);
    worker = null;
    for (const q of open) q.reject(new CheckError('the verifier stopped', { stage: 'load', kind: 'internal' }));
  }
}

/// One request to the worker. `transfer` hands ArrayBuffers over without a copy.
function ask(msg, onProgress, transfer = []) {
  const wk = spawnWorker();
  return new Promise((resolve, reject) => {
    wk.queue.push({ resolve, reject, onProgress });
    wk.w.postMessage(msg, transfer);
  });
}

async function showPinned() {
  try {
    await startWorker(chosen().id, '', '');
  } catch (e) {
    $('pinned').textContent = `(${e.message})`;
  }
}

const bare = (k) => String(k).trim().toLowerCase().replace(/^0x/, '');

/// True when the keys in force are not exactly the ones published with this page.
function customKeys(keys) {
  const same = (list, pinned) => list.length === 1 && bare(list[0]) === bare(pinned);
  return !same(keys.leaf, keys.pinned.leaf) || !same(keys.segment, keys.pinned.segment);
}

// ---------- proofs ----------

/// Downloads `ids` into `out` (id -> bytes). Stops at the first failure.
async function downloadProofs(endpoint, ids, out, onProgress) {
  let done = 0;
  let fetched = 0;
  onProgress(done, fetched);
  let next = 0;
  let failed = false;
  async function lane() {
    while (!failed && next < ids.length) {
      const id = ids[next++];
      let result, size;
      try {
        ({ result, size } = await rpc(endpoint, 'willow_getProof', [id]));
      } catch (e) {
        failed = true;
        throw e;
      }
      if (failed) return;
      try {
        out[id] = hexToBytes(result.proof);
      } catch (e) {
        failed = true;
        if (isRuntimeFailure(e)) throw e;
        throw new CheckError(`willow_getProof sent no proof, or malformed bytes, for ${id}.`, { kind: 'malformed-input' });
      }
      done += 1;
      fetched += size;
      onProgress(done, fetched);
    }
  }
  await Promise.all(Array.from({ length: Math.min(DOWNLOADS_AT_ONCE, ids.length) }, lane));
  return fetched;
}

/// Asks before a large download: "Download and verify", any `more` [label, value] options, and
/// Cancel. Resolves to 'go', an option's value, or false.
function askFirst(title, text, more = []) {
  return new Promise((resolve) => {
    const button = (label, value, cls) => el('button', { type: 'button', class: cls, onclick: () => resolve(value) }, label);
    const go = button('Download and verify', 'go', 'primary');
    showVerdict('warn', title, text, [go, ...more.map(([label, value]) => button(label, value, 'secondary')), button('Cancel', false, 'secondary')]);
    $('verdict').scrollIntoView({ block: 'nearest' });
    go.focus({ preventScroll: true });
  }).then((choice) => {
    $('result').hidden = true;
    return choice;
  });
}

function confirmDownload(count, g) {
  return askFirst(
    `Verifying this needs ${n(count)} SP1 proofs, about ${n(Math.round(count * PROOF_WIRE_MB))} MB to download`,
    'Each proof is about 1.3 MB, sent as about 2.5 MB of hex, and your browser keeps the proofs in memory while it verifies them. ' +
      `The number of proofs depends on the block range, not on the filter: ${proofsPer(g)}. ` +
      'Continue, or cancel and verify fewer blocks.',
  );
}

/// Asks before a willow_getLogsProof answer that carries whole blocks (see tailEstimate).
function confirmTail(t, from, to, ch) {
  const mb = (x) => n(Math.max(1, Math.round(x)));
  const count = `${n(t.blocks)} block${t.blocks === 1 ? '' : 's'}`;
  const where =
    t.segmentEnd == null
      ? `Willow has no final ${t.window}-block segment yet, so none of the ${count} in this range is in one.`
      : `${count} in this range ${t.blocks === 1 ? 'is' : 'are'} not in a final ${t.window}-block segment.`;
  const one = t.leaf === 1;
  const each = one
    ? `every log of each block, not only the logs your filter matches (about ${ch.blockMb} MB per block`
    : `every log of each block of their ${n(t.leaves)} lea${t.leaves === 1 ? 'f' : 'ves'} of ${n(t.leaf)} blocks, not only the logs your filter matches (about ${mb(t.leaf * ch.blockMb)} MB per leaf`;
  // The blocks before the first whole one are in final segments.
  const narrow = t.first > from ? { from, to: t.first - 1 } : null;
  return askFirst(
    `Verifying this needs about ${mb(t.totalMb)} MB to download`,
    `${where} For those, Willow's answer carries ${each}, at most ${TAIL_ANSWER_MAX_MB} MB), ` +
      `and each ${one ? 'block' : 'leaf'} needs its own SP1 proof (about ${PROOF_WIRE_MB} MB). Estimate: about ${mb(t.answerMb)} MB for the answer, then up to ${n(t.proofs)} proofs. ` +
      `Your browser keeps it all in memory while it verifies. Continue, ${narrow ? `verify ${blocksText(narrow.from, narrow.to)} instead (only blocks in final segments), ` : ''}or cancel and verify fewer blocks.`,
    narrow ? [[`Verify ${blocksText(narrow.from, narrow.to)} instead`, narrow]] : [],
  );
}

function cancelled(g) {
  return new CheckError(`You cancelled before the download. Verify fewer blocks to download less: Willow sends ${proofsPer(g)}.`, {
    title: 'Could not verify: download cancelled',
    tone: 'warn',
    stepNote: 'Cancelled',
  });
}

// ---------- the run ----------

/// A willow_getLogsProof refusal, in plain words. `head` is Willow's proof status (or null).
async function proofRequestError(e, from, to, endpoint, head) {
  if (e.rateLimited) return e;
  const msg = e.rpcMessage || '';
  // The log RPC's refusals were reworded so RPC routers fall back to another provider: "not found"
  // for what a store lacks (d1c13f67e), and -32002 "range too large: ..." for anything too large,
  // with no "limit" or "rate" in a message (f2cac6b63). Both wordings are read.
  const unproven = /(?:proof not found for|no proof yet for) block (\d+)/.exec(msg);
  if (unproven) return notProven(Number(unproven[1]), from, to, endpoint, head);
  if (/on-demand proving is (?:unavailable|not available)/.test(msg)) return provingUnavailable(from, to, head, msg);
  if (/not (?:found in|served by) this store: proofs/.test(msg)) return new CheckError('This endpoint does not serve proofs yet.');
  if (e.rpcCode === -32002 || e.rpcCode === -32005) {
    const plain = msg.replace(/^(?:block )?range too large: /, '');
    // The geometry the server names, else the proof status's: "final W-block segment", "each S-block leaf".
    const geo = (w, s) => ({ window: Number(w), leaf: s ? Number(s) : 1 });
    const tail = /blocks from (\d+) are not (?:yet )?in a final (\d+)-block segment(?:, so each (\d+)-block leaf)?/.exec(msg);
    if (tail) return notInSegment(Number(tail[1]), from, to, plain, head, null, geo(tail[2], tail[3]));
    const once = /once proven, (\d+) (?:blocks of|leaves of (\d+) blocks in) this range would be outside a final (\d+)-block segment/.exec(msg);
    if (once) return notInSegment(null, from, to, plain, head, Number(once[1]), geo(once[3], once[2]));
    if (/proof jobs not available for this key|proof jobs open|daily limit/.test(msg)) {
      const retry = provenPart(from, to, head);
      return new CheckError(
        `Willow will not prove more blocks on request for this API key right now: ${sentence(plain)}${retry ? ` ${blocksAre(retry)} already proven and can be verified now.` : ''}`,
        { title: "Could not verify: this key's proving limit is reached", tone: 'warn', retry, stepNote: msg },
      );
    }
    if (/can queue at most/.test(msg)) {
      return new CheckError(`Willow proves blocks that have no proof yet when you ask, but not this many in one request: ${sentence(plain)} Verify fewer blocks.`, {
        title: 'Could not verify: too many blocks to prove at once',
        tone: 'warn',
        stepNote: msg,
      });
    }
    return new CheckError(`Willow will not prove this range in one answer: ${plain}. Verify fewer blocks, or narrow the filter to fewer addresses or topics.`, {
      title: 'Could not verify: the answer is too large',
      tone: 'warn',
    });
  }
  return new CheckError(e.message);
}

/// Blocks from `b` are not in a final segment, or (`b` null) `count` blocks (leaves, on a fast
/// chain) would not be once proven: each leaf is proven on its own and carries every log of its
/// blocks, past one answer's limit. Without proof jobs, segments follow the chain, and the timing
/// is given while they are recent. `g`: the leaf and segment sizes.
function notInSegment(b, from, to, serverMessage, head, count, g) {
  const start = b ?? from;
  let retry = null;
  if (start > from) retry = { from, to: start - 1 };
  else if (to - from + 1 > EXAMPLE_BLOCKS) retry = { from: start, to: start + EXAMPLE_BLOCKS - 1 };
  const segmentEnd = head ? head.segmentEnd : null;
  const following = !!head && !head.jobs && segmentEnd != null;
  const current = following && head.tip != null && head.tip - segmentEnd <= SEGMENT_MAX_LAG;
  const one = g.leaf === 1;
  let how = `Willow proves blocks in final ${g.window}-block segments, and ${one ? 'blocks' : `${n(g.leaf)}-block leaves`} outside them one at a time.`;
  if (current) how = `Willow groups blocks into final ${g.window}-block segments about 15 to 30 minutes behind the chain (the newest ends at block ${segmentEnd}).`;
  else if (following) {
    const behind = head.tip != null && head.tip > segmentEnd ? `, ${n(head.tip - segmentEnd)} blocks behind the chain` : '';
    how = `Willow groups proven blocks into final ${g.window}-block segments; the newest ends at block ${segmentEnd}${behind}.`;
  }
  const where =
    b != null
      ? `For ${blocksText(b, to)} that is more than one answer can carry.`
      : `Once proven, ${n(count)} ${one ? 'blocks' : 'leaves'} of this range would be outside a segment, more than one answer can carry.`;
  let next = 'Verify fewer blocks.';
  if (retry) next = `Verify ${blocksText(retry.from, retry.to)} now, or ${current ? 'this range again in a few minutes' : 'fewer blocks'}.`;
  else if (current) next = 'Verify this range again in a few minutes.';
  const whole = one ? 'A block outside a segment is proven with every log it holds' : 'A leaf outside a segment is proven with every log of its blocks';
  return new CheckError(`${how} ${whole}, not only the logs your filter matches. ${where} ${next}`, {
    title: b != null ? `Could not verify: ${b === to ? `block ${b} is` : `blocks from ${b} are`} not in a final segment` : 'Could not verify: too many blocks outside a final segment',
    tone: 'warn',
    retry,
    stepNote: serverMessage,
  });
}

/// On-demand proving is unavailable: say so, and offer blocks that are already proven.
function provingUnavailable(from, to, head, serverMessage) {
  let retry = provenPart(from, to, head);
  if (!retry) {
    const ex = exampleRange(head);
    if (ex && !(ex.from === from && ex.to === to)) retry = ex;
  }
  return new CheckError(
    `Some blocks in ${blocksText(from, to)} have no proof yet, and Willow cannot prove blocks on request right now.` +
      (retry ? ` ${blocksAre(retry)} already proven: verify ${retry.from === retry.to ? 'it' : 'them'} now, or try this range again later.` : ' Try again later.'),
    { title: 'Could not verify: Willow cannot prove new blocks right now', tone: 'warn', retry, stepNote: serverMessage },
  );
}

/// The error for "no proof yet for block `b`", with a range worth trying instead: the proven
/// start of the request, or (at the chain tip, without proof jobs) the same span ending at the
/// newest proven block. When the proof status shows no proven block near the tip, it offers no tip
/// retry: it says so. With proof jobs, Willow says this only for blocks it cannot prove on request.
async function notProven(b, from, to, endpoint, head) {
  let tip = head && head.tip;
  if (tip == null) {
    try {
      tip = quantity((await rpc(endpoint, 'eth_blockNumber', [])).result, 'eth_blockNumber');
    } catch {}
  }
  if (isPublic(endpoint)) {
    const retry = b > from ? { from, to: b - 1 } : provenPart(from, to, head);
    const hint = retry ? ` ${blocksAre(retry)} proven: verify ${retry.from === retry.to ? 'it' : 'them'} instead.` : '';
    return new CheckError(
      `Willow has not proven block ${b} yet. Without an API key, the page verifies blocks Willow has already proven; with one, Willow proves new blocks when you ask.${hint}`,
      { title: `Could not verify: block ${b} is not proven yet`, tone: 'warn', retry },
    );
  }
  if (head && head.jobs) {
    const retry = b > from ? { from, to: b - 1 } : provenPart(from, to, head);
    const hint = retry ? ` ${blocksAre(retry)} proven: verify ${retry.from === retry.to ? 'it' : 'them'} instead.` : '';
    if (tip != null && b > tip) {
      return new CheckError(`Willow's log store has blocks up to ${tip}, and block ${b} is newer. Try again in a minute.${hint}`, {
        title: `Could not verify: Willow does not have block ${b} yet`,
        tone: 'warn',
        retry,
      });
    }
    return new CheckError(`Willow has no proof for block ${b} and cannot prove it on request: it is outside the blocks Willow's log store holds.${hint}`, {
      title: `Could not verify: Willow has no proof for block ${b}`,
      tone: 'warn',
      retry,
    });
  }
  const newest = head && head.nearTip;
  const stalled = !!head && newest == null;
  const nearTip = tip != null && b + 256 >= tip;
  let retry = null;
  if (b > from) retry = { from, to: b - 1 };
  else if (nearTip && b > 0 && !stalled) {
    const end = newest != null && newest < b ? newest : b - 1;
    retry = { from: Math.max(0, end - (to - from)), to: end };
  }
  const hint = retry ? ` Try ${blocksText(retry.from, retry.to)}.` : '';
  if (nearTip && stalled) {
    return new CheckError(
      `Willow has not proven block ${b}, and it is not serving proofs for recent blocks right now. ${retry ? `You can verify ${blocksText(retry.from, retry.to)} now and try the rest again later.` : 'Try again later.'}`,
      { title: 'Could not verify: Willow is not serving proofs for recent blocks right now', tone: 'warn', retry },
    );
  }
  if (nearTip) {
    const lag = newest != null && newest < b ? ` The newest proven block is ${newest}.` : '';
    return new CheckError(`Willow has not proven block ${b} yet. Block proofs follow the chain by up to a few minutes.${lag}${hint}`, {
      title: `Could not verify: block ${b} is not proven yet`,
      tone: 'warn',
      retry,
    });
  }
  return new CheckError(`Willow has no proof for block ${b}. Blocks from before proving started are not covered yet.${hint}`, {
    title: `Could not verify: Willow has no proof for block ${b}`,
    tone: 'warn',
    retry,
  });
}

// ---------- proofs made on request ----------

/// What a "proof queued" refusal says Willow is proving for this request, or null. Its error.data
/// is read field by field: { status, jobId, jobIds, from, to, estimatedSeconds, retryAfterSeconds }.
function queuedOf(e) {
  const d = e.rpcCode === -32000 ? e.rpcData : null;
  if (!d || typeof d !== 'object' || Array.isArray(d) || (d.status !== 'queued' && d.status !== 'running')) return null;
  const isId = (x) => typeof x === 'string' && JOB_ID.test(x);
  const num = (x) => (Number.isSafeInteger(x) && x >= 0 ? x : null);
  const from = num(d.from);
  const to = num(d.to);
  if (!isId(d.jobId) || from == null || to == null || from > to) return null;
  const jobIds = Array.isArray(d.jobIds) ? [...new Set(d.jobIds.filter(isId))] : [];
  if (!jobIds.includes(d.jobId)) jobIds.push(d.jobId);
  return { status: d.status, jobId: d.jobId, jobIds, from, to, estimated: num(d.estimatedSeconds), retryAfter: num(d.retryAfterSeconds) };
}

/// What a "proof not available yet" refusal for a fast chain's unfinished leaf says, or null: block
/// `from` is in a leaf that ends at block `end`, which the chain has not made yet. Its error.data is
/// read field by field: { status: "waiting", from, segmentEnd, retryAfterSeconds }.
function waitingOf(e) {
  const d = e.rpcCode === -32000 ? e.rpcData : null;
  if (!d || typeof d !== 'object' || Array.isArray(d) || d.status !== 'waiting') return null;
  const num = (x) => (Number.isSafeInteger(x) && x >= 0 ? x : null);
  const from = num(d.from);
  const end = num(d.segmentEnd);
  if (from == null || end == null || end < from) return null;
  return { from, end, retryAfter: num(d.retryAfterSeconds) };
}

/// A proof job from willow_getProofJob, or null when Willow does not show it (a job another key
/// queued) or sends something the page cannot read. Throws when Willow cannot be reached.
async function readJob(endpoint, id, wait) {
  let result;
  try {
    ({ result } = await wait.race(rpc(endpoint, 'willow_getProofJob', [id])));
  } catch (e) {
    if (e.rpcCode !== undefined) return null;
    throw e;
  }
  const num = (x) => (Number.isSafeInteger(x) && x >= 0 ? x : null);
  if (!result || typeof result !== 'object' || result.id !== id) return null;
  const status = ['queued', 'running', 'done', 'failed'].includes(result.status) ? result.status : null;
  const total = num(result.totalBlocks);
  if (!status || total == null) return null;
  return {
    status,
    total,
    proved: Math.min(num(result.provedBlocks) ?? 0, total),
    from: num(result.from),
    to: num(result.to),
    error: typeof result.error === 'string' ? result.error.slice(0, 300) : null,
  };
}

/// Waits while Willow proves the blocks `p` names (from a "proof queued" refusal), polling
/// willow_getProofJob. Returns once every job is done, or, when a job cannot be read, once it is
/// time to ask willow_getLogsProof again. Throws when a job fails, when the user cancels, or
/// WAIT_MAX_MS after the first refusal.
async function waitForProofs(endpoint, p, wait, from, to, head) {
  const every = clamp(p.retryAfter ?? POLL_MAX_S, POLL_MIN_S, POLL_MAX_S) * 1000;
  const reaskAt = Date.now() + clamp(p.retryAfter ?? REASK_MAX_S, POLL_MIN_S, REASK_MAX_S) * 1000;
  wait.show(p, every);
  for (let failures = 0; ; ) {
    await wait.pause(every);
    if (Date.now() - wait.started > WAIT_MAX_MS) throw notReady(p);
    let jobs;
    try {
      jobs = await Promise.all(p.jobIds.slice(0, JOBS_POLLED).map((id) => readJob(endpoint, id, wait)));
      failures = 0;
    } catch (e) {
      if (e.stopped) throw e;
      if (++failures >= POLL_FAILURES_MAX) throw lostContact(e, p);
      wait.update(null, `Could not reach Willow just now. Trying again in ${every / 1000} seconds.`);
      continue;
    }
    const failed = jobs.find((j) => j && j.status === 'failed');
    if (failed) throw jobFailed(failed, p, from, to, head);
    const shown = jobs.filter(Boolean);
    const all = shown.length === p.jobIds.length;
    if (all && shown.every((j) => j.status === 'done')) {
      for (const id of p.jobIds) wait.done.add(id);
      return;
    }
    wait.update(shown);
    if (!all && Date.now() >= reaskAt) return;
  }
}

function jobFailed(job, p, from, to, head) {
  const blocks = blocksText(job.from ?? p.from, job.to ?? p.to);
  const retry = provenPart(from, to, head);
  return new CheckError(
    `Willow's proof job for ${blocks} stopped with this error: ${sentence(job.error || 'no reason given')} Nothing was verified. ` +
      `Verifying this range again asks Willow to prove the blocks again.${retry ? ` ${blocksAre(retry)} already proven and can be verified now.` : ''}`,
    { title: `Could not verify: Willow could not prove ${blocks}`, tone: 'warn', retry, stepNote: 'The proof job failed' },
  );
}

const notReady = (p) =>
  new CheckError(`Willow has not finished proving ${blocksText(p.from, p.to)} after two hours. It keeps proving them; verify this range again later.`, {
    title: 'Could not verify: the proofs are not ready yet',
    tone: 'warn',
    stepNote: 'Stopped waiting after two hours',
  });

const stoppedWaiting = (p, forChain = false) =>
  new CheckError(
    forChain
      ? `You stopped waiting. Willow proves ${blocksText(p.from, p.to)} once the chain has made them; verify this range again later.`
      : `You stopped waiting. Willow keeps proving ${p ? blocksText(p.from, p.to) : 'the blocks'}; verify this range again later, and once they are proven it runs without waiting.`,
    { title: 'Could not verify: you stopped waiting', tone: 'warn', stepNote: 'Stopped waiting', stopped: true },
  );

const lostContact = (e, p) =>
  new CheckError(`The page lost contact with Willow while waiting for ${blocksText(p.from, p.to)}: ${sentence(e.message)} Willow keeps proving them; verify this range again later.`, {
    title: 'Could not verify: lost contact with Willow while waiting',
    tone: 'warn',
  });

/// A check waiting for Willow to prove its blocks: the card that shows it, the pauses between
/// polls, and Cancel, which ends the wait at once (whatever request is still out is ignored).
class Waiting {
  constructor() {
    this.started = Date.now();
    this.p = null;
    this.jobs = null;
    // Jobs seen done, and answers since that still said "queued" only for those.
    this.done = new Set();
    this.repeats = 0;
    this.chainWait = null;
    this.ended = new Promise(
      (_, reject) => (this.cancel = () => reject(this.p || !this.chainWait ? stoppedWaiting(this.p) : stoppedWaiting({ from: this.chainWait.from, to: this.chainWait.end }, true))),
    );
    this.ended.catch(() => {});
  }

  race(promise) {
    return Promise.race([promise, this.ended]);
  }

  pause(ms) {
    return this.race(sleep(ms));
  }

  show(p, every) {
    // Asked again for the same jobs: keep the progress shown so far.
    const same = this.p && this.p.jobIds.join() === p.jobIds.join();
    this.p = p;
    this.every = every;
    this.readyAt = p.estimated != null ? Date.now() + p.estimated * 1000 : null;
    if (!this.onScreen()) {
      showVerdict('wait', '', '', [el('button', { type: 'button', class: 'secondary', onclick: () => this.cancel() }, 'Cancel')]);
      $('waitProgress').hidden = false;
      $('verdict').scrollIntoView({ block: 'nearest' });
    }
    $('verdictTitle').textContent = `Willow is proving ${blocksText(p.from, p.to)} now`;
    this.update(same ? this.jobs : null);
  }

  /// Shows the estimate and `jobs`' progress (null before the first poll), or `note` instead.
  update(jobs, note) {
    const p = this.p;
    if (jobs) this.jobs = jobs;
    const left = this.readyAt == null ? null : this.readyAt - Date.now();
    const eta = left == null ? 'Willow did not say how long this takes.' : left > 0 ? `Ready in about ${minutesText(left)}.` : 'Taking longer than estimated.';
    const one = p.from === p.to;
    $('verdictText').textContent =
      `${eta} ${one ? 'This block has' : 'These blocks have'} no proof yet, so Willow is proving ${one ? 'it' : 'them'} for this verification. ` +
      `The page asks about the proof every ${this.every / 1000} seconds and verifies as soon as it is ready.`;
    let line;
    let total = 0;
    let proved = 0;
    if (note) line = note;
    else if (!jobs) line = p.status === 'running' ? 'Proving has started.' : 'Queued. Proving has not started yet.';
    else if (!jobs.length) line = 'Another request queued this proof, so there is no progress to show here.';
    else if (jobs.every((j) => j.status === 'queued')) line = 'Queued. Proving has not started yet.';
    else {
      for (const j of jobs) {
        total += j.total;
        proved += j.proved;
      }
      line = `${n(proved)} of ${n(total)} block${total === 1 ? '' : 's'} proven${jobs.length < p.jobIds.length ? ' (progress shown for some of the blocks)' : ''}.`;
    }
    $('waitText').textContent = line;
    const bar = $('waitBar');
    if (total) {
      bar.max = total;
      bar.value = proved;
    } else bar.removeAttribute('value');
    step('proof', 'active', `Waiting for Willow to prove ${blocksText(p.from, p.to)}. ${line}`);
  }

  /// Waits while the chain makes the rest of the leaf block `w.from` is in (a fast chain's leaf is
  /// proven once all its blocks exist), then returns so the page asks Willow again. Throws on
  /// Cancel, or WAIT_MAX_MS after the first refusal.
  async forChain(w, ch) {
    if (Date.now() - this.started > WAIT_MAX_MS) throw notReady({ from: w.from, to: w.end });
    this.chainWait = w;
    const every = clamp(w.retryAfter ?? REASK_MAX_S, POLL_MIN_S, REASK_MAX_S) * 1000;
    if (!this.onScreen()) {
      showVerdict('wait', '', '', [el('button', { type: 'button', class: 'secondary', onclick: () => this.cancel() }, 'Cancel')]);
      $('waitProgress').hidden = false;
      $('verdict').scrollIntoView({ block: 'nearest' });
    }
    $('verdictTitle').textContent = `Waiting for ${ch.name} to make block ${w.end}`;
    $('verdictText').textContent =
      `Block ${w.from} is in a leaf of blocks that ends at block ${w.end}, and ${ch.name} has not made that block yet. ` +
      `Willow proves a leaf once all its blocks exist. The page asks Willow again in ${Math.round(every / 1000)} seconds and verifies once the proofs are ready.`;
    $('waitText').textContent = `Waiting for block ${w.end}.`;
    $('waitBar').removeAttribute('value');
    step('proof', 'active', `Waiting for ${ch.name} to make block ${w.end}.`);
    await this.pause(every);
  }

  /// Whether the card is showing (a download ask can replace it between two waits).
  onScreen() {
    return !$('result').hidden && $('verdict').classList.contains('wait');
  }

  /// The proofs are ready: the card goes, and the check carries on.
  end() {
    if (this.onScreen()) $('result').hidden = true;
  }
}

/// The fields both checks use, or a CheckError for the form. The node is optional: without one,
/// the chain's two public nodes confirm the block.
function readCommon() {
  const ch = chosen();
  const endpoint = $('endpoint').value.trim();
  const node = $('node').value.trim();
  const other = $('compare').value.trim();
  if (!httpUrl(endpoint)) throw new CheckError('Enter a Willow RPC endpoint URL.');
  const named = endpointChain(endpoint);
  if (named != null && named !== ch.id) {
    const c = chainOf(named);
    throw new CheckError(`The endpoint URL is for chain ${named}${c ? ` (${c.name})` : ''}, but ${ch.name} is chosen. Choose that chain, or change the number after /rpc/ in the URL.`);
  }
  if (node && !httpUrl(node)) throw new CheckError(`Your ${ch.name} node must be an http(s) URL.`);
  if (other && !httpUrl(other)) throw new CheckError('The other provider must be an http(s) URL.');
  return { ch, endpoint, nodes: nodesFor(ch, node), other };
}

/// The checked kind: logs, transaction, receipt or blockReceipts.
const kindOf = () => (document.querySelector('input[name=kind]:checked') || { value: 'logs' }).value;

async function run(ev) {
  ev && ev.preventDefault();
  return kindOf() === 'logs' ? runLogs() : runItems();
}

/// The nodes that confirm a block: the one entered; else none on a chain the browser confirms
/// against Ethereum itself (null), else the chain's two public nodes.
const nodesFor = (ch, typed) => (typed ? [typed] : TRUSTLESS.has(ch.id) ? null : ch.nodes);
const hostsText = (list) => list.map(hostOf).join(' and ');
/// The block step's label before its result: where the block is asked.
const anchorLabel = (list, what) => (list ? `${what} on ${hostsText(list)}` : `${what} on Ethereum`);
/// The block step's label once its result is in.
function anchorDoneLabel(a, ch, typed, what, verified = 'confirmed on Ethereum') {
  if (a.signer) return `${what} signed by ${ch.name}'s sequencer`;
  if (a.via === 'nodes') return anchorLabel(typed || ch.nodes, what);
  return a.confirmed ? `${what} ${verified}` : `${what} on Ethereum`;
}

/// Block `number`'s hash against `hash`, by the browser against Ethereum (`list` null) or on the
/// nodes of `list`: the result of `confirmOnEthereum` or `confirmOnNodes`. An L2 block Ethereum
/// holds no commitment for yet is asked of the chain's public nodes instead, and says so.
async function confirmAnchor(ch, endpoint, list, number, hash) {
  if (list) return confirmOnNodes(ch, list, number, hash);
  const a = await confirmOnEthereum(endpoint, number, hash, ch);
  if (ch.id === 1 || a.confirmed || a.mismatch.length || a.failed) return a;
  const b = await confirmOnNodes(ch, ch.nodes, number, hash);
  b.pending = a.problems.map((x) => x.problem).join('; ');
  b.fallback = true;
  return b;
}

/// Block `number`'s hash on `node` against `hash`: { ok, time } | { mismatch } | { problem }. A
/// node without the block yet (it is seconds old) is asked again a few times.
async function confirmBlock(ch, node, number, hash) {
  try {
    const chain = quantity((await rpc(node, 'eth_chainId', [])).result, `The chain id from ${hostOf(node)}`);
    if (chain !== ch.id) throw new CheckError(`${hostOf(node)} is not ${ch.a} ${ch.name} node: it answers for chain ${chain}.`);
    for (let attempt = 0; ; attempt++) {
      const { result: header } = await rpc(node, 'eth_getBlockByNumber', [toHex(number), false]);
      if (header && typeof header.hash === 'string') {
        if (header.hash.toLowerCase() !== hash.toLowerCase()) return { mismatch: header.hash };
        const time = typeof header.timestamp === 'string' && /^0x[0-9a-fA-F]{1,12}$/.test(header.timestamp) ? hexNum(header.timestamp) : null;
        return { ok: true, time };
      }
      if (attempt >= NODE_RETRIES) throw new CheckError(`${hostOf(node)} does not have block ${number} yet.`);
      await sleep(NODE_RETRY_MS);
    }
  } catch (e) {
    return { problem: e.message };
  }
}

/// Block `number`'s hash on each node of `list`, asked at once, against `hash`: { via: 'nodes', ok:
/// [host], mismatch: [{ host, hash }], problems: [{ host, problem }], confirmed, time, public }.
/// Confirmed when a node has it and none has another block there; `time` is the block's time on a
/// node that has it; `public` when the nodes are the chain's public ones, not the user's.
async function confirmOnNodes(ch, list, number, hash) {
  const out = { via: 'nodes', ok: [], mismatch: [], problems: [], time: null, public: list === ch.nodes };
  const results = await Promise.all(list.map((node) => confirmBlock(ch, node, number, hash)));
  results.forEach((r, i) => {
    const host = hostOf(list[i]);
    if (r.ok) {
      out.ok.push(host);
      out.time = out.time ?? r.time;
    } else if (r.mismatch) out.mismatch.push({ host, hash: r.mismatch });
    else out.problems.push({ host, problem: r.problem });
  });
  out.confirmed = out.ok.length > 0 && !out.mismatch.length;
  return out;
}

/// What the nodes said about block `number`, or what the browser verified on Ethereum, for its step.
function anchorNote(a, number, hash) {
  if (a.via === 'ethereum') return a.note;
  if (a.mismatch.length) return a.mismatch.map((m) => `Proven hash ${hash}, ${m.host} has ${m.hash}.`).join(' ');
  const parts = a.ok.length ? [`Block ${number} hash ${hash} matches on ${a.ok.join(' and ')}.`] : [];
  for (const p of a.problems) parts.push(sentence(fromHost(p)));
  if (a.public && a.ok.length) parts.push(a.pending ? `They are run by others than Willow. Your browser could not verify this block against Ethereum yet: ${sentence(a.pending)}` : 'They are run by others than Willow; your browser does not verify this block against Ethereum yet.');
  return parts.join(' ');
}
/// A node's problem, naming the node unless the message already starts with it.
const fromHost = (p) => (p.problem.startsWith(p.host) ? p.problem : `${p.host}: ${p.problem}`);
const anchorState = (a) => (a.mismatch.length || a.failed ? 'fail' : a.confirmed && !a.fallback ? 'done' : 'warn');
/// How the block was confirmed, for the facts list.
const confirmedFact = (a, ch) =>
  a.signer
    ? `signed by ${a.ok[0]}, whose key Ethereum names; not on Ethereum yet`
    : a.via === 'ethereum' && ch.id !== 1
      ? a.level === 'final'
        ? 'settled on Ethereum'
        : `posted to Ethereum by ${claimer(ch)}, settles in about ${ch.finalDays} days`
      : `confirmed on ${a.ok.join(' and ')}${a.level === 'signed' ? ', not final yet' : ''}`;
const mismatchHosts = (a) => a.mismatch.map((m) => m.host).join(' and ');
const problemsText = (a) => (a.via === 'ethereum' ? a.problems.map((p) => p.problem).join('; ') : a.problems.map(fromHost).join('; '));
/// How the block was confirmed, as the end of a verdict.
function confirmedText(a, ch, what = 'The block', blockTime = null) {
  const line = ch.id === 1 ? '' : ` ${chainFinalLine(ch, blockTime)}`;
  if (a.via === 'ethereum' && ch.id !== 1 && a.level === 'signed') return `Your browser verified that ${what.toLowerCase()} is part of ${ch.name} as its sequencer signed it, the sequencer's key read from Ethereum; it is not on Ethereum yet.${line}`;
  if (a.via === 'ethereum' && ch.id !== 1 && a.level === 'final') return `Your browser verified against Ethereum that ${what.toLowerCase()} is part of ${ch.name}, and settled on Ethereum.`;
  if (a.via === 'ethereum' && ch.id !== 1) return `Your browser verified against Ethereum that ${what.toLowerCase()} is part of ${ch.name}: posted to Ethereum by ${claimer(ch)}, it settles in about ${ch.finalDays} days.${line}`;
  if (a.via === 'ethereum') return `Your browser verified against Ethereum's validators that ${what.toLowerCase()} is part of Ethereum${a.level === 'final' ? ', and final' : "; it is not final yet"}.`;
  if (a.public) return `${what} was confirmed by ${a.ok.length > 1 ? `two public ${ch.name} nodes` : `a public ${ch.name} node`} run by others (${a.ok.join(' and ')}), not yet against Ethereum.${line}`;
  return `${what} was confirmed on ${a.ok.join(' and ')}.${line}`;
}
/// Willow's data to confirm block `number` on Ethereum failed verification: the end of a verdict.
const failedText = (a, number) => sentence(`Willow's data to confirm block ${number} on Ethereum failed verification: ${a.detail}`);
/// A proven or reported other block at `number`: the verdict's title and text.
function mismatchTitle(a, number) {
  return a.via === 'ethereum' ? `Not verified: ${a.mismatch[0].host.replace(/, as [^,]*,$/, '')} has another block ${number}` : `Not verified: block ${number} is not on the chain ${mismatchHosts(a)} ${a.mismatch.length === 1 ? 'follows' : 'follow'}`;
}
function mismatchText(a, number, what) {
  const replaced = `If block ${number} is only seconds old, it may have been replaced by a reorganization; try again in a minute.`;
  const host = a.mismatch.length ? a.mismatch[0].host : '';
  if (a.via === 'ethereum' && host.endsWith(', as its sequencer signed it,')) return `${what}, but ${host.replace(/, as [^,]*,$/, '')}'s sequencer signed another block ${number} (${a.mismatch[0].hash}): your browser verified its signature, its key read from Ethereum. ${replaced}`;
  if (a.via === 'ethereum' && host !== 'Ethereum') return `${what}, but your browser verified against Ethereum that ${host.replace(/, as [^,]*,$/, '')}'s block ${number} has another hash (${a.mismatch[0].hash}). ${replaced}`;
  if (a.via === 'ethereum') return `${what}, but your browser verified against Ethereum's validators that Ethereum's block ${number} has another hash (${a.mismatch[0].hash}). ${replaced}`;
  return `${what}, but it is for a block whose hash differs from ${mismatchHosts(a)}'s. ${replaced}`;
}

// ---------- Ethereum's light client ----------

/// Where this page's light client starts: a finalized beacon block root, at least two sync-committee
/// periods older than the build so Willow's daily anchor is signed by a committee it reaches
/// (TRUSTLESS-ANCHOR.md). It is as much Willow's as the page's code: build.sh replaces this line
/// at each build with a fresh one, read from two beacon APIs (checkpoint.py).
const ETH_CHECKPOINT = { root: '0x44d5cf3ea4de47f80a300df272afa97b75c218ecefa4f7d87f83d085c41d80e1', slot: 15450112 };
/// Chains whose blocks the browser confirms against Ethereum itself; the others still ask public nodes.
const TRUSTLESS = new Set([1, 8453, 10, 130, 57073, 42161, 4663]);
// Arbitrum Nitro chains: their claims on Ethereum are BoLD assertions by bonded validators.
const NITRO = new Set([42161, 4663]);
/// Who put an L2's claim on Ethereum, what stands behind it until it is final, and how the page
/// names it. Robinhood Chain's validators are on a list its operator keeps (its bond is 1 ETH).
const CLAIMERS = { 42161: ['a bonded Arbitrum One validator', "is backed by its validator's bond"], 4663: ["a validator on Robinhood Chain's list", "rests on a validator Robinhood Chain's operator approved"] };
const claimer = (ch) => (CLAIMERS[ch.id] ? CLAIMERS[ch.id][0] : `${ch.name}'s proposer`);
const backing = (ch) => (CLAIMERS[ch.id] ? CLAIMERS[ch.id][1] : `rests on ${ch.name}'s proposer, who posted it`);
/// An L2 block's levels in the page's words: past its claim's challenge period, settled on Ethereum;
/// before, posted to Ethereum by whoever made the claim. Below settled, the chain's own view.
const postedText = (ch) => `Posted to Ethereum by ${claimer(ch)}, settles in about ${ch.finalDays} days`;
function chainFinalLine(ch, blockTime) {
  const young = Number.isFinite(blockTime) && Date.now() / 1000 - blockTime < 20 * 60;
  return `${ch.name} ${young ? 'will treat' : 'treats'} this block as final about 20 minutes in. This page doesn't verify that.`;
}
/// A block on a chain the browser verifies against Ethereum that only public nodes confirmed.
const unanchored = (a, ch) => TRUSTLESS.has(ch.id) && a.via === 'nodes' && a.public;
const claimName = (ch, a) => (NITRO.has(ch.id) ? `assertion ${shortHash(a.assertion)}` : `dispute game ${shortAddr(a.game)}`);
/// The light-client data this browser verified (the bootstrap and each period's update), kept
/// so the next visit downloads only what is new. It is verified again when read, never trusted.
const LIGHT_CLIENT_KEY = 'willow-verify:light-client';
/// A block newer than Ethereum's newest signed one: ask again this many times, this far apart.
const ANCHOR_RETRIES = 3;
const ANCHOR_RETRY_MS = 4000;
/// Light-client updates asked for at once (one per 27-hour period).
const UPDATES_PER_ASK = 16;

const anchorAsk = (msg) => ask({ ...msg, checkpoint: ETH_CHECKPOINT, now: Date.now() / 1000 });

/// The light-client data kept from an earlier visit, verified again in the worker, once per worker.
async function restoreLightClient() {
  const wk = spawnWorker();
  if (wk.lightClient) return;
  wk.lightClient = true;
  let saved = null;
  try {
    saved = localStorage.getItem(LIGHT_CLIENT_KEY);
  } catch {}
  if (saved) await anchorAsk({ type: 'anchor-restore', saved }).catch(() => {});
}

function keepLightClient(saved) {
  try {
    if (saved && saved !== 'null') localStorage.setItem(LIGHT_CLIENT_KEY, saved);
  } catch {}
}

/// Willow's anchor for block `number` (the response body as sent). A block newer than Ethereum's
/// newest signed head is asked for again a few times.
async function anchorAnswer(endpoint, number) {
  for (let attempt = 0; ; attempt++) {
    try {
      return (await rpc(endpoint, 'willow_getBlockAnchor', [toHex(number)])).text;
    } catch (e) {
      const waiting = e.rpcData && typeof e.rpcData === 'object' && e.rpcData.status === 'waiting';
      if (!waiting || attempt >= ANCHOR_RETRIES) throw e;
      await sleep(ANCHOR_RETRY_MS);
    }
  }
}

/// A whole number from the verifier's JSON, or a CheckError.
function wholeNumber(x) {
  if (!Number.isSafeInteger(x) || x < 0) throw new CheckError('the verifier asked for an unexpected period', { kind: 'internal' });
  return x;
}

/// Block `number`'s hash on chain `ch` against `hash`, verified in the browser against Ethereum:
/// Willow's anchor for the block, and the light-client data the verifier asks for to reach the
/// validators who signed it. On an L2 the anchor is the chain's dispute game or assertion on Ethereum.
/// { via: 'ethereum', confirmed, level ('final' | 'signed' | 'committed'), finalized, mismatch,
/// failed, problems, note }.
async function confirmOnEthereum(endpoint, number, hash, ch = CHAINS[0]) {
  const out = { via: 'ethereum', ok: [], mismatch: [], problems: [], confirmed: false, level: null, finalized: null, failed: false, note: '' };
  const lc = withChain(endpoint, 1);
  try {
    await restoreLightClient();
    const answer = await anchorAnswer(endpoint, number);
    let r;
    for (let round = 0; ; round++) {
      const m = await anchorAsk({ type: 'anchor-verify', chainId: ch.id, number, hash, response: answer });
      r = m.result;
      if (r.ok) {
        keepLightClient(m.saved);
        break;
      }
      if (round >= 3) throw new CheckError("Willow's light-client data did not reach the validators who signed this block.");
      if (r.kind === 'need-bootstrap') {
        const b = await rpc(lc, 'willow_lightClientBootstrap', [ETH_CHECKPOINT.root]);
        await anchorAsk({ type: 'anchor-bootstrap', response: b.text });
      } else {
        for (let from = wholeNumber(r.from), left = wholeNumber(r.count); left > 0; ) {
          const k = Math.min(left, UPDATES_PER_ASK);
          const u = await rpc(lc, 'willow_lightClientUpdates', [from, k]);
          const done = await anchorAsk({ type: 'anchor-updates', response: u.text });
          if (done.result.period == null) throw new CheckError(`Willow sent no light-client update for period ${from}.`);
          from += k;
          left -= k;
        }
      }
    }
    out.level = r.level;
    out.finalized = r.finalized;
    out.signers = r.signers ?? null;
    out.head = r.head ?? null;
    out.anchorBlock = r.anchorBlock ?? null;
    out.game = r.game || null;
    out.assertion = r.assertion || null;
    out.signer = r.signer || null;
    if (!r.matches) {
      out.mismatch.push({ host: ch.id === 1 ? 'Ethereum' : r.level === 'signed' ? `${ch.name}, as its sequencer signed it,` : `${ch.name}, as posted to Ethereum,`, hash: r.hash });
      out.note =
        ch.id !== 1 && r.level === 'signed'
          ? `${ch.name}'s sequencer signed another block ${n(number)}, with hash ${r.hash}, not ${hash}: your browser verified its signature, its key read from Ethereum.`
          : `${ch.id === 1 ? 'Ethereum' : ch.name}'s block ${n(number)} has hash ${r.hash}, not ${hash}: your browser verified this against Ethereum.`;
    } else {
      out.confirmed = true;
      out.ok.push(ch.id !== 1 && r.level === 'signed' ? `${ch.name}'s sequencer` : 'Ethereum');
      const claim = `${ch.name}'s claim for its block ${n(r.anchorBlock)} (${claimName(ch, r)})`;
      out.note =
        ch.id !== 1 && r.level === 'signed'
          ? `Signed by ${ch.name}'s sequencer, not on Ethereum yet. Your browser verified the sequencer's signature on block ${n(r.anchorBlock)}, its key read from Ethereum with Ethereum's validators' signatures and the contracts' own code, and found block ${n(number)} with this hash in ${ch.name}'s history from it.`
          : ch.id !== 1
          ? r.level === 'final'
            ? `Settled on Ethereum. Your browser verified, with Ethereum's validators' signatures and the contracts' own code, that ${claim} has passed its challenge period on Ethereum, and found block ${n(number)} with this hash in ${ch.name}'s history from it.`
            : `${postedText(ch)}. Your browser verified ${claim} on Ethereum, with Ethereum's validators' signatures and the contracts' own code, and found block ${n(number)} with this hash in ${ch.name}'s history from it.`
          : r.level === 'final'
            ? `Final on Ethereum. Your browser verified the signatures of Ethereum's validators (${r.signers} of 512) and found block ${n(number)} with this hash in the history of block ${n(r.finalized)}, which they finalized.`
            : `Signed by Ethereum's validators (${r.signers} of 512), not final yet. Your browser verified their signatures and found block ${n(number)} with this hash in the history of the block they signed, ${n(r.head)}.`;
    }
  } catch (e) {
    out.failed = e.kind === 'rejected' || e.kind === 'malformed-input';
    out.detail = e.message;
    out.problems.push({ host: 'Ethereum', problem: anchorProblem(e, ch) });
    out.note = sentence(`Your browser could not confirm this block against Ethereum: ${anchorProblem(e, ch)}`);
  }
  return out;
}

/// Why a block could not be confirmed on Ethereum, by the verifier's kind (its message only shown).
function anchorProblem(e, ch = CHAINS[0]) {
  if (e.kind === 'rejected' || e.kind === 'malformed-input') return `the data from Willow to confirm it failed verification (${e.message})`;
  if (e.kind === 'unknown-fork') return "Ethereum's latest upgrade is newer than this page, which needs an update to follow it";
  if (e.kind === 'stale-checkpoint') return "this page's starting point on Ethereum is more than 30 days old, too old to start from safely; the page needs an update";
  if (e.kind === 'unreachable') return `the data Willow sent cannot be followed from this page's starting point (${e.message})`;
  if (e.kind === 'declined') return `the claim on Ethereum Willow sent is not one this page relies on (${e.message})`;
  if (e.kind === 'disputed') return `the claim on Ethereum that covers this block is being disputed (${e.message})`;
  if (e.kind === 'not-covered') return `Willow's data does not reach this block (${e.message})`;
  if (e.kind === 'stale-head' && ch.id !== 1) return `${e.message}, too old to use; try again in a minute`;
  if (e.kind === 'stale-head') return `${e.message}, too old to say whether a block that is not final yet is part of Ethereum; try again in a minute`;
  if (e.kind === 'internal' || isRuntimeFailure(e)) return `your browser could not finish (${e.message})`;
  return e.message;
}

async function runLogs() {
  $('formError').textContent = '';
  const segmentKeys = $('segmentKeys').value.trim();
  const leafKeys = $('leafKeys').value.trim();
  let filterIn, ch, endpoint, nodes, other;
  try {
    ({ ch, endpoint, nodes, other } = readCommon());
    filterIn = readFilter();
  } catch (e) {
    $('formError').textContent = e.message;
    return;
  }
  remember();
  checkLink = null;
  $('run').disabled = true;
  $('result').hidden = true;
  const list = [
    ['range', 'Block range'],
    ['proof', 'Proof from Willow'],
    ['download', 'SP1 proofs'],
    ['verify', 'Verifying the proofs and completeness'],
  ];
  if (linkMode()) list.shift();
  // The link view shows the proven logs; comparing Willow's plain answer is the full checker's.
  const plainCompare = !linkMode();
  if (plainCompare) list.push(['compare', `Willow's eth_getLogs answer`]);
  if (other) list.push(['other', `eth_getLogs answer from ${hostOf(other)}`]);
  list.push(['anchor', anchorLabel(nodes, 'Last block')]);
  setupSteps(list);
  const t0 = performance.now();
  const facts = [];
  let current = 'range';
  // A narrower range the user chose instead; it runs once this one has stopped.
  let next = null;
  try {
    // 1. Fix the blocks before anything else, so every step checks the same ones.
    step('range', 'active', 'Resolving…');
    const { filter, from, to, tip } = await resolveRange(filterIn, endpoint);
    lastRange = { from, to };
    checkLink = { ch, filter, from, to };
    step('range', 'done', `${blocksText(from, to)}${tip !== null ? ` ("latest" was ${tip})` : ''}`);
    facts.push(['Blocks', `${from} to ${to} (${n(to - from + 1)} block${to === from ? '' : 's'})`]);

    // 2. The answer's proof material. Blocks proven outside a final segment come whole in the
    // answer itself, before any proof download, so a range with many of them asks first. Blocks
    // with no proof yet are proven on request: the page waits for them, then asks again.
    current = 'proof';
    let agreedProofs = 0;
    let agreedBlocks = 0;
    let wait = null;
    let answer, answerSize, answerText;
    let g = geometryOf(null);
    for (;;) {
      step('proof', 'active', "Reading Willow's proof status…");
      const head = await proofStatus(endpoint, ch);
      if (head) g = geometryOf(head);
      const tail = tailEstimate(from, to, head, ch);
      // No download ask while the status shows blocks with no proof: Willow proves them first, or says why not.
      if (tail && tail.answerMb > TAIL_ASK_MB && tail.blocks > agreedBlocks && firstUnproven(from, to, head) == null) {
        step('proof', 'pending', `About ${n(Math.round(tail.answerMb))} MB of whole blocks, then up to ${n(tail.proofs)} proofs. Waiting for you to continue.`);
        const choice = await confirmTail(tail, from, to, ch);
        if (!choice) throw cancelled(g);
        if (choice !== 'go') {
          next = choice;
          step('proof', 'skip', `Verifying ${blocksText(choice.from, choice.to)} instead`);
          return;
        }
        agreedProofs = tail.proofs;
        agreedBlocks = tail.blocks;
      }
      step('proof', 'active', 'Asking Willow…');
      try {
        ({ result: answer, size: answerSize, text: answerText } = await rpc(endpoint, 'willow_getLogsProof', [filter]));
        break;
      } catch (e) {
        const p = queuedOf(e);
        const chainWait = p ? null : waitingOf(e);
        if (chainWait) {
          wait = wait || new Waiting();
          await wait.forChain(chainWait, ch);
          continue;
        }
        if (!p) throw await proofRequestError(e, from, to, endpoint, head);
        wait = wait || new Waiting();
        // Jobs Willow already showed done, yet it still says "queued": stop rather than loop.
        if (p.jobIds.every((id) => wait.done.has(id)) && ++wait.repeats > 2) {
          throw new CheckError(`Willow reports the proofs for ${blocksText(p.from, p.to)} as made, but does not serve them yet. Try again in a few minutes.`, {
            title: 'Could not verify: the proofs are not available yet',
            tone: 'warn',
          });
        }
        await waitForProofs(endpoint, p, wait, from, to, head);
      }
    }
    if (wait) {
      wait.end();
      const after = `ready after ${elapsed(Date.now() - wait.started)}`;
      if (wait.p) facts.push(['Proven on request', `${blocksText(wait.p.from, wait.p.to)}, ${after}`]);
      else facts.push(['Waited for the chain', `${blocksText(wait.chainWait.from, wait.chainWait.end)}, ${after}`]);
    }
    if (!answer || !Array.isArray(answer.parts)) throw new CheckError('willow_getLogsProof sent an answer without parts.', { kind: 'malformed-input' });
    const segs = answer.parts.filter((p) => p && p.type === 'segment').length;
    // Multi-block leaves (a fast chain's leaf proves S blocks); Ethereum's leaves are single blocks.
    const leaves = answer.parts.filter((p) => p && p.type === 'leaf').length;
    const blks = answer.parts.length - segs - leaves;
    const partsText = `${n(segs)} segment${segs === 1 ? '' : 's'}, ${n(blks)} single block${blks === 1 ? '' : 's'}` +
      (leaves ? `, ${n(leaves)} multi-block lea${leaves === 1 ? 'f' : 'ves'}` : '');
    step('proof', 'done', `${partsText} (${bytes(answerSize)})`);

    // 3. The SP1 proofs the parts name. The worker keeps the ones that passed last time.
    current = 'download';
    const ids = [...new Set(answer.parts.map((p) => p && p.proof).filter((x) => typeof x === 'string'))];
    step('download', 'active', 'Starting the verifier…');
    const keys = await startWorker(ch.id, segmentKeys, leafKeys);
    const kept = new Set((await ask({ type: 'cached', ids })).ids);
    const todo = ids.filter((id) => !kept.has(id));
    // Ask unless the estimate already agreed to covers it.
    if (todo.length > LARGE_DOWNLOAD && todo.length > agreedProofs) {
      step('download', 'pending', `${n(todo.length)} proofs to download, about ${n(Math.round(todo.length * PROOF_WIRE_MB))} MB. Waiting for you to continue.`);
      if (!(await confirmDownload(todo.length, g))) throw cancelled(g);
    }
    step('download', 'active', `0 of ${todo.length}`);
    const fresh = {};
    const fetched = await downloadProofs(endpoint, todo, fresh, (k, b) => step('download', null, `${k} of ${todo.length} (${bytes(b)} downloaded)`));
    step('download', 'done', `${ids.length} proof${ids.length === 1 ? '' : 's'}, ${bytes(fetched)} downloaded${kept.size ? `, ${kept.size} already here` : ''}`);
    facts.push(['Downloaded', `${bytes(fetched + answerSize)} (${ids.length} SP1 proof${ids.length === 1 ? '' : 's'}${kept.size ? `, ${kept.size} reused from this session` : ''})`]);

    // 4. Everything that decides "proven" runs in the worker. The proof bytes move there
    // (transferred, not copied); this page keeps no copy.
    current = 'verify';
    step('verify', 'active', 'Verifying…');
    const times = [];
    const transfer = Object.values(fresh).map((b) => b.buffer);
    const v = await ask(
      { type: 'verify', response: answerText, filter: JSON.stringify(filter), ids, proofs: fresh },
      (m) => {
        if (!m.error) times.push(m.ms);
        step('verify', null, `${times.length} of ${ids.length} SP1 proofs valid${times.length ? `, last took ${secs(m.ms)}` : ''}`);
      },
      transfer,
    );
    const r = v.report;
    lastLogs = v.logs;
    step('verify', 'done', `All ${ids.length} SP1 proof${ids.length === 1 ? '' : 's'} valid. ${n(r.logs)} matching log${r.logs === 1 ? '' : 's'}, none missing (${secs(v.ms)}).`);
    facts.push(['Matching logs in the proofs', n(r.logs)]);
    facts.push(['Proof parts', partsText]);
    const avg = times.length ? times.reduce((a, b) => a + b, 0) / times.length : 0;
    facts.push(['SP1 proofs verified', times.length ? `${times.length}, ${secs(avg)} each on average (longest ${secs(Math.max(...times))})` : '0']);
    const custom = customKeys(keys);
    facts.push(['Program keys', `${custom ? 'Custom, entered under Advanced' : 'Published with this page'}: blocks ${keys.leaf.join(', ')}; segments ${keys.segment.join(', ')}`]);

    // 5. Willow's plain answer against the proven logs.
    const diffs = [];
    let mismatch = null;
    let compareProblem = null;
    let unreadable = null;
    if (plainCompare) {
      current = 'compare';
      step('compare', 'active', 'Fetching…');
      try {
        const { text: plain } = await rpc(endpoint, 'eth_getLogs', [filter]);
        const c = (await ask({ type: 'compare', response: plain })).result;
        if (c.exact) step('compare', 'done', `Matches the proven logs exactly (${n(c.matched)} log${c.matched === 1 ? '' : 's'}).`);
        else {
          mismatch = c;
          step('compare', 'fail', `${n(c.missing.length)} proven log${c.missing.length === 1 ? '' : 's'} missing or different, ${n(c.unproven.length)} entr${c.unproven.length === 1 ? 'y' : 'ies'} not proven.`);
          diffs.push(['Willow eth_getLogs', c]);
        }
      } catch (e) {
        if (e.kind === 'internal' || isRuntimeFailure(e)) throw e;
        if (e.kind === 'malformed-input') {
          unreadable = e.message;
          step('compare', 'fail', e.message);
        } else {
          compareProblem = e.message;
          step('compare', 'warn', e.message);
        }
      }
    }
    let otherNote = null;
    if (other) {
      current = 'other';
      step('other', 'active', 'Fetching…');
      try {
        const { text: plain } = await rpc(other, 'eth_getLogs', [filter]);
        const c = (await ask({ type: 'compare', response: plain })).result;
        if (c.exact) step('other', 'done', `Matches the proven logs exactly (${n(c.matched)} log${c.matched === 1 ? '' : 's'}).`);
        else {
          step('other', 'fail', `${n(c.missing.length)} proven log${c.missing.length === 1 ? '' : 's'} missing or different, ${n(c.unproven.length)} not proven.`);
          diffs.push([`${hostOf(other)} eth_getLogs`, c]);
          otherNote = `The answer from ${hostOf(other)} differs from the proven logs; see Differences.`;
        }
      } catch (e) {
        // Another provider is only compared, never part of the proof: nothing it sends stops the check.
        const browser = e.kind === 'internal' || (!(e instanceof CheckError) && isRuntimeFailure(e));
        const why = browser ? `your browser could not finish the comparison (${e.message})` : e.message;
        step('other', 'warn', why);
        otherNote = sentence(`The answer from ${hostOf(other)} could not be compared: ${why}`);
      }
    }

    // 6. The proofs link every block to the last one; confirm that one is canonical.
    current = 'anchor';
    const last = r.lastProvenBlock;
    const lastNum = hexNum(last.number);
    step('anchor', 'active', nodes ? `Asking for block ${lastNum}…` : `Verifying block ${lastNum} against Ethereum's validators…`);
    const anchor = await confirmAnchor(ch, endpoint, nodes, lastNum, last.hash);
    relabel('anchor', anchorDoneLabel(anchor, ch, nodes, 'Last block'));
    step('anchor', anchorState(anchor), anchorNote(anchor, lastNum, last.hash));
    facts.push(['Last proven block', `${lastNum}, hash ${last.hash} (${anchor.confirmed ? confirmedFact(anchor, ch) : 'not confirmed'})`]);
    facts.push(['Time', secs(performance.now() - t0)]);
    facts.push(['Endpoint', hostOf(endpoint)]);
    let checks = null;
    if (linkMode()) {
      const noun = eventNoun(filterIn);
      const blocks = to === from ? 'this block' : `these ${n(to - from + 1)} blocks`;
      const lv = levelOf(anchor, ch);
      checks = [
        r.logs ? { ok: true, text: `None missing in ${blocks}` } : { ok: true, text: `No ${noun}s in ${blocks}` },
        { ...lv, sub: [`Last block: ${n(lastNum)}`, ...(lv.sub || [])] },
        TRUST_ROW,
      ];
      relabel('proof', 'Willow sent a proof for every block in the range.');
      relabel('download', `Your browser downloaded the ${n(ids.length)} SP1 proof${ids.length === 1 ? '' : 's'} they name.`);
      relabel('verify', 'Your browser verified every proof.');
      relabel('anchor', anchorSentence(anchor, ch, 'the last block', lastNum, last.hash));
      for (const key of ['proof', 'download', 'verify', 'anchor']) step(key, null, '');
    }

    // Verdict.
    const range = blocksText(from, to);
    const logsText = `${n(r.logs)} log${r.logs === 1 ? '' : 's'}`;
    const customText = 'The proofs were verified against program keys entered under Advanced, not the keys published with this page, so this result holds only if you trust those keys.';
    let tone, title, text;
    let verified = false;
    if (anchor.mismatch.length) {
      tone = 'bad';
      title = mismatchTitle(anchor, lastNum);
      text = mismatchText(anchor, lastNum, 'The proofs are valid');
    } else if (anchor.failed) {
      tone = 'bad';
      title = 'Not verified: the data to confirm the last block failed verification';
      text = `The proofs are valid and show ${logsText} matching the filter in ${range}, but ${failedText(anchor, lastNum)}`;
    } else if (unreadable) {
      tone = 'bad';
      title = "Not verified: Willow's answer failed verification";
      text = `The proofs are valid and show ${logsText} matching the filter in ${range}, but Willow's plain eth_getLogs answer could not be read: ${sentence(unreadable)}`;
    } else if (mismatch) {
      tone = 'bad';
      title = `Not verified: Willow's eth_getLogs answer differs from the proven logs`;
      text = `The proofs are valid and show ${logsText} matching the filter in ${range}, but the plain answer leaves out or changes ${n(mismatch.missing.length)} and has ${n(mismatch.unproven.length)} that are not proven. See Differences.`;
    } else if (!anchor.confirmed || compareProblem) {
      tone = 'warn';
      title = `Proofs valid${custom ? ' with custom program keys' : ''}, but verification is not complete`;
      text = [
        `All proofs are valid for ${logsText} in ${range}.`,
        compareProblem && sentence(`Willow's eth_getLogs answer could not be compared: ${compareProblem}`),
        !anchor.confirmed && sentence(`The last block could not be confirmed: ${problemsText(anchor)}`),
        custom && customText,
      ]
        .filter(Boolean)
        .join(' ');
    } else {
      verified = true;
      const what = `${logsText} in ${range} ${r.logs === 1 ? 'is' : 'are'} complete and correct`;
      const body = `Every log that matches the filter in these blocks is ${plainCompare ? "in Willow's answer" : 'listed below'}, and none is altered or made up. ${confirmedText(anchor, ch, 'The last block')}`;
      if (custom) {
        tone = 'warn';
        title = `Verified with custom program keys: ${what}`;
        text = `${customText} ${body}`;
      } else if (unanchored(anchor, ch)) {
        tone = 'warn';
        title = 'Proofs valid, but the last block is not verified against Ethereum';
        // A link's checklist says it, in gold.
        text = linkMode() ? '' : body;
      } else if (linkMode()) {
        // A link's range reads like a transaction's card: how many, verified, and its checklist.
        const noun = eventNoun(filterIn);
        tone = 'ok';
        title = r.logs ? [`${n(r.logs)} ${noun}${r.logs === 1 ? '' : 's'} `, el('em', {}, 'verified')] : [`No ${noun}s, `, el('em', {}, 'verified')];
        text = '';
      } else {
        tone = 'ok';
        title = `Verified: ${what}`;
        text = body;
      }
    }
    if (otherNote) text += ` ${otherNote}`;
    showVerdict(tone, title, text, [], checks);
    // A link's facts are in its steps.
    if (!checks) showFacts(facts);
    $('coverage').hidden = false;
    showDiffs(diffs);
    showLogs(lastLogs, verified);
  } catch (e) {
    showFailure(e, current, facts);
  } finally {
    finish(next);
  }
}

/// A check that stopped before its verdict. Decided by kind and error type only: messages can
/// carry text the server chose.
function showFailure(e, current, facts) {
  const browser = e.kind === 'internal' || (!(e instanceof CheckError) && isRuntimeFailure(e));
  const failed = !browser && (e.kind === 'rejected' || e.kind === 'malformed-input');
  step(current, e.tone === 'warn' || browser ? 'warn' : 'fail', e.stepNote || e.message);
  let tone = e.tone || 'bad';
  let title = e.title || (failed ? "Not verified: Willow's answer failed verification" : 'Could not finish verifying');
  let text = failed ? `It is not proven complete and correct. What failed: ${e.message}` : e.message;
  if (browser) {
    tone = 'warn';
    title = 'Your browser could not finish verifying';
    text =
      `Verification stopped inside your browser before it reached a result (the browser reported: ${e.message}). ` +
      `This is not a result about Willow's answer: it is neither verified nor shown to be wrong. ` +
      `Running out of memory is the usual cause: try fewer blocks, close other tabs, or use a desktop browser.`;
  }
  const actions = [];
  if (e.retry) {
    actions.push(
      el('button', { type: 'button', class: 'secondary', onclick: () => retryWith(e.retry) }, `Verify ${blocksText(e.retry.from, e.retry.to)} instead`),
    );
  }
  showVerdict(tone, title, text, actions);
  showFacts(facts);
}

/// After any check: steps never reached are marked, the form is back, and a narrower range the
/// user chose (`next`) runs.
function finish(next) {
  for (const s of Object.values(steps)) if (s.li.dataset.state === 'pending') skipStep(s);
  $('run').disabled = false;
  const shared = linkMode() ? linkShown : checkLink;
  if (!next && shared && !$('result').hidden && $('verdict').classList.contains('ok')) {
    $('verdictActions').append(copyButton('Copy link', () => linkTo(shared), linkMode() ? 'primary' : 'secondary'));
  }
  if (!next && linkMode() && !$('result').hidden && !$('verdictChecks').hidden) {
    const more = el('button', { type: 'button', class: 'r-toggle', 'aria-expanded': 'false', 'aria-controls': 'result' }, 'More');
    const unfold = (open) => {
      document.body.classList.toggle('link-folded', !open);
      more.setAttribute('aria-expanded', String(open));
      more.textContent = open ? 'Less' : 'More';
    };
    more.addEventListener('click', () => unfold(document.body.classList.contains('link-folded')));
    $('verdictActions').append(more);
    $('linkProofActions').replaceChildren(el('button', { type: 'button', class: 'secondary', onclick: () => openChecker(linkShown) }, 'Technical details'));
    $('linkProof').hidden = false;
    unfold(!$('verdict').classList.contains('ok'));
  }
  // A check that did not verify shows its details without a click.
  if (!next && !$('verdict').classList.contains('ok')) for (const c of document.querySelectorAll('.fold, .fold-notes')) setFold(c, true);
  if (next) retryWith(next);
  else $('verdict').scrollIntoView({ block: 'nearest', behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
}

function retryWith({ from, to }) {
  // In the link view, a narrower range is another link: its heading and copied link name its blocks.
  if (linkMode() && linkShown && linkShown.filter) {
    location.assign(linkTo({ ...linkShown, from, to, filter: { ...linkShown.filter, fromBlock: toHex(from), toBlock: toHex(to) } }));
    return;
  }
  try {
    const f = readFilter();
    f.fromBlock = toHex(from);
    f.toBlock = toHex(to);
    $('filter').value = JSON.stringify(f, null, 2);
  } catch {
    return;
  }
  runLogs();
}

// ---------- transaction and receipt proofs ----------

/// Per kind: what is checked, and Willow's plain method for it.
const ITEM_KINDS = {
  transaction: { what: 'transaction', plain: 'eth_getTransactionByHash' },
  receipt: { what: 'receipt', plain: 'eth_getTransactionReceipt' },
  blockReceipts: { what: "block's receipts", plain: 'eth_getBlockReceipts' },
};

const shortHash = (h) => (typeof h === 'string' && h.length > 14 ? `${h.slice(0, 8)}…${h.slice(-6)}` : String(h));

/// A refusal of a transaction or receipt proof, in plain words.
function itemRequestError(e, ch) {
  if (e.rateLimited) return e;
  const msg = e.rpcMessage || '';
  const outside = /blocks? (\d+)(?:-\d+)? (?:is|are) outside the indexed range (\d+)-(\d+)/.exec(msg);
  if (outside) {
    return new CheckError(
      `Willow proves transactions and receipts from the ${ch.name} blocks it holds in full, the newest ${FULL_DAYS} days: right now blocks ${outside[2]} to ${outside[3]}. Block ${outside[1]} is not among them.`,
      { title: `Could not verify: block ${outside[1]} is outside the blocks Willow holds in full`, tone: 'warn', stepNote: msg },
    );
  }
  if (/transaction 0x[0-9a-fA-F]{64} is not in the full-block tier/.test(msg)) {
    return new CheckError(
      `Willow proves transactions from the ${ch.name} blocks it holds in full, the newest ${FULL_DAYS} days, and older ones whose proofs it keeps. This transaction is not among them: it may be older, not yet in a block, or on another chain.`,
      { title: 'Could not verify: Willow does not hold this transaction', tone: 'warn', stepNote: msg },
    );
  }
  const past = /block (\d+) has (\d+) transactions?; there is no transaction (\d+)/.exec(msg);
  if (past) {
    return new CheckError(`Block ${past[1]} has ${n(Number(past[2]))} transaction${past[2] === '1' ? '' : 's'}, numbered from 0. Enter an index below ${past[2]}.`, {
      title: `Could not verify: block ${past[1]} has no transaction ${past[3]}`,
      tone: 'warn',
      stepNote: msg,
    });
  }
  if (e.rpcCode === -32601) return new CheckError('This endpoint does not serve transaction and receipt proofs yet.');
  return new CheckError(e.message);
}

/// One comparison of another answer with the proven one, run in the worker. Returns
/// { result } or { problem } (the answer could not be fetched) or { unreadable } (it could not be read).
async function compareItems(url, method, params, which) {
  let text;
  try {
    ({ text } = await rpc(url, method, params));
  } catch (e) {
    if (isRuntimeFailure(e)) throw e;
    return { problem: e.message };
  }
  try {
    return { result: (await ask({ type: 'items-compare', which, response: text })).result };
  } catch (e) {
    if (e.kind === 'malformed-input') return { unreadable: e.message };
    throw e;
  }
}

/// A comparison in a few words, for its step.
function comparedText(c) {
  if (c.absent) return 'It answered null: it does not report what the proof shows.';
  // Fields, not their instances: a block's receipts report each uncovered field once per receipt.
  const k = new Set(c.unproven.map((f) => f.replace(/\[\d+\]/g, '[]'))).size;
  const uncovered = k ? `; ${n(k)} field${k === 1 ? '' : 's'} it reports ${k === 1 ? 'is' : 'are'} not covered by the proof` : '';
  if (c.exact) return `Matches the proven answer (${n(c.matched)} value${c.matched === 1 ? '' : 's'}${uncovered}).`;
  return `${n(c.differCount)} proven value${c.differCount === 1 ? '' : 's'} reported differently or left out${uncovered}.`;
}

/// The fields a proof does not cover, explained.
function uncoveredText(fields) {
  const plain = fields.map((f) => f.replace(/^\[\]\./, ''));
  const notes = [];
  const nitro = plain.filter((f) => f === 'gasUsedForL1' || f === 'timeboosted');
  if (nitro.length) notes.push(`${nitro.join(' and ')}: the chain's node reports ${nitro.length === 1 ? 'it' : 'them'}, and no block data determines ${nitro.length === 1 ? 'it' : 'them'}`);
  if (plain.includes('logs[].logIndex')) notes.push("each log's logIndex: it counts every earlier log in the block, and this proof carries only the receipt before this one (verify the block's receipts to prove it)");
  const rest = plain.filter((f) => !nitro.includes(f) && f !== 'logs[].logIndex');
  if (rest.length) notes.push(rest.join(', '));
  return notes.length ? `Not covered by this proof: ${notes.join('; ')}.` : 'Every field of this answer is covered by the proof.';
}

async function runItems() {
  $('formError').textContent = '';
  const kind = kindOf();
  const k = ITEM_KINDS[kind];
  const target = $('item').value.trim();
  let ch, endpoint, nodes, other;
  try {
    ({ ch, endpoint, nodes, other } = readCommon());
    if (!target) throw new CheckError(kind === 'blockReceipts' ? 'Enter a block: its number, its hash, or latest.' : 'Enter a transaction hash, or a block and the transaction\'s index, such as 26119978:5.');
  } catch (e) {
    $('formError').textContent = e.message;
    return;
  }
  remember();
  checkLink = kind !== 'blockReceipts' && (TX_HASH.test(target) || AT_INDEX.test(target)) ? { ch, tx: target.toLowerCase() } : null;
  $('run').disabled = true;
  $('result').hidden = true;
  const list = [
    ['proof', 'Proof from Willow'],
    ['verify', 'Verifying the proof against the block hash'],
    ['compare', `Willow's ${k.plain} answer`],
  ];
  if (kind === 'blockReceipts') list.push(['compareTxs', "Willow's eth_getBlockByHash transactions"]);
  if (other) list.push(['other', `${k.plain} answer from ${hostOf(other)}`]);
  list.push(['anchor', anchorLabel(nodes, 'The block')]);
  setupSteps(list);
  const t0 = performance.now();
  const facts = [];
  let current = 'proof';
  lastItems = null;
  try {
    // 1. The request is read in the wasm, which later checks the answer against it.
    step('proof', 'active', 'Asking Willow…');
    let req;
    try {
      req = await ask({ type: 'items-request', chainId: ch.id, kind, target });
    } catch (e) {
      if (e.kind !== 'invalid-request') throw e;
      $('progress').hidden = true;
      $('formError').textContent = e.message;
      return;
    }
    let proofText, proofSize;
    try {
      ({ text: proofText, size: proofSize } = await rpc(endpoint, req.method, req.params));
    } catch (e) {
      throw itemRequestError(e, ch);
    }
    step('proof', 'done', `${req.method}, ${bytes(proofSize)}`);

    // 2. Everything that decides "proven" runs in the worker.
    current = 'verify';
    step('verify', 'active', 'Verifying…');
    const tv = performance.now();
    const v = await ask({ type: 'items-verify', response: proofText });
    const ms = performance.now() - tv;
    const rep = v.report;
    const blockNum = hexNum(rep.block.number);
    const blockHash = rep.block.hash;
    const txHash = rep.transactionHash;
    const whole = kind === 'blockReceipts';
    step(
      'verify',
      'done',
      whole
        ? `The header hashes to block ${blockNum}'s hash, and all ${n(rep.transactions)} transactions and receipts rebuild its roots (${secs(ms)}).`
        : `The header hashes to block ${blockNum}'s hash, and the ${k.what} is under its root (${secs(ms)}).`,
    );
    facts.push(['Chain', `${ch.name} (${ch.id})`]);
    facts.push(['Block', `${blockNum}, hash ${blockHash}`]);
    if (Number.isSafeInteger(rep.block.timestamp)) facts.push(['Block time', new Date(rep.block.timestamp * 1000).toISOString().replace('.000Z', ' UTC').replace('T', ' ')]);
    if (!whole) facts.push(['Transaction', `${txHash}, index ${hexNum(rep.transactionIndex)} in the block`]);
    else facts.push(['Receipts', n(rep.transactions)]);
    facts.push(['Proof', `${req.method}, ${bytes(proofSize)}: ${n(rep.provenTransactions)} transaction${rep.provenTransactions === 1 ? '' : 's'} and ${n(rep.provenReceipts)} receipt${rep.provenReceipts === 1 ? '' : 's'}, verified in ${secs(ms)}`]);

    // 3. Willow's plain answer, field by field.
    const plainParams = whole ? [blockHash] : [txHash];
    const asks = [['compare', k.plain, plainParams, 'answer']];
    if (whole) asks.push(['compareTxs', 'eth_getBlockByHash', [blockHash, true], 'transactions']);
    const diffs = [];
    let mismatch = null;
    let unreadable = null;
    let compareProblem = null;
    for (const [key, method, params, which] of asks) {
      current = key;
      step(key, 'active', 'Fetching…');
      const c = await compareItems(endpoint, method, params, which);
      if (c.problem) {
        compareProblem = compareProblem || `${method}: ${c.problem}`;
        step(key, 'warn', c.problem);
      } else if (c.unreadable) {
        unreadable = unreadable || c.unreadable;
        step(key, 'fail', c.unreadable);
      } else {
        step(key, c.result.exact ? 'done' : 'fail', comparedText(c.result));
        if (!c.result.exact) {
          mismatch = mismatch || { method, c: c.result };
          diffs.push([`Willow ${method}`, c.result]);
        }
      }
    }
    let otherNote = null;
    if (other) {
      current = 'other';
      step('other', 'active', 'Fetching…');
      const notes = [];
      let worst = 'done';
      for (const [, method, params, which] of asks) {
        let c;
        try {
          c = await compareItems(other, method, params, which);
        } catch (e) {
          // Another provider is only compared, never part of the proof: nothing it sends stops the check.
          if (!(e.kind === 'internal' || isRuntimeFailure(e))) throw e;
          c = { problem: `your browser could not finish the comparison (${e.message})` };
        }
        if (c.result && c.result.exact) notes.push(`${method}: ${comparedText(c.result)}`);
        else {
          worst = c.result ? 'fail' : worst === 'fail' ? 'fail' : 'warn';
          if (c.result) diffs.push([`${hostOf(other)} ${method}`, c.result]);
          notes.push(`${method}: ${c.result ? comparedText(c.result) : c.problem || c.unreadable}`);
          otherNote = otherNote || (c.result ? `The answer from ${hostOf(other)} differs from the proven one; see Differences.` : sentence(`The answer from ${hostOf(other)} could not be compared: ${c.problem || c.unreadable}`));
        }
      }
      step('other', worst, notes.join(' '));
    }

    // 4. The proof is about the block whose hash it carries; confirm that block on the user's node.
    current = 'anchor';
    step('anchor', 'active', nodes ? `Asking for block ${blockNum}…` : `Verifying block ${blockNum} against Ethereum's validators…`);
    const anchor = await confirmAnchor(ch, endpoint, nodes, blockNum, blockHash);
    relabel('anchor', anchorDoneLabel(anchor, ch, nodes, 'The block'));
    step('anchor', anchorState(anchor), anchorNote(anchor, blockNum, blockHash));
    facts.push(['Block hash confirmed', anchor.confirmed ? confirmedFact(anchor, ch) : 'no']);
    facts.push(['Time', secs(performance.now() - t0)]);
    facts.push(['Endpoint', hostOf(endpoint)]);

    // Verdict.
    const thing = kind === 'transaction' ? `transaction ${shortHash(txHash)}` : kind === 'receipt' ? `the receipt of transaction ${shortHash(txHash)}` : `the ${n(rep.transactions)} receipts of block ${blockNum}`;
    const uncovered = rep.unproven.length ? ` ${n(rep.unproven.length)} field${rep.unproven.length === 1 ? ' is' : 's are'} not covered by this proof; see What this verifies.` : '';
    let tone, title, text;
    let verified = false;
    if (anchor.mismatch.length) {
      tone = 'bad';
      title = mismatchTitle(anchor, blockNum);
      text = mismatchText(anchor, blockNum, 'The proof is valid');
    } else if (anchor.failed) {
      tone = 'bad';
      title = 'Not verified: the data to confirm the block failed verification';
      text = `The proof of ${thing} is valid, but ${failedText(anchor, blockNum)}`;
    } else if (unreadable) {
      tone = 'bad';
      title = "Not verified: Willow's answer failed verification";
      text = `The proof of ${thing} is valid, but Willow's plain answer could not be read: ${sentence(unreadable)}`;
    } else if (mismatch) {
      tone = 'bad';
      title = `Not verified: Willow's ${mismatch.method} answer differs from the proven one`;
      text = mismatch.c.absent
        ? `The proof shows ${thing} in block ${blockNum}, but Willow's plain answer is null.`
        : `The proof of ${thing} is valid, but Willow's plain answer reports ${n(mismatch.c.differCount)} proven value${mismatch.c.differCount === 1 ? '' : 's'} differently or leaves ${mismatch.c.differCount === 1 ? 'it' : 'them'} out. See Differences.`;
    } else if (!anchor.confirmed || compareProblem) {
      tone = 'warn';
      title = 'Proof valid, but verification is not complete';
      text = [
        `The proof of ${thing} in block ${blockNum} is valid.`,
        compareProblem && sentence(`Willow's plain answer could not be compared: ${compareProblem}`),
        !anchor.confirmed && sentence(`The block could not be confirmed: ${problemsText(anchor)}`),
      ]
        .filter(Boolean)
        .join(' ');
      text += uncovered;
    } else if (unanchored(anchor, ch)) {
      verified = true;
      tone = 'warn';
      title = 'Proof valid, but the block is not verified against Ethereum';
      text = `Every field the proof covers matches block ${blockNum}, and Willow's plain answer reports each of them the same way. ${confirmedText(anchor, ch, 'The block', rep.block.timestamp)}${uncovered}`;
    } else {
      verified = true;
      tone = 'ok';
      title = `Verified: ${thing} ${whole ? 'are complete and correct' : 'is correct'}`;
      text = `Every field the proof covers matches block ${blockNum}, and Willow's plain answer reports each of them the same way. ${confirmedText(anchor, ch, 'The block', rep.block.timestamp)}${uncovered}`;
    }
    if (otherNote) text += ` ${otherNote}`;
    showVerdict(tone, title, text);
    showFacts(facts);
    $('itemCoverage').hidden = false;
    $('itemUncovered').textContent = uncoveredText(rep.unproven);
    showDiffs(diffs);
    showItems(kind, v, verified, ch, blockNum);
  } catch (e) {
    showFailure(e, current, facts);
  } finally {
    finish(null);
  }
}

/// The proven answer: a table of its fields (a transaction or a receipt), or of the block's
/// receipts. Called "proven" only when every check passed.
function showItems(kind, v, verified, ch, blockNum) {
  const head = $('itemTable').tHead;
  const body = $('itemTable').tBodies[0];
  const text = (x) => (typeof x === 'string' ? x : x === null ? 'null' : clip(JSON.stringify(x)));
  const notes = [];
  if (kind === 'blockReceipts') {
    head.replaceChildren(el('tr', {}, ...['Index', 'Transaction', 'Status', 'Gas used', 'Logs'].map((h) => el('th', {}, h))));
    const rows = Array.isArray(v.proven) ? v.proven : [];
    body.replaceChildren(
      ...rows.slice(0, SHOW_RECEIPTS).map((r, i) =>
        el(
          'tr',
          {},
          el('td', {}, String(i)),
          el('td', { class: 'mono' }, r && typeof r.transactionHash === 'string' ? r.transactionHash : ''),
          el('td', {}, r && r.status === '0x1' ? 'success' : r && r.status === '0x0' ? 'reverted' : ''),
          el('td', {}, r && typeof r.gasUsed === 'string' ? n(hexNum(r.gasUsed)) : ''),
          el('td', {}, r && Array.isArray(r.logs) ? n(r.logs.length) : ''),
        ),
      ),
    );
    if (rows.length > SHOW_RECEIPTS) notes.push(`Showing ${SHOW_RECEIPTS} of ${n(rows.length)} receipts. Download the JSON for all of them, with every transaction of the block.`);
    $('itemTitle').textContent = verified ? `Proven receipts of block ${blockNum}` : `Receipts in Willow's proof of block ${blockNum}`;
  } else {
    head.replaceChildren(el('tr', {}, el('th', {}, 'Field'), el('th', {}, 'Value')));
    const fields = v.proven && typeof v.proven === 'object' ? Object.entries(v.proven) : [];
    const rows = fields.map(([key, value]) => el('tr', {}, el('td', { class: 'mono' }, key), el('td', { class: 'value' }, el('div', { class: 'cell' }, key === 'logs' && Array.isArray(value) ? `${n(value.length)} log${value.length === 1 ? '' : 's'}: ${text(value)}` : text(value)))));
    for (const f of v.report.unproven) rows.push(el('tr', {}, el('td', { class: 'mono' }, f), el('td', { class: 'value uncovered' }, 'not covered by this proof')));
    body.replaceChildren(...rows);
    $('itemTitle').textContent = verified ? `Proven ${kind === 'receipt' ? 'receipt' : 'transaction'}` : `${kind === 'receipt' ? 'Receipt' : 'Transaction'} in Willow's proof`;
  }
  if (!verified) notes.unshift('Verification did not pass, so these values are not verified.');
  $('itemMore').textContent = notes.join(' ');
  const name = `willow-${verified ? 'proven' : 'unverified'}-${kind === 'blockReceipts' ? 'receipts' : kind}-${ch.id}-${blockNum}.json`;
  lastItems = { name, json: kind === 'blockReceipts' ? { receipts: v.proven, transactions: v.transactions } : v.proven };
  $('itemCard').hidden = false;
}

const clip = (s) => (s.length > 600 ? `${s.slice(0, 600)}…` : s);

function downloadItems() {
  if (!lastItems) return;
  saveJson(lastItems.json, lastItems.name);
}

// ---------- results ----------

function showVerdict(tone, title, text, actions = [], checks = null) {
  $('result').hidden = false;
  $('verdict').className = `verdict ${tone}`;
  $('verdictTitle').replaceChildren(...[title].flat());
  $('verdictText').textContent = text;
  $('verdictChecks').replaceChildren(...(checks ? checkRows(checks) : []));
  $('verdictChecks').hidden = !checks;
  $('verdictActions').replaceChildren(...actions);
  $('waitProgress').hidden = true;
  $('coverage').hidden = true;
  $('itemCoverage').hidden = true;
  $('details').hidden = true;
  $('diff').hidden = true;
  $('logsCard').hidden = true;
  $('itemCard').hidden = true;
}

function showFacts(facts) {
  if (!facts.length) return;
  $('facts').replaceChildren(...facts.flatMap(([k, v]) => [el('dt', {}, k), el('dd', {}, v)]));
  $('details').hidden = false;
}

function showDiffs(diffs) {
  if (!diffs.length) return;
  if (!diffs[0][1].missing) return showItemDiffs(diffs);
  const lines = [];
  for (const [who, c] of diffs) {
    lines.push(`${who}: ${c.matched} matched`);
    for (const l of c.missing.slice(0, 20)) lines.push(`  missing or different: block ${hexNum(l.blockNumber)} log ${hexNum(l.logIndex)} ${l.address}`);
    if (c.missing.length > 20) lines.push(`  … and ${c.missing.length - 20} more missing`);
    for (const u of c.unproven.slice(0, 20)) lines.push(`  not proven: ${JSON.stringify(u)}`);
    if (c.unproven.length > 20) lines.push(`  … and ${c.unproven.length - 20} more not proven`);
  }
  $('diffText').textContent = '"Missing or different" means a proven log the answer leaves out or reports with other values. "Not proven" means an entry in the answer that is not one of the proven logs.';
  $('diffBody').textContent = lines.join('\n');
  $('diff').hidden = false;
}

/// Comparisons of transaction and receipt answers with the proven ones.
function showItemDiffs(diffs) {
  const lines = [];
  for (const [who, c] of diffs) {
    if (c.absent) {
      lines.push(`${who}: null, where the proof shows a value`);
      continue;
    }
    lines.push(`${who}: ${c.matched} values matched`);
    for (const d of c.differ) lines.push(`  differs: ${d}`);
    if (c.differCount > c.differ.length) lines.push(`  … and ${c.differCount - c.differ.length} more`);
    for (const u of c.unproven) lines.push(`  not covered by the proof: ${u}`);
    for (const o of c.omitted) lines.push(`  left out, as some nodes do: ${o}`);
  }
  $('diffText').textContent = '"Differs" is a proven value the answer reports differently or leaves out. Fields not covered by the proof are listed, never counted against the answer.';
  $('diffBody').textContent = lines.join('\n');
  $('diff').hidden = false;
}

/// The logs from the proofs. Called "proven" only when every check passed.
function showLogs(logs, verified) {
  lastVerified = verified;
  const linked = linkMode();
  const ch = chosen();
  const body = $('logs').tBodies[0];
  const head = $('logs').tHead.rows[0].cells;
  head[1].textContent = linked ? 'Transaction' : 'Tx';
  head[3].textContent = linked ? 'Contract' : 'Address';
  head[4].textContent = linked ? 'Event' : 'First topic';
  // A link that names one contract or one event says so above the list, not on every row.
  $('logs').classList.toggle('one-address', linked && !!linkShown && typeof linkShown.filter.address === 'string');
  const t0 = linked && linkShown && linkShown.filter.topics ? linkShown.filter.topics[0] : null;
  $('logs').classList.toggle('one-event', typeof t0 === 'string');
  const txCell = (l) => {
    const at = `${hexNum(l.blockNumber)}:${hexNum(l.transactionIndex)}`;
    if (!linked || !verified) return String(hexNum(l.transactionIndex));
    return el('a', { href: linkTo({ ch, tx: at }), title: `Verify transaction ${hexNum(l.transactionIndex)} of block ${hexNum(l.blockNumber)}` }, 'Verify →');
  };
  body.replaceChildren(
    ...logs.slice(0, SHOW_LOGS).map((l) =>
      el(
        'tr',
        {},
        el('td', {}, linked ? n(hexNum(l.blockNumber)) : String(hexNum(l.blockNumber))),
        el('td', {}, txCell(l)),
        el('td', {}, String(hexNum(l.logIndex))),
        el('td', { class: 'mono', title: l.address }, linked ? shortAddr(l.address) : l.address),
        el('td', { class: linked && EVENTS[lower(l.topics[0])] ? '' : 'mono' }, (linked && EVENTS[lower(l.topics[0])]) || l.topics[0] || '(none)'),
      ),
    ),
  );
  const noun = linked && linkShown ? eventNoun(linkShown.filter) : null;
  $('logsTitle').textContent = !verified ? "Logs in Willow's proofs" : noun ? `The ${noun}s` : 'Proven logs';
  const notes = [];
  if (!verified) notes.push('Verification did not pass, so these logs are not verified.');
  if (logs.length > SHOW_LOGS) notes.push(`Showing ${SHOW_LOGS} of ${n(logs.length)}. Download the JSON for all of them.`);
  if (!logs.length) notes.push(verified ? 'No logs match the filter in these blocks. That, too, is proven.' : 'The proofs show no logs matching the filter in these blocks.');
  $('logsMore').textContent = notes.join(' ');
  $('logsCard').hidden = false;
}

function downloadLogs() {
  if (!lastLogs || !lastRange) return;
  saveJson(lastLogs, `willow-${lastVerified ? 'proven' : 'unverified'}-logs-${lastRange.from}-${lastRange.to}.json`);
}

function saveJson(value, name) {
  const blob = new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' });
  const a = el('a', { href: URL.createObjectURL(blob), download: name });
  document.body.append(a);
  a.click();
  setTimeout(() => {
    URL.revokeObjectURL(a.href);
    a.remove();
  }, 0);
}

// ---------- links: a check anyone can open ----------

const TX_HASH = /^0x[0-9a-fA-F]{64}$/;
const ADDRESS = /^0x[0-9a-fA-F]{40}$/;
const AT_INDEX = /^(\d{1,15}):(\d{1,6})$/;
// Events named in the link view, by first topic.
const TRANSFER = '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef';
const WETH_DEPOSIT = '0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c';
const WETH_WITHDRAWAL = '0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65';
// A swap router's fill: OrderFilled(address indexed sender, address indexed recipient, address
// tokenIn, address tokenOut, uint256 amountIn, uint256 amountOut, uint256 feeAmount, address target),
// the zero address for ETH; amountIn includes the fee, which is in ETH for a swap to ETH.
const ORDER_FILLED = '0xfb6c56e214cadb4768595d6c38ec36861d571a297ad429eb15de3b9f2f7fe09f';
const ZERO = '0x0000000000000000000000000000000000000000';
const EVENTS = {
  [ORDER_FILLED]: 'Order filled',
  [TRANSFER]: 'Transfer',
  [WETH_DEPOSIT]: 'Wrap (WETH Deposit)',
  [WETH_WITHDRAWAL]: 'Unwrap (WETH Withdrawal)',
  '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925': 'Approval',
  '0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822': 'Swap (Uniswap v2)',
  '0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67': 'Swap (Uniswap v3)',
  [SWAP_TOPIC]: 'Swap (Uniswap v4)',
};
// Tokens named per transfer, at most this many per transaction (two calls each, to a public node).
const TOKENS_NAMED = 12;
// Well-known tokens per chain, named here: any contract can call itself "USDC", so a token not on
// this list is shown with its address, its name marked as the contract's own.
const KNOWN_TOKENS = {
  1: { '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48': ['USDC', 6], '0xdac17f958d2ee523a2206206994597c13d831ec7': ['USDT', 6], '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2': ['WETH', 18], '0x6b175474e89094c44da98b954eedeac495271d0f': ['DAI', 18], '0x2260fac5e5542a773aa44fbcfedf7c193bc2c599': ['WBTC', 8] },
  8453: { '0x833589fcd6edb6e08f4c7c32d4f71b54bda02913': ['USDC', 6], '0x4200000000000000000000000000000000000006': ['WETH', 18], '0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf': ['cbBTC', 8], '0x50c5725949a6f0c72e6c4a641f24049a917db0cb': ['DAI', 18], '0xd9aaec86b65d86f6a7b5b1b0c42ffa531710b6ca': ['USDbC', 6] },
  42161: { '0xaf88d065e77c8cc2239327c5edb3a432268e5831': ['USDC', 6], '0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9': ['USD₮0', 6], '0x82af49447d8a07e3bd95bd0d56f35241523fbab1': ['WETH', 18], '0x912ce59144191c1204e64559fe8253a0e49e6548': ['ARB', 18], '0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f': ['WBTC', 8] },
  10: { '0x0b2c639c533813f4aa9d7837caf62653d097ff85': ['USDC', 6], '0x4200000000000000000000000000000000000006': ['WETH', 18], '0x4200000000000000000000000000000000000042': ['OP', 18], '0x94b008aa00579c1307b0ef2c499ad98a8ce58e58': ['USDT', 6] },
  130: { '0x078d782b760474a361dda0af3839290b0ef57ad6': ['USDC', 6], '0x4200000000000000000000000000000000000006': ['WETH', 18] },
  57073: { '0xf1815bd50389c46847f0bda824ec8da914045d14': ['USDC.e', 6], '0x4200000000000000000000000000000000000006': ['WETH', 18] },
};
// Swap routers whose OrderFilled events (see ORDER_FILLED) the receipt shows as the settlement:
// any contract can emit that event with any amounts, so only these are read.
const FILL_ROUTERS = new Set(['0xb1f3a7b816b0681188f54dfa400991b93adf00ed']);
// The link view's own state: the check shown, and what its buttons save or copy.
let linkShown = null;

/// The chain a link names, by id or by name.
function chainNamed(v) {
  const s = String(v ?? '').trim().toLowerCase();
  return (/^\d+$/.test(s) && chainOf(Number(s))) || CHAINS.find((c) => c.slugs.includes(s)) || null;
}

/// A block number in a link, decimal or 0x.
function blockParam(v, what) {
  const s = String(v ?? '').trim();
  const num = /^\d{1,15}$/.test(s) ? Number(s) : /^0x[0-9a-fA-F]{1,13}$/.test(s) ? hexNum(s) : NaN;
  if (!Number.isSafeInteger(num)) throw new CheckError(`The link's ${what} is not a block number.`);
  return num;
}

/// The check a link asks for, from the page's query string: null for none; { ch, lookup } for an empty
/// tx (a box to paste one into, on the chain named or Ethereum); { ch, tx } for a transaction (its hash,
/// or <block>:<index>); { ch, filter, from, to } for logs (from, to, address, topic0 to topic3, each list
/// comma-separated). Throws a CheckError for a link it cannot read.
function linkOf(search) {
  const q = new URLSearchParams(search);
  if (!q.has('tx') && !q.has('from') && !q.has('to')) return null;
  if (q.has('tx') && !q.get('tx').trim() && !q.has('chain')) return { ch: CHAINS[0], lookup: true };
  const ch = chainNamed(q.get('chain'));
  if (!ch) throw new CheckError(`The link names no chain Willow serves. It takes chain=${CHAINS.map((c) => c.slugs[0]).join(', ')} or a chain ID.`);
  if (q.has('tx')) {
    const tx = q.get('tx').trim();
    if (!tx) return { ch, lookup: true };
    if (!TX_HASH.test(tx) && !AT_INDEX.test(tx)) throw new CheckError("The link's transaction is not a transaction hash, or a block and an index such as 1234567:5.");
    return { ch, tx: tx.toLowerCase() };
  }
  const from = blockParam(q.get('from'), 'first block (from)');
  const to = blockParam(q.get('to'), 'last block (to)');
  if (from > to) throw new CheckError("The link's first block is after its last.");
  const list = (key, re, what) => {
    const v = (q.get(key) || '').trim();
    if (!v) return null;
    const items = [...new Set(v.split(',').map((x) => x.trim().toLowerCase()))];
    if (!items.every((x) => re.test(x))) throw new CheckError(`The link's ${what} is not a list of ${re === ADDRESS ? 'addresses' : '32-byte topics'}.`);
    return items.length === 1 ? items[0] : items;
  };
  const filter = { fromBlock: toHex(from), toBlock: toHex(to) };
  const address = list('address', ADDRESS, 'address');
  if (address) filter.address = address;
  const topics = [0, 1, 2, 3].map((i) => list(`topic${i}`, TX_HASH, `topic${i}`));
  while (topics.length && topics[topics.length - 1] === null) topics.pop();
  if (topics.length) filter.topics = topics;
  return { ch, filter, from, to };
}

/// A link to this page that runs check `c` (as linkOf reads it).
function linkTo(c) {
  const q = new URLSearchParams({ chain: c.ch.slugs[0] });
  if (c.tx) q.set('tx', c.tx);
  else {
    q.set('from', String(c.from));
    q.set('to', String(c.to));
    const f = c.filter;
    const join = (v) => (Array.isArray(v) ? v.join(',') : v);
    if (f.address) q.set('address', join(f.address));
    (f.topics || []).forEach((t, i) => t != null && q.set(`topic${i}`, join(t)));
  }
  return `${location.origin}${location.pathname}?${q}`;
}

// The full checker's last check, as a link would carry it (set as it starts), or null.
let checkLink = null;

/// A button that copies `text` and says so.
function copyButton(label, text, cls = 'secondary') {
  const b = el('button', { type: 'button', class: cls }, label);
  b.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(text());
      b.textContent = 'Copied';
    } catch {
      b.textContent = 'Could not copy';
    }
    setTimeout(() => (b.textContent = label), 1600);
  });
  return b;
}

// ---------- the link view: one transaction ----------

const shortAddr = (a) => (typeof a === 'string' && a.length === 42 ? `${a.slice(0, 6)}…${a.slice(-4)}` : String(a));
const lower = (x) => (typeof x === 'string' ? x.toLowerCase() : '');

/// A 0x quantity from the proven JSON as a BigInt, or null.
function big(x) {
  return typeof x === 'string' && /^0x[0-9a-fA-F]{1,64}$/.test(x) ? BigInt(x) : null;
}

/// `units` of a token with `decimals` decimals, for people: thousands separated, at most
/// `places` decimals (more for amounts that would otherwise show as 0).
function amountText(units, decimals, places = 6) {
  const neg = units < 0n;
  let v = neg ? -units : units;
  if (decimals == null) return `${neg ? '−' : ''}${v.toLocaleString('en-US')}`;
  const base = 10n ** BigInt(decimals);
  const whole = v / base;
  let frac = (v % base).toString().padStart(decimals, '0').replace(/0+$/, '');
  if (frac.length > places) {
    const firstDigit = frac.search(/[1-9]/);
    frac = frac.slice(0, whole > 0n || firstDigit < places ? places : firstDigit + 3).replace(/0+$/, '');
  }
  return `${neg ? '−' : ''}${whole.toLocaleString('en-US')}${frac ? `.${frac}` : ''}`;
}

/// When a block was made: its day and time in the reader's time zone, and the time in UTC.
function timeText(unix) {
  const d = new Date(unix * 1000);
  return { date: d.toLocaleDateString(undefined, { dateStyle: 'medium' }), time: d.toLocaleTimeString(undefined, { timeStyle: 'short' }), utc: `${d.toISOString().slice(11, 16)} UTC` };
}

/// A token's symbol and decimals, read with eth_call from the chain's public nodes (not proven:
/// they are only names for the proven amounts). { symbol, decimals } with nulls for what no node told.
async function tokenInfo(ch, token) {
  const call = async (data) => {
    for (const node of ch.nodes) {
      try {
        const { result } = await rpc(node, 'eth_call', [{ to: token, data }, 'latest']);
        if (typeof result === 'string' && /^0x[0-9a-fA-F]*$/.test(result) && result.length > 2) return result;
      } catch {}
    }
    return null;
  };
  const [sym, dec] = await Promise.all([call('0x95d89b41'), call('0x313ce567')]);
  let decimals = dec && dec.length === 66 ? Number(BigInt(dec)) : null;
  if (!(decimals >= 0 && decimals <= 36)) decimals = null;
  return { symbol: symbolOf(sym), decimals };
}

/// An ABI string (or bytes32) symbol, kept only if it is short and printable.
function symbolOf(hex) {
  if (!hex) return null;
  let bytesHex = null;
  try {
    if (hex.length >= 2 + 64 * 3 && BigInt(`0x${hex.slice(2, 66)}`) === 32n) {
      const len = Number(BigInt(`0x${hex.slice(66, 130)}`));
      if (len > 0 && len <= 32) bytesHex = hex.slice(130, 130 + len * 2);
    } else if (hex.length === 66) bytesHex = hex.slice(2).replace(/(00)+$/, '');
    if (!bytesHex) return null;
    const text = new TextDecoder('utf-8', { fatal: true }).decode(hexToBytes(`0x${bytesHex}`));
    return /^[\p{L}\p{N}\p{Sc} ._$+-]{1,16}$/u.test(text) ? text.trim() : null;
  } catch {
    return null;
  }
}

/// The swap router fills in a proven receipt's logs (see ORDER_FILLED).
function ordersOf(receipt) {
  const logs = Array.isArray(receipt && receipt.logs) ? receipt.logs : [];
  const word = (d, i) => d.slice(2 + 64 * i, 66 + 64 * i);
  const addr = (w) => `0x${w.slice(24)}`.toLowerCase();
  return logs
    .filter((l) => l && Array.isArray(l.topics) && l.topics.length === 3 && lower(l.topics[0]) === ORDER_FILLED && FILL_ROUTERS.has(lower(l.address)) && typeof l.data === 'string' && l.data.length === 2 + 64 * 6)
    .map((l) => ({
      by: lower(l.address),
      sender: addr(l.topics[1].slice(2)),
      recipient: addr(l.topics[2].slice(2)),
      tokenIn: addr(word(l.data, 0)),
      tokenOut: addr(word(l.data, 1)),
      amountIn: BigInt(`0x${word(l.data, 2)}`),
      amountOut: BigInt(`0x${word(l.data, 3)}`),
      fee: BigInt(`0x${word(l.data, 4)}`),
      target: addr(word(l.data, 5)),
    }));
}

/// The token movements in a proven receipt's logs: ERC-20 and NFT transfers, and WETH wraps.
function movementsOf(receipt) {
  const out = [];
  const logs = Array.isArray(receipt && receipt.logs) ? receipt.logs : [];
  const topicAddr = (t) => (typeof t === 'string' && t.length === 66 ? `0x${t.slice(26)}`.toLowerCase() : null);
  for (const l of logs) {
    if (!l || !Array.isArray(l.topics) || typeof l.address !== 'string') continue;
    const t0 = lower(l.topics[0]);
    const data = typeof l.data === 'string' ? l.data : '';
    if (t0 === TRANSFER && l.topics.length === 3 && data.length === 66) {
      out.push({ kind: 'erc20', token: lower(l.address), from: topicAddr(l.topics[1]), to: topicAddr(l.topics[2]), amount: BigInt(data) });
    } else if (t0 === TRANSFER && l.topics.length === 4) {
      out.push({ kind: 'nft', token: lower(l.address), from: topicAddr(l.topics[1]), to: topicAddr(l.topics[2]), id: BigInt(l.topics[3]) });
    } else if ((t0 === WETH_DEPOSIT || t0 === WETH_WITHDRAWAL) && l.topics.length === 2 && data.length === 66) {
      out.push({ kind: t0 === WETH_DEPOSIT ? 'wrap' : 'unwrap', token: lower(l.address), by: topicAddr(l.topics[1]), amount: BigInt(data) });
    }
  }
  return out;
}

/// The page's link view for a transaction: the proof of its receipt (which carries the
/// transaction), checked in the worker, the block confirmed against Ethereum's validators (on an L2
/// without that yet, on two public nodes), and the transaction shown as a receipt. Nothing on it
/// comes from anywhere but the proven bytes and Ethereum, except token names and decimals and, on
/// those L2s, the nodes' confirmation, each said so.
async function runTxLink(link) {
  const { ch, tx } = link;
  linkShown = { ...link, proof: null };
  const endpoint = defaultEndpoint(ch);
  const v = receiptSkeleton(ch);
  try {
    v.step('proof', 'active', 'Asking Willow…');
    const req = await ask({ type: 'items-request', chainId: ch.id, kind: 'receipt', target: tx });
    let proofText, proofSize;
    try {
      ({ text: proofText, size: proofSize } = await rpc(endpoint, req.method, req.params));
    } catch (e) {
      throw itemRequestError(e, ch);
    }
    linkShown.proof = proofText;
    v.step('proof', 'done', `${bytes(proofSize)} from ${hostOf(endpoint)}`);
    v.step('verify', 'active', 'Verifying…');
    const tv = performance.now();
    const checked = await ask({ type: 'items-verify', response: proofText });
    const ms = performance.now() - tv;
    const rep = checked.report;
    const receipt = checked.proven || {};
    const txn = checked.transaction || {};
    const blockNum = hexNum(rep.block.number);
    const blockHash = rep.block.hash;
    v.step('verify', 'done', `Block hash ${shortHash(blockHash)} · ${secs(ms)}`);
    const trustless = TRUSTLESS.has(ch.id);
    v.step('anchor', 'active', trustless ? `Verifying block ${n(blockNum)} against Ethereum…` : `Asking ${hostsText(ch.nodes)} for block ${n(blockNum)}…`);
    const moves = movementsOf(receipt);
    const orders = ordersOf(receipt);
    const named = [...orders.flatMap((o) => [o.tokenIn, o.tokenOut]), ...moves.filter((m) => m.kind !== 'nft').map((m) => m.token)];
    const known = KNOWN_TOKENS[ch.id] || {};
    const tokens = [...new Set(named.filter((t) => t !== ZERO && !known[t]))].slice(0, TOKENS_NAMED);
    const [anchor, names] = await Promise.all([
      confirmAnchor(ch, endpoint, trustless ? null : ch.nodes, blockNum, blockHash),
      Promise.all(tokens.map((t) => tokenInfo(ch, t))).then((info) => Object.fromEntries(tokens.map((t, i) => [t, info[i]]))),
    ]);
    v.step('anchor', anchorState(anchor));
    v.show({ ch, rep, receipt, txn, anchor, names, moves, orders });
  } catch (e) {
    v.fail(e);
  }
}

/// The checklist's block row: the block's level in the page's words, green when the browser
/// verified it against Ethereum, gold when it did not; `sub`, small lines under it. Below settled,
/// an L2 row says when the chain itself treats the block as final.
function levelOf(a, ch, blockTime = null) {
  const l2Line = ch.id === 1 ? [] : [chainFinalLine(ch, blockTime)];
  if (a.mismatch.length) return { ok: false, text: `${ch.name} has another block at this height` };
  if (a.failed) return { ok: false, text: "Willow's data for the block failed verification" };
  if (!a.confirmed) return { ok: false, text: 'Block not confirmed' };
  if (a.via === 'nodes') return { ok: false, text: a.public ? `Confirmed by public ${ch.name} nodes` : `Confirmed on ${a.ok.join(' and ')}`, sub: l2Line };
  if (ch.id === 1) return a.level === 'final' ? { ok: true, text: "Finalized on Ethereum, can't be reversed" } : { ok: true, text: "Signed by Ethereum's validators, not finalized yet", sub: ['Ethereum finalizes blocks about 13 minutes after they are made.'] };
  if (a.level === 'final') return { ok: true, text: "Settled on Ethereum, can't be reversed" };
  if (a.level === 'signed') return { ok: true, text: `Signed by ${ch.name}'s sequencer, not on Ethereum yet`, sub: l2Line };
  return { ok: true, text: `Posted to Ethereum, settles in about ${ch.finalDays} days`, sub: l2Line };
}

/// The checklist's last row, once a proof verified: what it rests on is the browser, not Willow.
const TRUST_ROW = { ok: true, text: 'Verified by your browser, no trust in Willow needed' };

/// A short checklist: each row's mark green or gold, its words, and small lines under it.
function checkRows(rows) {
  return rows.map((r) =>
    el('li', { 'data-tone': r.ok ? 'ok' : 'warn' },
      el('span', { class: 'mark', 'aria-hidden': 'true' }),
      el('div', {}, el('span', { class: 'r-check' }, r.text), ...(r.sub || []).map((s) => el('div', { class: 'muted small' }, s)))));
}

/// How the browser confirmed block `number` (`what`: "the block" or "the last block"), in one short
/// sentence.
function anchorSentence(a, ch, what, number, hash) {
  if (a.via === 'ethereum' && a.confirmed) {
    if (a.level === 'signed') return `Your browser verified ${ch.name}'s sequencer signed ${what}.`;
    return `Your browser verified ${what} against Ethereum.`;
  }
  if (a.via === 'nodes' && a.confirmed) {
    const who = a.public ? `${a.ok.length > 1 ? 'Two public' : 'A public'} ${ch.name} node${a.ok.length > 1 ? 's' : ''}, run by others,` : a.ok.join(' and ');
    return `${who} ${a.ok.length > 1 ? 'have' : 'has'} ${what} with this hash. Your browser could not verify it against Ethereum${a.pending ? `: ${sentence(a.pending)}` : '.'}`;
  }
  return anchorNote(a, n(number), hash);
}

/// The receipt card, shown while the check runs: { step, label, show, fail }. Folded, it shows the
/// verdict, what the transaction settled, a short checklist, Copy link, the explorer and More; More
/// unfolds the trade, how the browser verified it, and the proof.
function receiptSkeleton(ch) {
  const card = $('receipt');
  const seal = el('div', { class: 'seal checking', role: 'status' },
    el('span', { class: 'seal-mark', 'aria-hidden': 'true' }),
    el('h1', { class: 'seal-title' }, 'Verifying…'),
    el('p', { class: 'seal-date' }),
    el('p', { class: 'seal-text' }, `Your browser is verifying this ${ch.name} transaction.`));
  const sum = el('div', { class: 'r-sum' });
  const checks = el('ul', { class: 'r-checks' });
  const how = el('ol', { class: 'steps how' });
  const stepsHere = {};
  for (const [key, what] of [
    ['proof', 'Proof from Willow'],
    ['verify', 'Verified in your browser'],
    ['anchor', TRUSTLESS.has(ch.id) ? 'Block on Ethereum' : `Block on ${ch.name}`],
  ]) {
    const note = el('div', { class: 'note' });
    const li = el('li', { 'data-state': 'pending' }, el('span', { class: 'mark', 'aria-hidden': 'true' }), el('div', {}, el('div', { class: 'what' }, what), note));
    stepsHere[key] = { li, note };
    how.append(li);
  }
  const tradeTitle = el('h2', {}, 'Trade');
  const facts = el('div', { class: 'r-facts' });
  const trade = el('section', { class: 'r-group', hidden: '' }, tradeTitle, facts);
  const extra = el('div', { class: 'r-extra' });
  const movesList = el('div', {});
  const movesGroup = el('section', { class: 'r-group r-moves', hidden: '' }, el('h2', {}, 'Token movements'), movesList);
  const more = el('div', { class: 'r-more', id: 'receiptMore' },
    trade,
    movesGroup,
    el('section', { class: 'r-group r-how' }, el('h2', {}, 'How it was verified'), how),
    el('section', { class: 'r-group r-proof' }, el('h2', {}, 'Proof'), extra));
  const actions = el('div', { class: 'r-actions' });
  const toggle = el('button', { type: 'button', class: 'r-toggle', 'aria-expanded': 'false', 'aria-controls': 'receiptMore' }, 'More');
  const box = el('div', { class: 'card r-card checking' }, seal, sum, checks, actions, more);
  const unfold = (open) => {
    box.classList.toggle('open', open);
    toggle.setAttribute('aria-expanded', String(open));
    toggle.textContent = open ? 'Less' : 'More';
  };
  toggle.addEventListener('click', () => unfold(!box.classList.contains('open')));
  card.replaceChildren(box);
  card.hidden = false;
  let explorer = null;
  const step = (key, state, note) => {
    const s = stepsHere[key];
    if (state) s.li.dataset.state = state;
    if (note !== undefined) s.note.textContent = note;
  };
  const label = (key, text) => (stepsHere[key].li.querySelector('.what').textContent = text);
  const setSeal = (tone, title, text) => {
    seal.className = `seal ${tone}`;
    seal.querySelector('.seal-title').replaceChildren(...[title].flat());
    seal.querySelector('.seal-text').textContent = text;
  };
  const finish = () => {
    box.classList.remove('checking');
    for (const s of Object.values(stepsHere)) {
      if (s.li.dataset.state !== 'pending' && s.li.dataset.state !== 'active') continue;
      s.li.dataset.state = 'skip';
      s.note.textContent = 'Not run';
    }
    actions.replaceChildren(copyButton('Copy link', () => linkTo(linkShown), seal.classList.contains('ok') ? 'primary' : 'secondary'), ...(explorer ? [explorer] : []), toggle);
    extra.replaceChildren(
      ...(linkShown.proof ? [el('button', { type: 'button', class: 'secondary', onclick: () => saveText(linkShown.proof, `willow-receipt-proof-${ch.id}-${linkShown.tx.replace(':', '-').slice(0, 18)}.json`) }, 'Download the proof')] : []),
      el('button', { type: 'button', class: 'secondary', onclick: () => openChecker(linkShown) }, 'Technical details'),
    );
  };
  return {
    step,
    label,
    fail(e) {
      const browser = e.kind === 'internal' || (!(e instanceof CheckError) && isRuntimeFailure(e));
      const failed = !browser && (e.kind === 'rejected' || e.kind === 'malformed-input');
      const open = Object.entries(stepsHere).find(([, s]) => s.li.dataset.state === 'active' || s.li.dataset.state === 'pending');
      if (open) step(open[0], e.tone === 'warn' || browser ? 'warn' : 'fail', e.stepNote || e.message);
      if (browser) setSeal('warn', 'Your browser could not finish verifying', `It stopped before a result (${e.message}). This says nothing about the transaction. Try again, or in another browser.`);
      else if (failed) setSeal('bad', 'Not verified', `Willow's proof failed verification: ${sentence(e.message)} Nothing on this page is verified.`);
      else setSeal(e.tone === 'warn' ? 'warn' : 'bad', e.title || 'Could not verify this transaction', e.message);
      finish();
      unfold(true);
    },
    show(r) {
      const { rep, receipt, txn, anchor, names, moves, orders } = r;
      const blockNum = hexNum(rep.block.number);
      const ok = receipt.status === '0x1';
      const sender = lower(receipt.from);
      const what = orders.length && ok ? 'Swap' : 'Transaction';
      // The checklist says how far the block is; the seal's own words are kept for what went wrong.
      if (anchor.mismatch.length) setSeal('bad', 'Not verified', `The proof is valid, but ${mismatchHosts(anchor)} ${anchor.mismatch.length === 1 ? 'has' : 'have'} a different block ${n(blockNum)}. If it is seconds old, it may have been replaced; try again in a minute.`);
      else if (anchor.failed) setSeal('bad', 'Not verified', `Willow's proof is valid, but ${failedText(anchor, n(blockNum))}`);
      else if (!anchor.confirmed && anchor.via === 'ethereum') setSeal('warn', 'Proof valid, block not confirmed', `Willow's proof is valid, but your browser could not confirm block ${n(blockNum)} on Ethereum just now: ${sentence(problemsText(anchor))}`);
      else if (!anchor.confirmed) setSeal('warn', 'Proof valid, block not confirmed', `Willow's proof is valid, but no public node could confirm block ${n(blockNum)} just now (${problemsText(anchor)}). Try again in a minute.`);
      else if (unanchored(anchor, ch)) setSeal('warn', 'Proof valid, block not verified against Ethereum', '');
      else setSeal('ok', [`${what} `, el('em', {}, 'verified')], '');

      // The sender's own movements, netted per token: what the transaction settled for them.
      const net = new Map();
      const add = (key, amount) => net.set(key, (net.get(key) || 0n) + amount);
      for (const m of moves) if (m.kind === 'erc20' && m.from !== m.to) {
        if (m.from === sender) add(m.token, -m.amount);
        if (m.to === sender) add(m.token, m.amount);
      }
      // A reverted transaction moved no value: only its fee was paid.
      const value = ok ? big(txn.value) || 0n : 0n;
      const known = KNOWN_TOKENS[ch.id] || {};
      const decimalsOf = (t) => (t === ZERO ? 18 : known[t] ? known[t][1] : names[t] ? names[t].decimals : null);
      // ETH and listed tokens by name; any other by the name its contract gives, with its address,
      // and by address alone when its decimals are unknown (the amount is then in raw units).
      const tokenName = (t) => {
        if (t === ZERO) return 'ETH';
        if (known[t]) return known[t][0];
        const sym = names[t] && names[t].symbol;
        if (decimalsOf(t) == null) return `units of ${shortAddr(t)}`;
        return sym ? `${sym} (${shortAddr(t)})` : shortAddr(t);
      };
      const tokenParts = (t) => {
        if (t === ZERO || known[t] || decimalsOf(t) == null || !(names[t] && names[t].symbol)) return [tokenName(t), null];
        return [names[t].symbol, shortAddr(t)];
      };
      const line = (label, value, cls = '') => el('div', { class: `r-line ${cls}` }, el('span', { class: 'r-label' }, label), el('span', { class: 'r-value' }, ...[value].flat()));
      // One line a row: the symbol only; an unlisted token's address is under More.
      const shown = [];
      const amount = (units, t) => {
        if (!shown.includes(t)) shown.push(t);
        const [sym] = tokenParts(t);
        return el('span', { class: 'r-amt' }, amountText(units, decimalsOf(t)), ' ', el('span', { class: 'settle-sym', title: t || '' }, sym));
      };
      const time = Number.isSafeInteger(rep.block.timestamp) ? timeText(rep.block.timestamp) : null;
      const addrCell = (a) => el('span', { class: 'addr' }, el('span', { class: 'mono', title: a }, shortAddr(a)), copyButton('Copy', () => a, 'mini'));

      // A router's fill says who traded what for what, and the fee: that is the settlement. Without
      // one, the sender's netted movements are.
      const lines = [];
      if (!ok) lines.push(line('Status', 'Failed: it reverted, and only its fee was paid', 'failed'));
      for (const o of orders) {
        lines.push(line('Paid', amount(o.amountIn, o.tokenIn), 'out'), line('Received', amount(o.amountOut, o.tokenOut), 'in'));
        const feeToken = o.tokenOut === ZERO && o.tokenIn !== ZERO ? ZERO : o.tokenIn;
        if (o.fee > 0n) lines.push(line('Fee', `${amountText(o.fee, decimalsOf(feeToken))} ${tokenParts(feeToken)[0]}`));
      }
      if (!orders.length) {
        const settled = [];
        if (value > 0n) settled.push(['Paid', value, ZERO]);
        for (const [t, a] of net) if (a !== 0n) settled.push([a < 0n ? 'Paid' : 'Received', a < 0n ? -a : a, t]);
        settled.sort((x, y) => (x[0] === y[0] ? 0 : x[0] === 'Paid' ? -1 : 1));
        for (const [verb, units, t] of settled) lines.push(line(verb, amount(units, t), verb === 'Paid' ? 'out' : 'in'));
      }
      seal.querySelector('.seal-date').textContent = time ? `${time.date}, ${time.time} · ${ch.name}` : ch.name;
      sum.replaceChildren(...lines);
      checks.replaceChildren(...checkRows([{ ok: true, text: `This ${what === 'Swap' ? 'trade' : 'transaction'} is in ${ch.name} block ${n(blockNum)}` }, levelOf(anchor, ch, rep.block.timestamp), TRUST_ROW]));
      if (ch.explorer && ch.explorerName) explorer = el('a', { class: 'button secondary', href: `${ch.explorer}/tx/${rep.transactionHash}`, target: '_blank', rel: 'noopener noreferrer' }, ch.explorerName);

      // The fee: execution, plus the blob fee and the OP stack's L1 data fee where the receipt has
      // them. What is not added (an L1 fee the proof does not cover, an OP operator fee) is said.
      const gas = big(receipt.gasUsed);
      const price = big(receipt.effectiveGasPrice);
      const opStack = [8453, 10, 130, 57073].includes(ch.id);
      const l1 = big(receipt.l1Fee);
      const blob = (big(receipt.blobGasUsed) || 0n) * (big(receipt.blobGasPrice) || 0n);
      const fee = gas != null && price != null ? gas * price + blob + (l1 || 0n) : null;
      const left = [opStack && l1 == null && 'the L1 data fee, which this proof does not cover', receipt.operatorFeeScalar != null && "the chain's operator fee"].filter(Boolean);
      const feeText = fee == null ? null : left.length ? `At least ${amountText(fee, 18, 8)} ETH, plus ${left.join(' and ')}` : `${amountText(fee, 18, 8)} ETH`;
      // Who and what, once each: From and To only where they add to the trader and the router.
      const trader = orders.length ? orders[0].sender : null;
      const routers = new Set(orders.map((o) => o.by));
      const to = lower(receipt.to);
      const rows = [
        ...orders.flatMap((o) => [['Trader', addrCell(o.sender)], o.recipient !== o.sender ? ['Sent to', addrCell(o.recipient)] : null, ['Router', addrCell(o.by)]]),
        sender !== trader ? ['From', addrCell(receipt.from)] : null,
        receipt.to ? (to !== trader && !routers.has(to) ? ['To', addrCell(receipt.to)] : null) : receipt.contractAddress ? ['Created', addrCell(receipt.contractAddress)] : null,
        ...shown.filter((t) => tokenParts(t)[1]).map((t) => [`${tokenParts(t)[0]} token`, addrCell(t)]),
        time ? ['Time in UTC', `${new Date(rep.block.timestamp * 1000).toISOString().slice(0, 16).replace('T', ' ')} UTC`] : null,
        feeText ? ['Network fee', feeText] : null,
        ['Transaction hash', el('span', { class: 'addr' }, el('span', { class: 'mono', title: rep.transactionHash }, shortHash(rep.transactionHash)), copyButton('Copy', () => rep.transactionHash, 'mini'))],
        ok ? null : ['Status', 'Failed: the transaction reverted, and only its fee was paid'],
      ].filter(Boolean);
      // Every transfer the receipt holds, the route's hops under a swap, listed in a group of its own.
      if (r.moves.length) {
        movesList.replaceChildren(el('ul', { class: 'move-list' }, ...moves.map((m) => {
          // The symbol alone keeps each row on one line; the token's address is on hover.
          const sym = tokenParts(m.token)[0];
          const who = (a) => (a === trader ? 'trader' : a === sender ? 'sender' : shortAddr(a));
          if (m.kind === 'erc20') return el('li', {}, el('span', { class: 'move-amt' }, `${amountText(m.amount, decimalsOf(m.token))} `, el('span', { class: 'settle-sym', title: m.token }, sym)), el('span', { class: 'move-way' }, `${who(m.from)} → ${who(m.to)}`));
          if (m.kind === 'nft') return el('li', {}, el('span', { class: 'move-amt' }, `#${m.id.toString()} `, el('span', { class: 'settle-sym', title: m.token }, shortAddr(m.token))), el('span', { class: 'move-way' }, `${who(m.from)} → ${who(m.to)}`));
          return el('li', {}, el('span', { class: 'move-amt' }, `${amountText(m.amount, 18)} `, el('span', { class: 'settle-sym', title: m.token }, m.kind === 'wrap' ? 'ETH → WETH' : 'WETH → ETH')), el('span', { class: 'move-way' }, `${m.kind === 'wrap' ? 'wrapped' : 'unwrapped'} by ${who(m.by)}`));
        })));
        movesGroup.hidden = false;
      }
      tradeTitle.textContent = what === 'Swap' ? 'Trade' : 'Transaction';
      facts.replaceChildren(el('dl', { class: 'r-rows' }, ...rows.flatMap(([k, val]) => [el('dt', {}, k), el('dd', {}, val)])));
      trade.hidden = false;

      // How it was verified: a sentence a step, the details in small type.
      label('proof', 'Willow sent the transaction and receipt with proofs.');
      label('verify', `Your browser verified both are in ${ch.name} block ${n(blockNum)}.`);
      label('anchor', anchorSentence(anchor, ch, 'the block', blockNum, rep.block.hash));
      for (const key of ['proof', 'verify', 'anchor']) step(key, null, '');
      finish();
      if (!anchor.confirmed) unfold(true);
    },
  };
}

/// The link view's check in the full checker, filled in and run.
function openChecker(c) {
  document.body.classList.remove('link-mode', 'link-folded');
  $('linkProof').hidden = true;
  // A new history entry, so the browser's Back returns to the link's card.
  history.pushState(null, '', location.pathname);
  $('receipt').hidden = true;
  $('chain').value = String(c.ch.id);
  const prev = chainShown;
  chainShown = c.ch;
  showChain(prev);
  $('endpoint').value = defaultEndpoint(c.ch);
  if (c.tx) {
    document.querySelector('input[name=kind][value=receipt]').checked = true;
    showKind();
    $('item').value = c.tx;
  } else {
    document.querySelector('input[name=kind][value=logs]').checked = true;
    showKind();
    setFilter(JSON.stringify(c.filter, null, 2));
  }
  // "Technical details" opens the form at the top of the page and runs the same verification at once,
  // its full result below the form.
  window.scrollTo({ top: 0 });
  $('form').requestSubmit();
}

function saveText(text, name) {
  const blob = new Blob([text], { type: 'application/json' });
  const a = el('a', { href: URL.createObjectURL(blob), download: name });
  document.body.append(a);
  a.click();
  setTimeout(() => {
    URL.revokeObjectURL(a.href);
    a.remove();
  }, 0);
}

// ---------- the link view: logs ----------

/// What a link's events are called: swaps when every event it asks for is a swap or a router's fill.
function eventNoun(f) {
  const t0 = [((f && f.topics) || [])[0]].flat().filter(Boolean).map(lower);
  return t0.length && t0.every((t) => /^(Order filled|Swap)/.test(EVENTS[t] || '')) ? 'swap' : 'event';
}

/// The filter in words, for the link view's heading.
function filterWords(f, ch) {
  const many = (v) => (Array.isArray(v) ? v : v ? [v] : []);
  const parts = [];
  const addrs = many(f.address);
  const t0 = many((f.topics || [])[0]);
  const named = t0.map((t) => EVENTS[lower(t)] || shortHash(t));
  parts.push(t0.length ? `${named.join(' or ')} events` : 'Events');
  if (addrs.length) parts.push(`from ${addrs.map(shortAddr).join(' or ')}`);
  (f.topics || []).slice(1).forEach((t, i) => {
    const vals = many(t);
    if (vals.length) parts.push(`with topic ${i + 1} ${vals.map((x) => (/^0x0{24}/.test(x) ? shortAddr(`0x${x.slice(26)}`) : shortHash(x))).join(' or ')}`);
  });
  return `${parts.join(' ')} on ${ch.name}`;
}

/// A card whose heading gets a Show details toggle: folded, it shows only the heading
/// (`notesOnly`: the steps without their notes).
function foldable(card, notesOnly) {
  if (card.querySelector(':scope > .fold-head')) return;
  const h = card.querySelector(':scope > h2');
  const toggle = el('button', { type: 'button', class: 'link', 'aria-expanded': 'false' }, 'Show details');
  toggle.addEventListener('click', () => setFold(card, !card.classList.contains('open')));
  const head = el('div', { class: 'fold-head' });
  h.replaceWith(head);
  head.append(h, toggle);
  card.classList.add(notesOnly ? 'fold-notes' : 'fold');
}

function setFold(card, open) {
  const toggle = card.querySelector(':scope > .fold-head button');
  if (!toggle) return;
  card.classList.toggle('open', open);
  toggle.setAttribute('aria-expanded', String(open));
  toggle.textContent = open ? 'Hide details' : 'Show details';
}

/// The link view for logs: the full checker's log check, run with the keyless endpoint and the
/// chain's public nodes, without the form.
function runLogsLink(link) {
  const { ch, filter, from, to } = link;
  linkShown = link;
  $('chain').value = String(ch.id);
  const prev = chainShown;
  chainShown = ch;
  showChain(prev);
  $('endpoint').value = defaultEndpoint(ch);
  $('node').value = '';
  document.querySelector('input[name=kind][value=logs]').checked = true;
  showKind();
  setFilter(JSON.stringify(filter, null, 2));
  const noun = eventNoun(filter);
  $('linkIntro').replaceChildren(
    el('p', { class: 'r-kicker' }, el('span', { class: `chain-dot c${ch.id}`, 'aria-hidden': 'true' }), `${ch.name} · blocks ${n(from)}–${n(to)}`),
    el('h1', { class: 'r-title' }, `Every ${noun} in ${n(to - from + 1)} block${from === to ? '' : 's'}`),
    el('p', { class: 'lede' }, `${filterWords(filter, ch)}.`),
  );
  $('linkIntro').hidden = false;
  $('progress').querySelector('h2').textContent = 'How it was verified';
  runLogs();
}

/// Starts the link in the page's address, if any. True when it did.
function startLink() {
  let link;
  try {
    link = linkOf(location.search);
  } catch (e) {
    document.body.classList.add('link-mode');
    $('linkIntro').replaceChildren(el('h1', { class: 'r-title' }, 'This link cannot be read'), el('p', { class: 'lede' }, e.message), el('p', {}, el('a', { href: location.pathname }, 'Open the proof verifier')));
    $('linkIntro').hidden = false;
    return true;
  }
  if (!link) return false;
  // A link with an empty tx (from before the box was folded into the form) opens the form's
  // Transaction tab on its chain.
  if (link.lookup) {
    history.replaceState(null, '', location.pathname);
    $('chain').value = String(link.ch.id);
    const prev = chainShown;
    chainShown = link.ch;
    showChain(prev);
    $('endpoint').value = defaultEndpoint(link.ch);
    document.querySelector('input[name=kind][value=receipt]').checked = true;
    showKind();
    return false;
  }
  document.body.classList.add('link-mode');
  document.title = link.tx ? `${link.ch.name} transaction · Willow proof verifier` : `${link.ch.name} events · Willow proof verifier`;
  if (link.tx) runTxLink(link);
  else runLogsLink(link);
  return true;
}

// ---------- setup ----------

function exampleFilter(from, to, ch = chosen()) {
  return JSON.stringify({ fromBlock: from, toBlock: to, address: ch.poolManager, topics: [SWAP_TOPIC] }, null, 2);
}

const NOT_SERVING = 'Willow has no proven blocks to verify right now. Try again later.';

/// The example over proven blocks, from Willow's public proof status (see exampleRange): { filter },
/// or { notServing } when nothing is proven and nothing can be proven on request; null if unknown.
/// With nothing proven yet but proving on request, the example is "latest": Willow proves it first.
async function provenExample(endpoint, ch) {
  const head = await proofStatus(endpoint, ch);
  if (!head) return null;
  const r = exampleRange(head);
  if (r) return { filter: exampleFilter(toHex(r.from), toHex(r.to), ch) };
  if (head.jobs && head.jobs.available && !isPublic(endpoint)) return { filter: exampleFilter('latest', 'latest', ch) };
  return { notServing: true };
}

// The filter the page last put in as an example: a chain change replaces it, never a filter typed.
let exampleShown = null;
function setFilter(f) {
  $('filter').value = f;
  exampleShown = f;
}

async function useExample() {
  const ch = chosen();
  $('formError').textContent = '';
  const endpoint = $('endpoint').value.trim();
  const ex = await provenExample(httpUrl(endpoint) ? endpoint : defaultEndpoint(ch), ch);
  if (chosen() !== ch) return;
  if (ex && ex.filter) {
    setFilter(ex.filter);
    return;
  }
  if (ex && ex.notServing) {
    setFilter(exampleFilter('latest', 'latest', ch));
    $('formError').textContent = NOT_SERVING;
    return;
  }
  // No proof status: recent blocks from the endpoint (they may not all be proven yet).
  if (!httpUrl(endpoint)) {
    setFilter(exampleFilter('latest', 'latest', ch));
    $('formError').textContent = 'Enter your endpoint to pick recent block numbers.';
    return;
  }
  try {
    const tip = quantity((await rpc(endpoint, 'eth_blockNumber', [])).result, `The latest block from ${hostOf(endpoint)}`);
    const to = tip - 2;
    setFilter(exampleFilter(toHex(to - EXAMPLE_BLOCKS + 1), toHex(to), ch));
  } catch (e) {
    setFilter(exampleFilter('latest', 'latest', ch));
    $('formError').textContent = e.message;
  }
}

/// An example for the transaction, receipt and block checks, from a block Willow holds in full, 10
/// blocks behind its newest (from its public status, no key): that block, or one of its
/// transactions (the second, past the system transaction L2 blocks start with), by hash when the
/// endpoint answers with the block, else by position.
async function useItemExample() {
  const ch = chosen();
  const kind = kindOf();
  $('formError').textContent = '';
  const endpoint = $('endpoint').value.trim();
  const c = await healthOf(httpUrl(endpoint) ? endpoint : defaultEndpoint(ch), '', ch.id);
  if (chosen() !== ch || kindOf() !== kind) return;
  const tip = c && Number.isSafeInteger(c.tip) && c.tip > 10 ? c.tip : null;
  if (tip == null) {
    $('formError').textContent = "Willow's status could not be read just now. Enter a transaction or a block yourself.";
    return;
  }
  const block = tip - 10;
  if (kind === 'blockReceipts') {
    $('item').value = String(block);
    return;
  }
  let target = `${block}:0`;
  if (httpUrl(endpoint)) {
    try {
      const { result } = await rpc(endpoint, 'eth_getBlockByNumber', [toHex(block), false]);
      const txs = result && Array.isArray(result.transactions) ? result.transactions : [];
      const h = txs[Math.min(1, txs.length - 1)];
      if (typeof h === 'string' && /^0x[0-9a-fA-F]{64}$/.test(h)) target = h;
    } catch {}
  }
  if (chosen() === ch && kindOf() === kind) $('item').value = target;
}

/// The fields for the chosen chain: the endpoint's /rpc/<id>/ path, the node typed for this chain
/// (a public node only as the placeholder), and the example filter, unless one was typed. `prev`:
/// the chain chosen before, whose node is kept for later.
function showChain(prev) {
  const ch = chosen();
  if (prev) nodes[prev.id] = $('node').value.trim();
  $('node').value = nodes[ch.id] || '';
  const trustless = TRUSTLESS.has(ch.id);
  $('node').placeholder = trustless ? 'Empty: verified against Ethereum' : `Empty: ${hostsText(ch.nodes)}`;
  $('nodeLabel').textContent = `Your ${ch.name} node (optional)`;
  $('nodeHint').textContent = trustless
    ? ch.id === 1
      ? "Used only to confirm the hash of the block the proof ends at. Left empty, your browser verifies that hash against Ethereum's validators itself, from a starting point built into this page, and relies on no node. A node entered here is asked instead."
      : `Used only to confirm the hash of the block the proof ends at. Left empty, your browser verifies it against Ethereum: what ${ch.name} posted there, or for a newer block the signature of ${ch.name}'s sequencer, whose key Ethereum names. Only when Willow's data for that cannot be verified are two public nodes from different operators asked (${hostsText(ch.nodes)}), and the result then says so. A node entered here is asked instead.`
    : `Used only to confirm the hash of the block the proof ends at. Left empty, two public nodes from different operators confirm it (${hostsText(ch.nodes)}), and you rely on them for that hash. ` +
      'Enter a node you run to rely on no one else.';
  const ep = $('endpoint').value.trim();
  $('endpoint').value = ep ? withChain(ep, ch.id) : defaultEndpoint(ch);
  $('endpoint').placeholder = `${ORIGIN}/v1/rpc/${ch.id}/${PUBLIC_KEY}`;
  if (prev) {
    if ($('filter').value === exampleShown) {
      setFilter(exampleFilter('latest', 'latest', ch));
      const initial = exampleShown;
      provenExample($('endpoint').value, ch).then((ex) => {
        if (ex && ex.filter && chosen() === ch && $('filter').value === initial) setFilter(ex.filter);
      });
    }
    $('item').value = '';
    itemValues.tx = itemValues.block = '';
    remember();
  }
}

// What was typed for a transaction (transaction and receipt checks) and for a block.
const itemValues = { tx: '', block: '' };
let shownKind = 'logs';

/// The fields for the chosen kind of check.
function showKind() {
  const kind = kindOf();
  const logs = kind === 'logs';
  const block = kind === 'blockReceipts';
  const group = (k) => (k === 'logs' ? null : k === 'blockReceipts' ? 'block' : 'tx');
  const was = group(shownKind);
  const now = group(kind);
  if (was) itemValues[was] = $('item').value;
  if (now && now !== was) $('item').value = itemValues[now];
  shownKind = kind;
  if (logs && !$('pinned').textContent) showPinned();
  $('logsField').hidden = !logs;
  $('itemField').hidden = logs;
  $('keysFields').hidden = !logs;
  $('itemLabel').textContent = block ? 'Block' : 'Transaction';
  $('item').placeholder = block ? 'A block number, a block hash, or latest' : 'A transaction hash, or <block>:<index>';
  $('compareHint').textContent = logs
    ? 'Its eth_getLogs answer for the same blocks is compared with the proven logs too. The provider must allow browser requests.'
    : `Its ${ITEM_KINDS[kind].plain} answer${block ? ' and the block\'s transactions' : ''} ${block ? 'are' : 'is'} compared with the proven one too. The provider must allow browser requests.`;
}

function remember() {
  // A link's endpoint and nodes are its own, not the visitor's.
  if (linkMode()) return;
  try {
    if ($('remember').checked) {
      nodes[chosen().id] = $('node').value.trim();
      const kept = Object.fromEntries(Object.entries(nodes).filter(([, v]) => v));
      localStorage.setItem(STORE_KEY, JSON.stringify({ endpoint: $('endpoint').value.trim(), chain: chosen().id, nodes: kept }));
    } else {
      localStorage.removeItem(STORE_KEY);
    }
  } catch {}
}

function restore() {
  let saved = null;
  try {
    saved = JSON.parse(localStorage.getItem(STORE_KEY) || 'null');
  } catch {}
  if (!saved || typeof saved !== 'object') saved = null;
  if (saved && saved.nodes && typeof saved.nodes === 'object') {
    for (const [id, url] of Object.entries(saved.nodes)) if (typeof url === 'string' && chainOf(Number(id))) nodes[id] = url;
  } else if (saved && typeof saved.node === 'string' && saved.node) {
    // Saved before the chain choice: the page was Ethereum's.
    nodes[1] = saved.node;
  }
  // The keyless endpoint the page offered before the public path: nothing it could check.
  const endpoint = saved && typeof saved.endpoint === 'string' && !(saved.endpoint.startsWith(`${ORIGIN}/`) && /\/rpc\/\d+\/?$/.test(saved.endpoint)) ? saved.endpoint : '';
  const ch = chainOf(endpointChain(endpoint)) || chainOf(saved && saved.chain) || CHAINS[0];
  $('chain').replaceChildren(...CHAINS.map((c) => el('option', { value: String(c.id) }, c.name)));
  $('chain').value = String(ch.id);
  $('endpoint').value = endpoint || defaultEndpoint(ch);
  $('remember').checked = !!saved;
  showChain(null);
  showKind();
  // Start with the newest proven blocks; keep "latest" until the proof status answers,
  // and never replace a filter the user has already changed.
  setFilter(exampleFilter('latest', 'latest', ch));
  const initial = exampleShown;
  provenExample($('endpoint').value, ch).then((ex) => {
    if (ex && ex.filter && chosen() === ch && $('filter').value === initial) setFilter(ex.filter);
  });
}

restore();
$('form').addEventListener('submit', run);
$('example').addEventListener('click', useExample);
$('itemExample').addEventListener('click', useItemExample);
$('download').addEventListener('click', downloadLogs);
$('itemDownload').addEventListener('click', downloadItems);
$('remember').addEventListener('change', remember);
let chainShown = chosen();
$('chain').addEventListener('change', () => {
  const prev = chainShown;
  chainShown = chosen();
  showChain(prev);
});
// An endpoint pasted for another served chain chooses that chain.
$('endpoint').addEventListener('change', () => {
  const named = chainOf(endpointChain($('endpoint').value.trim()));
  if (named && named !== chosen()) {
    $('chain').value = String(named.id);
    const prev = chainShown;
    chainShown = named;
    showChain(prev);
  }
});
$('node').addEventListener('input', () => (nodes[chosen().id] = $('node').value.trim()));
for (const r of document.querySelectorAll('input[name=kind]')) r.addEventListener('change', showKind);
// A link runs its check at once. Otherwise the log verifier loads with the page for a log check
// (its published keys are shown under Advanced); a transaction or receipt check needs only the
// smaller verifier.
// Back (or Forward) to a link after "Technical details" opened the form: show that link again.
window.addEventListener('popstate', () => location.reload());
if (!startLink()) {
  // The result first, its Details and the steps' notes folded until asked for.
  foldable($('details'), false);
  foldable($('progress'), true);
  if (kindOf() === 'logs') showPinned();
}
