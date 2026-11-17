// A short synthetic chain for flows the recorded fixtures can't cover (several consecutive
// blocks): header RLPs chained by parent hash, which the mock verifiers verify for real.
import { bytesToHex, hexToBytes, keccakHex, toQuantity } from '../../src/hex.js';
import { encode } from '../../src/rlp.js';
import type { World } from './world.js';

const num = (n: number): Uint8Array => (n === 0 ? new Uint8Array() : hexToBytes(`0x${n.toString(16).padStart(Math.ceil(n.toString(16).length / 2) * 2, '0')}`));

export interface SynthBlock {
  number: number;
  hash: string;
  header: string;
  parentHash: string;
}

export function chain(from: number, to: number): SynthBlock[] {
  const out: SynthBlock[] = [];
  let parent = `0x${'00'.repeat(32)}`;
  for (let n = from; n <= to; n++) {
    const fields = Array.from({ length: 15 }, () => new Uint8Array());
    fields[0] = hexToBytes(parent);
    fields[8] = num(n);
    fields[11] = num(1_700_000_000 + n);
    const header = bytesToHex(encode(fields));
    const hash = keccakHex(hexToBytes(header));
    out.push({ number: n, hash, header, parentHash: parent });
    parent = hash;
  }
  return out;
}

/** Each block as a block proof and a block-receipts proof (its receipts' logs given), by hash and number. */
export function register(w: World, chainId: number, blocks: SynthBlock[], logsOf: (b: SynthBlock) => object[] = () => []): void {
  for (const b of blocks) {
    const base = { chainId: toQuantity(chainId), blockNumber: toQuantity(b.number), blockHash: b.hash, header: b.header };
    const report = { chainId, block: { number: toQuantity(b.number), hash: b.hash, timestamp: 1_700_000_000 + b.number, parentHash: b.parentHash }, unproven: [] };
    const block = { number: toQuantity(b.number), hash: b.hash, parentHash: b.parentHash, transactions: [] };
    const be = { chainId, kind: 'block', method: 'willow_getBlockProof', result: { ...base, transactions: [] }, proven: block, report: { ...report, method: 'willow_getBlockProof' }, full: { proven: block, unproven: [] } };
    w.addProof([b.hash], be);
    w.addProof([toQuantity(b.number)], be);
    const logs = logsOf(b).map((l, i) => ({ blockNumber: toQuantity(b.number), blockHash: b.hash, transactionIndex: '0x0', logIndex: toQuantity(i), removed: false, transactionHash: keccakHex(hexToBytes(b.hash)), data: '0x', topics: [], ...l }));
    const re = { chainId, kind: 'blockReceipts', method: 'willow_getBlockReceiptsProof', result: { ...base, receipts: [`0x${b.number.toString(16).padStart(2, '0')}`] }, proven: [{ logs }], report: { ...report, method: 'willow_getBlockReceiptsProof' } };
    w.addProof([b.hash], re);
    w.addProof([toQuantity(b.number)], re);
    w.hashes.set(`${chainId}:${b.number}`, b.hash);
  }
}
