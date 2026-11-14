// viem through willowTransport (design 6.3): getBalance, readContract, estimateGas,
// prepareTransactionRequest, getBlock, getLogs, watchBlocks, waitForTransactionReceipt, against the
// mock server built from the fixtures. W2's state and calls are moved onto W1's Ethereum block
// (World.rebase) so one head serves the whole flow.
import { createPublicClient, parseAbi, type PublicClient } from 'viem';
import { base, mainnet, unichain } from 'viem/chains';
import { afterEach, describe, expect, it } from 'vitest';
import { createVerifier } from '../src/client.js';
import { toQuantity } from '../src/hex.js';
import { willowTransport } from '../src/viem.js';
import { WillowVerificationError } from '../src/errors.js';
import { mockModules } from './support/mock-verifiers.js';
import { serve, type Running } from './support/mock-server.js';
import { blockOf, fullWorld, headAt } from './support/setup.js';
import { fixture, ITEMS, json, type World } from './support/world.js';
import { join } from 'node:path';

const ETH_BLOCK = fixture('block/ethereum-26391511.json');
const EOA = fixture('state/eoa.json').request.params[0] as `0x${string}`;
const USDC = '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48';

let open: { server: Running; close: () => void }[] = [];
afterEach(async () => {
  for (const o of open.splice(0)) {
    o.close();
    await o.server.close();
  }
});

async function viemOn(chainId: number, chain: any, prepare: (w: World) => void): Promise<{ client: PublicClient; world: World }> {
  const world = fullWorld();
  prepare(world);
  const server = await serve(world);
  const t0 = Date.now();
  const v = createVerifier({ chainId, url: server.url(chainId), verifiers: mockModules(world), storage: false, checkpoint: { root: `0x${'11'.repeat(32)}`, slot: 1 }, now: () => world.nowSec * 1000 + (Date.now() - t0), batchWindowMs: 2 });
  open.push({ server, close: () => v.close() });
  return { client: createPublicClient({ chain, transport: willowTransport(v), pollingInterval: 20 }) as PublicClient, world };
}

const ethereumAtOneHead = (w: World) => {
  headAt(w, 1, blockOf('ethereum-26391511.json'));
  for (const f of ['state/eoa.json', 'state/usdc-slots-code.json']) w.addState(w.rebase(fixture(f), ETH_BLOCK));
  for (const f of ['call/call-usdc-balance.json', 'call/call-revert.json', 'call/estimate-transfer.json']) w.addCall(w.rebase(fixture(f), ETH_BLOCK));
};

describe('viem on Ethereum', () => {
  it('getBalance, readContract (a value and a revert), estimateGas', async () => {
    const { client } = await viemOn(1, mainnet, ethereumAtOneHead);
    expect(await client.getBalance({ address: EOA })).toBe(BigInt(fixture('state/eoa.json').expect.eth_getBalance));
    const holder = `0x${fixture('call/call-usdc-balance.json').request.params[0].data.slice(-40)}` as `0x${string}`;
    const abi = parseAbi(['function balanceOf(address) view returns (uint256)', 'function transfer(address,uint256) returns (bool)']);
    expect(await client.readContract({ address: USDC, abi, functionName: 'balanceOf', args: [holder] })).toBe(BigInt(fixture('call/call-usdc-balance.json').expect.result));
    const revert = fixture('call/call-revert.json').request.params[0];
    const data = revert.data as string;
    const to = `0x${data.slice(34, 74)}` as `0x${string}`;
    const amount = BigInt(`0x${data.slice(74)}`);
    await expect(client.simulateContract({ account: revert.from, address: USDC, abi, functionName: 'transfer', args: [to, amount] })).rejects.toThrow(/ERC20: transfer amount exceeds balance/);
    const est = fixture('call/estimate-transfer.json').request.params[0];
    expect(await client.estimateGas({ account: est.from, to: est.to, value: 1n })).toBe(21000n);
  });

  it('prepareTransactionRequest: chain id, nonce, fees and gas, all verified', async () => {
    const { client } = await viemOn(1, mainnet, ethereumAtOneHead);
    const est = fixture('call/estimate-transfer.json').request.params[0];
    const fee = fixture('fees/ethereum-26391511.fee.json').verified;
    const tx = await client.prepareTransactionRequest({ account: est.from, to: est.to, value: 1n, chain: mainnet });
    expect(tx.nonce).toBe(Number(fixture('state/eoa.json').expect.eth_getTransactionCount));
    expect(tx.gas).toBe(21000n);
    expect(tx.maxPriorityFeePerGas).toBe(BigInt(fee.eth_maxPriorityFeePerGas));
    expect(tx.type).toBe('eip1559');
  });

  it('getBlock, with and without transactions', async () => {
    const { client } = await viemOn(1, mainnet, ethereumAtOneHead);
    const b = await client.getBlock();
    expect(b.hash).toBe(ETH_BLOCK.result.blockHash);
    expect(b.transactions.length).toBe(ETH_BLOCK.node.eth_getBlockByNumber.transactions.length);
    const full = await client.getBlock({ includeTransactions: true });
    expect(full.transactions[0]).toMatchObject({ hash: ETH_BLOCK.node.eth_getBlockByNumber.transactions[0].hash });
  });
});

