#!/usr/bin/env python3
"""Independent oracle for the Base KAT (block 48,932,815): decodes the header and
the 115 receipt trie values with its own RLP reader (no shared code with
logset-core), rebuilds the match-all ERC record stream and writes the expected
statement to 48932815.expected.json. `core/src/tests.rs::base_real_block_kat`
asserts the guest's statement equals it.

  python3 oracle.py        # needs pycryptodome (Crypto.Hash.keccak)
"""
import json
import os
from Crypto.Hash import keccak

HERE = os.path.dirname(os.path.abspath(__file__))
BLOCK = 48932815
CHAIN_ID = 8453
BLOCK_TAG = b"COMPLETE_LOG_SET_BLOCK_V1".ljust(32, b"\0")


def k256(b):
    h = keccak.new(digest_bits=256)
    h.update(b)
    return h.digest()


def item(b, at):
    """(payload_start, payload_end, is_list, next_at) of the RLP item at `at`."""
    x = b[at]
    if x < 0x80:
        return at, at + 1, False, at + 1
    if x < 0xB8:
        n = x - 0x80
        return at + 1, at + 1 + n, False, at + 1 + n
    if x < 0xC0:
        ll = x - 0xB7
        n = int.from_bytes(b[at + 1 : at + 1 + ll], "big")
        s = at + 1 + ll
        return s, s + n, False, s + n
    if x < 0xF8:
        n = x - 0xC0
        return at + 1, at + 1 + n, True, at + 1 + n
    ll = x - 0xF7
    n = int.from_bytes(b[at + 1 : at + 1 + ll], "big")
    s = at + 1 + ll
    return s, s + n, True, s + n


def items(b, start, end):
    out, at = [], start
    while at < end:
        s, e, is_list, at = item(b, at)
        out.append((s, e, is_list))
    assert at == end
    return out


def main():
    header = open(os.path.join(HERE, f"{BLOCK}.header.rlp"), "rb").read()
    hs, he, hl, hn = item(header, 0)
    assert hl and hn == len(header)
    hf = items(header, hs, he)
    parent = header[hf[0][0] : hf[0][1]]
    receipts_root = header[hf[5][0] : hf[5][1]]
    number = int.from_bytes(header[hf[8][0] : hf[8][1]], "big")
    assert number == BLOCK

    raw = open(os.path.join(HERE, f"{BLOCK}.receipts.bin"), "rb").read()
    values, at = [], 0
    while at < len(raw):
        n = int.from_bytes(raw[at : at + 4], "little")
        values.append(raw[at + 4 : at + 4 + n])
        at += 4 + n

    records = bytearray()
    k = 0
    types = {}
    deposit_tails = []
    for tx, v in enumerate(values):
        ty = v[0] if v[0] < 0xC0 else 0
        types[ty] = types.get(ty, 0) + 1
        body = v[1:] if ty else v
        s, e, is_list, nxt = item(body, 0)
        assert is_list and nxt == len(body)
        f = items(body, s, e)
        if ty == 0x7E:
            assert len(f) in (4, 5, 6)
            deposit_tails.append(
                [int.from_bytes(body[a:b], "big") for (a, b, _) in f[4:]]
            )
        else:
            assert len(f) == 4
        ls, le, _ = f[3]
        for (a, b, lst) in items(body, ls, le):
            assert lst
            parts = items(body, a, b)
            emitter = body[parts[0][0] : parts[0][1]]
            topics = [body[x:y] for (x, y, _) in items(body, parts[1][0], parts[1][1])]
            data = body[parts[2][0] : parts[2][1]]
            assert len(emitter) == 20 and len(topics) <= 4
            records += tx.to_bytes(4, "big") + k.to_bytes(4, "big") + emitter
            records += bytes([len(topics)]) + b"".join(topics)
            records += len(data).to_bytes(4, "big") + data
            k += 1

    block_logs_hash = k256(BLOCK_TAG + BLOCK.to_bytes(8, "big") + k.to_bytes(4, "big") + records)
    out = {
        "name": "base-48932815",
        "chain_id": CHAIN_ID,
        "block": BLOCK,
        "receipts": len(values),
        "receipt_types": {f"0x{t:02x}": c for t, c in sorted(types.items())},
        "deposit_tails": deposit_tails,
        "receipts_root": "0x" + receipts_root.hex(),
        "start_parent_hash": "0x" + parent.hex(),
        "end_block_hash": "0x" + k256(header).hex(),
        "log_count": k,
        "log_set_root": "0x" + block_logs_hash.hex(),
    }
    with open(os.path.join(HERE, f"{BLOCK}.expected.json"), "w") as fh:
        json.dump(out, fh, indent=2)
        fh.write("\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
