// The lying server (design 6.3, W7): every mutation in the table must reach the app as
// WillowVerificationError (or, for denials, WillowUnverifiableError or an absence) and never as a
// value. It runs through the proxy against the mock server: with the mock verifiers, or with
// WILLOW_VERIFY_DIST=<a page build> its real items and state modules (anchors stay the mock's).
// Once W3's proof methods are served, the same table runs against the live server:
//   WILLOW_LIVE_URL=https://api.willow.tech WILLOW_KEY=<key> WILLOW_VERIFY_DIST=<a W5 page build> npx vitest run test/lying.test.ts
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { createVerifier } from '../src/client.js';
import { distModules } from '../src/wasm.js';
import { startProxy, type Proxy } from './support/lying-proxy.js';
import { proofModules } from './support/mock-verifiers.js';
import { serve, type Running } from './support/mock-server.js';
import { anchorSeen, cases } from './support/mutations.js';
import { blockOf, fullWorld, HEAD_FIXTURE, headAt } from './support/setup.js';
import type { World } from './support/world.js';

const LIVE = process.env.WILLOW_LIVE_URL;
const KEY = process.env.WILLOW_KEY ?? 'test-key';
const W2_BLOCK = { number: 0x192b4b5, hash: '0x8a27150df15c313310e346ad53e1e04825a7004e0e01096b5cf095e6c7d89552' };

let world: World;
let server: Running | null = null;
let proxy: Proxy;

beforeAll(async () => {
  world = fullWorld();
  if (!LIVE) server = await serve(world);
  proxy = await startProxy(LIVE ?? server!.base);
});
afterAll(async () => {
  await proxy.close();
  await server?.close();
});

describe('the lying server', () => {
  const table = cases();
  it('holds a case for each mutation the design lists', () => {
    const names = table.map((c) => c.name).join('\n');
    for (const want of ['the anchor of another block', 'stale head', 'another chain', 'another block for a blockHash request', 'an item dropped', "ids swapped", 'a lower pending nonce']) expect(names).toContain(want);
  });

  for (const c of table) {
    it.skipIf(!!LIVE && !!c.fixtureOnly)(c.name, async () => {
      let clock = world.nowSec;
      if (!LIVE) {
        world.intercept = undefined;
        c.setup?.(world);
        if (c.head === 'w2') world.head(1, W2_BLOCK);
        else headAt(world, c.chainId, blockOf(HEAD_FIXTURE[c.chainId]));
      }
      const client = createVerifier({
        chainId: c.chainId,
        url: proxy.url(c.chainId, KEY),
        verifiers: LIVE ? distModules(process.env.WILLOW_VERIFY_DIST!) : proofModules(world),
        storage: false,
        ...(LIVE ? {} : { checkpoint: { root: `0x${'11'.repeat(32)}`, slot: 15_613_952 } }),
        now: () => clock * 1000,
        batchWindowMs: 2,
        waitMs: { anchor: 5, jobs: 5 },
      });
      try {
        proxy.set(null);
        if (c.warm !== false) {
          await client.request({ method: 'eth_blockNumber' });
          const seen = proxy.seen.get(`willow_getBlockAnchor ${JSON.stringify(['latest'])}`);
          if (seen?.length) anchorSeen.set(`${c.chainId}:latest`, seen.at(-1).result);
        }
        if (c.replayAfterSec) {
          clock += c.replayAfterSec;
          // The honest server's head moves on with the clock: only the replay is stale.
          if (!LIVE) world.heads.get(c.chainId)!.time = clock - 2;
        }
        proxy.set(c.mutation);
        const r = await c.ask(client).then(
          (v) => ({ v }),
          (e) => ({ e }),
        );
        expect(proxy.applied(), 'the mutation applied').toBeGreaterThan(0);
        if ('code' in c.expect) {
          expect('v' in r, 'no value reached the app').toBe(false);
          expect((r as { e: any }).e).toMatchObject({ code: c.expect.code, data: { kind: c.expect.kind } });
        } else if (c.expect.outcome === 'null') expect(r).toEqual({ v: null });
        else expect(c.expect.check((r as { v: unknown }).v)).toBe(true);
      } finally {
        proxy.set(null);
        client.close();
      }
    });
  }
});
