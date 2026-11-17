import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { createVerifier, type VerificationEvent, type WillowOptions, type WillowVerifier } from '../../src/client.js';
import type { VerifierModules } from '../../src/verifiers.js';
import { proofModules, type Mocks } from './mock-verifiers.js';
import { serve, type Running } from './mock-server.js';
import { fixture, fixtureNames, ITEMS, json, World } from './world.js';

/** Chains by fixture prefix. */
export const CHAIN: Record<string, number> = { ethereum: 1, base: 8453, op: 10, unichain: 130, ink: 57073, 'arbitrum-one': 42161, robinhood: 4663 };
export const chainOf = (file: string): number => CHAIN[Object.keys(CHAIN).find((p) => file.startsWith(`${p}-`))!];

/** Every fixture in one world: W1 blocks and fees, W2 state and calls (Ethereum, Base), the shipped items. */
export function fullWorld(): World {
  const w = new World();
  for (const f of fixtureNames('block')) if (f !== 'ethereum-1500000-uncles-unproven.json') w.addBlock(fixture(`block/${f}`));
  for (const f of fixtureNames('fees')) {
    const x = fixture(`fees/${f}`);
    if (f.endsWith('.fee.json')) w.addFees(x);
    else w.addFeeHistory(x);
  }
  for (const f of fixtureNames('state')) w.addState(fixture(`state/${f}`), f === 'base-usdc.json' ? 8453 : 1);
  for (const f of fixtureNames('call')) w.addCall(fixture(`call/${f}`));
  for (const f of readdirSync(ITEMS).filter((x) => x.endsWith('.json'))) w.addItem(json(join(ITEMS, f)));
  return w;
}

/** The W1 block fixture whose block is `chainId`'s head in tests (fee fixtures are at the same blocks). */
export const HEAD_FIXTURE: Record<number, string> = {
  1: 'ethereum-26391511.json',
  8453: 'base-56049971.json',
  10: 'op-159400794.json',
  130: 'unichain-63652013.json',
  57073: 'ink-60901963.json',
  42161: 'arbitrum-one-523278185.json',
  4663: 'robinhood-111702114.json',
};

export function headAt(w: World, chainId: number, block: { number: number; hash: string }, level: 'signed' | 'final' = 'signed'): void {
  w.head(chainId, { number: block.number, hash: block.hash, level, path: chainId === 1 ? (level === 'final' ? 'history' : 'head') : 'sequencer' });
}

/** The block of a W1 fixture: number and hash. */
export function blockOf(file: string): { number: number; hash: string } {
  const r = fixture(`block/${file}`).result;
  return { number: parseInt(r.blockNumber, 16), hash: r.blockHash.toLowerCase() };
}

export interface Harness {
  world: World;
  server: Running;
  client: WillowVerifier;
  events: VerificationEvent[];
  close: () => Promise<void>;
}

export async function harness(o: { chainId: number; world?: World; mocks?: Mocks; verifiers?: VerifierModules; options?: Partial<WillowOptions>; key?: string; url?: (s: Running) => string }): Promise<Harness> {
  const world = o.world ?? fullWorld();
  const server = await serve(world);
  const events: VerificationEvent[] = [];
  const client = createVerifier({
    chainId: o.chainId,
    url: o.url ? o.url(server) : server.url(o.chainId, o.key),
    verifiers: o.verifiers ?? proofModules(world, o.mocks),
    storage: false,
    checkpoint: { root: `0x${'11'.repeat(32)}`, slot: 15_613_952 },
    now: () => world.nowSec * 1000,
    batchWindowMs: 2,
    waitMs: { anchor: 5, jobs: 5 },
    onVerification: (e) => events.push(e),
    ...o.options,
  });
  return {
    world,
    server,
    client,
    events,
    close: async () => {
      client.close();
      await server.close();
    },
  };
}

/** The items of every HTTP request the server saw, by method. */
export const methodsSent = (w: World): string[] => w.log.flatMap((r) => r.items.map((i) => i.method));
