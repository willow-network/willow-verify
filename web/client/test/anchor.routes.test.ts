// Where each light client's data is asked for, with a scripted backend: Ethereum's on
// chain 1's route, Sepolia's and BNB's on the verified chain's own route, BNB's updates 64 at a time,
// each kept under its own storage key; and BNB's newest block served at minLevel final. Sepolia's
// own blocks (11155111) as Ethereum's, on Sepolia's light client.
import { describe, expect, it } from 'vitest';
import { Anchors } from '../src/anchor.js';
import { lightClientOf, type Backend } from '../src/backend.js';
import { chooseStarts, SHIPPED_STARTS } from '../src/checkpoint.js';
import type { Reply } from '../src/http.js';
import type { KeyValueStorage } from '../src/storage.js';
import type { AnchorFailure, AnchorVerified, LightClientName } from '../src/verifiers.js';

type Step = AnchorVerified | AnchorFailure;

/** A backend whose `anchorVerify` gives `steps` in turn, and that records the other calls. */
function scripted(steps: Step[], saved = '{"kept":1}') {
  const calls: string[] = [];
  const backend: Backend = {
    itemRequest: async () => ({ ok: false, kind: 'internal', message: '' }) as any,
    itemVerify: async () => ({ ok: false, kind: 'internal', message: '' }) as any,
    stateRequest: async () => ({ ok: false, kind: 'internal', message: '' }) as any,
    stateVerify: async () => ({ ok: false, kind: 'internal', message: '' }) as any,
    logsVerify: async () => ({ ok: false, kind: 'internal', message: '' }) as any,
    anchorRestore: async (_s, _n, lc?: LightClientName) => (calls.push(`restore:${lc ?? 'ethereum'}`), { ok: true }),
    anchorBootstrap: async (_r, _n, lc?: LightClientName) => (calls.push(`bootstrap:${lc ?? 'ethereum'}`), { ok: true, period: 1 }),
    anchorUpdates: async (_r, lc?: LightClientName) => (calls.push(`updates:${lc ?? 'ethereum'}`), { ok: true, period: 2 }),
    anchorStart: async (lc) => (calls.push(`start:${lc}`), { ok: true }),
    anchorVerify: async () => {
      const r = steps.shift()!;
      return { result: r, saved: r.ok ? saved : null };
    },
    close: () => {},
  };
  return { backend, calls };
}

function willow() {
  const asked: { chainId: number; method: string; params: unknown[] }[] = [];
  const rpc = async (chainId: number, method: string, params: unknown[]): Promise<Reply> => {
    asked.push({ chainId, method, params });
    const value = { jsonrpc: '2.0', id: 1, result: method === 'willow_getBlockAnchor' ? { blockNumber: '0x10' } : [] } as Reply['value'];
    return { value, text: JSON.stringify(value) };
  };
  return { rpc, asked };
}

const memory = (): KeyValueStorage & { m: Map<string, string> } => {
  const m = new Map<string, string>();
  return { m, get: (k) => m.get(k) ?? null, set: (k, v) => void m.set(k, v) };
};

const ok = (extra: Partial<AnchorVerified> = {}): AnchorVerified => ({ ok: true, level: 'final', path: 'game', hash: `0x${'11'.repeat(32)}`, matches: true, ...extra });
const need = (kind: 'need-bootstrap' | 'need-updates', lightClient?: LightClientName, from = 0, count = 0): AnchorFailure =>
  ({ ok: false, kind, message: '', ...(lightClient ? { lightClient } : {}), ...(kind === 'need-updates' ? { from, count } : {}) }) as AnchorFailure;

