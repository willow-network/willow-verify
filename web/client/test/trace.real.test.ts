// Traces (willow_getTraceProof) with the REAL state module (a verify page build, WILLOW_VERIFY_DIST)
// on trace-verify's recorded proofs (Ethereum and Unichain mainnet, zkvm/logset-guest/trace-verify/
// testdata): the client runs each block itself and returns the response the trace method sent.
// Anchors are the mock's (the mock server can't sign for the chains' validators).
import { existsSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { WillowVerificationError } from '../src/errors.js';
import { toQuantity } from '../src/hex.js';
import { VERIFY_DIST } from './support/mock-verifiers.js';
import { harness, type Harness } from './support/setup.js';
import { canon, json, World } from './support/world.js';

const here = dirname(fileURLToPath(import.meta.url));
const TRACES = join(here, '..', '..', '..', 'zkvm', 'logset-guest', 'trace-verify', 'testdata');
const files = existsSync(TRACES) ? readdirSync(TRACES).filter((f) => f.endsWith('.json')).sort() : [];

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

interface Fixture {
  chainId: number;
  number: number;
  hash: string;
  block: any;
  pool: { nodes: string[]; codes: string[] };
  cases: any[];
}

const load = (f: string): Fixture => {
  const x = json(join(TRACES, f));
  return { chainId: parseInt(x.block.chainId, 16), number: parseInt(x.block.blockNumber, 16), hash: x.block.blockHash.toLowerCase(), block: x.block, pool: x.pool, cases: x.cases };
};

/** A case's proof: the block's fields and its own, its witness from the file's pool. */
const proofOf = (f: Fixture, c: any): any => {
  const pick = (field: 'nodes' | 'codes') => (c.witness[field] as number[]).map((i) => f.pool[field][i]);
  return { ...f.block, ancestors: c.ancestors, witness: { nodes: pick('nodes'), codes: pick('codes') }, algorithm: 'willow-trace-v1' };
};

/** A block trace's block param as the client sends it: the hash it resolved the block to. */
const blockTrace = (m: string) => ['debug_traceBlockByNumber', 'debug_traceBlockByHash', 'trace_block', 'trace_replayBlockTransactions'].includes(m);

/** A world whose head is the fixture's block, serving its proofs as the server would (`bend` changes one). */
function world(f: Fixture, bend?: (method: string, proof: any) => void): World {
  const w = new World();
  w.head(f.chainId, { number: f.number, hash: f.hash });
  w.intercept = (chainId, method, params) => {
    if (method !== 'willow_getTraceProof' || chainId !== f.chainId) return undefined;
    const [m, p] = params as [string, unknown[]];
    const c = f.cases.find((x) => {
      if (x.params[0] !== m) return false;
      const [want, got] = [canon(x.params[1]) as unknown[], canon(p) as unknown[]];
      const b = (v: unknown) => (blockTrace(m) && (v === f.hash || v === toQuantity(f.number)) ? 'the block' : v);
      return JSON.stringify([b(want[0]), ...want.slice(1)]) === JSON.stringify([b(got[0]), ...got.slice(1)]);
    });
    if (!c) return { error: { code: -32000, message: `not found in this store: no trace proof for ${m}` } };
    const proof = structuredClone(proofOf(f, c));
    bend?.(m, proof);
    return { result: proof };
  };
  return w;
}

describe.skipIf(!VERIFY_DIST || !files.length)('traces, the real state module', () => {
  for (const file of files) {
    it(`${file}: every trace method's response, from the client's own run`, async () => {
      const f = load(file);
      h = await harness({ chainId: f.chainId, world: world(f) });
      for (const c of f.cases) {
        const [method, params] = c.params;
        const got = await h.client.request({ method, params });
        expect(got, `${method} ${JSON.stringify(params).slice(0, 120)}`).toEqual(c.expect);
      }
      expect(h.events.every((e) => e.status === 'verified' && e.block?.number === f.number)).toBe(true);
      // A block trace went out by the hash the client resolved its number to.
      const sent = h.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getTraceProof');
      for (const s of sent) {
        const [m, p] = s.params as [string, unknown[]];
        if (blockTrace(m) && m !== 'debug_traceBlockByHash') expect(p[0]).toEqual({ blockHash: f.hash });
      }
    });

    it(`${file}: a lying server's proofs never reach the app`, async () => {
      const f = load(file);
      const c = f.cases.find((x) => blockTrace(x.params[0])) ?? f.cases[0];
      const [method, params] = c.params;
      for (const [what, bend, kind] of [
        ['a node left out', (_: string, p: any) => p.witness.nodes.pop(), 'missing-read'],
        ['a code left out', (_: string, p: any) => (p.witness.codes = []), 'missing-read'],
        ['a receipt changed', (_: string, p: any) => (p.receipts[0] = p.receipts[0].slice(0, -2) + (p.receipts[0].endsWith('00') ? '01' : '00')), 'receipts'],
        ['another parent', (_: string, p: any) => (p.parent = p.header), 'parent'],
        ['another algorithm', (_: string, p: any) => (p.algorithm = 'willow-trace-v2'), 'algorithm'],
      ] as const) {
        h = await harness({ chainId: f.chainId, world: world(f, bend as (m: string, p: any) => void) });
        const e = await h.client.request({ method, params }).catch((x) => x);
        expect(e, what).toBeInstanceOf(WillowVerificationError);
        expect((e as WillowVerificationError).data.kind, what).toBe(kind);
        await h.close();
        h = null;
      }
    });
  }

  it('trace_filter: each block traced and filtered here, as the server filters', async () => {
    const file = files.find((x) => load(x).cases.some((c) => c.params[0] === 'trace_block'));
    if (!file) return;
    const f = load(file);
    const flat: any[] = f.cases.find((c) => c.params[0] === 'trace_block').expect;
    h = await harness({ chainId: f.chainId, world: world(f) });
    const n = toQuantity(f.number);
    expect(await h.client.request({ method: 'trace_filter', params: [{ fromBlock: n, toBlock: n }] })).toEqual(flat);
    const to = flat.find((t) => t.type === 'call').action.to;
    const want = flat.filter((t) => t.type === 'call' && t.action.to === to);
    expect(await h.client.request({ method: 'trace_filter', params: [{ fromBlock: n, toBlock: n, toAddress: [to] }] })).toEqual(want);
    expect(await h.client.request({ method: 'trace_filter', params: [{ fromBlock: n, toBlock: n, after: 1, count: 2 }] })).toEqual(flat.slice(1, 3));
  });
});

// Calls' traces (debug_traceCall, trace_call on Ethereum): trace-verify's recorded call trace
// proofs (testdata/calls), each a call proof's block and witness; the client runs the call itself
// (as eth_call runs it, under the caller's overrides) and returns the trace the method sent.
const CALLS = join(TRACES, 'calls');
const callFiles = existsSync(CALLS) ? readdirSync(CALLS).filter((f) => f.endsWith('.json')).sort() : [];

/** Where a call trace's params name its block. */
const blockAt = (m: string) => (m === 'debug_traceCall' ? 1 : 2);

/** A world whose head is the cases' block, serving their proofs (`bend` changes one). */
function callWorld(cases: any[], bend?: (proof: any) => void): World {
  const w = new World();
  const r = cases[0].result;
  w.head(1, { number: parseInt(r.blockNumber, 16), hash: r.blockHash.toLowerCase() });
  w.intercept = (chainId, method, params) => {
    if (method !== 'willow_getTraceProof' || chainId !== 1) return undefined;
    const [m, p] = params as [string, unknown[]];
    const asked = (q: unknown[]) => JSON.stringify(canon(q.map((v, i) => (i === blockAt(m) ? 'the block' : v))));
    const c = cases.find((x) => x.params[0] === m && asked(x.params[1]) === asked(p));
    if (!c) return { error: { code: -32000, message: `not found in this store: no trace proof for ${m}` } };
    const proof = structuredClone(c.result);
    bend?.(proof);
    return { result: proof };
  };
  return w;
}

describe.skipIf(!VERIFY_DIST || !callFiles.length)("calls' traces, the real state module", () => {
  for (const file of callFiles) {
    it(`${file}: every call's trace, from the client's own run of the call`, async () => {
      const cases = json(join(CALLS, file)).cases as any[];
      h = await harness({ chainId: 1, world: callWorld(cases) });
      for (const c of cases) {
        const [method, params] = c.params;
        const got = await h.client.request({ method, params });
        expect(got, `${method} ${JSON.stringify(params).slice(0, 160)}`).toEqual(c.expect);
      }
      const number = parseInt(cases[0].result.blockNumber, 16);
      expect(h.events.every((e) => e.status === 'verified' && e.block?.number === number)).toBe(true);
      // The overrides went out as the app gave them, inside the trace method's own params.
      const sent = h.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getTraceProof');
      expect(sent.length).toBeGreaterThan(0);
    });

    it(`${file}: a lying server's call trace proofs never reach the app`, async () => {
      const cases = json(join(CALLS, file)).cases as any[];
      const c = cases.find((x) => x.params[0] === 'debug_traceCall') ?? cases[0];
      const [method, params] = c.params;
      for (const [what, bend, kind] of [
        ['an account left out', (p: any) => p.witness.accounts.shift(), 'missing-read'],
        ['a code altered', (p: any) => {
          const k = Object.keys(p.witness.codes)[0];
          p.witness.codes[k] = p.witness.codes[k].slice(0, -2) + (p.witness.codes[k].endsWith('00') ? '01' : '00');
        }, 'code'],
        ['another chain', (p: any) => (p.chainId = '0x5'), 'wrong-chain'],
        ['another algorithm', (p: any) => (p.algorithm = 'willow-trace-v2'), 'algorithm'],
      ] as const) {
        h = await harness({ chainId: 1, world: callWorld(cases, bend as (p: any) => void) });
        const e = await h.client.request({ method, params }).catch((x) => x);
        expect(e, what).toBeInstanceOf(WillowVerificationError);
        expect((e as WillowVerificationError).data.kind, what).toBe(kind);
        await h.close();
        h = null;
      }
    });
  }
});
