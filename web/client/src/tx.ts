import { secp256k1 } from '@noble/curves/secp256k1';
import { bytesToHex, keccak } from './hex.js';
import { concat, decode, encode, isList, uint, type Rlp } from './rlp.js';

/** A signed transaction as `eth_sendRawTransaction` sends it. */
export interface SentTransaction {
  /** The transaction hash: keccak of its EIP-2718 bytes, without a blob transaction's sidecar. */
  hash: string;
  /** The signer, recovered from the signature; null when the signature does not recover. */
  from: string | null;
  nonce: bigint;
  /** null for a legacy transaction without EIP-155 replay protection. */
  chainId: bigint | null;
}

/** Fields before the signature, by EIP-2718 type (the nonce's index is 1 in each). */
const UNSIGNED_FIELDS: Record<number, number> = { 1: 8, 2: 9, 3: 11, 4: 10 };

const bigEndian = (b: Uint8Array, at: number, n: number): number => {
  let v = 0;
  for (let i = 0; i < n; i++) v = v * 256 + b[at + i];
  return v;
};

/** Where the RLP item at `at` ends. */
function itemEnd(b: Uint8Array, at: number): number {
  const p = b[at];
  if (p < 0x80) return at + 1;
  if (p < 0xb8) return at + 1 + (p - 0x80);
  if (p < 0xc0) return at + 1 + (p - 0xb7) + bigEndian(b, at + 1, p - 0xb7);
  if (p < 0xf8) return at + 1 + (p - 0xc0);
  return at + 1 + (p - 0xf7) + bigEndian(b, at + 1, p - 0xf7);
}

/** The bytes of a list's first item, as they are (no re-encoding). Call after `decode` accepted it. */
function firstItem(list: Uint8Array): Uint8Array {
  if (list[0] < 0xc0) throw new Error('not an RLP list');
  const s = list[0] < 0xf8 ? 1 : 1 + (list[0] - 0xf7);
  return list.subarray(s, itemEnd(list, s));
}

function recover(sighash: Uint8Array, bit: number, r: bigint, s: bigint): string | null {
  try {
    if (bit !== 0 && bit !== 1) return null;
    const pub = new secp256k1.Signature(r, s).addRecoveryBit(bit).recoverPublicKey(sighash).toRawBytes(false);
    return bytesToHex(keccak(pub.subarray(1)).subarray(12));
  } catch {
    return null;
  }
}

const bytesOf = (n: bigint): Uint8Array => {
  if (n === 0n) return new Uint8Array();
  const hex = n.toString(16);
  const h = hex.length % 2 ? `0${hex}` : hex;
  return Uint8Array.from(h.match(/../g)!.map((x) => parseInt(x, 16)));
};

/** Decodes a signed transaction's bytes; throws when they are not one. */
export function decodeSent(raw: Uint8Array): SentTransaction {
  if (raw.length === 0) throw new Error('empty transaction');
  const type = raw[0];
  if (type >= 0xc0) {
    const f = decode(raw);
    if (!isList(f) || f.length !== 9) throw new Error('a legacy transaction has 9 fields');
    const v = uint(f[6]);
    const r = uint(f[7]);
    const s = uint(f[8]);
    const unsigned = f.slice(0, 6);
    let chainId: bigint | null = null;
    let bit: number;
    if (v === 27n || v === 28n) bit = Number(v - 27n);
    else if (v >= 35n) {
      chainId = (v - 35n) >> 1n;
      bit = Number((v - 35n) & 1n);
      unsigned.push(bytesOf(chainId), new Uint8Array(), new Uint8Array());
    } else throw new Error(`a legacy transaction's v is ${v}`);
    return { hash: bytesToHex(keccak(raw)), from: recover(keccak(encode(unsigned)), bit, r, s), nonce: uint(f[0]), chainId };
  }
  const count = UNSIGNED_FIELDS[type];
  if (count === undefined) throw new Error(`unknown transaction type ${type}`);
  let body = raw.subarray(1);
  let canonical = raw;
  if (type === 3) {
    const outer = decode(body);
    // The network form wraps the transaction with its blobs: [tx, (version,) blobs, commitments, proofs].
    if (isList(outer) && isList(outer[0])) {
      body = firstItem(body);
      canonical = concat([Uint8Array.of(3), body]);
    }
  }
  const f = decode(body) as Rlp[];
  if (!isList(f) || f.length !== count + 3) throw new Error(`a type ${type} transaction has ${count + 3} fields`);
  const sighash = keccak(concat([Uint8Array.of(type), encode(f.slice(0, count))]));
  const from = recover(sighash, Number(uint(f[count])), uint(f[count + 1]), uint(f[count + 2]));
  return { hash: bytesToHex(keccak(canonical)), from, nonce: uint(f[1]), chainId: uint(f[0]) };
}