function anchors(
  backend: Backend,
  rpc: ReturnType<typeof willow>['rpc'],
  storage: KeyValueStorage | null,
  minLevel: 'signed' | 'final' = 'signed',
  refreshStart?: (lc: LightClientName) => Promise<boolean>,
) {
  return new Anchors({
    refreshStart,
    backend,
    rpc,
    storage,
    checkpointRoot: async (lc) => (lc === 'sepolia' ? '0xsepolia' : lc === 'bnb' ? '0xbnb' : lc === 'berachain' ? '0xberachain' : '0xethereum'),
    startHeight: async (lc) => (lc === 'berachain' ? 28_955_000 : undefined),
    minLevel,
    headMaxAgeMs: () => 2_000,
    maxHeadAgeSec: 600,
    nowMs: () => 1_000_000_000_000,
  });
}

describe('light clients on their routes', () => {
  it("Sepolia's bootstrap and updates are asked on the testnet's route, kept under Sepolia's key", async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'sepolia'), need('need-updates', 'sepolia', 1413, 2), ok()]);
    const w = willow();
    const storage = memory();
    const a = anchors(backend, w.rpc, storage);
    await a.verify(1301, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked).toEqual([
      { chainId: 1301, method: 'willow_lightClientBootstrap', params: ['0xsepolia'] },
      { chainId: 1301, method: 'willow_lightClientUpdates', params: [1413, 2] },
    ]);
    expect(calls).toEqual(['bootstrap:sepolia', 'updates:sepolia']);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client:sepolia']);
  });

  it("BNB's walk is asked on chain 56's route, 64 epochs at a time", async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'bnb'), need('need-updates', 'bnb', 133_511, 130), ok({ path: 'parlia', anchorTime: 999_999_990 })]);
    const w = willow();
    const a = anchors(backend, w.rpc, null);
    await a.verify(56, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [56, 'willow_lightClientBootstrap', ['0xbnb']],
      [56, 'willow_lightClientUpdates', [133_511, 64]],
      [56, 'willow_lightClientUpdates', [133_575, 64]],
      [56, 'willow_lightClientUpdates', [133_639, 2]],
    ]);
    expect(calls).toEqual(['bootstrap:bnb', 'updates:bnb', 'updates:bnb', 'updates:bnb']);
  });

  it("Ethereum's light client stays on chain 1's route, under its key, for an L2", async () => {
    const { backend } = scripted([need('need-bootstrap'), need('need-updates', undefined, 1906, 20), ok()]);
    const w = willow();
    const storage = memory();
    await anchors(backend, w.rpc, storage).verify(1868, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [1, 'willow_lightClientBootstrap', ['0xethereum']],
      [1, 'willow_lightClientUpdates', [1906, 16]],
      [1, 'willow_lightClientUpdates', [1922, 4]],
    ]);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client']);
  });

  it("opBNB's anchors rest on BNB's light client: asked on opBNB's route, kept under BNB's key", async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'bnb'), need('need-updates', 'bnb', 133_511, 2), ok({ path: 'output', level: 'committed' })]);
    const w = willow();
    const storage = memory();
    await anchors(backend, w.rpc, storage).verify(204, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [204, 'willow_lightClientBootstrap', ['0xbnb']],
      [204, 'willow_lightClientUpdates', [133_511, 2]],
    ]);
    expect(calls).toEqual(['bootstrap:bnb', 'updates:bnb']);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client:bnb']);
  });

  it("ApeChain's anchors rest on Ethereum's light client, on chain 1's route", async () => {
    const { backend } = scripted([need('need-bootstrap'), need('need-updates', undefined, 1906, 2), ok({ path: 'node', level: 'committed' })]);
    const w = willow();
    const storage = memory();
    await anchors(backend, w.rpc, storage).verify(33139, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [1, 'willow_lightClientBootstrap', ['0xethereum']],
      [1, 'willow_lightClientUpdates', [1906, 2]],
    ]);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client']);
  });

  it("each light client's kept data is restored with its own name", async () => {
    const { backend, calls } = scripted([ok()]);
    const storage = memory();
    storage.set('willow-rpc:light-client', '{}');
    storage.set('willow-rpc:light-client:bnb', '{}');
    await anchors(backend, willow().rpc, storage).verify(56, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(calls).toEqual(['restore:ethereum', 'restore:bnb']);
  });

  it("BNB's newest block is served at minLevel final; an L2's is not", async () => {
    const { backend } = scripted([ok({ path: 'parlia', level: 'final', anchorTime: 999_999_995 })]);
    const head = await anchors(backend, willow().rpc, null, 'final').head(56, 'latest');
    expect([head.number, head.level, head.expires]).toEqual([16, 'final', Infinity]);
    const { backend: b } = scripted([]);
    await expect(anchors(b, willow().rpc, null, 'final').head(1868, 'latest')).rejects.toMatchObject({ data: { kind: 'anchor-level' } });
  });

  it("Berachain's start is asked by hash and height, its updates in one ask, on chain 80094's route", async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'berachain'), need('need-updates', 'berachain', 28_955_000, 1_000_000), ok({ path: 'cometbft', anchorTime: 999_999_990 })]);
    const w = willow();
    const storage = memory();
    await anchors(backend, w.rpc, storage).verify(80094, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [80094, 'willow_lightClientBootstrap', ['0xberachain', 28_955_000]],
      [80094, 'willow_lightClientUpdates', [28_955_000, 1_000_000]],
    ]);
    expect(calls).toEqual(['bootstrap:berachain', 'updates:berachain']);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client:berachain']);
  });

  it("Berachain's newest block is final, aged by its header's time", async () => {
    const { backend } = scripted([ok({ path: 'cometbft', level: 'final', anchorTime: 999_999_995 })]);
    const head = await anchors(backend, willow().rpc, null, 'final').head(80094, 'latest');
    expect([head.number, head.level, head.expires, head.time]).toEqual([16, 'final', Infinity, 999_999_995]);
    const { backend: b } = scripted([ok({ path: 'cometbft', level: 'final', anchorTime: 999_000_000 })]);
    await expect(anchors(b, willow().rpc, null, 'final').head(80094, 'latest')).rejects.toMatchObject({ data: { kind: 'anchor-stale-head' } });
  });

  it("Sepolia's own blocks use Sepolia's light client on chain 11155111's route", async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'sepolia'), need('need-updates', 'sepolia', 1415, 1), ok({ path: 'hops' })]);
    const w = willow();
    const storage = memory();
    await anchors(backend, w.rpc, storage).verify(11155111, 16, `0x${'11'.repeat(32)}`, '{}');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([
      [11155111, 'willow_lightClientBootstrap', ['0xsepolia']],
      [11155111, 'willow_lightClientUpdates', [1415, 1]],
    ]);
    expect(calls).toEqual(['bootstrap:sepolia', 'updates:sepolia']);
    expect([...storage.m.keys()]).toEqual(['willow-rpc:light-client:sepolia']);
  });

  it("Sepolia's newest block at minLevel final is its finalized one, its signed head counts 10 minutes, as Ethereum's", async () => {
    const { backend } = scripted([ok({ path: 'history', level: 'final', finalized: 16, head: 40, headTime: 999_999_990 })]);
    const w = willow();
    const head = await anchors(backend, w.rpc, null, 'final').head(11155111, 'latest');
    expect(w.asked.map((x) => [x.chainId, x.method, x.params])).toEqual([[11155111, 'willow_getBlockAnchor', ['finalized']]]);
    expect([head.number, head.level, head.expires]).toEqual([16, 'final', Infinity]);
    const { backend: b } = scripted([ok({ path: 'head', level: 'signed', finalized: 8, head: 16, headTime: 999_999_990 })]);
    const signed = await anchors(b, willow().rpc, null).head(11155111, 'latest');
    expect([signed.number, signed.level, signed.expires]).toEqual([16, 'signed', (999_999_990 + 600) * 1000]);
  });

  it("a BNB block one validator sealed counts 10 minutes from its time", async () => {
    const { backend } = scripted([ok({ path: 'parlia', level: 'signed', anchorTime: 999_999_990 })]);
    const got = await anchors(backend, willow().rpc, null).block(56, 16, `0x${'11'.repeat(32)}`);
    expect(got.expires).toBe((999_999_990 + 600) * 1000);
  });
});

