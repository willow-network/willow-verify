// What can't be verified (design 3.3): absences and refusals pass through; values that can't be
// verified throw WillowUnverifiableError (-32099) or, with `flag`, pass marked; a proof that
// fails is always WillowVerificationError (-32098).
import { afterEach, describe, expect, it } from 'vitest';
import { UNVERIFIABLE, VERIFICATION_FAILED, WillowRpcError, WillowUnverifiableError, WillowVerificationError } from '../src/errors.js';
import { toQuantity } from '../src/hex.js';
import { blockOf, harness, headAt, type Harness } from './support/setup.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const BASE = blockOf('base-56049971.json');
const ETH = blockOf('ethereum-26391511.json');

async function on(chainId: number, unverifiable: 'throw' | 'flag' = 'throw', extra: Record<string, unknown> = {}): Promise<Harness> {
  const x = await harness({ chainId, options: { unverifiable, ...extra } });
  headAt(x.world, chainId, chainId === 1 ? ETH : BASE);
  return x;
}

describe('error codes', () => {
  it('are -32098 and -32099, with data.kind', () => {
    expect(VERIFICATION_FAILED).toBe(-32098);
    expect(UNVERIFIABLE).toBe(-32099);
    expect(new WillowVerificationError('header', 'x', 'eth_getBlockByNumber')).toMatchObject({ code: -32098, data: { kind: 'header', detail: 'x', method: 'eth_getBlockByNumber' } });
    expect(new WillowUnverifiableError('tag')).toMatchObject({ code: -32099, data: { kind: 'tag' } });
  });
});

describe('L2 "safe" and "finalized"', () => {
  it('throw by default', async () => {
    h = await on(8453);
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: ['safe', false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'tag' } });
  });

  it("with flag, the block's contents are verified and the tag's meaning is marked", async () => {
    h = await on(8453, 'flag');
    h.world.standard.set(h.world.key(8453, 'eth_getBlockByNumber', ['finalized', false]), { result: { number: toQuantity(BASE.number) } });
    const b: any = await h.client.request({ method: 'eth_getBlockByNumber', params: ['finalized', false] });
    expect(b.hash).toBe(BASE.hash);
    expect(h.events.at(-1)).toMatchObject({ status: 'unverifiable', kind: 'tag', block: BASE, level: 'signed' });
  });

  it('Ethereum "safe" is refused, as the server refuses it', async () => {
    h = await on(1);
    await expect(h.client.request({ method: 'eth_getBalance', params: [`0x${'11'.repeat(20)}`, 'safe'] })).rejects.toMatchObject({ code: -32000, message: 'not found in this store: the "safe" block tag' });
  });
});