describe('viem on the L2s', () => {
  it('getLogs over one block on Unichain, by its block-receipts proof', async () => {
    const c = json(join(ITEMS, 'uni-block.json'));
    const n = parseInt(c.block.number, 16);
    const { client } = await viemOn(130, unichain, (w) => w.head(130, { number: n, hash: c.block.hash.toLowerCase() }));
    const all = c.answers[0].result.flatMap((r: any) => r.logs);
    const logs = await client.getLogs({ address: all[0].address, fromBlock: BigInt(n), toBlock: BigInt(n) });
    expect(logs.length).toBe(all.filter((l: any) => l.address === all[0].address).length);
    expect(logs[0]).toMatchObject({ transactionHash: all[0].transactionHash, logIndex: Number(all[0].logIndex) });
  });

  it('watchBlocks emits the verified head', async () => {
    const head = blockOf('base-56049971.json');
    const { client } = await viemOn(8453, base, (w) => headAt(w, 8453, head));
    const block = await new Promise<any>((resolve, reject) => {
      const stop = client.watchBlocks({ onBlock: (b) => (stop(), resolve(b)), onError: (e) => (stop(), reject(e)), emitOnBegin: true });
    });
    expect(block.hash).toBe(head.hash);
  });

  it('waitForTransactionReceipt: the receipt is proven, then the verified head moves to its second confirmation', async () => {
    const r = json(join(ITEMS, 'base-swap-receipt.json'));
    const n = parseInt(r.block.number, 16);
    let heads = 0;
    const { client } = await viemOn(8453, base, (w) => {
      w.head(8453, { number: n, hash: r.block.hash.toLowerCase() });
      w.intercept = (_c, m, p) => {
        if (m === 'willow_getBlockAnchor' && p[0] === 'latest' && ++heads === 3) w.head(8453, { number: n + 1, hash: `0x${'5a'.repeat(32)}` });
        return undefined;
      };
    });
    const receipt = await client.waitForTransactionReceipt({ hash: r.target, confirmations: 2, pollingInterval: 20, timeout: 10_000 });
    expect(receipt.transactionHash).toBe(r.target);
    expect(receipt.blockHash).toBe(r.block.hash);
    expect(heads).toBeGreaterThanOrEqual(3);
  });

  it('a verification failure reaches viem as an error, never a value', async () => {
    const head = blockOf('base-56049971.json');
    const { client } = await viemOn(8453, base, (w) => {
      headAt(w, 8453, head);
      const e = w.proofs.get(w.key(8453, 'willow_getBlockProof', [head.hash]))!;
      w.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { result: { ...e.result, header: `${e.result.header.slice(0, -2)}00` } } : undefined);
    });
    const err: any = await client.getBlock().catch((x) => x);
    expect(err).toBeInstanceOf(Error);
    expect(err.walk((x: unknown) => x instanceof WillowVerificationError)).toMatchObject({ code: -32098, data: { kind: 'header' } });
    void toQuantity;
  });
});