describe("Berachain's start", () => {
  const newer = { height: SHIPPED_STARTS.berachain.height + 100_000, hash: `0x${'ab'.repeat(32)}`, time: SHIPPED_STARTS.berachain.time + 200_000 };
  const fetchWith = (body: unknown) => (async () => ({ json: async () => body })) as unknown as typeof fetch;

  it('is the shipped one while fresh, then checkpoint.json\'s newer one', async () => {
    expect(lightClientOf(80094)).toBe('berachain');
    const fresh = await chooseStarts({ fetch: fetchWith({ berachain: newer }), nowSec: () => SHIPPED_STARTS.berachain.time + 60, ...pinned() });
    expect(fresh.berachain).toEqual(SHIPPED_STARTS.berachain);
    const later = await chooseStarts({ fetch: fetchWith({ berachain: newer }), nowSec: () => SHIPPED_STARTS.berachain.time + 7 * 3600, ...pinned() });
    expect(later.berachain).toEqual(newer);
    // Not a start, or older: the shipped one stays.
    const junk = await chooseStarts({ fetch: fetchWith({ berachain: { height: -1, hash: '0x12', time: 1 } }), nowSec: () => SHIPPED_STARTS.berachain.time + 7 * 3600, ...pinned() });
    expect(junk.berachain).toEqual(SHIPPED_STARTS.berachain);
  });

  it("is checkpoint.json's only when a CometBFT API has its header hash and none another", async () => {
    const later = () => SHIPPED_STARTS.berachain.time + 7 * 3600;
    const apis = (ids: Record<string, string | null>) =>
      (async (u: string) => {
        const url = String(u);
        if (url.endsWith('/checkpoint.json')) return { ok: true, json: async () => ({ berachain: newer }) };
        const api = Object.keys(ids).find((k) => url.startsWith(k));
        const id = api ? ids[api] : null;
        if (id === null || !url.includes(`height=${newer.height}`)) return { ok: false, json: async () => ({}) };
        return { ok: true, json: async () => ({ result: { signed_header: { commit: { block_id: { hash: id.slice(2).toUpperCase() } } } } }) };
      }) as unknown as typeof fetch;
    const cp = 'https://w.test/checkpoint.json';
    const pick = (ids: Record<string, string | null>) => chooseStarts({ fetch: apis(ids), checkpointUrl: cp, cometApis: Object.keys(ids), nowSec: later, ...pinned() });
    expect((await pick({ 'https://a.test': newer.hash, 'https://b.test': null })).berachain).toEqual(newer);
    expect((await pick({ 'https://a.test': newer.hash, 'https://b.test': `0x${'ef'.repeat(32)}` })).berachain).toEqual(SHIPPED_STARTS.berachain);
    expect((await pick({ 'https://a.test': null, 'https://b.test': null })).berachain).toEqual(SHIPPED_STARTS.berachain);
  });

  it("is not asked of the CometBFT APIs unless it is newer than the start in use (after)", async () => {
    const asked: string[] = [];
    const f = (async (u: string) => {
      asked.push(String(u));
      return String(u).endsWith('/checkpoint.json') ? { ok: true, json: async () => ({ berachain: newer }) } : { ok: false, body: null, json: async () => ({}) };
    }) as unknown as typeof fetch;
    const got = await chooseStarts({ fetch: f, checkpointUrl: 'https://w.test/checkpoint.json', cometApis: ['https://a.test'], after: { berachain: newer }, nowSec: () => SHIPPED_STARTS.berachain.time + 7 * 3600, ...pinned() });
    expect(got.berachain).toEqual(SHIPPED_STARTS.berachain);
    expect(asked).toEqual(['https://w.test/checkpoint.json']);
  });

  /** The other starts passed in, so only Berachain's is chosen. */
  function pinned() {
    return { sepolia: SHIPPED_STARTS.sepolia, bnb: SHIPPED_STARTS.bnb };
  }
});

