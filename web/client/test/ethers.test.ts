// ethers v6 through WillowProvider (design 6.3): getFeeData, getBalance, call, estimateGas,
// getBlock, getLogs, against the mock server built from the fixtures (W2's moved onto W1's block).
import { join } from 'node:path';
import { Contract, Interface } from 'ethers';
import { afterEach, describe, expect, it } from 'vitest';
import { WillowProvider } from '../src/ethers.js';
import { WillowVerificationError } from '../src/errors.js';
import { mockModules } from './support/mock-verifiers.js';
import { serve, type Running } from './support/mock-server.js';
import { blockOf, fullWorld, headAt } from './support/setup.js';
import { fixture, ITEMS, json, type World } from './support/world.js';

const ETH_BLOCK = fixture('block/ethereum-26391511.json');
const USDC = '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48';

let open: { server: Running; p: WillowProvider }[] = [];
afterEach(async () => {
  for (const o of open.splice(0)) {
    o.p.destroy();
    await o.server.close();
  }
});

async function provider(chainId: number, prepare: (w: World) => void): Promise<WillowProvider> {
  const world = fullWorld();
  prepare(world);
  const server = await serve(world);
  const t0 = Date.now();
  const p = new WillowProvider({ chainId, url: server.url(chainId), verifiers: mockModules(world), storage: false, checkpoint: { root: `0x${'11'.repeat(32)}`, slot: 1 }, now: () => world.nowSec * 1000 + (Date.now() - t0), batchWindowMs: 2 });
  open.push({ server, p });
  return p;
}

const ethereumAtOneHead = (w: World) => {
  headAt(w, 1, blockOf('ethereum-26391511.json'));
  for (const f of ['state/eoa.json', 'state/usdc-slots-code.json']) w.addState(w.rebase(fixture(f), ETH_BLOCK));
  for (const f of ['call/call-usdc-balance.json', 'call/call-revert.json', 'call/estimate-transfer.json']) w.addCall(w.rebase(fixture(f), ETH_BLOCK));
};

describe('ethers on Ethereum', () => {
  it('getFeeData: the gas price, the tip and the block, all verified', async () => {
    const p = await provider(1, ethereumAtOneHead);
    const fee = fixture('fees/ethereum-26391511.fee.json').verified;
    const d = await p.getFeeData();
    expect(d.gasPrice).toBe(BigInt(fee.eth_gasPrice));
    expect(d.maxPriorityFeePerGas).toBe(BigInt(fee.eth_maxPriorityFeePerGas));
    expect(d.maxFeePerGas).toBe(BigInt(ETH_BLOCK.node.eth_getBlockByNumber.baseFeePerGas) * 2n + BigInt(fee.eth_maxPriorityFeePerGas));
  });

  it('getBalance, call, a revert, estimateGas', async () => {
    const p = await provider(1, ethereumAtOneHead);
    const eoa = fixture('state/eoa.json');
    expect(await p.getBalance(eoa.request.params[0])).toBe(BigInt(eoa.expect.eth_getBalance));
    const bal = fixture('call/call-usdc-balance.json');
    expect(await p.call(bal.request.params[0])).toBe(bal.expect.result);
    const usdc = new Contract(USDC, new Interface(['function transfer(address,uint256) returns (bool)']), p);
    const rev = fixture('call/call-revert.json').request.params[0];
    const [to, amount] = new Interface(['function transfer(address,uint256)']).decodeFunctionData('transfer', rev.data);
    await expect(usdc.transfer.staticCall(to, amount, { from: rev.from })).rejects.toMatchObject({ code: 'CALL_EXCEPTION', reason: 'ERC20: transfer amount exceeds balance' });
    const est = fixture('call/estimate-transfer.json').request.params[0];
    expect(await p.estimateGas({ from: est.from, to: est.to, value: 1n })).toBe(21000n);
  });

  it('getBlock, with transactions prefetched', async () => {
    const p = await provider(1, ethereumAtOneHead);
    const b = await p.getBlock('latest');
    expect(b!.hash).toBe(ETH_BLOCK.result.blockHash);
    const full = await p.getBlock('latest', true);
    expect(full!.prefetchedTransactions[0].hash).toBe(ETH_BLOCK.node.eth_getBlockByNumber.transactions[0].hash);
  });

  it('a verification failure is thrown as WillowVerificationError itself', async () => {
    const p = await provider(1, (w) => {
      ethereumAtOneHead(w);
      const e = w.proofs.get(w.key(1, 'willow_getBlockProof', [ETH_BLOCK.result.blockHash]))!;
      w.intercept = (_c, m) => (m === 'willow_getBlockProof' ? { result: { ...e.result, withdrawals: '0xc0' } } : undefined);
    });
    await expect(p.getBlock('latest')).rejects.toBeInstanceOf(WillowVerificationError);
  });
});

describe('ethers on the L2s', () => {
  it('getLogs over one block on Unichain', async () => {
    const c = json(join(ITEMS, 'uni-block.json'));
    const n = parseInt(c.block.number, 16);
    const p = await provider(130, (w) => w.head(130, { number: n, hash: c.block.hash.toLowerCase() }));
    const all = c.answers[0].result.flatMap((r: any) => r.logs);
    const logs = await p.getLogs({ address: all[0].address, fromBlock: n, toBlock: n });
    expect(logs.map((l) => [l.transactionHash, l.index])).toEqual(all.filter((l: any) => l.address === all[0].address).map((l: any) => [l.transactionHash, Number(l.logIndex)]));
  });
});
