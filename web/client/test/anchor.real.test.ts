// The anchor flow (design section 2, appendix D) with the REAL anchor module: a verify page build
// (WILLOW_VERIFY_DIST), on W4's I5 fixtures. A build older than W4's `anchorTime` field (the
// 10/21 one) gets it from withAnchorTime, from each fixture's own `expect`; W5's builds carry it.
import { afterEach, describe, expect, it } from 'vitest';
import { Anchors } from '../src/anchor.js';
import { InlineBackend } from '../src/backend.js';
import { Batcher } from '../src/http.js';
import { toNumber } from '../src/hex.js';
import type { KeyValueStorage } from '../src/storage.js';
import { mockModules, realModule, VERIFY_DIST, withAnchorTime } from './support/mock-verifiers.js';
import { harness, type Harness } from './support/setup.js';
import { flip } from './support/lying-proxy.js';
import { serve } from './support/mock-server.js';
import { fixture, World } from './support/world.js';

const CHAINS = [1, 8453, 10, 130, 57073, 42161, 4663];
const tag = (name: string) => fixture(`anchor-tags/${name}.json`);
const CHECKPOINT = tag('checkpoint');
const SAVED_AT = tag('latest-1').savedAt as number;

function i5World(): World {
  const w = new World();
  w.nowSec = SAVED_AT;
  w.lightClient = { bootstrap: tag('bootstrap'), updates: tag('updates') };
  for (const c of CHAINS) w.realAnchors.set(`${c}:latest`, tag(`latest-${c}`).response.result);
  w.realAnchors.set('1:finalized', tag('finalized-1').response.result);
  return w;
}

const anchorTime = (c: number): number | undefined => (c === 1 ? undefined : tag(`latest-${c}`).expect.anchorTime);

async function realAnchor() {
  const { Anchor } = await realModule('anchor');
  return { Anchor: withAnchorTime(Anchor, anchorTime) };
}

const memory = (): KeyValueStorage & { m: Map<string, string> } => {
  const m = new Map<string, string>();
  return { m, get: (k) => m.get(k) ?? null, set: (k, v) => void m.set(k, v) };
};

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

async function client(chainId: number, world = i5World(), options: Record<string, unknown> = {}) {
  return harness({ chainId, world, verifiers: { ...mockModules(world), anchor: realAnchor }, options: { checkpoint: CHECKPOINT, now: () => world.nowSec * 1000, ...options } });
}

