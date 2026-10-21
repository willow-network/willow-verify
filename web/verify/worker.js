// Runs the wasm verifiers off the main thread: the log verifier (SP1) and the transaction and
// receipt verifier, each loaded on first use. `response` is a JSON-RPC response body as the
// server sent it (the wasm parses it; the page never re-serialises server data). Requests are
// answered in order. Messages:
//   {type:'init', chainId, segmentKeys, leafKeys}               -> {type:'ready', keys} | {type:'error'}
//   {type:'cached', ids}                                        -> {type:'cached', ids} (the ones kept here)
//   {type:'verify', response, filter, ids, proofs: {id: bytes}} -> {type:'checked', id, ms, error}* then {type:'verified', report, logs} | {type:'error'}
//   {type:'compare', response}                                  -> {type:'compared', result} | {type:'error'}
// Transaction and receipt proofs (no init needed):
//   {type:'items-request', chainId, kind, target}               -> {type:'items-requested', method, params} | {type:'error'}
//   {type:'items-verify', response}                             -> {type:'items-verified', report, proven, transactions, transaction, transactionUnproven} | {type:'error'}
//   {type:'items-compare', which, response}                     -> {type:'compared', result} | {type:'error'}
// Block confirmation on Ethereum (every message carries checkpoint: {root, slot} and now, in seconds):
//   {type:'anchor-restore', saved}                              -> {type:'anchor-done', result} | {type:'error'} (saved: verified again)
//   {type:'anchor-bootstrap', response}                         -> {type:'anchor-done', result} | {type:'error'}
//   {type:'anchor-updates', response}                           -> {type:'anchor-done', result} | {type:'error'}
//   {type:'anchor-verify', chainId, number, hash, response}     -> {type:'anchor-verified', result, saved} | {type:'error'}
//     result.ok, or result.kind 'need-bootstrap' / 'need-updates' (from, count): what to fetch first.
// Errors carry the verifier's kind: 'rejected' or 'malformed-input' (Willow's answer failed a
// check), 'invalid-filter', 'invalid-keys', 'invalid-request' (what was typed), 'unknown-fork',
// 'stale-checkpoint', 'unreachable', 'stale-head' (the light client cannot start, follow or say); or 'internal' when
// something threw (out of memory, a wasm trap, the module not loading). The verifier returns its
// verdicts and never throws them, so 'internal' is never a verdict on an answer, though a very
// large answer can still run the browser out of memory. Kinds are never guessed from message
// text, which can carry strings the server chose.
import initLogs, { Verifier } from './__GLUE__';
import initItems, { ItemVerifier } from './__ITEMS_GLUE__';
import initAnchor, { Anchor } from './__ANCHOR_GLUE__';

// Proofs that passed in the last check, for a repeat check of the same blocks. A new check
// releases every proof it does not use, and nothing is kept from a large one.
const KEEP = 20;
let kept = new Map();
let verifier = null;
let items = null;
// Ethereum's light client: the committees it verified live as long as the worker.
let anchor = null;
// Each module loads once, when a message first needs it. Every message awaits its module and reports a failed load.
let logsReady = null;
let itemsReady = null;
let anchorReady = null;
const logsModule = () => (logsReady ??= initLogs({ module_or_path: new URL('./__WASM__', import.meta.url) }));
const itemsModule = () => (itemsReady ??= initItems({ module_or_path: new URL('./__ITEMS_WASM__', import.meta.url) }));
const anchorModule = () => (anchorReady ??= initAnchor({ module_or_path: new URL('./__ANCHOR_WASM__', import.meta.url) }));
const moduleFor = (type) => (type.startsWith('items-') ? itemsModule() : type.startsWith('anchor-') ? anchorModule() : logsModule());

const text = (e) => String(e && e.message ? e.message : e);
const crashed = (step, e) => postMessage({ type: 'error', step, kind: 'internal', message: text(e) });
const refused = (step, r) => postMessage({ type: 'error', step, kind: r.kind, message: r.message });

// One message at a time, in order: each waits for the one before it.
let queue = Promise.resolve();
onmessage = ({ data: m }) => {
  queue = queue.then(() => handle(m)).catch((e) => crashed(m.type, e));
};

