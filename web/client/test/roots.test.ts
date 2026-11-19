// Anchors of the roots alone (Polygon PoS's checkpoints, anchor-verify polygon.rs): they count for
// transactions, receipts and logs, their block hashes left out, never for a block, state, a parent
// link or the newest block; and an anchor of another header is a mismatch. Base's fixtures with
// mock anchors of that kind: the client decides by the anchor's `covers`, not by the chain.
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { Anchors } from '../src/anchor.js';
import type { Backend } from '../src/backend.js';
import { toQuantity } from '../src/hex.js';
import type { Reply } from '../src/http.js';
import type { AnchorFailure, AnchorVerified } from '../src/verifiers.js';
import { blockOf, harness, type Harness } from './support/setup.js';
import { fixture, ITEMS, json, World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const ROOTS = { level: 'final' as const, path: 'checkpoint' };
const RECEIPT = json(join(ITEMS, 'base-swap-receipt.json'));
const STATE = fixture('state/base-usdc.json');

function world(): World {
  const w = new World();
  w.addBlock(fixture('block/base-56049971.json'), { anchor: ROOTS });
  w.addItem(RECEIPT, { anchor: ROOTS });
  w.addState(STATE, 8453);
  w.anchor({ chainId: 8453, number: parseInt(STATE.result.blockNumber, 16), hash: STATE.result.blockHash, time: w.nowSec - 3600, ...ROOTS });
  return w;
}

describe('an anchor of the roots alone', () => {
  it('a receipt verifies, its block hashes left out', async () => {
    h = await harness({ chainId: 8453, world: world() });
    const r: any = await h.client.request({ method: 'eth_getTransactionReceipt', params: [RECEIPT.proof.params[0]] });
    expect(r.transactionHash).toBe(RECEIPT.answers[0].result.transactionHash);
    expect(r.logs.length).toBe(RECEIPT.answers[0].result.logs.length);
    expect('blockHash' in r).toBe(false);
    expect(r.logs.some((l: any) => 'blockHash' in l)).toBe(false);
    const e = h.events.at(-1)!;
    // The anchor leaves out the block hashes; the items verifier's own (a log's position in the block, which one
    // receipt doesn't prove) comes with the real module (WILLOW_VERIFY_DIST), not the mock.
    expect([e.status, e.level]).toEqual(['verified', 'final']);
    expect(e.unproven.filter((f: string) => f !== 'logs[].logIndex')).toEqual(['blockHash', 'logs[].blockHash']);
  });

  it("a block's transaction count verifies; the block itself does not", async () => {
    h = await harness({ chainId: 8453, world: world() });
    const b = blockOf('base-56049971.json');
    const n = fixture('block/base-56049971.json').node.eth_getBlockByNumber.transactions.length;
    expect(await h.client.request({ method: 'eth_getBlockTransactionCountByHash', params: [b.hash] })).toBe(toQuantity(n));
    expect(h.events.at(-1)!.status).toBe('verified');
    // Kept after the count, asked again for the whole block: its hash is not anchored.
    await expect(h.client.request({ method: 'eth_getBlockByHash', params: [b.hash, false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-covers' } });
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: [toQuantity(b.number), false] })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-covers' } });
  });

  it('state at a block anchored by its roots is unverifiable', async () => {
    h = await harness({ chainId: 8453, world: world() });
    const [address] = STATE.request.params;
    await expect(h.client.request({ method: 'eth_getStorageAt', params: [address, '0x1', { blockHash: STATE.result.blockHash }] })).rejects.toMatchObject({
      code: -32099,
      data: { kind: 'anchor-covers' },
    });
  });

  it('under flag, the block comes back flagged, never verified', async () => {
    h = await harness({ chainId: 8453, world: world(), options: { unverifiable: 'flag' } });
    const b = blockOf('base-56049971.json');
    expect(await h.client.request({ method: 'eth_getBlockByHash', params: [b.hash, false] })).toBeTruthy();
    const e = h.events.at(-1)!;
    expect([e.status, e.kind]).toEqual(['unverifiable', 'anchor-covers']);
  });
});

/** Anchors over a backend whose `anchorVerify` gives `result`. */
function anchorsWith(result: AnchorVerified | AnchorFailure) {
  const backend = {
    anchorRestore: async () => ({ ok: true }),
    anchorBootstrap: async () => ({ ok: true, period: 1 }),
    anchorUpdates: async () => ({ ok: true, period: 2 }),
    anchorVerify: async () => ({ result, saved: null }),
    close: () => {},
  } as unknown as Backend;
  const rpc = async (_c: number, _m: string, _p: unknown[]): Promise<Reply> => {
    const value = { jsonrpc: '2.0', id: 1, result: { blockNumber: '0x10' } } as Reply['value'];
    return { value, text: JSON.stringify(value) };
  };
  return new Anchors({ backend, rpc, storage: null, checkpointRoot: async () => '0x00', minLevel: 'signed', headMaxAgeMs: () => 2_000, maxHeadAgeSec: 600, nowMs: () => 1_000_000_000_000 });
}

const HASH = `0x${'11'.repeat(32)}`;
const PARENT = `0x${'22'.repeat(32)}`;
const roots = (rootsMatch = true): AnchorVerified => ({ ok: true, level: 'final', path: 'checkpoint', covers: 'roots', headerHash: HASH, rootsMatch });

describe('Anchors under roots alone', () => {
  it('keeps the anchor as roots: no parent counted, no hash to send a request by', async () => {
    const a = anchorsWith(roots());
    const got = await a.block(137, 16, HASH);
    expect([got.covers, got.level, got.path]).toEqual(['roots', 'final', 'checkpoint']);
    a.parent(got, PARENT);
    expect(a.hashOf(137, 15)).toBeUndefined();
    expect(a.hashOf(137, 16)).toBeUndefined();
    expect(a.known(137, 16)).toBe(true);
  });

  it('another header than the one anchored is a mismatch', async () => {
    await expect(anchorsWith(roots(false)).block(137, 16, HASH)).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-mismatch' } });
  });

  it('the newest block is not anchored by roots', async () => {
    await expect(anchorsWith(roots()).head(137, 'latest')).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-covers' } });
  });
});