describe('a light client whose start aged out', () => {
  const stale = (): AnchorFailure => ({ ok: false, kind: 'stale-checkpoint', message: "this page's starting point on Berachain is more than a day old" }) as AnchorFailure;

  it('takes a newer start once and bootstraps from it (a long-running client keeps Berachain)', async () => {
    const { backend, calls } = scripted([stale(), need('need-bootstrap', 'berachain'), ok({ path: 'cometbft', anchorTime: 999_999_990 })]);
    const refreshed: LightClientName[] = [];
    const a = anchors(backend, willow().rpc, null, 'signed', async (lc) => (refreshed.push(lc), true));
    await expect(a.verify(80094, 16, `0x${'11'.repeat(32)}`, '{}')).resolves.toMatchObject({ ok: true });
    expect(refreshed).toEqual(['berachain']);
    expect(calls).toEqual(['bootstrap:berachain']);
  });

  it('refuses as before when there is no newer start, or the newer one is stale too', async () => {
    const none = scripted([stale()]);
    await expect(anchors(none.backend, willow().rpc, null, 'signed', async () => false).verify(80094, 16, `0x${'11'.repeat(32)}`, '{}')).rejects.toMatchObject({
      code: -32099,
      data: { kind: 'anchor-stale-checkpoint' },
    });
    const twice = scripted([stale(), stale()]);
    const refreshed: LightClientName[] = [];
    await expect(
      anchors(twice.backend, willow().rpc, null, 'signed', async (lc) => (refreshed.push(lc), true)).verify(80094, 16, `0x${'11'.repeat(32)}`, '{}'),
    ).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-checkpoint' } });
    expect(refreshed).toEqual(['berachain']);
  });

  it('takes a newer start when the bootstrap itself refuses the old one (never bootstrapped, as the wasm checks)', async () => {
    const { backend, calls } = scripted([need('need-bootstrap', 'berachain'), need('need-bootstrap', 'berachain'), ok({ path: 'cometbft', anchorTime: 999_999_990 })]);
    let boots = 0;
    backend.anchorBootstrap = async (_r, _n, lc) => {
      calls.push(`bootstrap:${lc}`);
      return ++boots === 1 ? { ok: false, kind: 'stale-checkpoint', message: "this page's starting point on Berachain is more than a day old" } : { ok: true, period: 1 };
    };
    const refreshed: LightClientName[] = [];
    const a = anchors(backend, willow().rpc, null, 'signed', async (lc) => (refreshed.push(lc), true));
    await expect(a.verify(80094, 16, `0x${'11'.repeat(32)}`, '{}')).resolves.toMatchObject({ ok: true });
    expect(refreshed).toEqual(['berachain']);
    expect(calls).toEqual(['bootstrap:berachain', 'bootstrap:berachain']);
  });

  it('a refresh does not use up a catch-up round (a bootstrap and two updates still fit)', async () => {
    const steps = [stale(), need('need-bootstrap', 'berachain'), need('need-updates', 'berachain', 1, 1), need('need-updates', 'berachain', 2, 1), ok({ path: 'cometbft', anchorTime: 999_999_990 })];
    const { backend } = scripted(steps);
    await expect(anchors(backend, willow().rpc, null, 'signed', async () => true).verify(80094, 16, `0x${'11'.repeat(32)}`, '{}')).resolves.toMatchObject({ ok: true });
  });

  it("leaves Ethereum's checkpoint alone (its staleness is chosen at start, not here)", async () => {
    const { backend } = scripted([{ ...stale(), message: 'the checkpoint is more than 30 days old' } as AnchorFailure]);
    const refreshed: LightClientName[] = [];
    await expect(anchors(backend, willow().rpc, null, 'signed', async (lc) => (refreshed.push(lc), true)).verify(1, 16, `0x${'11'.repeat(32)}`, '{}')).rejects.toMatchObject({
      data: { kind: 'anchor-stale-checkpoint' },
    });
    expect(refreshed).toEqual([]);
  });
});
