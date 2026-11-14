// Regression tests for faults found in self-review, and the checkpoint choice (section 2).
import { afterEach, describe, expect, it } from 'vitest';
import { chooseCheckpoint, SHIPPED_CHECKPOINT, slotTime } from '../src/checkpoint.js';
import { toQuantity } from '../src/hex.js';
import type { KeyValueStorage } from '../src/storage.js';
import { blockOf, harness, headAt, type Harness } from './support/setup.js';
import { chain, register } from './support/synth.js';
import { World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

describe('self-review regressions', () => {
  it('an app that changes a returned block does not change the verified one kept in the cache', async () => {
    h = await harness({ chainId: 8453 });
    const head = blockOf('base-56049971.json');
    headAt(h.world, 8453, head);
    const a: any = await h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] });
    a.hash = '0xforged';
    a.transactions.length = 0;
    const b: any = await h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] });
    expect(b.hash).toBe(head.hash);
    expect(b.transactions.length).toBeGreaterThan(0);
  });

  it("a log filter's failed poll is asked again from the same block", async () => {
    const w = new World();
    const blocks = chain(100, 104);
    register(w, 8453, blocks, () => [{ address: `0x${'ee'.repeat(20)}` }]);
    const t0 = Date.now();
    // Synthetic blocks: only the mock items verifier takes them.
    h = await harness({ chainId: 8453, world: w, mocks: { items: 'mock' }, options: { now: () => w.nowSec * 1000 + (Date.now() - t0) } });
    w.head(8453, { number: 100, hash: blocks[0].hash, time: w.nowSec });
    const id = await h.client.request({ method: 'eth_newFilter', params: [{}] });
    w.head(8453, { number: 102, hash: blocks[2].hash, time: w.nowSec });
    await new Promise((r) => setTimeout(r, 2100));
    w.intercept = (_c, m) => (m === 'willow_getBlockReceiptsProof' ? { error: { code: -32603, message: 'busy' } } : undefined);
    await expect(h.client.request({ method: 'eth_getFilterChanges', params: [id] })).rejects.toMatchObject({ code: -32603 });
    w.intercept = undefined;
    const logs = (await h.client.request({ method: 'eth_getFilterChanges', params: [id] })) as any[];
    expect(logs.map((l) => l.blockNumber)).toEqual([toQuantity(101), toQuantity(102)]);
  });

  it('the light client is written to storage when it changes, not on every anchor', async () => {
    let writes = 0;
    const m = new Map<string, string>();
    const storage: KeyValueStorage = { get: (k) => m.get(k) ?? null, set: (k, v) => void (writes++, m.set(k, v)) };
    h = await harness({ chainId: 1, options: { storage } });
    headAt(h.world, 1, blockOf('ethereum-26391511.json'));
    for (const f of ['ethereum-17034874.json', 'ethereum-1500000.json']) await h.client.request({ method: 'eth_getBlockByHash', params: [blockOf(f).hash, false] });
    await h.client.request({ method: 'eth_blockNumber' });
    expect(writes).toBe(1);
  });

  it('Nitro fee history with rewards is one proof, whatever the count (its rewards need no bodies)', async () => {
    h = await harness({ chainId: 42161 });
    headAt(h.world, 42161, blockOf('arbitrum-one-523278185.json'));
    await h.client.request({ method: 'eth_feeHistory', params: ['0x20', 'latest', [50]] }).catch(() => undefined);
    const asked = h.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getFeeHistoryProof');
    expect(asked.map((i) => (i.params as unknown[])[0])).toEqual(['0x20']);
  });
});

describe('the checkpoint', () => {
  const fresh = slotTime(SHIPPED_CHECKPOINT.slot) + 3600;
  const stale = slotTime(SHIPPED_CHECKPOINT.slot) + 31 * 24 * 3600;
  const newer = { root: `0x${'ab'.repeat(32)}`, slot: SHIPPED_CHECKPOINT.slot + 8192 * 4 };
  const fetchOf = (routes: Record<string, unknown>) =>
    (async (url: string) => {
      const k = Object.keys(routes).find((r) => String(url).includes(r));
      if (!k) throw new Error(`unreachable ${url}`);
      return new Response(JSON.stringify(routes[k]));
    }) as unknown as typeof fetch;

  it('the one passed in, else the shipped one while it is young enough', async () => {
    expect(await chooseCheckpoint({ checkpoint: newer, fetch: fetchOf({}), nowSec: () => stale })).toEqual(newer);
    expect(await chooseCheckpoint({ fetch: fetchOf({}), nowSec: () => fresh })).toEqual(SHIPPED_CHECKPOINT);
  });

  it("once the shipped one is old: Willow's checkpoint.json, confirmed by the beacon APIs", async () => {
    const ok = fetchOf({ 'checkpoint.json': newer, 'beacon-a': { data: { root: newer.root, canonical: true } }, 'beacon-b': { data: { root: newer.root } } });
    expect(await chooseCheckpoint({ fetch: ok, nowSec: () => stale, beaconApis: ['https://beacon-a', 'https://beacon-b'] })).toEqual(newer);
  });

  it('a beacon API that disagrees, or none that answers, leaves the client unable to start (unverifiable)', async () => {
    const lie = fetchOf({ 'checkpoint.json': newer, 'beacon-a': { data: { root: `0x${'cd'.repeat(32)}` } } });
    await expect(chooseCheckpoint({ fetch: lie, nowSec: () => stale, beaconApis: ['https://beacon-a'] })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint' } });
    const silent = fetchOf({ 'checkpoint.json': newer });
    await expect(chooseCheckpoint({ fetch: silent, nowSec: () => stale, beaconApis: ['https://beacon-a'] })).rejects.toMatchObject({ code: -32099 });
    const older = fetchOf({ 'checkpoint.json': SHIPPED_CHECKPOINT });
    await expect(chooseCheckpoint({ fetch: older, nowSec: () => stale, beaconApis: false })).rejects.toMatchObject({ code: -32099 });
  });
});

describe('self-review regressions, round 2', () => {
  it('an anchor module that is not there is unverifiable (no-verifier), not a raw error', async () => {
    h = await harness({ chainId: 8453, mocks: { anchor: async () => Promise.reject(new Error('anchor_bg.wasm not found')) } });
    headAt(h.world, 8453, blockOf('base-56049971.json'));
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'no-verifier' } });
  });

  it('a worker answers block and state calls without a checkpoint; only the light client needs one', async () => {
    const { MessageChannel } = await import('node:worker_threads');
    const { InlineBackend } = await import('../src/backend.js');
    const { serveBackend, WorkerBackend } = await import('../src/worker-rpc.js');
    const { mockModules } = await import('./support/mock-verifiers.js');
    const ch = new MessageChannel();
    serveBackend(ch.port2 as any, (cp) => new InlineBackend(mockModules(new World()), cp));
    const b = new WorkerBackend(ch.port1 as any, async () => Promise.reject(new Error('no checkpoint can be chosen')));
    try {
      await expect(b.itemRequest(1, 'block', '0x10')).resolves.toMatchObject({ ok: true, method: 'willow_getBlockProof' });
      await expect(b.anchorRestore('[]', 1)).rejects.toThrow('no checkpoint can be chosen');
    } finally {
      ch.port1.close();
      ch.port2.close();
    }
  });
});
