// Batching and coalescing (design 3.2 step 4): calls within batchWindowMs go out as one JSON-RPC
// batch of at most 100, identical calls share one item, and a batch response must hold every item.
import { afterEach, describe, expect, it } from 'vitest';
import { Batcher, splitArray } from '../src/http.js';
import { blockOf, harness, headAt, type Harness } from './support/setup.js';
import { fixture } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const ETH = blockOf('ethereum-26391511.json');

describe('the client', () => {
  it('a block, a balance and the gas price at latest: one batch for the head, one for the three proofs', async () => {
    // Batching, on the mocks: the account is rebased onto the block (World.rebase), which only the
    // mock state verifier takes, and a real module's first load can outlast the 2 ms batch window.
    h = await harness({ chainId: 1, mocks: { items: 'mock', state: 'mock' } });
    const w = h.world;
    headAt(w, 1, ETH);
    w.addState(w.rebase(fixture('state/eoa.json'), fixture('block/ethereum-26391511.json')));
    const eoa = fixture('state/eoa.json').request.params[0];
    await Promise.all([
      h.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] }),
      h.client.request({ method: 'eth_getBalance', params: [eoa, 'latest'] }),
      h.client.request({ method: 'eth_gasPrice' }),
    ]);
    const batches = w.log.map((r) => r.items.map((i) => i.method).sort());
    expect(batches).toEqual([
      ['eth_chainId', 'willow_getBlockAnchor'],
      ['willow_getBlockProof', 'willow_getFeeProof', 'willow_getStateProof'],
    ]);
  });

  it('identical calls in flight share one item', async () => {
    h = await harness({ chainId: 8453 });
    headAt(h.world, 8453, blockOf('base-56049971.json'));
    await h.client.request({ method: 'eth_blockNumber' });
    h.world.log.length = 0;
    await Promise.all(Array.from({ length: 5 }, () => h!.client.request({ method: 'eth_getBlockByNumber', params: ['latest', false] })));
    expect(h.world.log.flatMap((r) => r.items)).toHaveLength(1);
  });

  it('more than 100 items go out as several batches of at most 100', async () => {
    h = await harness({ chainId: 8453 });
    headAt(h.world, 8453, blockOf('base-56049971.json'));
    await h.client.request({ method: 'eth_blockNumber' });
    h.world.log.length = 0;
    const hashes = Array.from({ length: 130 }, (_, i) => `0x${i.toString(16).padStart(64, '0')}`);
    const out = await Promise.all(hashes.map((x) => h!.client.request({ method: 'eth_getTransactionReceipt', params: [x] })));
    expect(out.every((x) => x === null)).toBe(true);
    // Sent 100 first, then 30; the server may take the smaller request in first (the full suite's
    // load made it 2 runs in 8): what is checked is the batches, not their order of arrival.
    expect(h.world.log.map((r) => r.items.length).sort((a, b) => b - a)).toEqual([100, 30]);
  });

  it('eth_blockNumber called in a burst refreshes the head once', async () => {
    h = await harness({ chainId: 8453 });
    headAt(h.world, 8453, blockOf('base-56049971.json'));
    await Promise.all(Array.from({ length: 4 }, () => h!.client.request({ method: 'eth_blockNumber' })));
    expect(h.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getBlockAnchor')).toHaveLength(1);
  });
});

describe('the batcher', () => {
  const reply = (body: unknown) => async () => new Response(JSON.stringify(body), { status: 200, headers: { 'content-type': 'application/json' } });

  it('splits a batch response into each item exactly as sent', () => {
    expect(splitArray(' [ {"a":"x,]}"} , [1,[2]], "s\\"" , 3 ] ')).toEqual(['{"a":"x,]}"}', '[1,[2]]', '"s\\""', '3']);
    expect(splitArray('[]')).toEqual([]);
    expect(splitArray('{"error":1}')).toBeNull();
    expect(splitArray('[1,2')).toBeNull();
  });

  it('an item missing from the batch response fails verification (kind batch); the others resolve', async () => {
    const b = new Batcher({ fetch: reply([{ jsonrpc: '2.0', id: 1, result: '0x1' }]) as any, windowMs: 1 });
    const [one, two] = await Promise.allSettled([b.call('http://x/v1/rpc/1/k', 'a', []), b.call('http://x/v1/rpc/1/k', 'b', [])]);
    expect(one).toMatchObject({ status: 'fulfilled', value: { value: { result: '0x1' } } });
    expect(two).toMatchObject({ status: 'rejected', reason: { code: -32098, data: { kind: 'batch' } } });
  });

  it('an item twice in the batch response fails verification', async () => {
    const b = new Batcher({ fetch: reply([{ jsonrpc: '2.0', id: 1, result: '0x1' }, { jsonrpc: '2.0', id: 1, result: '0x2' }]) as any, windowMs: 1 });
    await expect(b.call('http://x/v1/rpc/1/k', 'a', [])).rejects.toMatchObject({ code: -32098, data: { kind: 'batch' } });
  });

  it('a whole-request error reaches every item as the server sent it', async () => {
    const b = new Batcher({ fetch: reply({ jsonrpc: '2.0', id: null, error: { code: -32001, message: 'invalid API key' } }) as any, windowMs: 1 });
    await expect(b.call('http://x/v1/rpc/1/k', 'a', [])).rejects.toMatchObject({ code: -32001, message: 'invalid API key' });
  });

  it('HTTP 429 is asked again after its Retry-After', async () => {
    let n = 0;
    const f = async () => (n++ === 0 ? new Response('', { status: 429, headers: { 'retry-after': '1' } }) : new Response(JSON.stringify([{ jsonrpc: '2.0', id: 1, result: 'ok' }])));
    const b = new Batcher({ fetch: f as any, windowMs: 1 });
    await expect(b.call('http://x/v1/rpc/1/k', 'a', [])).resolves.toMatchObject({ value: { result: 'ok' } });
    expect(n).toBe(2);
  });
});
