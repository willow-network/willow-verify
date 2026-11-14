// State, calls and estimates (design 1.3.3 to 1.3.5) on W2's fixtures.
import { afterEach, describe, expect, it } from 'vitest';
import { harness, methodsSent, type Harness } from './support/setup.js';
import { fixture, fixtureNames } from './support/world.js';

let h: Harness | null = null;
afterEach(async () => {
  await h?.close();
  h = null;
});

/** W2's Ethereum fixtures are all at block 26391733. */
const W2_BLOCK = { number: 0x192b4b5, hash: '0x8a27150df15c313310e346ad53e1e04825a7004e0e01096b5cf095e6c7d89552' };

async function ethereum(): Promise<Harness> {
  const x = await harness({ chainId: 1 });
  x.world.head(1, W2_BLOCK);
  return x;
}

const stateProofs = (x: Harness) => x.world.log.flatMap((r) => r.items).filter((i) => i.method === 'willow_getStateProof');

describe('state', () => {
  it('an EOA: balance, nonce and code at latest go out as one state proof', async () => {
    h = await ethereum();
    const f = fixture('state/eoa.json');
    const a = f.request.params[0];
    const [bal, nonce, code] = await Promise.all([
      h.client.request({ method: 'eth_getBalance', params: [a, 'latest'] }),
      h.client.request({ method: 'eth_getTransactionCount', params: [a, 'latest'] }),
      h.client.request({ method: 'eth_getCode', params: [a, 'latest'] }),
    ]);
    expect([bal, nonce, code]).toEqual([f.expect.eth_getBalance, f.expect.eth_getTransactionCount, '0x']);
    expect(stateProofs(h)).toHaveLength(1);
    expect(stateProofs(h)[0].params).toEqual([a, [], { blockHash: W2_BLOCK.hash }, { code: true }]);
    expect(h.events.map((e) => [e.status, e.level, e.block?.number])).toEqual(Array(3).fill(['verified', 'signed', W2_BLOCK.number]));
  });

  it('an absent account is proven absent: zero balance, nonce, slot and no code', async () => {
    h = await ethereum();
    const f = fixture('state/absent.json');
    const a = f.request.params[0];
    expect(await h.client.request({ method: 'eth_getBalance', params: [a, 'latest'] })).toBe('0x0');
    expect(await h.client.request({ method: 'eth_getTransactionCount', params: [a, 'latest'] })).toBe('0x0');
    expect(await h.client.request({ method: 'eth_getStorageAt', params: [a, '0x0', 'latest'] })).toBe(`0x${'00'.repeat(32)}`);
    expect(await h.client.request({ method: 'eth_getCode', params: [a, 'latest'] })).toBe('0x');
  });

  it('USDC: each slot, the code, eth_getProof, and the code from its cache the second time', async () => {
    h = await ethereum();
    const f = fixture('state/usdc-slots-code.json');
    const [a, keys] = f.request.params;
    const slots = await Promise.all(keys.map((k: string) => h!.client.request({ method: 'eth_getStorageAt', params: [a, k, 'latest'] })));
    expect(slots).toEqual(f.expect.eth_getStorageAt);
    // Slots of one account at one block, asked at once, are one proof.
    expect(stateProofs(h)).toHaveLength(1);
    expect(await h.client.request({ method: 'eth_getCode', params: [a, 'latest'] })).toBe(f.expect.eth_getCode);
    expect(await h.client.request({ method: 'eth_getProof', params: [a, keys, 'latest'] })).toEqual(f.expect.eth_getProof);
    const before = stateProofs(h).length;
    expect(await h.client.request({ method: 'eth_getCode', params: [a, 'latest'] })).toBe(f.expect.eth_getCode);
    const again = stateProofs(h).slice(before);
    expect(again).toHaveLength(1);
    expect((again[0].params as any[])[3]).toEqual({ code: false });
  });

  it('Base: a slot at a block named by hash', async () => {
    h = await harness({ chainId: 8453 });
    const f = fixture('state/base-usdc.json');
    const [a, [k], b] = f.request.params;
    h.world.head(8453, { number: parseInt(f.result.blockNumber, 16), hash: b.blockHash });
    expect(await h.client.request({ method: 'eth_getStorageAt', params: [a, k, b] })).toBe(f.expect.eth_getStorageAt[0]);
    expect(await h.client.request({ method: 'eth_getBalance', params: [a, b] })).toBe(f.expect.eth_getBalance);
  });

  it('the pending nonce is the proven one until this client sends', async () => {
    h = await ethereum();
    const f = fixture('state/eoa.json');
    expect(await h.client.request({ method: 'eth_getTransactionCount', params: [f.request.params[0], 'pending'] })).toBe(f.expect.eth_getTransactionCount);
  });

  it('a bad address or slot is invalid params, before anything is sent', async () => {
    h = await ethereum();
    await expect(h.client.request({ method: 'eth_getBalance', params: ['0x12', 'latest'] })).rejects.toMatchObject({ code: -32602 });
    await expect(h.client.request({ method: 'eth_getStorageAt', params: [`0x${'11'.repeat(20)}`, 'zz', 'latest'] })).rejects.toMatchObject({ code: -32602 });
  });
});

describe('calls and estimates on Ethereum', () => {
  for (const file of fixtureNames('call')) {
    it(file, async () => {
      h = await ethereum();
      const f = fixture(`call/${file}`);
      const method = f.request.method === 'willow_getCallProof' ? 'eth_call' : 'eth_estimateGas';
      const params = method === 'eth_call' ? [f.request.params[0], 'latest'] : [f.request.params[0]];
      const run = h.client.request({ method, params });
      if ('result' in f.expect) await expect(run).resolves.toBe(f.expect.result);
      else await expect(run).rejects.toMatchObject({ code: f.expect.error.code, message: f.expect.error.message, ...(f.expect.error.data ? { data: f.expect.error.data } : {}) });
      // The client's own outcome equals the public node's (geth v1.17.9), as W2 recorded it.
      const ref = f.reference[method];
      if ('result' in ref) expect(f.expect.result).toBe(ref.result);
      else expect(f.expect.error).toEqual(ref.error);
      const e = h.events.at(-1)!;
      expect(e.block).toEqual(W2_BLOCK);
      expect(e.status).toBe('verified');
    });
  }

  it('the same call at the same head is proven once', async () => {
    h = await ethereum();
    const f = fixture('call/call-usdc-balance.json');
    const p = [f.request.params[0], 'latest'];
    await h.client.request({ method: 'eth_call', params: p });
    await h.client.request({ method: 'eth_call', params: p });
    expect(methodsSent(h.world).filter((m) => m === 'willow_getCallProof')).toHaveLength(1);
  });
});
