// Fees (design 1.3.6 to 1.3.8): eth_gasPrice, eth_maxPriorityFeePerGas, eth_blobBaseFee and
// eth_feeHistory, on W1's fixtures from all seven chains.
import { afterEach, describe, expect, it } from 'vitest';
import { bytesToHex, hexToBytes, keccakHex, toQuantity } from '../src/hex.js';
import { encode } from '../src/rlp.js';
import { blockOf, chainOf, harness, HEAD_FIXTURE, headAt, methodsSent, type Harness } from './support/setup.js';
import { fixture, fixtureNames } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

describe('fee methods on every chain', () => {
  for (const file of fixtureNames('fees').filter((f) => f.endsWith('.fee.json'))) {
    it(file, async () => {
      const chainId = chainOf(file);
      const f = fixture(`fees/${file}`);
      h = await harness({ chainId });
      headAt(h.world, chainId, blockOf(HEAD_FIXTURE[chainId]));
      const [gas, tip] = await Promise.all([
        h.client.request({ method: 'eth_gasPrice' }),
        h.client.request({ method: 'eth_maxPriorityFeePerGas' }),
      ]);
      expect(gas).toBe(f.verified.eth_gasPrice);
      expect(tip).toBe(f.verified.eth_maxPriorityFeePerGas);
      // Both come from one fee proof at the head.
      expect(methodsSent(h.world).filter((m) => m === 'willow_getFeeProof')).toHaveLength(1);
      const blob = h.client.request({ method: 'eth_blobBaseFee' });
      if (f.verified.eth_blobBaseFee === null) await expect(blob).rejects.toMatchObject({ code: -32601 });
      else await expect(blob).resolves.toBe(f.verified.eth_blobBaseFee);
      // Where the public node runs the same formula (the OP stack's tip), the values agree.
      if ([8453, 10, 130, 57073].includes(chainId)) expect(tip).toBe(f.node.responses.eth_maxPriorityFeePerGas);
    });
  }
});

describe('eth_feeHistory', () => {
  for (const file of fixtureNames('fees').filter((f) => f.endsWith('.history.json'))) {
    it(file, async () => {
      const chainId = chainOf(file);
      const f = fixture(`fees/${file}`);
      h = await harness({ chainId });
      headAt(h.world, chainId, blockOf(HEAD_FIXTURE[chainId]));
      const [count, newest, pct] = f.request.params;
      expect(await h.client.request({ method: 'eth_feeHistory', params: [count, newest, pct] })).toEqual(f.verified);
      // "latest" is the anchored head, the same block.
      expect(await h.client.request({ method: 'eth_feeHistory', params: [count, 'latest', pct] })).toEqual(f.verified);
    });
  }

  it('with rewards, more than 16 blocks are split into proofs of 16 and merged', async () => {
    // Synthetic headers: only the mock items verifier takes them.
    h = await harness({ chainId: 8453, mocks: { items: 'mock' } });
    const w = h.world;
    // Headers 100..119 chained by parent hash; the mock verifier verifies the links, numbers and hashes.
    const headers: string[] = [];
    let parent = `0x${'00'.repeat(32)}`;
    for (let n = 100; n <= 119; n++) {
      const fields = Array.from({ length: 15 }, () => new Uint8Array());
      fields[0] = hexToBytes(parent);
      fields[8] = n ? hexToBytes(toQuantity(n).length % 2 ? `0x0${toQuantity(n).slice(2)}` : toQuantity(n)) : new Uint8Array();
      fields[11] = Uint8Array.of(1);
      const rlp = bytesToHex(encode(fields));
      headers.push(rlp);
      parent = keccakHex(hexToBytes(rlp));
    }
    const hashOf = (n: number) => keccakHex(hexToBytes(headers[n - 100]));
    const part = (from: number, to: number) => {
      const r = { chainId: '0x2105', formula: 'willow-fee-v1', blockNumber: toQuantity(to), blockHash: hashOf(to), headers: headers.slice(from - 100, to - 99) };
      const k = to - from + 1;
      const proven = {
        oldestBlock: toQuantity(from),
        reward: Array.from({ length: k }, (_, i) => [toQuantity(from + i)]),
        baseFeePerGas: Array.from({ length: k + 1 }, (_, i) => toQuantity(1000 + from + i)),
        gasUsedRatio: Array.from({ length: k }, (_, i) => (from + i) / 1000),
      };
      w.addProof([toQuantity(k), toQuantity(to), [50]], { chainId: 8453, kind: 'feeHistory', method: 'willow_getFeeHistoryProof', result: r, proven, report: { method: 'willow_getFeeHistoryProof', chainId: 8453, block: { number: toQuantity(to), hash: hashOf(to), timestamp: 1 }, unproven: [], formula: 'willow-fee-v1' } });
      w.anchor({ chainId: 8453, number: to, hash: hashOf(to), level: 'signed', path: 'sequencer', time: w.nowSec - 5 });
    };
    part(104, 119);
    part(100, 103);
    headAt(w, 8453, { number: 119, hash: hashOf(119) });
    const out: any = await h.client.request({ method: 'eth_feeHistory', params: ['0x14', 'latest', [50]] });
    expect(out.oldestBlock).toBe(toQuantity(100));
    expect(out.reward).toEqual(Array.from({ length: 20 }, (_, i) => [toQuantity(100 + i)]));
    expect(out.baseFeePerGas).toEqual(Array.from({ length: 21 }, (_, i) => toQuantity(1100 + i)));
    expect(out.gasUsedRatio).toHaveLength(20);
    const asked = h.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getFeeHistoryProof').map((i) => i.params);
    expect(asked).toEqual([
      ['0x10', '0x77', [50]],
      ['0x4', '0x67', [50]],
    ]);
  });
});
