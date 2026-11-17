// The worker protocol (worker-rpc.ts) over a MessageChannel: the same backend calls as inline,
// in order, with errors (a missing module among them) carried across.
import { MessageChannel } from 'node:worker_threads';
import { afterEach, describe, expect, it } from 'vitest';
import { InlineBackend, isNoVerifier } from '../src/backend.js';
import { serveBackend, WorkerBackend } from '../src/worker-rpc.js';
import { mockModules } from './support/mock-verifiers.js';
import { fixture, World } from './support/world.js';

const ports: MessageChannel[] = [];
afterEach(() => {
  for (const c of ports.splice(0)) {
    c.port1.close();
    c.port2.close();
  }
});

function pair(w: World, mocks: Parameters<typeof mockModules>[1] = {}) {
  const ch = new MessageChannel();
  ports.push(ch);
  serveBackend(ch.port2 as any, (cp) => new InlineBackend(mockModules(w, mocks), cp));
  return new WorkerBackend(ch.port1 as any, async () => ({ root: `0x${'11'.repeat(32)}`, slot: 1 }));
}

describe('the worker backend', () => {
  it('requests and verifies a block proof as inline does', async () => {
    const w = new World();
    const f = fixture('block/base-56049971.json');
    w.addBlock(f);
    const b = pair(w);
    const inline = new InlineBackend(mockModules(w), async () => ({ root: '0x', slot: 1 }));
    const target = f.result.blockHash;
    expect(await b.itemRequest(8453, 'block', target)).toEqual(await inline.itemRequest(8453, 'block', target));
    const body = JSON.stringify({ jsonrpc: '2.0', id: 1, result: f.result });
    const [x, y] = await Promise.all([b.itemVerify(8453, 'block', target, body, true), inline.itemVerify(8453, 'block', target, body, true)]);
    expect(x).toEqual(y);
    expect(x.ok).toBe(true);
  });

  it('runs the anchor calls in order on one light client', async () => {
    const w = new World();
    const e = w.head(1, { number: 5, hash: `0x${'ab'.repeat(32)}` });
    const b = pair(w);
    const body = JSON.stringify({ jsonrpc: '2.0', id: 1, result: w.anchorBody(e) });
    const r = await Promise.all([b.anchorRestore('[]', w.nowSec), b.anchorVerify(1, 5, e.hash, body, w.nowSec), b.anchorVerify(1, 6, e.hash, body, w.nowSec)]);
    expect(r[1].result).toMatchObject({ ok: true, matches: true });
    expect(r[2].result).toMatchObject({ ok: false, kind: 'rejected' });
  });

  it('a module that is not there arrives as a no-verifier error', async () => {
    const b = pair(new World(), { state: 'none' });
    const e = await b.stateRequest(1, 'state', '[]').catch((x) => x);
    expect(isNoVerifier(e)).toBe(true);
  });
});
