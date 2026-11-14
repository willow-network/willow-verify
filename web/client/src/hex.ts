import { keccak_256 } from '@noble/hashes/sha3';

export const ZERO_HASH = `0x${'00'.repeat(32)}`;

export function hexToBytes(h: string): Uint8Array {
  if (typeof h !== 'string' || !/^0x([0-9a-fA-F]{2})*$/.test(h)) throw new Error(`not hex data: ${String(h).slice(0, 80)}`);
  const out = new Uint8Array((h.length - 2) / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.slice(2 + 2 * i, 4 + 2 * i), 16);
  return out;
}

export function bytesToHex(b: Uint8Array): string {
  let s = '0x';
  for (const x of b) s += x.toString(16).padStart(2, '0');
  return s;
}

export const keccak = (b: Uint8Array): Uint8Array => keccak_256(b);
export const keccakHex = (b: Uint8Array): string => bytesToHex(keccak_256(b));

/** A `0x` quantity or a decimal string or a safe integer → number; null when it is none. */
export function toNumber(x: unknown): number | null {
  if (typeof x === 'number') return Number.isSafeInteger(x) && x >= 0 ? x : null;
  if (typeof x === 'bigint') return x >= 0n && x <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(x) : null;
  if (typeof x !== 'string') return null;
  if (/^0x[0-9a-fA-F]{1,14}$/.test(x)) return parseInt(x, 16);
  if (/^[0-9]{1,15}$/.test(x)) return parseInt(x, 10);
  return null;
}

export const toQuantity = (n: number | bigint): string => `0x${n.toString(16)}`;

export const isHash = (x: unknown): x is string => typeof x === 'string' && /^0x[0-9a-fA-F]{64}$/.test(x);
export const isAddress = (x: unknown): x is string => typeof x === 'string' && /^0x[0-9a-fA-F]{40}$/.test(x);

export const sameHex = (a: unknown, b: unknown): boolean => typeof a === 'string' && typeof b === 'string' && a.toLowerCase() === b.toLowerCase();

/** A storage key as 32 bytes, lowercase, for comparing keys written differently. */
export function slotKey(k: string): string {
  if (typeof k !== 'string' || !/^0x[0-9a-fA-F]{0,64}$/.test(k)) throw new Error(`not a storage key: ${String(k).slice(0, 80)}`);
  return `0x${k.slice(2).toLowerCase().padStart(64, '0')}`;
}
