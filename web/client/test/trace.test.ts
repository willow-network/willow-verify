// Traces without a module: calls' traces off Ethereum and other chains' traces have no proof
// (unverifiable), and trace_filter's matching is the server's (alloy's TraceFilterMatcher,
// trace-verify's filtered).
import { afterEach, describe, expect, it } from 'vitest';
import { traceFiltered } from '../src/client.js';
import { WillowUnverifiableError } from '../src/errors.js';
import { harness, type Harness } from './support/setup.js';
import { World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const A = '0x' + 'aa'.repeat(20);
const B = '0x' + 'bb'.repeat(20);
const C = '0x' + 'cc'.repeat(20);
const call = (from: string, to: string) => ({ type: 'call', action: { from, to, callType: 'call' }, result: {} });
const create = (from: string, address: string | null) => ({ type: 'create', action: { from }, result: address ? { address } : null });
const traces = [call(A, B), call(B, C), create(A, C), create(B, null), { type: 'suicide', action: { address: C, refundAddress: A } }, { type: 'reward', action: { author: B } }];

describe('traces with no proof', () => {
  for (const [method, params] of [
    ['debug_traceCall', [{ to: A, data: '0x' }, 'latest', { tracer: 'callTracer' }]],
    ['trace_call', [{ to: A, data: '0x' }, ['trace'], 'latest']],
  ] as const) {
    it(`${method} on Unichain is unverifiable: thrown, or (flag) the server's response marked`, async () => {
      const w = new World();
      w.standard.set(`130|${method}|*`, { result: { proven: false } });
      h = await harness({ chainId: 130, world: w });
      await expect(h.client.request({ method, params: params as unknown as unknown[] })).rejects.toBeInstanceOf(WillowUnverifiableError);
      await h.close();
      h = await harness({ chainId: 130, world: w, options: { unverifiable: 'flag' } });
      expect(await h.client.request({ method, params: params as unknown as unknown[] })).toEqual({ proven: false });
      expect(h.events.at(-1)?.status).toBe('unverifiable');
      expect(h.world.log.flatMap((r) => r.items).some((i) => i.method === 'willow_getTraceProof')).toBe(false);
    });
  }

  it("another chain's traces are unverifiable, never sent as a proof", async () => {
    const w = new World();
    w.standard.set('8453|trace_transaction|*', { result: [] });
    h = await harness({ chainId: 8453, world: w });
    await expect(h.client.request({ method: 'trace_transaction', params: ['0x' + '11'.repeat(32)] })).rejects.toBeInstanceOf(WillowUnverifiableError);
    expect(h.world.log.flatMap((r) => r.items).some((i) => i.method === 'willow_getTraceProof')).toBe(false);
  });
});

describe('trace_filter matching', () => {
  it('no addresses: every trace', () => {
    expect(traceFiltered({}, [traces])).toEqual(traces);
  });
  it('union: from or to, and a side not given matches through the other', () => {
    expect(traceFiltered({ fromAddress: [A] }, [traces])).toEqual([traces[0], traces[2]]);
    expect(traceFiltered({ toAddress: [C] }, [traces])).toEqual([traces[1], traces[2]]);
    expect(traceFiltered({ fromAddress: [A], toAddress: [C] }, [traces])).toEqual([traces[0], traces[1], traces[2]]);
    // A creation with no address matches a to-filter only when there is none; a reward's author is its "to".
    expect(traceFiltered({ toAddress: [B] }, [traces])).toEqual([traces[0], traces[5]]);
    expect(traceFiltered({ toAddress: [A] }, [traces])).toEqual([traces[4]]);
  });
  it('intersection: both', () => {
    expect(traceFiltered({ fromAddress: [A], toAddress: [C], mode: 'intersection' }, [traces])).toEqual([traces[2]]);
    expect(traceFiltered({ fromAddress: [B], mode: 'intersection' }, [traces])).toEqual([traces[1], traces[3]]);
  });
  it('after and count, as numbers or quantities, over the blocks in order', () => {
    expect(traceFiltered({ after: 1, count: 2 }, [traces.slice(0, 2), traces.slice(2)])).toEqual(traces.slice(1, 3));
    expect(traceFiltered({ after: '0x2', count: '0x1' }, [traces])).toEqual([traces[2]]);
    expect(traceFiltered({ after: 6 }, [traces])).toEqual([]);
  });
  it('addresses in any case', () => {
    expect(traceFiltered({ fromAddress: [A.toUpperCase().replace('0X', '0x')] }, [traces])).toEqual([traces[0], traces[2]]);
  });
});
