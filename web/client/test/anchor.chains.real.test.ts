// The chains added after the first seven (#992, #993, #994) through the REAL anchor module (a verify
// page build, WILLOW_VERIFY_DIST, built with them), on the anchors eth-anchor's recorders saved from
// public endpoints (web/verify/test/anchor/chains/): Ethereum's light client for Soneium, Zora and
// World Chain; Sepolia's for the testnets, its data on the testnet's own route; BNB's validators.
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { Anchors } from '../src/anchor.js';
import { InlineBackend, lightClientOf, type AnchorStart } from '../src/backend.js';
import type { Reply } from '../src/http.js';
import type { KeyValueStorage } from '../src/storage.js';
import { mockModules, realModule, VERIFY_DIST } from './support/mock-verifiers.js';
import { World } from './support/world.js';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '../../verify/test/anchor/chains');
const read = (f: string): any => JSON.parse(readFileSync(join(DIR, f), 'utf8'));
const saved = (chain: number): { name: string; f: any }[] =>
  readdirSync(DIR)
    .filter((n) => n.startsWith(`l2-${chain}-`))
    .map((name) => ({ name, f: read(name) }));

const period = (u: any) => Math.floor(Number(u.data.attested_header.beacon.slot) / 8192);

interface Asked {
  chainId: number;
  method: string;
  params: unknown[];
}

/** Willow as the recorders saw it: each light client's data on its route, and the saved anchors. */
function willow(anchors: Map<string, unknown>) {
  const asked: Asked[] = [];
  const bnb = read('bnb.json');
  const reply = (result: unknown): Reply => {
    const value = { jsonrpc: '2.0', id: 1, result } as Reply['value'];
    return { value, text: JSON.stringify(value) };
  };
  const rpc = async (chainId: number, method: string, params: unknown[]): Promise<Reply> => {
    asked.push({ chainId, method, params });
    const lc = chainId === 1 ? undefined : lightClientOf(chainId);
    if (method === 'willow_lightClientBootstrap') {
      return reply(lc === 'bnb' ? bnb.bootstrap : read(lc === 'sepolia' ? 'sepolia-bootstrap.json' : 'bootstrap.json'));
    }
    if (method === 'willow_lightClientUpdates') {
      const [from, count] = params as [number, number];
      const within = (k: number) => k >= from && k < from + count;
      if (lc === 'bnb') return reply(bnb.updates.filter((u: any) => within(u.epoch)));
      return reply(read(lc === 'sepolia' ? 'sepolia-updates.json' : 'updates.json').filter((u: any) => within(period(u))));
    }
    if (method === 'willow_getBlockAnchor') {
      const r = anchors.get(`${chainId}:${String(params[0])}`);
      if (r === undefined) return { value: { error: { code: -32000, message: 'not saved' } }, text: '{"error":{"code":-32000}}' };
      return reply(r);
    }
    throw new Error(`unexpected ${method}`);
  };
  return { rpc, asked };
}

function starts(): AnchorStart {
  const c = read('checkpoint.json');
  const b = read('bnb.json').start;
  return { root: c.root, slot: c.slot, sepolia: read('sepolia-checkpoint.json'), bnb: { number: b.number, hash: b.hash, time: 0 } };
}

const memory = (): KeyValueStorage & { m: Map<string, string> } => {
  const m = new Map<string, string>();
  return { m, get: (k) => m.get(k) ?? null, set: (k, v) => void m.set(k, v) };
};

function anchorsAt(nowSec: number, anchors: Map<string, unknown>, storage: KeyValueStorage | null = null, minLevel: 'signed' | 'final' = 'signed') {
  const w = willow(anchors);
  const backend = new InlineBackend({ ...mockModules(new World()), anchor: () => realModule('anchor') }, async () => starts());
  const a = new Anchors({
    backend,
    rpc: w.rpc,
    storage,
    checkpointRoot: async (lc) => (lc === 'sepolia' ? starts().sepolia!.root : lc === 'bnb' ? starts().bnb!.hash : starts().root),
    minLevel,
    headMaxAgeMs: () => 2_000,
    maxHeadAgeSec: 10 ** 9,
    nowMs: () => nowSec * 1000,
  });
  return { a, asked: w.asked };
}

const quantity = (n: number) => `0x${n.toString(16)}`;

describe.skipIf(!VERIFY_DIST)('the real anchor module on the new chains', () => {
  for (const chain of [1868, 7777777, 480, 11155420, 84532, 1301, 763373, 421614, 46630]) {
    it(`chain ${chain}: each saved anchor gives the public node's hash at its level, its light client's data on its route`, async () => {
      const all = saved(chain);
      expect(all.length).toBeGreaterThan(0);
      for (const { name, f } of all) {
        const { a, asked } = anchorsAt(f.savedAt, new Map([[`${chain}:${quantity(f.block)}`, f.result]]));
        const got = await a.block(chain, f.block, f.hash);
        expect([name, got.level, got.hash]).toEqual([name, f.expect.level, f.hash.toLowerCase()]);
        const lc = asked.filter((x) => x.method.startsWith('willow_lightClient'));
        expect(lc.length).toBeGreaterThan(0);
        // Ethereum's data on chain 1's route; Sepolia's on the testnet's own.
        const route = lightClientOf(chain) ? chain : 1;
        expect(lc.every((x) => x.chainId === route)).toBe(true);
        // Another hash is not the block's.
        const { a: b } = anchorsAt(f.savedAt, new Map([[`${chain}:${quantity(f.block)}`, f.result]]));
        await expect(b.block(chain, f.block, `0x${'ab'.repeat(32)}`)).rejects.toMatchObject({ code: -32098, data: { kind: 'anchor-mismatch' } });
      }
    });
  }

  it("BNB: blocks final through its validators' votes, the walk on chain 56's route, kept and restored", async () => {
    const bnb = read('bnb.json');
    const storage = memory();
    for (const x of bnb.anchors) {
      const { a, asked } = anchorsAt(x.savedAt, new Map([[`56:${quantity(x.block)}`, x.result]]), storage);
      const got = await a.block(56, x.block, x.hash);
      expect([got.level, got.hash, got.path]).toEqual(['final', x.hash.toLowerCase(), 'parlia']);
      for (const y of asked.filter((q) => q.method.startsWith('willow_lightClient'))) {
        expect(y.chainId).toBe(56);
        if (y.method === 'willow_lightClientUpdates') expect((y.params as number[])[1]).toBeLessThanOrEqual(64);
        else expect(y.params).toEqual([bnb.start.hash]);
      }
    }
    // The epochs were kept, under BNB's own key, and a new client verifies them again, unasked.
    expect(storage.m.has('willow-rpc:light-client:bnb')).toBe(true);
    expect(storage.m.has('willow-rpc:light-client')).toBe(false);
    const x = bnb.anchors[0];
    const { a, asked } = anchorsAt(x.savedAt, new Map([[`56:${quantity(x.block)}`, x.result]]), storage);
    expect((await a.block(56, x.block, x.hash)).level).toBe('final');
    expect(asked.filter((q) => q.method.startsWith('willow_lightClient'))).toEqual([]);
  });

  it("BNB: the newest block is its newest finalized one, served at minLevel final", async () => {
    const bnb = read('bnb.json');
    const newest = bnb.anchors.reduce((m: any, x: any) => (x.block > m.block ? x : m));
    const { a } = anchorsAt(newest.savedAt, new Map([['56:latest', newest.result]]), null, 'final');
    const head = await a.head(56, 'latest');
    expect([head.number, head.level, head.hash]).toEqual([newest.block, 'final', newest.hash.toLowerCase()]);
  });
});