async function handle(m) {
  try {
    await moduleFor(m.type);
  } catch (e) {
    return crashed('load', e);
  }
  if (m.type === 'init') {
    try {
      if (verifier) verifier.free();
      verifier = new Verifier(m.chainId, m.segmentKeys || '', m.leafKeys || '');
      const keys = JSON.parse(verifier.keys());
      if (keys.ok) postMessage({ type: 'ready', keys });
      else refused('init', keys);
    } catch (e) {
      crashed('init', e);
    }
  } else if (m.type === 'cached') {
    postMessage({ type: 'cached', ids: m.ids.filter((id) => kept.has(id)) });
  } else if (m.type === 'verify') {
    const proofs = new Map(Object.entries(m.proofs));
    for (const id of m.ids) if (!proofs.has(id) && kept.has(id)) proofs.set(id, kept.get(id));
    kept = new Map();
    const passed = [];
    let t0 = 0;
    const getProof = (id) => {
      t0 = performance.now();
      return proofs.get(id);
    };
    const onChecked = (id, error) => {
      if (!error) passed.push(id);
      postMessage({ type: 'checked', id, ms: performance.now() - t0, error: error ?? null });
    };
    const start = performance.now();
    try {
      const r = JSON.parse(verifier.verify(m.response, m.filter, getProof, onChecked));
      if (!r.ok) return refused('verify', r);
      if (passed.length <= KEEP) for (const id of passed) kept.set(id, proofs.get(id));
      postMessage({ type: 'verified', report: r.report, logs: r.logs, ms: performance.now() - start });
    } catch (e) {
      crashed('verify', e);
    } finally {
      proofs.clear();
    }
  } else if (m.type === 'compare') {
    try {
      const r = JSON.parse(verifier.compare(m.response));
      if (r.ok) postMessage({ type: 'compared', tag: m.tag, result: r.result });
      else refused('compare', r);
    } catch (e) {
      crashed('compare', e);
    }
  } else if (m.type === 'items-request') {
    try {
      if (items) items.free();
      items = new ItemVerifier(m.chainId);
      const r = JSON.parse(items.request(m.kind, m.target));
      if (r.ok) postMessage({ type: 'items-requested', method: r.method, params: r.params });
      else refused('items-request', r);
    } catch (e) {
      crashed('items-request', e);
    }
  } else if (m.type === 'items-verify') {
    try {
      const r = JSON.parse(items.verify(m.response));
      if (r.ok) postMessage({ type: 'items-verified', report: r.report, proven: r.proven, transactions: r.provenTransactions ?? null, transaction: r.provenTransaction ?? null, transactionUnproven: r.provenTransactionUnproven ?? [] });
      else refused('items-verify', r);
    } catch (e) {
      crashed('items-verify', e);
    }
  } else if (m.type.startsWith('anchor-')) {
    try {
      if (!anchor) anchor = new Anchor(m.checkpoint.root, m.checkpoint.slot);
      let r;
      if (m.type === 'anchor-restore') r = JSON.parse(anchor.restore(m.saved, m.now));
      else if (m.type === 'anchor-bootstrap') r = JSON.parse(anchor.bootstrap(m.response, m.now));
      else if (m.type === 'anchor-updates') r = JSON.parse(anchor.updates(m.response));
      else if (m.type === 'anchor-verify') r = JSON.parse(anchor.verify(m.chainId, m.number, m.hash, m.response, m.now));
      else return crashed(m.type, 'unknown message');
      if (m.type === 'anchor-verify' && (r.ok || r.kind === 'need-bootstrap' || r.kind === 'need-updates')) {
        if (r.kind === 'need-bootstrap') anchor.reset();
        postMessage({ type: 'anchor-verified', result: r, saved: r.ok ? anchor.saved() : null });
      } else if (r.ok) postMessage({ type: 'anchor-done', result: r });
      else refused(m.type, r);
    } catch (e) {
      crashed(m.type, e);
    }
  } else if (m.type === 'items-compare') {
    try {
      const r = JSON.parse(items.compare(m.which, m.response));
      if (r.ok) postMessage({ type: 'compared', result: r.result });
      else refused('items-compare', r);
    } catch (e) {
      crashed('items-compare', e);
    }
  }
}
