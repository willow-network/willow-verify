// Runs the wasm verifiers off the main thread: the log verifier (SP1 proofs, and every check of
// an answer but the OpenVM proofs' own), its OpenVM proof module, the transaction, receipt, block
// and fee verifier, the state, call, estimate and trace verifier, and the light clients that
// confirm blocks (Ethereum's, Sepolia's and BNB Smart Chain's), each loaded on first use. The
// OpenVM module loads when a response's proofs include an OpenVM one; the log verifier calls it
// for each such proof, under the key it chose. `response` is a JSON-RPC response body as the
// server sent it (the wasm parses it; the page never re-serialises server data). Requests are
// handled in order. Messages:
//   {type:'init', chainId, segmentKeys, leafKeys}               -> {type:'ready', keys} | {type:'error'}
//   {type:'cached', ids}                                        -> {type:'cached', ids} (the ones kept here)
//   {type:'verify', response, filter, ids, proofs: {id: bytes}} -> {type:'checked', id, ms, error, system}* then {type:'verified', report, logs} | {type:'error'}
//     (system: 'sp1' or 'openvm', the proof's own, read from its bytes in the wasm)
//   {type:'compare', response}                                  -> {type:'compared', result} | {type:'error'}
// Transaction, receipt, block and fee proofs (no init needed; kind: transaction, receipt,
// blockReceipts, block, fees, feeHistory):
//   {type:'items-request', chainId, kind, target}               -> {type:'items-requested', method, params} | {type:'error'}
//   {type:'items-verify', response}                             -> {type:'items-verified', report, proven, transactions, transaction, transactionUnproven} | {type:'error'}
//   {type:'items-block', full}                                  -> {type:'items-block', proven, unproven} | {type:'error'} (the last block proof verified)
//   {type:'items-compare', which, response}                     -> {type:'compared', result} | {type:'error'}
// State, call, estimate and trace proofs (kind: state, call, estimate, trace; target: the proof method's params as JSON text):
//   {type:'state-request', chainId, kind, target}               -> {type:'state-requested', method, params} | {type:'error'}
//   {type:'state-verify', response}                             -> {type:'state-verified', report, proven} | {type:'error'}
//   {type:'state-compare', method, response}                    -> {type:'compared', result} | {type:'error'}
// Block confirmation (every message carries checkpoint: {root, slot}, Ethereum's start, and now, in seconds; for a chain
// another light client anchors, lightClient 'sepolia' or 'bnb' and start, where it starts: {root, slot} or {number, hash}):
//   {type:'anchor-restore', saved}                              -> {type:'anchor-done', result} | {type:'error'} (saved: verified again)
//   {type:'anchor-bootstrap', response}                         -> {type:'anchor-done', result} | {type:'error'}
//   {type:'anchor-updates', response}                           -> {type:'anchor-done', result} | {type:'error'}
//   {type:'anchor-verify', chainId, number, hash, response}     -> {type:'anchor-verified', result, saved} | {type:'error'}
//     result.ok, or result.kind 'need-bootstrap' / 'need-updates' (from, count, lightClient): what to fetch first.
// Errors carry the verifier's kind: 'rejected' or 'malformed-input' (Willow's answer failed a
// check; an items error also carries the 'fault' that failed; a state error's kind is what failed,
// such as 'account', 'slot', 'outcome'), 'invalid-filter', 'invalid-keys', 'invalid-request' (what was typed), 'unknown-fork',
// 'stale-checkpoint', 'unreachable', 'stale-head' (the light client cannot start, follow or say); or 'internal' when
// something threw (out of memory, a wasm trap, the module not loading). The verifier returns its
// verdicts and never throws them, so 'internal' is never a verdict on an answer, though a very
// large answer can still run the browser out of memory. Kinds are never guessed from message
// text, which can carry strings the server chose.
import initLogs, { Verifier } from './__GLUE__';
import initOpenVm, { OpenVmVerifier } from './__OPENVM_GLUE__';
import initItems, { ItemVerifier } from './__ITEMS_GLUE__';
import initState, { StateVerifier } from './__STATE_GLUE__';
import initAnchor, { Anchor } from './__ANCHOR_GLUE__';

// Proofs that passed in the last check, for a repeat check of the same blocks. A new check
// releases every proof it does not use, and nothing is kept from a large one.
const KEEP = 20;
let kept = new Map();
let verifier = null;
// The OpenVM proof check, made once: it holds the pinned verifying keys.
let openvm = null;
let items = null;
let state = null;
// The light clients (Ethereum's, Sepolia's, BNB Smart Chain's): what they verified lives as long as the worker.
let anchor = null;
// The other light clients given their start, each once: a start forgets what that light client verified.
const started = new Set();
// Each module loads once, when a message first needs it. Every message awaits its module and reports a failed load.
let logsReady = null;
let openvmReady = null;
let itemsReady = null;
let stateReady = null;
let anchorReady = null;
const logsModule = () => (logsReady ??= initLogs({ module_or_path: new URL('./__WASM__', import.meta.url) }));
const openvmModule = () => (openvmReady ??= initOpenVm({ module_or_path: new URL('./__OPENVM_WASM__', import.meta.url) }));
// An OpenVM proof's first bytes ("OVM2"). Only decides whether to load the OpenVM module: which
// system a proof is, and the keys it is checked under, are decided in the log verifier.
const isOpenVm = (b) => b && b.length >= 4 && b[0] === 0x4f && b[1] === 0x56 && b[2] === 0x4d && b[3] === 0x32;

