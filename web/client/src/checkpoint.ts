import { WillowUnverifiableError } from './errors.js';
import { isHash } from './hex.js';

export interface Checkpoint {
  /** A finalized beacon block root. */
  root: string;
  slot: number;
}

/**
 * The package's starting point on Ethereum. The build that refreshes the page's ETH_CHECKPOINT
 * (web/verify/build.sh, from checkpoint.py) rewrites this line; here it is the page's own.
 */
export const SHIPPED_CHECKPOINT: Checkpoint = { root: '0x655955cfa7904a0e3fc1aee3f05028314c679e2ad62d1f210061e84c68347cc8', slot: 15630336 };

/** The verifier refuses a starting point older than this (anchor-verify lc.rs `MAX_START_AGE_SECS`). */
export const MAX_START_AGE_SECS = 30 * 24 * 3600;
export const CHECKPOINT_URL = 'https://api.willow.tech/verify/checkpoint.json';
export const BEACON_APIS = ['https://ethereum-beacon-api.publicnode.com', 'https://lodestar-mainnet.chainsafe.io'];
const GENESIS_TIME = 1_606_824_023;

export const slotTime = (slot: number): number => GENESIS_TIME + 12 * slot;

const isCheckpoint = (c: unknown): c is Checkpoint =>
  !!c && typeof c === 'object' && isHash((c as Checkpoint).root) && Number.isSafeInteger((c as Checkpoint).slot) && (c as Checkpoint).slot > 0;

export interface CheckpointOptions {
  checkpoint?: Checkpoint;
  checkpointUrl?: string;
  /** Beacon APIs that must confirm a fetched checkpoint; false: none (browsers, where CORS is unknown). */
  beaconApis?: string[] | false;
  fetch: typeof fetch;
  nowSec: () => number;
}

/**
 * The light client's starting point: the one passed in; else the shipped one; else, once that is
 * older than the verifier accepts, checkpoint.json from Willow, confirmed by `beaconApis` (at
 * least one confirms, none disagrees).
 */
export async function chooseCheckpoint(o: CheckpointOptions): Promise<Checkpoint> {
  if (o.checkpoint) {
    if (!isCheckpoint(o.checkpoint)) throw new Error('checkpoint must be {root: 0x…32 bytes, slot}');
    return o.checkpoint;
  }
  // A day's margin: a session started near the limit still verifies until it ends.
  if (o.nowSec() - slotTime(SHIPPED_CHECKPOINT.slot) < MAX_START_AGE_SECS - 24 * 3600) return SHIPPED_CHECKPOINT;
  const stale = (why: string) => new WillowUnverifiableError('anchor-stale-checkpoint', why);
  let c: unknown;
  try {
    const r = await o.fetch(o.checkpointUrl ?? CHECKPOINT_URL, { headers: { accept: 'application/json' }, signal: AbortSignal.timeout(30_000) });
    c = await r.json();
  } catch (e) {
    throw stale(`the shipped checkpoint is old and ${o.checkpointUrl ?? CHECKPOINT_URL} could not be read`);
  }
  if (!isCheckpoint(c) || c.slot <= SHIPPED_CHECKPOINT.slot) throw stale('checkpoint.json holds no newer checkpoint');
  const apis = o.beaconApis === false ? [] : (o.beaconApis ?? BEACON_APIS);
  if (apis.length) {
    let confirmed = 0;
    for (const api of apis) {
      let root: unknown;
      try {
        const r = await o.fetch(`${api.replace(/\/$/, '')}/eth/v1/beacon/headers/${c.slot}`, { headers: { accept: 'application/json' }, signal: AbortSignal.timeout(30_000) });
        const j: any = await r.json();
        root = j?.data?.canonical === false ? null : j?.data?.root;
      } catch {
        continue;
      }
      if (typeof root !== 'string' || root.toLowerCase() !== c.root.toLowerCase()) throw stale(`${api} does not confirm checkpoint ${c.root} at slot ${c.slot}`);
      confirmed++;
    }
    if (!confirmed) throw stale('no beacon API could confirm the fetched checkpoint');
  }
  return { root: c.root, slot: c.slot };
}