describe('methods without a proof', () => {
  it("a value throws; with flag it passes marked; an error passes through", async () => {
    h = await on(1);
    h.world.standard.set('1|eth_coinbase|*', { result: `0x${'22'.repeat(20)}` });
    await expect(h.client.request({ method: 'eth_coinbase' })).rejects.toMatchObject({ code: -32099, data: { kind: 'method' } });
    await expect(h.client.request({ method: 'eth_mining' })).rejects.toMatchObject({ code: -32601, name: 'WillowRpcError' });
    expect(h.events.at(-1)!.status).toBe('refused');
    await h.close();
    h = await on(1, 'flag');
    h.world.standard.set('1|eth_coinbase|*', { result: `0x${'22'.repeat(20)}` });
    expect(await h.client.request({ method: 'eth_coinbase' })).toBe(`0x${'22'.repeat(20)}`);
    expect(h.events.at(-1)).toMatchObject({ status: 'unverifiable', kind: 'method' });
  });

  it('L2 eth_call and eth_estimateGas: the server refuses them (-32601), and that passes through', async () => {
    h = await on(8453);
    h.world.standard.set('8453|eth_call|*', { error: { code: -32601, message: 'eth_call is not served on chain 8453' } });
    h.world.standard.set('8453|eth_estimateGas|*', { error: { code: -32601, message: 'eth_estimateGas is not served on chain 8453: Willow does not execute this chain\'s transactions yet' } });
    await expect(h.client.request({ method: 'eth_call', params: [{ to: `0x${'11'.repeat(20)}` }, 'latest'] })).rejects.toMatchObject({ code: -32601 });
    await expect(h.client.request({ method: 'eth_estimateGas', params: [{ to: `0x${'11'.repeat(20)}` }] })).rejects.toMatchObject({ code: -32601 });
  });

  it('eth_call with overrides the verifier does not run (movePrecompileToAddress): a value from the server is unverifiable', async () => {
    h = await on(1);
    h.world.standard.set('1|eth_call|*', { result: '0x01' });
    const a = `0x${'11'.repeat(20)}`;
    await expect(h.client.request({ method: 'eth_call', params: [{ to: a }, 'latest', { [a]: { movePrecompileToAddress: `0x${'22'.repeat(20)}` } }] })).rejects.toMatchObject({
      code: -32099,
      data: { kind: 'method' },
    });
  });

  it('local methods are served here', async () => {
    h = await on(8453);
    expect(await h.client.request({ method: 'eth_chainId' })).toBe('0x2105');
    expect(await h.client.request({ method: 'net_version' })).toBe('8453');
    expect(await h.client.request({ method: 'eth_accounts' })).toEqual([]);
    expect(await h.client.request({ method: 'eth_syncing' })).toBe(false);
    expect(await h.client.request({ method: 'web3_clientVersion' })).toMatch(/^willow-rpc\//);
    expect(h.events.every((e) => e.status === 'local')).toBe(true);
  });
});

describe('refusals and proofs not served', () => {
  it('a proof method this key or path does not serve (-32601) is unverifiable; with flag, the standard response passes', async () => {
    h = await on(8453);
    h.world.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { error: { code: -32601, message: 'the method willow_getBlockProof is not available for this API key: its plan has no proofs' } } : undefined);
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'not-on-plan' } });
    await h.close();
    h = await on(8453, 'flag');
    h.world.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { error: { code: -32601, message: 'no proofs' } } : undefined);
    h.world.standard.set(h.world.key(8453, 'eth_getBlockByNumber', ['latest', false]), { result: { number: '0x1', plain: true } });
    expect(await h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })).toEqual({ number: '0x1', plain: true });
    expect(h.events.at(-1)).toMatchObject({ status: 'unverifiable', kind: 'not-on-plan' });
  });

  it('state on the public path is unverifiable (-32601 there)', async () => {
    h = await harness({ chainId: 1, key: 'public' });
    headAt(h.world, 1, ETH);
    await expect(h.client.request({ method: 'eth_getBalance', params: [`0x${'11'.repeat(20)}`, 'latest'] })).rejects.toMatchObject({ code: -32099, data: { kind: 'not-on-plan' } });
  });

  it('any other refusal passes through as the server sent it', async () => {
    h = await on(8453);
    h.world.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { error: { code: -32005, message: 'rate limited', data: { retryAfterSeconds: 1 } } } : undefined);
    const e = await h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] }).catch((x) => x);
    expect(e).toBeInstanceOf(WillowRpcError);
    expect(e).toMatchObject({ code: -32005, message: 'rate limited', data: { retryAfterSeconds: 1 } });
  });

  it('an anchor the server can\'t give makes the block unverifiable; with flag the proven block passes marked', async () => {
    h = await on(1);
    const old = blockOf('ethereum-17034874.json');
    h.world.anchors.delete(`1:${old.number}`);
    await expect(h.client.request({ method: 'eth_getBlockByHash', params: [old.hash, false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-refused' } });
    await h.close();
    h = await on(1, 'flag');
    h.world.anchors.delete(`1:${old.number}`);
    const b: any = await h.client.request({ method: 'eth_getBlockByHash', params: [old.hash, false] });
    expect(b.hash).toBe(old.hash);
    expect(h.events.at(-1)).toMatchObject({ status: 'unverifiable', kind: 'anchor-refused', block: old });
  });

  it('eth_getLogs over more than 8 blocks without log proofs is unverifiable', async () => {
    h = await on(1);
    h.world.intercept = (_c, m) => (m === 'willow_getLogsProof' ? { error: { code: -32601, message: 'the method willow_getLogsProof is not available for this API key: its plan has no proofs' } } : undefined);
    await expect(h.client.request({ method: 'eth_getLogs', params: [{ fromBlock: toQuantity(ETH.number - 20), toBlock: 'latest' }] })).rejects.toMatchObject({ code: -32099, data: { kind: 'logs-range' } });
  });

  it('a verifier module that is not there is unverifiable, not a failure', async () => {
    h = await harness({ chainId: 1, mocks: { state: 'none' } });
    headAt(h.world, 1, ETH);
    await expect(h.client.request({ method: 'eth_getBalance', params: [`0x${'11'.repeat(20)}`, 'latest'] })).rejects.toMatchObject({ code: -32099, data: { kind: 'no-verifier' } });
  });
});

describe('levels', () => {
  it('a block anchored below minLevel is unverifiable', async () => {
    h = await harness({ chainId: 8453, options: { minLevel: 'committed' } });
    headAt(h.world, 8453, BASE);
    const old = blockOf('base-56049971.json');
    await expect(h.client.request({ method: 'eth_getBlockByHash', params: [old.hash, false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-level' } });
    // An L2's newest block is only ever signed: "latest" can't be at committed.
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-level' } });
  });

  it('on Ethereum at minLevel final, "latest" is the finalized block', async () => {
    h = await harness({ chainId: 1, options: { minLevel: 'final' } });
    const old = blockOf('ethereum-17034874.json');
    // An Ethereum anchor's time is its signed head's (headTime), which is recent also for a final block.
    h.world.finalized.set(1, { chainId: 1, number: old.number, hash: old.hash, level: 'final', path: 'history', time: h.world.nowSec - 10 });
    h.world.anchor({ chainId: 1, number: old.number, hash: old.hash, level: 'final', path: 'history', time: h.world.nowSec - 10 });
    expect(await h.client.request({ method: 'eth_blockNumber' })).toBe(toQuantity(old.number));
    const b: any = await h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] });
    expect(b.hash).toBe(old.hash);
    expect(h.events.at(-1)!.level).toBe('final');
  });
});

describe('a proof that fails', () => {
  it('always throws, also with flag', async () => {
    h = await on(8453, 'flag');
    const e = h.world.proofs.get(h.world.key(8453, 'willow_getBlockProof', [BASE.hash]))!;
    const lie = { ...e.result, transactions: e.result.transactions.slice(1) };
    h.world.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { result: lie } : undefined);
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })).rejects.toMatchObject({ code: -32098, data: { kind: 'transactions' } });
    expect(h.events.at(-1)).toMatchObject({ status: 'failed', kind: 'transactions' });
  });

  it('an endpoint for another chain fails every request', async () => {
    h = await on(8453);
    h.world.standard.set('8453|eth_chainId|*', { result: '0xa' });
    await expect(h.client.request({ method: 'eth_chainId' })).rejects.toMatchObject({ code: -32098, data: { kind: 'wrong-chain' } });
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })).rejects.toBeInstanceOf(WillowVerificationError);
  });
});
