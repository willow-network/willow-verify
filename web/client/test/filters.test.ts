// Local filters (design 3.2) over verified blocks and logs, and eth_getLogs over more than 8 blocks
// by willow_getLogsProof (1.3.12): its proof job polled, its proofs fetched, the last block anchored.
import { createPublicClient, parseAbiItem } from 'viem';
import { base } from 'viem/chains';
import { afterEach, describe, expect, it } from 'vitest';
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

async function synthetic(to = 112) {
  const w = new World();
  const blocks = chain(100, to);
  register(w, 8453, blocks, (b) => (b.number % 2 ? [{ address: EMITTER, topics: [TOPIC, toQuantity(b.number).padEnd(66, '0')] }] : [{ address: `0x${'aa'.repeat(20)}` }]));
  const at = (n: number) => blocks[n - 100];
  // Only the head is anchored on its own; every other block is reached by parent hashes.
  const t0 = Date.now();
  // Synthetic blocks: only the mock items verifier takes them.
  const x = await harness({ chainId: 8453, world: w, mocks: { items: 'mock' }, options: { now: () => w.nowSec * 1000 + (Date.now() - t0) } });
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
