// Transactions, receipts, block receipts and eth_getLogs by receipts (design 1.3.10, 1.3.12) with
// the REAL items module (a verify page build, WILLOW_VERIFY_DIST) on the shipped proofs
// in web/verify/test/items; anchors are the mock's (no recorded anchors exist for these blocks).
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { toQuantity } from '../src/hex.js';
import { flip } from './support/lying-proxy.js';
import { mockModules, realModule, VERIFY_DIST } from './support/mock-verifiers.js';
import { harness, type Harness } from './support/setup.js';
import { ITEMS, json, World } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

const METHOD: Record<string, string> = { transaction: 'eth_getTransactionByHash', receipt: 'eth_getTransactionReceipt', blockReceipts: 'eth_getBlockReceipts' };

/** Every proven field equals the server's own standard response; what is left out is listed as unproven. */
function provenSubset(proven: unknown, standard: unknown, path = ''): string[] {
  if (proven === null || typeof proven !== 'object') return JSON.stringify(proven)?.toLowerCase() === JSON.stringify(standard)?.toLowerCase() ? [] : [path];
  const bad: string[] = [];
  for (const [k, v] of Object.entries(proven as Record<string, unknown>)) bad.push(...provenSubset(v, (standard as Record<string, unknown>)?.[k], `${path}.${k}`));
  return bad;
}

async function realItems() {
  const { ItemVerifier } = await realModule('items');
  return { ItemVerifier };
}

describe.skipIf(!VERIFY_DIST)('the real items module', () => {
  for (const file of readdirSync(ITEMS).filter((f) => f.endsWith('.json')).sort()) {
    const c = json(join(ITEMS, file));
    // A receipt named by <block>:<index> has no standard method to come from.
    it.skipIf(c.kind === 'receipt' && c.target.includes(':'))(file, async () => {
      const w = new World();
      w.addItem(c);
      h = await harness({ chainId: c.chainId, world: w, verifiers: { ...mockModules(w), items: realItems } });
      const out = await h.client.request({ method: METHOD[c.kind], params: [c.target] });
      expect(provenSubset(out, c.answers[0].result)).toEqual([]);
      const e = h.events.at(-1)!;
      expect(e.status).toBe('verified');
      expect(e.block!.hash).toBe(c.block.hash.toLowerCase());
      expect(e.level).toBe('final');
    });
  }

  it('eth_getLogs over one block goes by its block-receipts proof, filtered by address', async () => {
    const c = json(join(ITEMS, 'uni-block.json'));
    const w = new World();
    w.addItem(c);
    const n = parseInt(c.block.number, 16);
    w.head(130, { number: n, hash: c.block.hash.toLowerCase() });
    h = await harness({ chainId: 130, world: w, verifiers: { ...mockModules(w), items: realItems } });
    const all = c.answers[0].result.flatMap((r: any) => r.logs);
    const address = all[0].address;
    const logs = (await h.client.request({ method: 'eth_getLogs', params: [{ fromBlock: toQuantity(n), toBlock: 'latest', address }] })) as any[];
    expect(logs).toEqual(all.filter((l: any) => l.address.toLowerCase() === address.toLowerCase()));
    expect(logs.length).toBeGreaterThan(0);
    const byHash = (await h.client.request({ method: 'eth_getLogs', params: [{ blockHash: c.block.hash }] })) as any[];
    expect(byHash).toEqual(all);
  });

  it('a tampered receipt proof fails verification', async () => {
    const c = json(join(ITEMS, 'eth-receipt.json'));
    const w = new World();
    w.addItem(c);
    const e = [...w.proofs.values()][0];
    const it0 = e.result.receiptProofs[e.result.receiptProofs.length - 1];
    it0.proof[0] = flip(it0.proof[0], 40);
    h = await harness({ chainId: 1, world: w, verifiers: { ...mockModules(w), items: realItems } });
    // From W5's build every failure carries W1's fault (I6), so the kind is what failed.
    await expect(h.client.request({ method: 'eth_getTransactionReceipt', params: [c.target] })).rejects.toMatchObject({ code: -32098, data: { kind: 'receipts' } });
  });

  it('an unknown transaction hash is null, as the standard method returns', async () => {
    const w = new World();
    w.head(1, { number: 1, hash: `0x${'aa'.repeat(32)}` });
    h = await harness({ chainId: 1, world: w, verifiers: { ...mockModules(w), items: realItems } });
    expect(await h.client.request({ method: 'eth_getTransactionReceipt', params: [`0x${'12'.repeat(32)}`] })).toBeNull();
    expect(await h.client.request({ method: 'eth_getTransactionByHash', params: [`0x${'12'.repeat(32)}`] })).toBeNull();
    expect(h.events.map((x) => x.status)).toEqual(['absent', 'absent']);
  });
});
