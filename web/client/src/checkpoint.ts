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

/** BNB Smart Chain's light client starts at an epoch block; `time` is its timestamp (seconds). */
export interface BnbStart {
  number: number;
  hash: string;
  time: number;
}

/** Where the other light clients start: Sepolia's (the testnets) and BNB's. */
export interface Starts {
  sepolia: Checkpoint;
  bnb: BnbStart;
}

/**
 * The package's starts besides Ethereum's, rewritten by the same build: Sepolia's slot 11,575,968
 * (2026-11-14, after Gloas; the same root at publicnode and ChainSafe) and BNB's epoch block
 * 133,510,000 (2026-11-14, the same hash at three operators).
 */
export const SHIPPED_STARTS: Starts = {
  sepolia: { root: '0x35f9d71dbf378dce8387786beaeb03b7496bb245a2d88692a76fd38df40923fd', slot: 11575968 },
  bnb: { number: 133510000, hash: '0x51e9a0b4ab8fe7bfa6745a14c99860342f866d4b341828092356db2f8748a35f', time: 1794644124 },
};

const SEPOLIA_GENESIS_TIME = 1_655_733_600;
/** The verifier refuses a BNB start older than BNB's 7-day unbonding (anchor-verify bnb.rs). */
export const MAX_BNB_START_AGE_SECS = 7 * 24 * 3600;
/** A shipped BNB start older than this is replaced by `checkpoint.json`'s, when it holds a newer one. */
export const BNB_REFRESH_SECS = 24 * 3600;

const isBnbStart = (b: unknown): b is BnbStart =>
  !!b &&
  typeof b === 'object' &&
  isHash((b as BnbStart).hash) &&
  Number.isSafeInteger((b as BnbStart).number) &&
  (b as BnbStart).number % 1000 === 0 &&
  Number.isSafeInteger((b as BnbStart).time);

export interface StartsOptions {
  sepolia?: Checkpoint;
  bnb?: BnbStart;
  checkpointUrl?: string;
  fetch: typeof fetch;
  nowSec: () => number;
}

/**
 * The other light clients' starts: the ones passed in; else the shipped ones; else, once one is
 * older than its verifier accepts (a day's margin), `checkpoint.json`'s `sepolia` or `bnb` when it
 * holds a newer one. A start that stays too old only fails the chains it anchors.
 */
export async function chooseStarts(o: StartsOptions): Promise<Starts> {
  const now = o.nowSec();
  const sepoliaOld = (c: Checkpoint) => now - (SEPOLIA_GENESIS_TIME + 12 * c.slot) >= MAX_START_AGE_SECS - 24 * 3600;
  // BNB's verifier takes 7 days, but a cold client walks every epoch since its start (about 2 s a
  // day): a newer start is taken once the shipped one is a day old.
  const bnbOld = (b: BnbStart) => now - b.time >= BNB_REFRESH_SECS;
  let sepolia = o.sepolia ?? SHIPPED_STARTS.sepolia;
  let bnb = o.bnb ?? SHIPPED_STARTS.bnb;
  if ((!o.sepolia && sepoliaOld(sepolia)) || (!o.bnb && bnbOld(bnb))) {
    let c: any = null;
    try {
      const r = await o.fetch(o.checkpointUrl ?? CHECKPOINT_URL, { headers: { accept: 'application/json' }, signal: AbortSignal.timeout(30_000) });
      c = await r.json();
    } catch {
      c = null;
    }
    if (!o.sepolia && isCheckpoint(c?.sepolia) && c.sepolia.slot > sepolia.slot) sepolia = { root: c.sepolia.root, slot: c.sepolia.slot };
    if (!o.bnb && isBnbStart(c?.bnb) && c.bnb.number > bnb.number) bnb = { number: c.bnb.number, hash: c.bnb.hash, time: c.bnb.time };
  }
  return { sepolia, bnb };
}
