// The lying server's table (design 6.3, W7): one mutation per field of every proof shape, and the
// listed attacks on anchors and batches. `kind` is the client's `data.kind`, which for proof
// fields is the verifier's own: W1's Fault (blockproof tests_block.rs, tests_fees.rs) and W2's
// Kind (state-verify tests.rs) name the same mutations the same way.
import { hexToBytes, keccakHex, toNumber, toQuantity } from '../../src/hex.js';
import type { WillowVerifier } from '../../src/client.js';
import { flip, type Mutation, type Req } from './lying-proxy.js';
import { fixture, type World } from './world.js';

export interface Case {
  name: string;
  chainId: number;
  /** The block that is the head on the mock server ("w1": the W1 fixture's; "w2": W2's Ethereum block). */
  head: 'w1' | 'w2';
  ask: (c: WillowVerifier) => Promise<unknown>;
  mutation: Mutation;
  /** The error the app must get, or `null`/`values` for denials that may pass (an absence) and benign reorders. */
  expect: { code: -32098 | -32099; kind: string } | { outcome: 'null' } | { outcome: 'values'; check: (v: unknown) => boolean };
  /** Mock-world only (needs fixtures the live server does not hold, or a world change). */
  fixtureOnly?: boolean;
  /** Advance the client's clock this many seconds after the warm-up (the server's head moves with it). */
  replayAfterSec?: number;
  /** false: no unmutated eth_blockNumber first (the mutation must meet the client's first requests). */
  warm?: false;
  /** The honest mock server's part, where the fixtures hold none (mock world only). */
  setup?: (w: World) => void;
}

const V = (kind: string) => ({ code: -32098 as const, kind });
const U = (kind: string) => ({ code: -32099 as const, kind });

/** A mutation of `method`'s result: `f` changes it in place (return false when it does not apply). */
function on(method: string, f: (r: any, req: Req) => boolean | void, name = method): Mutation {
  return { name, item: (resp, req) => req.method === method && !!resp.result && f(resp.result, req) !== false };
}

const swap = (a: any[], i: number, j: number) => ([a[i], a[j]] = [a[j], a[i]]);

const eoa = () => fixture('state/eoa.json').request.params[0];
const usdc = () => fixture('state/usdc-slots-code.json');
const callBalance = () => fixture('call/call-usdc-balance.json');
const transfer = () => fixture('call/estimate-transfer.json');
const askBlock = (c: WillowVerifier) => c.request({ method: 'eth_getBlockByNumber', params: ['latest', false] });
const askBalance = (c: WillowVerifier) => c.request({ method: 'eth_getBalance', params: [eoa(), 'latest'] });

