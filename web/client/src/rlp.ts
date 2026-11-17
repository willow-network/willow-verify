/** RLP, as far as a raw transaction needs: decode a whole item, encode lists of byte strings. */
export type Rlp = Uint8Array | Rlp[];

export function decode(b: Uint8Array): Rlp {
  const [item, end] = item_(b, 0);
  if (end !== b.length) throw new Error('trailing bytes after the RLP item');
  return item;
}

function length(b: Uint8Array, at: number, n: number): number {
  if (at + n > b.length) throw new Error('RLP length runs past the input');
  if (b[at] === 0) throw new Error('RLP length with a leading zero');
  let v = 0;
  for (let i = 0; i < n; i++) v = v * 256 + b[at + i];
  if (v < 56) throw new Error('RLP long form for a short item');
  return v;
}

function item_(b: Uint8Array, at: number): [Rlp, number] {
  if (at >= b.length) throw new Error('RLP input ends early');
  const p = b[at];
  if (p < 0x80) return [b.subarray(at, at + 1), at + 1];
  if (p < 0xb8) {
    const n = p - 0x80;
    if (at + 1 + n > b.length) throw new Error('RLP string runs past the input');
    if (n === 1 && b[at + 1] < 0x80) throw new Error('RLP single byte not in canonical form');
    return [b.subarray(at + 1, at + 1 + n), at + 1 + n];
  }
  if (p < 0xc0) {
    const ll = p - 0xb7;
    const n = length(b, at + 1, ll);
    const s = at + 1 + ll;
    if (s + n > b.length) throw new Error('RLP string runs past the input');
    return [b.subarray(s, s + n), s + n];
  }
  let s: number;
  let n: number;
  if (p < 0xf8) {
    n = p - 0xc0;
    s = at + 1;
  } else {
    const ll = p - 0xf7;
    n = length(b, at + 1, ll);
    s = at + 1 + ll;
  }
  const end = s + n;
  if (end > b.length) throw new Error('RLP list runs past the input');
  const out: Rlp[] = [];
  for (let i = s; i < end; ) {
    const [x, next] = item_(b, i);
    out.push(x);
    i = next;
  }
  return [out, end];
}

function header(len: number, short: number, long: number): Uint8Array {
  if (len < 56) return Uint8Array.of(short + len);
  const bytes: number[] = [];
  for (let v = len; v > 0; v = Math.floor(v / 256)) bytes.unshift(v % 256);
  return Uint8Array.of(long + bytes.length, ...bytes);
}

const concat = (parts: Uint8Array[]): Uint8Array => {
  const out = new Uint8Array(parts.reduce((t, p) => t + p.length, 0));
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
};

export function encode(x: Rlp): Uint8Array {
  if (x instanceof Uint8Array) {
    if (x.length === 1 && x[0] < 0x80) return x;
    return concat([header(x.length, 0x80, 0xb7), x]);
  }
  const body = concat(x.map(encode));
  return concat([header(body.length, 0xc0, 0xf7), body]);
}

export const isList = (x: Rlp | undefined): x is Rlp[] => Array.isArray(x);

/** A byte string as an unsigned integer. */
export function uint(x: Rlp | undefined): bigint {
  if (!(x instanceof Uint8Array)) throw new Error('expected an RLP integer');
  if (x.length > 0 && x[0] === 0) throw new Error('RLP integer with a leading zero');
  let v = 0n;
  for (const b of x) v = (v << 8n) | BigInt(b);
  return v;
}

export { concat };
