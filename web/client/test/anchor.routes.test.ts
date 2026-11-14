// Where each light client's data is asked for (#993, #994), with a scripted backend: Ethereum's on
// chain 1's route, Sepolia's and BNB's on the verified chain's own route, BNB's updates 64 at a time,
// each kept under its own storage key; and BNB's newest block served at minLevel final.
import { describe, expect, it } from 'vitest';
import { Anchors } from '../src/anchor.js';
import type { Backend } from '../src/backend.js';
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

function anchors(backend: Backend, rpc: ReturnType<typeof willow>['rpc'], storage: KeyValueStorage | null, minLevel: 'signed' | 'final' = 'signed') {
  return new Anchors({
    backend,
    rpc,
    storage,
    checkpointRoot: async (lc) => (lc === 'sepolia' ? '0xsepolia' : lc === 'bnb' ? '0xbnb' : '0xethereum'),
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

  it("a BNB block one validator sealed counts 10 minutes from its time", async () => {
    const { backend } = scripted([ok({ path: 'parlia', level: 'signed', anchorTime: 999_999_990 })]);
    const got = await anchors(backend, willow().rpc, null).block(56, 16, `0x${'11'.repeat(32)}`);
    expect(got.expires).toBe((999_999_990 + 600) * 1000);
  });
});
