// Local filters over verified blocks and logs, and eth_getLogs over more than 8 blocks
// by willow_getLogsProof: its proof job polled, its proofs fetched, the last block anchored.
import { createPublicClient, parseAbiItem } from 'viem';
import { base } from 'viem/chains';
import { afterEach, describe, expect, it } from 'vitest';
import { BLOCK_FILTER_MAX, type WillowOptions } from '../src/client.js';
import { hexToBytes, keccakHex, toQuantity } from '../src/hex.js';
import { willowTransport } from '../src/viem.js';
import { harness, type Harness } from './support/setup.js';
import { chain, register } from './support/synth.js';
import { World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const EMITTER = `0x${'ee'.repeat(20)}`;
const TOPIC = keccakHex(new TextEncoder().encode('Ping(uint256)'));

async function synthetic(to = 112, options: Partial<WillowOptions> = {}) {
  const w = new World();
  const blocks = chain(100, to);
  register(w, 8453, blocks, (b) => (b.number % 2 ? [{ address: EMITTER, topics: [TOPIC, toQuantity(b.number).padEnd(66, '0')] }] : [{ address: `0x${'aa'.repeat(20)}` }]));
  const at = (n: number) => blocks[n - 100];
  // Only the head is anchored on its own; every other block is reached by parent hashes.
  const t0 = Date.now();
  // Synthetic blocks: only the mock items verifier takes them.
  const x = await harness({ chainId: 8453, world: w, mocks: { items: 'mock' }, options: { now: () => w.nowSec * 1000 + (Date.now() - t0), ...options } });
  const headTo = (n: number) => w.head(8453, { number: n, hash: at(n).hash, time: w.nowSec });
  return { x, w, at, headTo };
}

const anchorsAsked = (w: World) => w.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getBlockAnchor').map((i) => (i.params as unknown[])[0]);

describe('local filters', () => {
  it('a block filter returns each new block hash, anchored through parent hashes from the head', async () => {
    const s = await synthetic();
    h = s.x;
    s.headTo(100);
    const id = await h.client.request({ method: 'eth_newBlockFilter' });
    s.headTo(104);
    await new Promise((r) => setTimeout(r, 2100));
    const hashes = await h.client.request({ method: 'eth_getFilterChanges', params: [id] });
    expect(hashes).toEqual([101, 102, 103, 104].map((n) => s.at(n).hash));
    // Only "latest" was anchored: blocks 101 to 103 by the parent hashes of the blocks above them.
    expect(anchorsAsked(s.w).every((p) => p === 'latest')).toBe(true);
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual([]);
  });

  it('a log filter returns the matching logs of the new blocks, then nothing until the next', async () => {
    const s = await synthetic();
    h = s.x;
    s.headTo(100);
    const id = await h.client.request({ method: 'eth_newFilter', params: [{ address: EMITTER, topics: [TOPIC] }] });
    s.headTo(105);
    await new Promise((r) => setTimeout(r, 2100));
    const logs = (await h.client.request({ method: 'eth_getFilterChanges', params: [id] })) as any[];
    expect(logs.map((l) => l.blockNumber)).toEqual([toQuantity(101), toQuantity(103), toQuantity(105)]);
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual([]);
    const all = (await h.client.request({ method: 'eth_getFilterLogs', params: [id] })) as any[];
    expect(all.length).toBe(1);
    expect(await h.client.request({ method: 'eth_uninstallFilter', params: [id] })).toBe(true);
    expect(await h.client.request({ method: 'eth_uninstallFilter', params: [id] })).toBe(false);
    await expect(h.client.request({ method: 'eth_getFilterChanges', params: [id] })).rejects.toMatchObject({ message: 'filter not found' });
  });

  it("viem's watchEvent polls the local filter", async () => {
    const s = await synthetic();
    h = s.x;
    s.headTo(100);
    const client = createPublicClient({ chain: base, transport: willowTransport(h.client), pollingInterval: 30 });
    const seen: bigint[] = [];
    await new Promise<void>((resolve, reject) => {
      const stop = client.watchEvent({
        address: EMITTER as `0x${string}`,
        event: parseAbiItem('event Ping(uint256 indexed n)'),
        onLogs: (logs) => {
          seen.push(...logs.map((l) => l.blockNumber!));
          if (seen.length >= 2) (stop(), resolve());
        },
        onError: (e) => (stop(), reject(e)),
      });
      setTimeout(() => s.headTo(103), 200);
    });
    expect(seen).toEqual([101n, 103n]);
    expect(h.world.log.flatMap((r) => r.items).some((i) => i.method === 'eth_newFilter')).toBe(false);
  });
});

describe('eth_getLogs over more than 8 blocks', () => {
  it('waits for the proof job, fetches the proofs alone, verifies, and anchors the last block', async () => {
    const s = await synthetic(130);
    h = s.x;
    s.headTo(130);
    const filter = { fromBlock: toQuantity(110), toBlock: toQuantity(130), address: EMITTER };
    const result = { fromBlock: filter.fromBlock, toBlock: filter.toBlock, parts: [{ type: 'segment', proof: 'l1/110-130' }] };
    const logs = [{ address: EMITTER, topics: [TOPIC], data: '0x', blockNumber: toQuantity(111), transactionIndex: '0x0', logIndex: '0x0' }];
    const proof = `0x${'ab'.repeat(64)}`;
    (s.w as any).logs = new Map([[`8453|${JSON.stringify(filter)}`, { proofs: { 'l1/110-130': proof }, result, verified: { report: { fromBlock: filter.fromBlock, toBlock: filter.toBlock, logs: 1, lastProvenBlock: { number: toQuantity(130), hash: s.at(130).hash } }, logs } }]]);
    let asked = 0;
    let polled = 0;
    s.w.intercept = (_c, m) => {
      if (m === 'willow_getLogsProof') return asked++ === 0 ? { error: { code: -32000, message: 'proof queued', data: { status: 'queued', jobId: 'j-1-0000abcd', jobIds: ['j-1-0000abcd'], from: 110, to: 130, retryAfterSeconds: 2 } } } : { result };
      if (m === 'willow_getProofJob') return { result: { id: 'j-1-0000abcd', status: polled++ ? 'done' : 'running', totalBlocks: 21, provedBlocks: 21 } };
      if (m === 'willow_getProof') return { result: { proof } };
      return undefined;
    };
    expect(await h.client.request({ method: 'eth_getLogs', params: [{ ...filter, toBlock: 'latest' }] })).toEqual(logs);
    expect(asked).toBe(2);
    expect(polled).toBe(2);
    const proofRequest = s.w.log.find((r) => r.items.some((i) => i.method === 'willow_getProof'))!;
    expect(proofRequest.items).toHaveLength(1);
    expect(h.events.at(-1)).toMatchObject({ status: 'verified', block: { number: 130, hash: s.at(130).hash }, unproven: ['[].blockHash', '[].transactionHash'] });
  });

  it('a proof whose bytes are not the ones it names fails verification', async () => {
    const s = await synthetic(130);
    h = s.x;
    s.headTo(130);
    const filter = { fromBlock: toQuantity(110), toBlock: toQuantity(130) };
    const result = { parts: [{ proof: 'l1/110-130' }] };
    (s.w as any).logs = new Map([[`8453|${JSON.stringify(filter)}`, { proofs: { 'l1/110-130': `0x${'ab'.repeat(64)}` }, result, verified: { report: {}, logs: [] } }]]);
    s.w.intercept = (_c, m) => (m === 'willow_getLogsProof' ? { result } : m === 'willow_getProof' ? { result: { proof: `0x${'cd'.repeat(64)}` } } : undefined);
    await expect(h.client.request({ method: 'eth_getLogs', params: [filter] })).rejects.toMatchObject({ code: -32098, data: { kind: 'logs' } });
    void hexToBytes;
  });
});

describe('local filters: what goes to the server', () => {
  const sent = (w: World) => w.log.flatMap((r) => r.items);
  const settle = () => new Promise((r) => setTimeout(r, 2100));

  it("under flag, a log filter's poll that can't be verified asks eth_getLogs over the poll's range, never the filter method with a local id", async () => {
    const s = await synthetic(112, { unverifiable: 'flag' });
    h = s.x;
    s.headTo(100);
    const id = await h.client.request({ method: 'eth_newFilter', params: [{ address: EMITTER, topics: [TOPIC] }] });
    s.headTo(104);
    await settle();
    const served = [{ address: EMITTER, topics: [TOPIC], data: '0x', blockNumber: toQuantity(103), blockHash: s.at(103).hash, transactionIndex: '0x0', logIndex: '0x0' }];
    s.w.intercept = (_c, m) =>
      m === 'willow_getBlockReceiptsProof' ? { error: { code: -32601, message: 'the method willow_getBlockReceiptsProof is not on this plan' } } : m === 'eth_getLogs' ? { result: served } : undefined;
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual(served);
    expect(h.events.at(-1)).toMatchObject({ method: 'eth_getFilterChanges', status: 'unverifiable', kind: 'not-on-plan' });
    const range = (from: number, to: number) => [{ fromBlock: toQuantity(from), toBlock: toQuantity(to), address: EMITTER, topics: [TOPIC] }];
    expect(sent(s.w).filter((i) => i.method === 'eth_getLogs').map((i) => i.params)).toEqual([range(101, 104)]);
    // The filter moves on: the next poll's range starts above the last one.
    s.headTo(106);
    await settle();
    await h.client.request({ method: 'eth_getFilterChanges', params: [id] });
    expect(sent(s.w).filter((i) => i.method === 'eth_getLogs').map((i) => i.params)).toEqual([range(101, 104), range(105, 106)]);
    expect(sent(s.w).some((i) => /^eth_(newFilter|newBlockFilter|getFilterChanges|getFilterLogs|uninstallFilter)$/.test(i.method))).toBe(false);
  });

  it("under flag, eth_getFilterLogs that can't be verified asks eth_getLogs with the filter as it was installed", async () => {
    const s = await synthetic(112, { unverifiable: 'flag' });
    h = s.x;
    s.headTo(112);
    const filter = { fromBlock: toQuantity(101), toBlock: toQuantity(112), address: EMITTER };
    const id = await h.client.request({ method: 'eth_newFilter', params: [filter] });
    // 12 blocks: a log proof, which this plan doesn't serve.
    s.w.intercept = (_c, m) => (m === 'willow_getLogsProof' ? { error: { code: -32601, message: 'log proofs are not on this plan' } } : m === 'eth_getLogs' ? { result: [] } : undefined);
    expect(await h.client.request({ method: 'eth_getFilterLogs', params: [id] })).toEqual([]);
    expect(h.events.at(-1)).toMatchObject({ method: 'eth_getFilterLogs', status: 'unverifiable', kind: 'logs-range' });
    expect(sent(s.w).filter((i) => i.method === 'eth_getLogs').map((i) => i.params)).toEqual([[filter]]);
    expect(sent(s.w).some((i) => i.method === 'eth_getFilterLogs')).toBe(false);
  });

  it("under flag, a block filter's poll that can't be verified is unverifiable: no server request stands for it", async () => {
    const s = await synthetic(112, { unverifiable: 'flag' });
    h = s.x;
    s.headTo(100);
    const id = await h.client.request({ method: 'eth_newBlockFilter' });
    s.headTo(103);
    await settle();
    s.w.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { error: { code: -32601, message: 'block proofs are not on this plan' } } : undefined);
    await expect(h.client.request({ method: 'eth_getFilterChanges', params: [id] })).rejects.toMatchObject({ code: -32099, data: { kind: 'not-on-plan' } });
    expect(sent(s.w).some((i) => i.method === 'eth_getFilterChanges')).toBe(false);
  });

  it('a block filter returns at most BLOCK_FILTER_MAX blocks a poll, oldest first; the next poll goes on from there', async () => {
    const top = 100 + BLOCK_FILTER_MAX + 300;
    const s = await synthetic(top);
    h = s.x;
    s.headTo(100);
    const id = await h.client.request({ method: 'eth_newBlockFilter' });
    s.headTo(top);
    // The newest block of the first window is not the head: it is anchored on its own.
    const edge = 100 + BLOCK_FILTER_MAX;
    s.w.anchor({ chainId: 8453, number: edge, hash: s.at(edge).hash, level: 'signed', path: 'sequencer', time: s.w.nowSec });
    await settle();
    const numbers = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => from + i);
    const asked = () => sent(s.w).filter((i) => i.method === 'willow_getBlockProof').map((i) => parseInt(String((i.params as unknown[])[0]), 16));
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual(numbers(101, edge).map((n) => s.at(n).hash));
    expect(Math.max(...asked())).toBe(edge);
    expect(asked()).toHaveLength(BLOCK_FILTER_MAX);
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual(numbers(edge + 1, top).map((n) => s.at(n).hash));
    expect(await h.client.request({ method: 'eth_getFilterChanges', params: [id] })).toEqual([]);
  }, 120_000);
});
