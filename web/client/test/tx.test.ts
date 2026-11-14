// eth_sendRawTransaction (design 1.3.9): the hash is computed here from the bytes, the sender and
// nonce recorded for the pending-nonce rule (1.3.3). Verified against transactions viem signs.
import { keccak256, type Hex } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { afterEach, describe, expect, it } from 'vitest';
import { bytesToHex, hexToBytes, toQuantity } from '../src/hex.js';
import { decode, encode, type Rlp } from '../src/rlp.js';
import { decodeSent } from '../src/tx.js';
import { harness, type Harness } from './support/setup.js';
import { fixture } from './support/world.js';

const account = privateKeyToAccount('0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d');
const to = '0x4b1d5e7c9a3f2e6d8c0b1a2f3e4d5c6b7a8f9e01';
const base = { to, value: 1n, gas: 21000n, nonce: 7 } as const;

const signed: Record<string, () => Promise<Hex>> = {
  'legacy, no replay protection': () => account.signTransaction({ ...base, type: 'legacy', gasPrice: 2n }),
  'legacy, EIP-155': () => account.signTransaction({ ...base, type: 'legacy', gasPrice: 2n, chainId: 1 }),
  'EIP-2930': () => account.signTransaction({ ...base, type: 'eip2930', gasPrice: 2n, chainId: 1, accessList: [{ address: to, storageKeys: [`0x${'01'.padStart(64, '0')}`] }] }),
  'EIP-1559': () => account.signTransaction({ ...base, type: 'eip1559', maxFeePerGas: 3n, maxPriorityFeePerGas: 1n, chainId: 1, data: '0x1234' }),
  'EIP-4844': () =>
    account.signTransaction({ ...base, type: 'eip4844', maxFeePerGas: 3n, maxPriorityFeePerGas: 1n, maxFeePerBlobGas: 5n, chainId: 1, blobVersionedHashes: [`0x01${'ab'.repeat(31)}`] }),
  'EIP-7702': async () => {
    const authorization = await account.signAuthorization({ contractAddress: to, chainId: 1, nonce: 8 });
    return account.signTransaction({ ...base, type: 'eip7702', maxFeePerGas: 3n, maxPriorityFeePerGas: 1n, chainId: 1, authorizationList: [authorization] });
  },
};

describe('decodeSent', () => {
  for (const [name, sign] of Object.entries(signed)) {
    it(`${name}: the hash is keccak of the bytes, the sender and nonce recover`, async () => {
      const raw = await sign();
      const t = decodeSent(hexToBytes(raw));
      expect(t.hash).toBe(keccak256(raw));
      expect(t.from).toBe(account.address.toLowerCase());
      expect(t.nonce).toBe(7n);
      expect(t.chainId).toBe(name.includes('no replay') ? null : 1n);
    });
  }

  it("a blob transaction's network form hashes without its sidecar", async () => {
    const raw = hexToBytes(await signed['EIP-4844']());
    const body = decode(raw.subarray(1)) as Rlp[];
    const blob = new Uint8Array(64).fill(7);
    const network = Uint8Array.of(3, ...encode([body, [blob], [new Uint8Array(48).fill(1)], [new Uint8Array(48).fill(2)]]));
    const t = decodeSent(network);
    expect(t.hash).toBe(keccak256(bytesToHex(raw) as Hex));
    expect(t.from).toBe(account.address.toLowerCase());
  });

  it('bytes that are no transaction throw', () => {
    expect(() => decodeSent(Uint8Array.of(0x05, 0xc0))).toThrow();
    expect(() => decodeSent(Uint8Array.of(0xc1, 0x80))).toThrow();
  });
});

describe('eth_sendRawTransaction and the pending nonce', () => {
  let h: Harness | null = null;
  afterEach(async () => {
    await h?.close();
    h = null;
  });

  const W2_BLOCK = { number: 0x192b4b5, hash: '0x8a27150df15c313310e346ad53e1e04825a7004e0e01096b5cf095e6c7d89552' };

  it("the hash returned is the client's own; the next pending nonce is past it", async () => {
    // The sender's account is synthetic: only the mock state verifier takes it.
    h = await harness({ chainId: 1, mocks: { state: 'mock' } });
    h.world.head(1, W2_BLOCK);
    const raw = await signed['EIP-1559']();
    h.world.standard.set('1|eth_sendRawTransaction|*', { result: keccak256(raw) });
    expect(await h.client.request({ method: 'eth_sendRawTransaction', params: [raw] })).toBe(keccak256(raw));
    // An address of the fixtures with a proven nonce below 8: the eoa fixture's is far above.
    const absent = fixture('state/absent.json');
    expect(absent.request.params[0].toLowerCase()).not.toBe(account.address.toLowerCase());
    h.world.addState(retarget(absent, account.address));
    expect(await h.client.request({ method: 'eth_getTransactionCount', params: [account.address, 'pending'] })).toBe(toQuantity(8));
    expect(await h.client.request({ method: 'eth_getTransactionCount', params: [account.address, 'latest'] })).toBe('0x0');
  });

  it('a server that returns another hash fails verification', async () => {
    h = await harness({ chainId: 1 });
    h.world.head(1, W2_BLOCK);
    const raw = await signed['EIP-1559']();
    h.world.standard.set('1|eth_sendRawTransaction|*', { result: `0x${'00'.repeat(32)}` });
    await expect(h.client.request({ method: 'eth_sendRawTransaction', params: [raw] })).rejects.toMatchObject({ code: -32098, data: { kind: 'tx-hash' } });
  });

  it("the provider's refusal passes through, marked", async () => {
    h = await harness({ chainId: 1 });
    h.world.head(1, W2_BLOCK);
    h.world.standard.set('1|eth_sendRawTransaction|*', { error: { code: -32000, message: 'nonce too low' } });
    await expect(h.client.request({ method: 'eth_sendRawTransaction', params: [await signed['EIP-1559']()] })).rejects.toMatchObject({ code: -32000, message: 'nonce too low' });
    expect(h.events.at(-1)).toMatchObject({ status: 'unverifiable', kind: 'provider-error' });
  });
});

/** The absent-account fixture as another (also absent) address: the mock verifies by equality, not tries. */
function retarget(f: any, address: string): any {
  const g = structuredClone(f);
  g.request.params[0] = address;
  g.result.account.address = address.toLowerCase();
  g.expect.eth_getProof.address = address.toLowerCase();
  return g;
}
