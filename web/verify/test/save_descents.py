"""Saves the header descents the anchor tests check: for an L2 anchor fixture whose claimed block is
M, the RLP headers of blocks M-1, M-2, ... down to M-COUNT, each re-encoded from the chain's public
RPC and checked against its hash and its child's parentHash.

    python3 -I save_descents.py

Writes test/anchor/descent-<chain>-<M>.json: {"chainId", "top", "descent": [rlp hex, ...]}.
"""

import json
import os
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
# Robinhood Chain's committed anchor of block 92,797,219 (l2-4663-92797219.json).
CASES = [(4663, 92797219, 48, "https://rpc.mainnet.chain.robinhood.com")]


def call(url, calls):
    body = json.dumps([{"jsonrpc": "2.0", "id": i, "method": m, "params": p} for i, (m, p) in enumerate(calls)]).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json", "User-Agent": "willow-anchor-tests"})
    with urllib.request.urlopen(req, timeout=60) as r:
        out = json.loads(r.read())
    out.sort(key=lambda x: x["id"])
    return [x["result"] for x in out]


def enc_bytes(b):
    if len(b) == 1 and b[0] < 0x80:
        return b
    if len(b) < 56:
        return bytes([0x80 + len(b)]) + b
    n = len(b).to_bytes((len(b).bit_length() + 7) // 8, "big")
    return bytes([0xB7 + len(n)]) + n + b


def enc_list(items):
    p = b"".join(items)
    if len(p) < 56:
        return bytes([0xC0 + len(p)]) + p
    n = len(p).to_bytes((len(p).bit_length() + 7) // 8, "big")
    return bytes([0xF7 + len(n)]) + n + p


def hx(s):
    return bytes.fromhex(s[2:]) if s and s != "0x" else b""


def q(s):
    n = int(s, 16)
    return n.to_bytes((n.bit_length() + 7) // 8, "big") if n else b""


FIELDS = [("parentHash", hx), ("sha3Uncles", hx), ("miner", hx), ("stateRoot", hx), ("transactionsRoot", hx),
          ("receiptsRoot", hx), ("logsBloom", hx), ("difficulty", q), ("number", q), ("gasLimit", q), ("gasUsed", q),
          ("timestamp", q), ("extraData", hx), ("mixHash", hx), ("nonce", hx)]
OPT = [("baseFeePerGas", q), ("withdrawalsRoot", hx), ("blobGasUsed", q), ("excessBlobGas", q),
       ("parentBeaconBlockRoot", hx), ("requestsHash", hx)]


def rlp_header(b):
    items = [enc_bytes(f(b[k])) for k, f in FIELDS]
    for k, f in OPT:
        if b.get(k) is None:
            break
        items.append(enc_bytes(f(b[k])))
    return enc_list(items)


RC = [0x0000000000000001, 0x0000000000008082, 0x800000000000808A, 0x8000000080008000, 0x000000000000808B,
      0x0000000080000001, 0x8000000080008081, 0x8000000000008009, 0x000000000000008A, 0x0000000000000088,
      0x0000000080008009, 0x000000008000000A, 0x000000008000808B, 0x800000000000008B, 0x8000000000008089,
      0x8000000000008003, 0x8000000000008002, 0x8000000000000080, 0x000000000000800A, 0x800000008000000A,
      0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008]
ROT = [[0, 36, 3, 41, 18], [1, 44, 10, 45, 2], [62, 6, 43, 15, 61], [28, 55, 25, 21, 56], [27, 20, 39, 8, 14]]
MASK = (1 << 64) - 1


def rol(x, n):
    return ((x << n) | (x >> (64 - n))) & MASK if n else x


def keccak_f(a):
    for rc in RC:
        c = [a[x][0] ^ a[x][1] ^ a[x][2] ^ a[x][3] ^ a[x][4] for x in range(5)]
        d = [c[(x - 1) % 5] ^ rol(c[(x + 1) % 5], 1) for x in range(5)]
        a = [[a[x][y] ^ d[x] for y in range(5)] for x in range(5)]
        b = [[0] * 5 for _ in range(5)]
        for x in range(5):
            for y in range(5):
                b[y][(2 * x + 3 * y) % 5] = rol(a[x][y], ROT[x][y])
        a = [[b[x][y] ^ ((~b[(x + 1) % 5][y]) & b[(x + 2) % 5][y]) for y in range(5)] for x in range(5)]
        a[0][0] ^= rc
    return a


def keccak(data):
    """Keccak-256 as Ethereum hashes (not SHA3-256)."""
    rate, p = 136, bytearray(data) + b"\x01"
    while len(p) % rate:
        p.append(0)
    p[-1] |= 0x80
    a = [[0] * 5 for _ in range(5)]
    for off in range(0, len(p), rate):
        for i in range(rate // 8):
            a[i % 5][i // 5] ^= int.from_bytes(p[off + 8 * i : off + 8 * i + 8], "little")
        a = keccak_f(a)
    return b"".join(a[i % 5][i // 5].to_bytes(8, "little") for i in range(4))


assert keccak(b"").hex() == "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"


for chain, top, count, url in CASES:
    blocks = []
    for a in range(top, top - count - 1, -16):
        blocks += call(url, [("eth_getBlockByNumber", [hex(n), False]) for n in range(a, max(a - 16, top - count - 1), -1)])
    assert [int(b["number"], 16) for b in blocks] == list(range(top, top - count - 1, -1))
    for child, b in zip(blocks, blocks[1:]):
        assert child["parentHash"] == b["hash"]
    out = []
    for b in blocks:
        r = rlp_header(b)
        assert "0x" + keccak(r).hex() == b["hash"], b["number"]
        out.append("0x" + r.hex())
    path = os.path.join(HERE, "anchor", f"descent-{chain}-{top}.json")
    with open(path, "w") as f:
        json.dump({"chainId": chain, "top": top, "topHash": blocks[0]["hash"], "descent": out[1:]}, f, indent=1)
        f.write("\n")
    print(f"{path}: {count} headers below {top}")