/// The OpenVM check, loaded and made once, after confirming it verifies under exactly the OpenVM
/// keys the log verifier trusts (both read pins/program-keys.txt; a mismatch is a broken build).
async function openvmCheck() {
  if (openvm) return openvm;
  await openvmModule();
  const o = new OpenVmVerifier();
  const k = JSON.parse(o.keys());
  const want = JSON.parse(verifier.keys()).pinned.openvm;
  if (!k.ok || k.leaf !== want.leaf[0] || k.l1 !== want.segment[0]) {
    o.free();
    throw new Error(`the OpenVM module trusts other programs than the log verifier: ${JSON.stringify(k)}`);
  }
  openvm = o;
  return openvm;
}
const itemsModule = () => (itemsReady ??= initItems({ module_or_path: new URL('./__ITEMS_WASM__', import.meta.url) }));
const stateModule = () => (stateReady ??= initState({ module_or_path: new URL('./__STATE_WASM__', import.meta.url) }));
const anchorModule = () => (anchorReady ??= initAnchor({ module_or_path: new URL('./__ANCHOR_WASM__', import.meta.url) }));
const moduleFor = (type) =>
  type.startsWith('items-') ? itemsModule() : type.startsWith('state-') ? stateModule() : type.startsWith('anchor-') ? anchorModule() : logsModule();

const text = (e) => String(e && e.message ? e.message : e);
const crashed = (step, e) => postMessage({ type: 'error', step, kind: 'internal', message: text(e) });
const refused = (step, r) => postMessage({ type: 'error', step, kind: r.kind, message: r.message, ...(r.fault ? { fault: r.fault } : {}) });

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
    const onChecked = (id, error, system) => {
      if (!error) passed.push(id);
      postMessage({ type: 'checked', id, ms: performance.now() - t0, error: error ?? null, system: system ?? null });
    };
    const start = performance.now();
    try {
      // Without the OpenVM module, an OpenVM proof fails in the log verifier ("no OpenVM module").
      let checkOpenVm;
      if ([...proofs.values()].some(isOpenVm)) {
        const o = await openvmCheck();
        checkOpenVm = (key, proof, pv) => o.check(key, proof, pv);
      }
      const r = JSON.parse(verifier.verify(m.response, m.filter, getProof, onChecked, checkOpenVm));
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
      const lc = m.lightClient || null;
      if (lc && !started.has(lc)) {
        const s = JSON.parse(anchor.start(lc, JSON.stringify(m.start)));
        if (!s.ok) return refused(m.type, s);
        started.add(lc);
      }
      let r;
      if (m.type === 'anchor-restore') r = JSON.parse(lc ? anchor.restoreFor(lc, m.saved, m.now) : anchor.restore(m.saved, m.now));
      else if (m.type === 'anchor-bootstrap') r = JSON.parse(lc ? anchor.bootstrapFor(lc, m.response, m.now) : anchor.bootstrap(m.response, m.now));
      else if (m.type === 'anchor-updates') r = JSON.parse(lc ? anchor.updatesFor(lc, m.response) : anchor.updates(m.response));
      else if (m.type === 'anchor-verify') r = JSON.parse(anchor.verify(m.chainId, m.number, m.hash, m.response, m.now));
      else return crashed(m.type, 'unknown message');
      if (m.type === 'anchor-verify' && (r.ok || r.kind === 'need-bootstrap' || r.kind === 'need-updates')) {
        // A need-bootstrap names the light client it is for (none: Ethereum's), which starts over.
        if (r.kind === 'need-bootstrap') r.lightClient ? anchor.resetFor(r.lightClient) : anchor.reset();
        postMessage({ type: 'anchor-verified', result: r, saved: r.ok ? (lc ? anchor.savedFor(lc) : anchor.saved()) : null });
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
  } else if (m.type === 'items-block') {
    try {
      const r = JSON.parse(items.block(!!m.full));
      if (r.ok) postMessage({ type: 'items-block', proven: r.proven, unproven: r.unproven });
      else refused('items-block', r);
    } catch (e) {
      crashed('items-block', e);
    }
  } else if (m.type === 'state-request') {
    try {
      if (state) state.free();
      state = new StateVerifier(m.chainId);
      const r = JSON.parse(state.request(m.kind, m.target));
      if (r.ok) postMessage({ type: 'state-requested', method: r.method, params: r.params });
      else refused('state-request', r);
    } catch (e) {
      crashed('state-request', e);
    }
  } else if (m.type === 'state-verify') {
    try {
      const r = JSON.parse(state.verify(m.response));
      if (r.ok) postMessage({ type: 'state-verified', report: r.report, proven: r.proven });
      else refused('state-verify', r);
    } catch (e) {
      crashed('state-verify', e);
    }
  } else if (m.type === 'state-compare') {
    try {
      const r = JSON.parse(state.compare(m.method, m.response));
      if (r.ok) postMessage({ type: 'compared', result: r.result });
      else refused('state-compare', r);
    } catch (e) {
      crashed('state-compare', e);
    }
  }
}