describe.skipIf(!VERIFY_DIST)('the real anchor module on I5', () => {
  for (const c of CHAINS) {
    it(`chain ${c}: eth_blockNumber is the anchored head, its hash the public node's`, async () => {
      h = await client(c);
      const t = tag(`latest-${c}`);
      const n = await h.client.request({ method: 'eth_blockNumber' });
      expect(toNumber(n)).toBe(toNumber(t.response.result.blockNumber));
      const e = h.events.at(-1)!;
      expect(e.status).toBe('verified');
      expect(e.level).toBe('signed');
      expect(e.block!.hash).toBe(t.expect.hash);
    });
  }

  it('the light client catches up once (bootstrap, updates), and the next client restores it from storage', async () => {
    const storage = memory();
    h = await client(1, i5World(), { storage });
    await h.client.request({ method: 'eth_blockNumber' });
    const sent = h.world.log.flatMap((r) => r.items.map((i) => i.method));
    expect(sent.filter((m) => m === 'willow_lightClientBootstrap')).toHaveLength(1);
    expect(sent.filter((m) => m === 'willow_lightClientUpdates')).toHaveLength(1);
    expect(storage.m.get('willow-rpc:light-client')?.length).toBeGreaterThan(1000);
    await h.close();
    h = await client(1, i5World(), { storage });
    await h.client.request({ method: 'eth_blockNumber' });
    const again = h.world.log.flatMap((r) => r.items.map((i) => i.method));
    expect(again).not.toContain('willow_lightClientBootstrap');
    expect(again).not.toContain('willow_lightClientUpdates');
  });

  it('rule 3: Ethereum "latest" anchored by another path than head fails verification', async () => {
    const w = i5World();
    w.realAnchors.set('1:latest', tag('finalized-1').response.result);
    h = await client(1, w);
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-head' } });
  });

  it('rule 2: "finalized" must be the light client\'s finalized block, at level final', async () => {
    const w = i5World();
    w.realAnchors.set('1:finalized', tag('latest-1').response.result);
    const s = await serve(w);
    try {
      const anchors = anchorsFor(w, s.url(1), { root: CHECKPOINT.root, slot: CHECKPOINT.slot });
      await expect(anchors.head(1, 'finalized')).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-head' } });
      w.realAnchors.set('1:finalized', tag('finalized-1').response.result);
      const f = await anchorsFor(w, s.url(1), CHECKPOINT).head(1, 'finalized');
      expect(f).toMatchObject({ number: toNumber(tag('finalized-1').response.result.blockNumber), level: 'final', path: 'history', hash: tag('finalized-1').expect.hash });
    } finally {
      await s.close();
    }
  });

  it('rule 1: an L2 head older than maxHeadAgeSec is unverifiable', async () => {
    const w = i5World();
    w.nowSec = tag('latest-8453').expect.anchorTime + 601;
    h = await client(8453, w);
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-head' } });
  });

  it("Ethereum's signed head over 600 s old is unverifiable (the module's stale-head)", async () => {
    const w = i5World();
    w.nowSec = SAVED_AT + 1200;
    h = await client(1, w);
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32099, data: { kind: 'anchor-stale-head' } });
  });

  it("a forged validators' signature fails verification", async () => {
    const w = i5World();
    const r = structuredClone(tag('latest-1').response.result);
    r.update.data.sync_aggregate.sync_committee_signature = flip(r.update.data.sync_aggregate.sync_committee_signature, 10);
    w.realAnchors.set('1:latest', r);
    h = await client(1, w);
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-rejected' } });
  });

  it('an anchor that names another block than the head fails verification', async () => {
    const w = i5World();
    const r = structuredClone(tag('latest-1').response.result);
    r.blockNumber = `0x${(toNumber(r.blockNumber)! - 1).toString(16)}`;
    w.realAnchors.set('1:latest', r);
    h = await client(1, w);
    await expect(h.client.request({ method: 'eth_blockNumber' })).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-rejected' } });
  });

  for (const c of CHAINS) {
    it(`chain ${c}: an anchor that proves another hash for the block is a mismatch`, async () => {
      const w = i5World();
      const s = await serve(w);
      try {
        const t = tag(`latest-${c}`);
        const n = toNumber(t.response.result.blockNumber)!;
        const anchors = anchorsFor(w, s.url(c), CHECKPOINT, s.url(1));
        const body = { text: JSON.stringify(t.response), value: t.response };
        await expect(anchors.block(c, n, `0x${'ee'.repeat(32)}`, Promise.resolve(body))).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-mismatch' } });
        const ok = await anchors.block(c, n, t.expect.hash, Promise.resolve(body));
        expect(ok).toMatchObject({ number: n, hash: t.expect.hash, level: 'signed' });
      } finally {
        await s.close();
      }
    });
  }
});

function anchorsFor(w: World, url: string, checkpoint: { root: string; slot: number }, lcUrl = url): Anchors {
  const batcher = new Batcher({ fetch, windowMs: 1 });
  const backend = new InlineBackend({ anchor: realAnchor }, async () => checkpoint);
  return new Anchors({
    backend,
    rpc: (chainId, method, params) => batcher.call(chainId === 1 ? lcUrl : url, method, params),
    storage: null,
    checkpointRoot: async () => checkpoint.root,
    minLevel: 'signed',
    headMaxAgeMs: () => 2000,
    maxHeadAgeSec: 600,
    nowMs: () => w.nowSec * 1000,
    waitingMs: 5,
    anchorWaitMs: 50,
  });
}