export function cases(): Case[] {
  const items = (file: string) => fixture(`../../../verify/test/items/${file}`);
  const out: Case[] = [];
  const add = (c: Case) => out.push(c);

  // ---- willow_getBlockProof (I1) ----
  const block = (name: string, f: (r: any) => boolean | void, kind: string) => add({ name: `block: ${name}`, chainId: 8453, head: 'w1', ask: askBlock, mutation: on('willow_getBlockProof', f), expect: V(kind) });
  block('a byte of the header', (r) => void (r.header = flip(r.header, 10)), 'header');
  block('blockHash', (r) => void (r.blockHash = flip(r.blockHash)), 'header');
  block('blockNumber', (r) => void (r.blockNumber = toQuantity(toNumber(r.blockNumber)! + 1)), 'header');
  block('chainId', (r) => void (r.chainId = '0x1'), 'wrong-chain');
  block('a transaction changed', (r) => void (r.transactions[1] = flip(r.transactions[1], 6)), 'transactions');
  block('a transaction dropped', (r) => void r.transactions.pop(), 'transactions');
  block('transactions reordered', (r) => void swap(r.transactions, 1, 2), 'transactions');
  block('no transactions', (r) => void delete r.transactions, 'missing');
  block('a withdrawal on the OP stack', (r) => void (r.withdrawals = '0xc4c3808080'), 'withdrawals');
  block("a deposit's receipt", (r) => void (r.receiptProofs[0].value = flip(r.receiptProofs[0].value, 4)), 'receipts');
  add({
    name: 'block: another block for a blockHash request',
    chainId: 1,
    head: 'w1',
    ask: (c) => c.request({ method: 'eth_getBlockByHash', params: [fixture('block/ethereum-26391511.json').result.blockHash, false] }),
    mutation: on('willow_getBlockProof', (r) => void Object.assign(r, structuredClone(fixture('block/ethereum-17034874.json').result))),
    expect: V('wrong-block'),
  });
  const pre = () => fixture('block/ethereum-1500000.json').result;
  add({ name: 'block: an uncle changed', chainId: 1, head: 'w1', ask: (c) => c.request({ method: 'eth_getBlockByHash', params: [pre().blockHash, false] }), mutation: on('willow_getBlockProof', (r) => void (r.uncles[0] = flip(r.uncles[0], 8))), expect: V('uncles') });
  // Uncles both given and left unproven: malformed, as W1's tamper suite has it (tests_block.rs).
  add({ name: 'block: uncles marked unproven', chainId: 1, head: 'w1', ask: (c) => c.request({ method: 'eth_getBlockByHash', params: [pre().blockHash, false] }), mutation: on('willow_getBlockProof', (r) => void (r.unproven = ['uncles', 'size'])), expect: V('malformed') });
  add({
    name: "block: a response from another chain",
    chainId: 8453,
    head: 'w1',
    ask: askBlock,
    mutation: on('willow_getBlockProof', (r) => void Object.assign(r, structuredClone(fixture('block/ink-60901963.json').result))),
    expect: V('wrong-chain'),
  });

  // ---- willow_getFeeProof and willow_getFeeHistoryProof (I4) ----
  const fee = (name: string, f: (r: any) => boolean | void, kind: string) => add({ name: `fees: ${name}`, chainId: 1, head: 'w1', ask: (c) => c.request({ method: 'eth_gasPrice' }), mutation: on('willow_getFeeProof', f), expect: V(kind) });
  fee('another formula', (r) => void (r.formula = 'willow-fee-v2'), 'unknown-formula');
  fee('chainId', (r) => void (r.chainId = '0xa'), 'wrong-chain');
  fee('blockHash', (r) => void (r.blockHash = flip(r.blockHash)), 'header');
  fee('blockNumber', (r) => void (r.blockNumber = toQuantity(toNumber(r.blockNumber)! - 1)), 'header');
  fee('a byte of the newest header', (r) => void (r.headers[3] = flip(r.headers[3], 10)), 'header');
  fee('two headers swapped', (r) => void swap(r.headers, 0, 1), 'broken-link');
  fee('a header dropped', (r) => void r.headers.shift(), 'wrong-range');
  fee('a body for another block', (r) => void swap(r.bodies, 0, 1), 'body');
  fee('a body dropped', (r) => void r.bodies.pop(), 'missing');
  fee('a transaction of a body', (r) => void (r.bodies[0].transactions[0] = flip(r.bodies[0].transactions[0], 6)), 'transactions');
  const history = (name: string, f: (r: any) => boolean | void, kind: string) =>
    add({ name: `fee history: ${name}`, chainId: 8453, head: 'w1', ask: (c) => c.request({ method: 'eth_feeHistory', params: ['0x4', 'latest', [10, 50, 90]] }), mutation: on('willow_getFeeHistoryProof', f), expect: V(kind) });
  history('two headers swapped', (r) => void swap(r.headers, 1, 2), 'broken-link');
  history("a receipt's cumulative gas", (r) => void (r.bodies[0].receipts[0] = flip(r.bodies[0].receipts[0], 6)), 'receipts');
  history('another formula', (r) => void (r.formula = 'x'), 'unknown-formula');
  history('chainId', (r) => void (r.chainId = '0x1'), 'wrong-chain');

  // ---- willow_getStateProof (I2) ----
  const state = (name: string, ask: (c: WillowVerifier) => Promise<unknown>, f: (r: any) => boolean | void, kind: string) => add({ name: `state: ${name}`, chainId: 1, head: 'w2', ask, mutation: on('willow_getStateProof', f), expect: V(kind) });
  state('a byte of the header', askBalance, (r) => void (r.header = flip(r.header, 10)), 'header');
  state('blockHash', askBalance, (r) => void (r.blockHash = flip(r.blockHash)), 'header');
  state('blockNumber', askBalance, (r) => void (r.blockNumber = '0x1'), 'header');
  state('chainId', askBalance, (r) => void (r.chainId = '0x2'), 'wrong-chain');
  state('balance', askBalance, (r) => void (r.account.balance = '0x1'), 'account');
  state('a lower pending nonce', (c) => c.request({ method: 'eth_getTransactionCount', params: [eoa(), 'pending'] }), (r) => void (r.account.nonce = '0x1'), 'account');
  state('codeHash', askBalance, (r) => void (r.account.codeHash = flip(r.account.codeHash)), 'account');
  state('storageHash', askBalance, (r) => void (r.account.storageHash = flip(r.account.storageHash)), 'account');
  state('an account proof node', askBalance, (r) => void (r.account.accountProof[1] = flip(r.account.accountProof[1], 10)), 'account');
  state('a proof of another account', askBalance, (r) => void (r.account = structuredClone(usdc().result.account)), 'account');
  const slot = (c: WillowVerifier) => c.request({ method: 'eth_getStorageAt', params: [usdc().request.params[0], usdc().request.params[1][0], 'latest'] });
  state('a slot value', slot, (r) => void (r.account.storageProof[0].value = '0x1'), 'slot');
  state('the proof of another slot', slot, (r) => void (r.account.storageProof[0].key = '0x2'), 'slot');
  state('a storage proof node', slot, (r) => void (r.account.storageProof[0].proof[1] = flip(r.account.storageProof[0].proof[1], 10)), 'slot');
  const code = (c: WillowVerifier) => c.request({ method: 'eth_getCode', params: [usdc().request.params[0], 'latest'] });
  state('code swapped (same length, other hash)', code, (r) => void (r.code = flip(r.code)), 'code');
  state('code left out', code, (r) => void delete r.code, 'code');
  add({
    name: 'state: another block for the block asked',
    chainId: 1,
    head: 'w2',
    ask: askBalance,
    mutation: on('willow_getStateProof', (r) => void Object.assign(r, { ...['chainId', 'blockNumber', 'blockHash', 'header'].reduce((o, k) => ({ ...o, [k]: fixture('block/ethereum-26391511.json').result[k] }), {}) })),
    expect: V('wrong-block'),
  });

  // ---- willow_getCallProof and willow_getEstimateGasProof (I3) ----
  const askCall = (c: WillowVerifier) => c.request({ method: 'eth_call', params: [callBalance().request.params[0], 'latest'] });
  const call = (name: string, f: (r: any) => boolean | void, kind: string) => add({ name: `call: ${name}`, chainId: 1, head: 'w2', ask: askCall, mutation: on('willow_getCallProof', f), expect: V(kind) });
  call('an account dropped from the witness', (r) => void r.witness.accounts.pop(), 'missing-read');
  call('a shared node', (r) => void (r.witness.nodes[0] = flip(r.witness.nodes[0], 10)), 'account');
  call('code left out', (r) => void (r.witness.codes = {}), 'code');
  call('another result', (r) => void (r.result = { returned: flip(r.result.returned) }), 'outcome');
  call('a revert for a result', (r) => void (r.result = { reverted: '0x' }), 'outcome');
  call('a byte of the header', (r) => void (r.header = flip(r.header, 10)), 'header');
  call('chainId', (r) => void (r.chainId = '0x2'), 'wrong-chain');
  call('blockHash', (r) => void (r.blockHash = flip(r.blockHash)), 'header');
  call('blockNumber', (r) => void (r.blockNumber = toQuantity(toNumber(r.blockNumber)! + 1)), 'header');
  const askEstimate = (c: WillowVerifier) => c.request({ method: 'eth_estimateGas', params: [transfer().request.params[0]] });
  const est = (name: string, f: (r: any) => boolean | void, kind: string) => add({ name: `estimate: ${name}`, chainId: 1, head: 'w2', ask: askEstimate, mutation: on('willow_getEstimateGasProof', f), expect: V(kind) });
  est('another algorithm', (r) => void (r.algorithm = 'geth-v1.16.0-estimate'), 'algorithm');
  est('the estimate + 1', (r) => void (r.estimate = { gas: toQuantity(toNumber(r.estimate.gas)! + 1) }), 'estimate');
  est('the estimate - 1', (r) => void (r.estimate = { gas: toQuantity(toNumber(r.estimate.gas)! - 1) }), 'estimate');
  est('an account dropped from the witness', (r) => void r.witness.accounts.pop(), 'missing-read');

  // ---- transaction, receipt and block-receipts proofs (shipped) ----
  const receipt = items('eth-receipt.json');
  const askReceipt = (c: WillowVerifier) => c.request({ method: 'eth_getTransactionReceipt', params: [receipt.target] });
  add({ name: 'receipt: a proof node', chainId: 1, head: 'w1', ask: askReceipt, mutation: on('willow_getReceiptProof', (r) => void (r.receiptProofs.at(-1).proof[0] = flip(r.receiptProofs.at(-1).proof[0], 40))), expect: V('receipts') });
  add({ name: 'receipt: a byte of the header', chainId: 1, head: 'w1', ask: askReceipt, mutation: on('willow_getReceiptProof', (r) => void (r.header = flip(r.header, 100))), expect: V('header') });
  const tx = items('arb-tx.json');
  add({ name: 'transaction: a proof node', chainId: 42161, head: 'w1', ask: (c) => c.request({ method: 'eth_getTransactionByHash', params: [tx.target] }), mutation: on('willow_getTransactionProof', (r) => void (r.transactionProofs.at(-1).proof[0] = flip(r.transactionProofs.at(-1).proof[0], 40))), expect: V('transactions') });
  const uni = items('uni-block.json');
  add({ name: 'block receipts: a receipt', chainId: 130, head: 'w1', ask: (c) => c.request({ method: 'eth_getBlockReceipts', params: [uni.target] }), mutation: on('willow_getBlockReceiptsProof', (r) => void (r.receipts[0] = flip(r.receipts[0], 20))), expect: V('receipts') });
  add({
    name: 'receipt: "not found" for a receipt the server has is an absence (a denial, never a value)',
    chainId: 1,
    head: 'w1',
    ask: askReceipt,
    mutation: { name: 'not found', item: (resp, req) => req.method === 'willow_getReceiptProof' && (delete resp.result, (resp.error = { code: -32000, message: `not found in this store: transaction ${receipt.target} is not in the full-block tier` }), true) },
    expect: { outcome: 'null' },
  });

  // ---- anchors ----
  const old = fixture('block/ethereum-17034874.json').result;
  add({
    name: 'anchor: the anchor of another block',
    chainId: 1,
    head: 'w1',
    ask: (c) => c.request({ method: 'eth_getBlockByNumber', params: [old.blockNumber, false] }),
    mutation: { name: 'another anchor', item: (resp, req, _c) => req.method === 'willow_getBlockAnchor' && req.params[0] === old.blockNumber && ((resp.result = structuredClone(anchorSeen.get('1:latest'))), true) },
    expect: V('anchor-rejected'),
  });
  add({
    name: "anchor: a hash the validators did not sign",
    chainId: 1,
    head: 'w1',
    ask: (c) => c.request({ method: 'eth_getBlockByNumber', params: [old.blockNumber, false] }),
    mutation: on('willow_getBlockAnchor', (r, req) => req.params[0] === old.blockNumber && void (r.mock.hash = flip(r.mock.hash))),
    expect: V('anchor-rejected'),
  });
  add({
    name: 'anchor: a stale head replayed (a denial: unverifiable, not a verdict)',
    chainId: 1,
    head: 'w1',
    ask: (c) => c.request({ method: 'eth_blockNumber' }),
    replayAfterSec: 1200,
    mutation: { name: 'replay', item: (resp, req) => req.method === 'willow_getBlockAnchor' && req.params[0] === 'latest' && !!anchorSeen.get('1:latest') && ((resp.result = structuredClone(anchorSeen.get('1:latest'))), true) },
    expect: U('anchor-stale-head'),
  });

  // ---- batches and endpoints ----
  add({
    name: 'batch: an item dropped',
    chainId: 1,
    head: 'w2',
    ask: (c) => Promise.all([askBalance(c), askCall(c)]),
    mutation: { name: 'drop', batch: (resps, reqs) => reqs.some((r) => r.method === 'willow_getCallProof') && (resps.splice(resps.findIndex((x) => reqs.find((r) => r.id === x.id)?.method === 'willow_getCallProof'), 1), true) },
    expect: V('batch'),
  });
  add({
    name: 'batch: an item twice',
    chainId: 1,
    head: 'w2',
    ask: (c) => Promise.all([askBalance(c), askCall(c)]),
    mutation: { name: 'twice', batch: (resps, reqs) => reqs.some((r) => r.method === 'willow_getCallProof') && (resps.push(structuredClone(resps[0])), true) },
    expect: V('batch'),
  });
  const two = (c: WillowVerifier) => Promise.all([c.request({ method: 'eth_getBlockByHash', params: [old.blockHash, false] }), c.request({ method: 'eth_getBlockByHash', params: [pre().blockHash, false] })]);
  add({
    name: "batch: two items' ids swapped",
    chainId: 1,
    head: 'w1',
    ask: two,
    mutation: {
      name: 'ids swapped',
      batch: (resps, reqs) => {
        const ids = reqs.filter((r) => r.method === 'willow_getBlockProof').map((r) => r.id);
        if (ids.length < 2) return false;
        const [a, b] = ids.map((id) => resps.find((x) => x.id === id));
        [a.id, b.id] = [b.id, a.id];
        return true;
      },
    },
    expect: V('wrong-block'),
  });
  add({
    name: 'batch: items reordered, ids kept (valid JSON-RPC: the values are right)',
    chainId: 1,
    head: 'w1',
    ask: two,
    mutation: { name: 'reorder', batch: (resps) => resps.length > 1 && (resps.reverse(), true) },
    expect: { outcome: 'values', check: (v: any) => v[0].hash === old.blockHash && v[1].hash === pre().blockHash },
  });
  add({
    name: 'endpoint: eth_chainId of another chain',
    chainId: 8453,
    head: 'w1',
    warm: false,
    ask: askBlock,
    mutation: { name: 'chain id', item: (resp, req) => req.method === 'eth_chainId' && ((resp.result = '0xa'), true) },
    expect: V('wrong-chain'),
  });
  add({
    name: 'eth_sendRawTransaction: another hash',
    chainId: 1,
    head: 'w2',
    fixtureOnly: true,
    setup: (w) => (w.intercept = (_c, m, p) => (m === 'eth_sendRawTransaction' ? { result: keccakHex(hexToBytes(p[0] as string)) } : undefined)),
    ask: async (c) => c.request({ method: 'eth_sendRawTransaction', params: [await signedTransfer()] }),
    mutation: { name: 'hash', item: (resp, req) => req.method === 'eth_sendRawTransaction' && ((resp.result = `0x${'00'.repeat(32)}`), true) },
    expect: V('tx-hash'),
  });
  return out;
}

/** Anchor responses the proxy has passed on, by `${chainId}:${param}`, for replays. */
export const anchorSeen = new Map<string, unknown>();

/** An EIP-1559 transfer, signed here by viem with a test key. */
async function signedTransfer(): Promise<string> {
  const { privateKeyToAccount } = await import('viem/accounts');
  const a = privateKeyToAccount('0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d');
  return a.signTransaction({ to: '0x4b1d5e7c9a3f2e6d8c0b1a2f3e4d5c6b7a8f9e01', value: 1n, gas: 21000n, nonce: 7, type: 'eip1559', maxFeePerGas: 3n, maxPriorityFeePerGas: 1n, chainId: 1 });
}
