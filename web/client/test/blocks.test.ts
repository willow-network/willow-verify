// Blocks (design 1.3.1): eth_getBlockByNumber / ByHash, false and true, and the transaction counts,
// on W1's fixtures from all seven chains.
import { afterEach, describe, expect, it } from 'vitest';
import { toQuantity } from '../src/hex.js';
import { blockOf, chainOf, harness, headAt, methodsSent, type Harness } from './support/setup.js';
import { fixture, fixtureNames, World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const FILES = fixtureNames('block').filter((f) => f !== 'ethereum-1500000-uncles-unproven.json');

/** `got` without the transactions' proven `blockTimestamp` where the node's `want` leaves it out (Nitro's nodes do). */
function withoutOmitted(got: any, want: any): unknown {
  const out = structuredClone(got);
  out.transactions?.forEach((t: any, i: number) => {
    if (t && typeof t === 'object' && !('blockTimestamp' in (want.transactions?.[i] ?? {}))) delete t.blockTimestamp;
  });
  return out;
}

describe('blocks on every chain', () => {
  for (const file of FILES) {
    it(`${file}: latest, by number and by hash, false and true, counts`, async () => {
      const chainId = chainOf(file);
      const f = fixture(`block/${file}`);
      const b = blockOf(file);
      h = await harness({ chainId });
      headAt(h.world, chainId, b);
      const entry = h.world.proofs.get(h.world.key(chainId, 'willow_getBlockProof', [b.hash]))!;
      const c = h.client;
      expect(await c.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })).toEqual(entry.proven);
      expect(withoutOmitted(await c.request({ method: 'eth_getBlockByNumber', params: ['latest', true] }), entry.full!.proven)).toEqual(entry.full!.proven);
      expect(await c.request({ method: 'eth_getBlockByHash', params: [b.hash, false] })).toEqual(entry.proven);
      expect(await c.request({ method: 'eth_getBlockByNumber', params: [toQuantity(b.number), false] })).toEqual(entry.proven);
      const n = f.node.eth_getBlockByNumber.transactions.length;
      expect(await c.request({ method: 'eth_getBlockTransactionCountByHash', params: [b.hash] })).toBe(toQuantity(n));
      expect(await c.request({ method: 'eth_getBlockTransactionCountByNumber', params: ['latest'] })).toBe(toQuantity(n));
      for (const e of h.events) {
        expect(e.status).toBe('verified');
        expect(e.block).toEqual({ number: b.number, hash: b.hash });
        expect(e.level).toBe('signed');
      }
      // The block is fetched once, then served from the cache by hash.
      expect(methodsSent(h.world).filter((m) => m === 'willow_getBlockProof').length).toBeLessThanOrEqual(2);
    });
  }

  it('a block that is not the head: its anchor goes in the same batch as its proof (3.2 step 3)', async () => {
    h = await harness({ chainId: 1 });
    headAt(h.world, 1, blockOf('ethereum-26391511.json'));
    await h.client.request({ method: 'eth_blockNumber' });
    h.world.log.length = 0;
    const old = blockOf('ethereum-17034874.json');
    await h.client.request({ method: 'eth_getBlockByNumber', params: [toQuantity(old.number), false] });
    const batch = h.world.log.find((r) => r.items.some((i) => i.method === 'willow_getBlockProof'))!;
    expect(batch.items.map((i) => i.method).sort()).toEqual(['willow_getBlockAnchor', 'willow_getBlockProof']);
    expect(h.events.at(-1)!.block).toEqual(old);
  });

  it('a block newer than the signed head waits for its anchor (the server says waiting), then is anchored', async () => {
    h = await harness({ chainId: 1 });
    headAt(h.world, 1, blockOf('ethereum-26391511.json'));
    const old = blockOf('ethereum-17034874.json');
    let asked = 0;
    const waiting = { error: { code: -32000, message: `block ${old.number} is newer than the signed head`, data: { status: 'waiting', retryAfterSeconds: 4 } } };
    h.world.intercept = (_c, m, p) => (m === 'willow_getBlockAnchor' && p[0] === toQuantity(old.number) && ++asked < 3 ? waiting : undefined);
    await h.client.request({ method: 'eth_getBlockByNumber', params: [toQuantity(old.number), false] });
    expect(asked).toBe(3);
    expect(h.events.at(-1)).toMatchObject({ status: 'verified', block: old });
  });

  it('a block whose anchor is still waiting past anchorWaitMs is unverifiable (-32099, anchor-waiting)', async () => {
    h = await harness({ chainId: 1, options: { anchorWaitMs: 40 } });
    headAt(h.world, 1, blockOf('ethereum-26391511.json'));
    const old = blockOf('ethereum-17034874.json');
    let asked = 0;
    const waiting = { error: { code: -32000, message: `block ${old.number} is newer than the signed head`, data: { status: 'waiting', retryAfterSeconds: 4 } } };
    h.world.intercept = (_c, m, p) => (m === 'willow_getBlockAnchor' && p[0] === toQuantity(old.number) && ++asked ? waiting : undefined);
    const started = Date.now();
    await expect(h.client.request({ method: 'eth_getBlockByNumber', params: [toQuantity(old.number), false] })).rejects.toMatchObject({
      code: -32099,
      data: { kind: 'anchor-waiting' },
    });
    expect(asked).toBeGreaterThan(1);
    expect(Date.now() - started).toBeGreaterThanOrEqual(40);
  });

  it("a block by hash: its anchor is a second round trip, and its parent's hash is then anchored without one", async () => {
    h = await harness({ chainId: 1 });
    headAt(h.world, 1, blockOf('ethereum-26391511.json'));
    const pre = blockOf('ethereum-1500000.json');
    await h.client.request({ method: 'eth_getBlockByHash', params: [pre.hash, false] });
    const sent = h.world.log.map((r) => r.items.map((i) => i.method));
    expect(sent.findIndex((m) => m.includes('willow_getBlockAnchor'))).toBeGreaterThan(sent.findIndex((m) => m.includes('willow_getBlockProof')));
  });

  it('a pre-merge block whose uncles the server does not hold: they are left out and listed as unproven', async () => {
    const w = new World();
    const f = fixture('block/ethereum-1500000-uncles-unproven.json');
    const b = w.addBlock(f);
    h = await harness({ chainId: 1, world: w });
    headAt(w, 1, b);
    const block = (await h.client.request({ method: 'eth_getBlockByHash', params: [b.hash, false] })) as Record<string, unknown>;
    expect(block.uncles).toBeUndefined();
    expect(block.size).toBeUndefined();
    expect(h.events.at(-1)!.unproven).toEqual(['size', 'uncles']);
  });

  it('a block by hash the server does not have: its refusal passes through', async () => {
    h = await harness({ chainId: 8453 });
    headAt(h.world, 8453, blockOf('base-56049971.json'));
    await expect(h.client.request({ method: 'eth_getBlockByHash', params: [`0x${'ab'.repeat(32)}`, false] })).rejects.toMatchObject({ code: -32000, name: 'WillowRpcError' });
    expect(h.events.at(-1)!.status).toBe('refused');
  });
});
