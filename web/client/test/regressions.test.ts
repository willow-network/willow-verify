// Regression tests for faults found in self-review, and the checkpoint choice.
import { afterEach, describe, expect, it } from 'vitest';
import { BEACON_APIS, chooseCheckpoint, SHIPPED_CHECKPOINT, slotTime } from '../src/checkpoint.js';
import { createVerifier } from '../src/client.js';
import { toQuantity } from '../src/hex.js';
import type { KeyValueStorage } from '../src/storage.js';
import { mockModules } from './support/mock-verifiers.js';
import { blockOf, fullWorld, harness, headAt, type Harness } from './support/setup.js';
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

  // Node's default beacon APIs as they respond to a checkpoint.json slot: publicnode serves the
  // header; lodestar keeps headers for about ten hours and responds to an older slot as below (11/17).
  const day29 = slotTime(SHIPPED_CHECKPOINT.slot) + 29 * 24 * 3600 + 60;
  const lodestar404 = (slot: number) => new Response(JSON.stringify({ code: 404, message: `Block not found for id '${slot}'` }), { status: 404, headers: { 'content-type': 'application/json' } });
  const beacons =
    (c: { root: string; slot: number }, publicnode: () => Response = () => new Response(JSON.stringify({ execution_optimistic: false, finalized: true, data: { root: c.root, canonical: true } }))) =>
    async (url: string | URL | Request) => {
      const u = String(url);
      if (u.includes('checkpoint.json')) return new Response(JSON.stringify(c));
      if (u.startsWith(`${BEACON_APIS[0]}/eth/v1/beacon/headers/${c.slot}`)) return publicnode();
      if (u.startsWith(`${BEACON_APIS[1]}/eth/v1/beacon/headers/${c.slot}`)) return lodestar404(c.slot);
      throw new Error(`unreachable ${u}`);
    };

  it('past the 29-day point, a beacon API that responds with an HTTP error (lodestar: 404 for a past slot) is skipped, not a disagreement', async () => {
    expect(BEACON_APIS[1]).toBe('https://lodestar-mainnet.chainsafe.io');
    expect(await chooseCheckpoint({ fetch: beacons(newer) as typeof fetch, nowSec: () => day29 })).toEqual(newer);
    // A response without `data` is skipped too; one that names another root still refuses.
    const noData = beacons(newer, () => new Response(JSON.stringify({ code: 200, message: 'ok' })));
    await expect(chooseCheckpoint({ fetch: noData as typeof fetch, nowSec: () => day29 })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint', detail: 'no beacon API could confirm the fetched checkpoint' } });
    const other = beacons(newer, () => new Response(JSON.stringify({ data: { root: `0x${'cd'.repeat(32)}`, canonical: true } })));
    await expect(chooseCheckpoint({ fetch: other as typeof fetch, nowSec: () => day29 })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint' } });
    const notCanonical = beacons(newer, () => new Response(JSON.stringify({ data: { root: newer.root, canonical: false } })));
    await expect(chooseCheckpoint({ fetch: notCanonical as typeof fetch, nowSec: () => day29 })).rejects.toMatchObject({ code: -32099 });
    const down = beacons(newer, () => new Response('bad gateway', { status: 502 }));
    await expect(chooseCheckpoint({ fetch: down as typeof fetch, nowSec: () => day29 })).rejects.toMatchObject({ code: -32099, data: { detail: 'no beacon API could confirm the fetched checkpoint' } });
  });

  it('a client started past the 29-day point verifies with the default beacon APIs, lodestar responding 404', async () => {
    const w = fullWorld();
    w.nowSec = day29;
    const head = blockOf('ethereum-26391511.json');
    headAt(w, 1, head);
    const fake = beacons(newer);
    const fetchVia = (async (url: string | URL | Request, init?: RequestInit) => (/checkpoint\.json|beacon|lodestar/.test(String(url)) ? fake(url) : fetch(url, init))) as typeof fetch;
    h = await harness({ chainId: 1, world: w, options: { checkpoint: undefined, fetch: fetchVia } });
    expect(await h.client.request({ method: 'eth_blockNumber' })).toBe(toQuantity(head.number));
    expect(h.events.at(-1)).toMatchObject({ status: 'verified', block: head });
  });

  it('a checkpoint choice that failed (the confirming beacon API down) is tried again on the next request', async () => {
    const w = fullWorld();
    w.nowSec = day29;
    const head = blockOf('ethereum-26391511.json');
    headAt(w, 1, head);
    let down = true;
    const fake = beacons(newer, () => (down ? new Response('unavailable', { status: 503 }) : new Response(JSON.stringify({ data: { root: newer.root, canonical: true } }))));
    const fetchVia = (async (url: string | URL | Request, init?: RequestInit) => (/checkpoint\.json|beacon|lodestar/.test(String(url)) ? fake(url) : fetch(url, init))) as typeof fetch;
    h = await harness({ chainId: 1, world: w, options: { checkpoint: undefined, fetch: fetchVia } });
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint' } });
    down = false;
    w.nowSec += 10;
    expect(await h.client.request({ method: 'eth_blockNumber' })).toBe(toQuantity(head.number));
  });

  it('with a saved light client, a failing request makes one attempt at the start, which stands for 10 s; once one is chosen the saved light client is restored', async () => {
    const w = fullWorld();
    w.nowSec = day29;
    const head = blockOf('ethereum-26391511.json');
    headAt(w, 1, head);
    let down = true;
    let asked = 0;
    const fake = beacons(newer, () => (asked++, down ? new Response('unavailable', { status: 503 }) : new Response(JSON.stringify({ data: { root: newer.root, canonical: true } }))));
    const fetchVia = (async (url: string | URL | Request, init?: RequestInit) => (/checkpoint\.json|beacon|lodestar/.test(String(url)) ? fake(url) : fetch(url, init))) as typeof fetch;
    const kept = new Map([['willow-rpc:light-client', '{"kept":true}']]);
    const storage: KeyValueStorage = { get: (k) => kept.get(k) ?? null, set: (k, v) => void kept.set(k, v) };
    const mods = mockModules(w);
    let restores = 0;
    const anchor = async () => {
      const x: any = await mods.anchor();
      return { ...x, Anchor: class extends x.Anchor { restore(...a: unknown[]) { restores++; return super.restore(...a); } } };
    };
    h = await harness({ chainId: 1, world: w, verifiers: { ...mods, anchor }, options: { checkpoint: undefined, fetch: fetchVia, storage } });
    const unverifiable = { code: -32099, data: { kind: 'anchor-stale-checkpoint' } };
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject(unverifiable);
    expect([asked, restores]).toEqual([1, 0]);
    w.nowSec += 9;
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject(unverifiable);
    expect(asked).toBe(1);
    down = false;
    w.nowSec += 1;
    expect(await h.client.request({ method: 'eth_blockNumber' })).toBe(toQuantity(head.number));
    expect([asked, restores]).toEqual([2, 1]);
  });

  it('a request with no start stops before the anchor module, so a slow first load of it makes no second attempt; a clock that goes back ends the wait', async () => {
    const w = fullWorld();
    w.nowSec = day29;
    const head = blockOf('ethereum-26391511.json');
    headAt(w, 1, head);
    let down = true;
    let asked = 0;
    const fake = beacons(newer, () => (asked++, down ? new Response('unavailable', { status: 503 }) : new Response(JSON.stringify({ data: { root: newer.root, canonical: true } }))));
    const fetchVia = (async (url: string | URL | Request, init?: RequestInit) => (/checkpoint\.json|beacon|lodestar/.test(String(url)) ? fake(url) : fetch(url, init))) as typeof fetch;
    const mods = mockModules(w);
    let loads = 0;
    // The anchor module's first load takes past the 10 s a failed start stands.
    const anchor = async () => {
      if (loads++ === 0) w.nowSec += 11;
      return mods.anchor();
    };
    h = await harness({ chainId: 1, world: w, verifiers: { ...mods, anchor }, options: { checkpoint: undefined, fetch: fetchVia, storage: false } });
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint' } });
    expect([asked, loads]).toEqual([1, 0]);
    // The clock goes back: the failure no longer stands.
    w.nowSec -= 60;
    headAt(w, 1, head);
    down = false;
    expect(await h.client.request({ method: 'eth_blockNumber' })).toBe(toQuantity(head.number));
    expect(asked).toBe(2);
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

describe('self-review regressions, round 3', () => {
  const codeOf = (f: () => unknown): unknown => {
    try {
      f();
    } catch (e) {
      return (e as { code?: unknown }).code;
    }
    return 'no error';
  };

  it('a chain id the verifiers cannot take (over 32 bits) is refused (-32602), not truncated to another chain', () => {
    const make = (chainId: number) => () => createVerifier({ chainId, verifiers: mockModules(new World()), storage: false }).close();
    // The verifiers take a u32: 2^32 + 1 would be verified and anchored as chain 1, 2^32 + 8453 as Base.
    for (const chainId of [2 ** 32 + 1, 2 ** 32, 2 ** 32 + 8453, Number.MAX_SAFE_INTEGER]) expect(codeOf(make(chainId))).toBe(-32602);
    expect(codeOf(make(2 ** 32 - 1))).toBe('no error');
  });
});

describe('anchor failure texts', () => {
  it("speak of the light client and this verifier, not the page, in the package", async () => {
    const { anchorFailure } = await import('../src/anchor.js');
    const cases: [string, string][] = [
      ["this page's starting point on Berachain is more than a day old", "the light client's starting point on Berachain is more than a day old"],
      ["this page's starting point on BNB is more than 7 days old", "the light client's starting point on BNB is more than 7 days old"],
      ['the bootstrap is not this page\'s starting header on Berachain', "the bootstrap is not the light client's starting header on Berachain"],
      ["BNB block 100 is before this page's starting point", "BNB block 100 is before the light client's starting point"],
      ['the bootstrap is not for this page\'s checkpoint', "the bootstrap is not for the light client's checkpoint"],
      ['height 5 is not signed by two thirds of the validators this page follows alone', 'height 5 is not signed by two thirds of the validators the light client follows alone'],
      ['CometBFT block protocol 12 is not one this page knows', 'CometBFT block protocol 12 is not one this verifier knows'],
      ["Berachain's light client data is from an upgrade newer than this page", "Berachain's light client data is from an upgrade newer than this verifier"],
    ];
    for (const [wasm, want] of cases) {
      const e = anchorFailure({ ok: false, kind: 'stale-checkpoint', message: wasm } as never, 'eth_blockNumber');
      expect(e.message).toContain(want);
      expect(e.message).not.toContain("this page's");
    }
  });
});

describe("a long-running client's Berachain start", () => {
  it("is replaced by checkpoint.json's newer one once it ages out, and the light client bootstraps from it", async () => {
    const { MockAnchor } = await import('./support/mock-verifiers.js');
    const { SHIPPED_STARTS } = await import('../src/checkpoint.js');
    const shipped = SHIPPED_STARTS.berachain;
    const newer = { height: shipped.height + 20_000, hash: `0x${'cd'.repeat(32)}`, time: shipped.time + 20 * 3600 };
    const world = new World();
    world.nowSec = shipped.time + 3600;
    world.head(80094, { number: 100, hash: `0x${'01'.repeat(32)}` });
    world.lightClient.bootstrap = { header: {} };
    const asked: unknown[][] = [];
    world.intercept = (_c, m, p) => void (m === 'willow_lightClientBootstrap' && asked.push(p));
    const started: string[] = [];
    let start = { height: 0, hash: '' };
    let booted = false;
    class Bera extends MockAnchor {
      start(lc: string, s: string) {
        started.push(`${lc} ${JSON.parse(s).height ?? ''}`);
        if (lc === 'berachain') [start, booted] = [JSON.parse(s), false];
        return '{"ok":true}';
      }
      bootstrapFor() {
        booted = true;
        return '{"ok":true,"period":0}';
      }
      updatesFor() {
        return '{"ok":true,"period":1}';
      }
      restoreFor() {
        return '{"ok":false}';
      }
      savedFor() {
        return 'null';
      }
      resetFor() {}
      verify(chainId: number, number: number, hash: string, response: string, now: number) {
        if (chainId === 80094) {
          const t = start.height === shipped.height ? shipped.time : newer.time;
          if (now - t > 24 * 3600) return JSON.stringify({ ok: false, kind: 'stale-checkpoint', message: "this page's starting point on Berachain is more than a day old" });
          if (!booted) return JSON.stringify({ ok: false, kind: 'need-bootstrap', message: '', lightClient: 'berachain' });
        }
        return super.verify(chainId, number, hash, response, now);
      }
    }
    let served: unknown = {};
    const cpUrl = 'https://checkpoint.test/checkpoint.json';
    const h = await harness({
      chainId: 80094,
      world,
      verifiers: mockModules(world, { anchor: async () => ({ Anchor: Bera as never }) }),
      options: {
        checkpointUrl: cpUrl,
        cometApis: ['https://comet.test'],
        fetch: ((u: RequestInfo | URL, i?: RequestInit) => {
          const url = String(u);
          if (url === cpUrl) return Promise.resolve(new Response(JSON.stringify(served)));
          if (url === `https://comet.test/commit?height=${newer.height}`) {
            return Promise.resolve(new Response(JSON.stringify({ result: { signed_header: { commit: { block_id: { hash: newer.hash.slice(2) } } } } })));
          }
          return fetch(u, i);
        }) as typeof fetch,
      },
    });
    try {
      await expect(h.client.request({ method: 'eth_blockNumber' })).resolves.toBe(toQuantity(100));
      // A day on: the shipped start is too old; checkpoint.json now has a newer one.
      world.nowSec = shipped.time + 25 * 3600;
      world.head(80094, { number: 200, hash: `0x${'02'.repeat(32)}` });
      served = { berachain: newer };
      await expect(h.client.request({ method: 'eth_blockNumber' })).resolves.toBe(toQuantity(200));
      expect(started.filter((s) => s.startsWith('berachain'))).toEqual([`berachain ${shipped.height}`, `berachain ${newer.height}`]);
      expect(asked).toEqual([
        [shipped.hash, shipped.height],
        [newer.hash, newer.height],
      ]);
    } finally {
      await h.close();
    }
  });
});

describe("Berachain's CometBFT APIs", () => {
  it('are asked by a Berachain client in Node by default, and by no other client', async () => {
    const { SHIPPED_STARTS, BERA_COMET_APIS } = await import('../src/checkpoint.js');
    const shipped = SHIPPED_STARTS.berachain;
    const newer = { height: shipped.height + 20_000, hash: `0x${'cd'.repeat(32)}`, time: shipped.time + 5 * 3600 };
    const cpUrl = 'https://checkpoint.test/checkpoint.json';
    const run = async (chainId: number) => {
      const asked: string[] = [];
      const world = new World();
      world.nowSec = shipped.time + 7 * 3600;
      world.head(chainId, { number: 100, hash: `0x${'01'.repeat(32)}` });
      const h = await harness({
        chainId,
        world,
        options: {
          checkpointUrl: cpUrl,
          fetch: ((u: RequestInfo | URL, i?: RequestInit) => {
            const url = String(u);
            if (url === cpUrl) return Promise.resolve(new Response(JSON.stringify({ berachain: newer })));
            if (BERA_COMET_APIS.some((a) => url.startsWith(a))) {
              asked.push(url);
              return Promise.resolve(new Response('{}', { status: 404 }));
            }
            return fetch(u, i);
          }) as typeof fetch,
        },
      });
      try {
        await h.client.request({ method: 'eth_blockNumber' }).catch(() => undefined);
      } finally {
        await h.close();
      }
      return asked;
    };
    expect(await run(8453)).toEqual([]);
    const bera = await run(80094);
    for (const api of BERA_COMET_APIS) expect(bera.some((u) => u.startsWith(api))).toBe(true);
  });

  it("an injected verifier's failure without a message is still a Willow error", async () => {
    const { anchorFailure } = await import('../src/anchor.js');
    const e = anchorFailure({ ok: false, kind: 'rejected' } as never, 'eth_blockNumber');
    expect(e).toMatchObject({ code: -32098 });
  });
});
