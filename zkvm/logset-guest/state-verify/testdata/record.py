#!/usr/bin/env python3
"""Record the state, call and estimate fixtures (I2, I3) from public nodes, at one recent block.

    python3 record.py [--geth URL] [--base URL]

The Ethereum node must be geth v1.17 (`web3_clientVersion`): its eth_call and eth_estimateGas
responses are what the crate's own runs must equal. Each fixture holds the request, the proof
result as Willow sends it, the standard response the client derives from it (`expect`), and the
node's own standard response at the same block (`reference`).

The witness of a call is what the node's eth_createAccessList says the call reads, plus the
sender, the recipient, the block's beneficiary, a creation's new address and the EIP-2935 slots of
the block hashes it reads. The crate's strict run proves it complete: a read left out fails.
Needs `requests`, `rlp` and `pycryptodome`.
"""

import argparse
import json
import pathlib
import sys
import time

import requests
import rlp
from Crypto.Hash import keccak

HERE = pathlib.Path(__file__).resolve().parent
ZERO32 = "0x" + "00" * 32
KECCAK_EMPTY = "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
EMPTY_ROOT = "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"
HISTORY = "0x0000f90827f1c53a10cb7a02335b175320002935"
RING = 8191

USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
UNISWAP_V2_ROUTER = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"
# Binance 14: an EOA holding ETH (and USDC, which USDC blacklists).
HOLDER = "0x28c6c06298d514db089934071355e5743bf21d60"
# Uniswap v3's USDC/WETH pool: a contract holding USDC.
USDC_SENDER = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"
# Random addresses: no account.
NOBODY = "0x6c3f9e1a2b7d4c58e0f1a9b3d2c4e6f8a1b3c5d7"
NOBODY2 = "0x4b1d5e7c9a3f2e6d8c0b1a2f3e4d5c6b7a8f9e01"
BASE_USDC = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"


def k256(b: bytes) -> bytes:
    return keccak.new(digest_bits=256, data=b).digest()


def hx(b: bytes) -> str:
    return "0x" + b.hex()


def unhex(s: str) -> bytes:
    s = s[2:] if s.startswith("0x") else s
    return bytes.fromhex(s if len(s) % 2 == 0 else "0" + s)


def q(n: int) -> str:
    return hex(n)


def word(n: int) -> str:
    return n.to_bytes(32, "big").hex()


def addr_word(a: str) -> str:
    return a[2:].rjust(64, "0")


class Node:
    def __init__(self, url: str):
        self.url = url
        self.n = 0

    def raw(self, method: str, params):
        """The whole response object, without `jsonrpc` and `id`."""
        for attempt in range(6):
            self.n += 1
            body = {"jsonrpc": "2.0", "id": self.n, "method": method, "params": params}
            try:
                r = requests.post(self.url, json=body, timeout=30)
                out = r.json()
            except Exception as e:  # noqa: BLE001
                print(f"  {method}: {e}; retrying", file=sys.stderr)
                time.sleep(2 + 2 * attempt)
                continue
            err = out.get("error") or {}
            if err.get("code") in (-32005, -32029, 429) or "rate" in str(err.get("message", "")).lower():
                time.sleep(2 + 2 * attempt)
                continue
            out.pop("id", None)
            out.pop("jsonrpc", None)
            time.sleep(0.15)
            return out
        raise SystemExit(f"{method} {params}: no response from {self.url}")

    def call(self, method: str, params):
        out = self.raw(method, params)
        if "error" in out:
            raise SystemExit(f"{method} {json.dumps(params)[:200]}: {out['error']}")
        return out["result"]


def header_rlp(block: dict) -> bytes:
    """The header's RLP from its JSON, checked against its hash."""
    def num(k):
        v = int(block[k], 16)
        return b"" if v == 0 else v.to_bytes((v.bit_length() + 7) // 8, "big")

    fields = [
        unhex(block["parentHash"]), unhex(block["sha3Uncles"]), unhex(block["miner"]),
        unhex(block["stateRoot"]), unhex(block["transactionsRoot"]), unhex(block["receiptsRoot"]),
        unhex(block["logsBloom"]), num("difficulty"), num("number"), num("gasLimit"),
        num("gasUsed"), num("timestamp"), unhex(block["extraData"]), unhex(block["mixHash"]),
        unhex(block["nonce"]),
    ]
    for k in ("baseFeePerGas", "withdrawalsRoot", "blobGasUsed", "excessBlobGas",
              "parentBeaconBlockRoot", "requestsHash"):
        if k not in block:
            break
        fields.append(num(k) if k in ("baseFeePerGas", "blobGasUsed", "excessBlobGas") else unhex(block[k]))
    out = rlp.encode(fields)
    assert hx(k256(out)) == block["hash"], f"header RLP of block {block['number']} does not hash to it"
    return out


def created(sender: str, nonce: int) -> str:
    return hx(k256(rlp.encode([unhex(sender), nonce]))[12:])


def exists(p: dict) -> bool:
    # Geth names an absent account's code and storage by zero hashes.
    return p["codeHash"] != ZERO32


def answer(p: dict, keys) -> dict:
    """The eth_getProof object as Willow sends it (state/mod.rs `proof_answer`)."""
    live = exists(p)
    return {
        "address": p["address"].lower(),
        "accountProof": p["accountProof"],
        "balance": q(int(p["balance"], 16)),
        "codeHash": p["codeHash"] if live else KECCAK_EMPTY,
        "nonce": q(int(p["nonce"], 16)),
        "storageHash": p["storageHash"] if live else EMPTY_ROOT,
        "storageProof": [
            {"key": k, "value": q(int(s["value"], 16)), "proof": s["proof"]}
            for k, s in zip(keys, p["storageProof"])
        ],
    }


def share(accounts: list) -> dict:
    """anchor-verify `share_nodes`: every node seen more than once moves to `nodes`."""
    seen = {}
    for a in accounts:
        for n in a["accountProof"] + [n for s in a["storageProof"] for n in s["proof"]]:
            seen[n] = seen.get(n, 0) + 1
    pool, index = [], {}

    def ref(n):
        if seen[n] < 2:
            return n
        if n not in index:
            index[n] = len(pool)
            pool.append(n)
        return f"#{index[n]}"

    out = []
    for a in accounts:
        a = dict(a)
        a["accountProof"] = [ref(n) for n in a["accountProof"]]
        a["storageProof"] = [dict(s, proof=[ref(n) for n in s["proof"]]) for s in a["storageProof"]]
        out.append(a)
    return {"accounts": out, "nodes": pool}


def named(chain_id: int, block: dict, rlp_bytes: bytes) -> dict:
    return {
        "chainId": q(chain_id),
        "blockNumber": block["number"],
        "blockHash": block["hash"],
        "header": hx(rlp_bytes),
    }


def write(path: pathlib.Path, value: dict):
    path.write_text(json.dumps(value, indent=1) + "\n")
    print(f"  wrote {path.relative_to(HERE)} ({path.stat().st_size} bytes)")


def state_fixture(node, chain_id, block, rlp_bytes, name, address, keys, at, code):
    n = block["number"]
    p = node.call("eth_getProof", [address, keys, n])
    params = [address, keys, at] + ([{"code": True}] if code else [])
    result = named(chain_id, block, rlp_bytes)
    result["account"] = answer(p, keys)
    code_hex = node.call("eth_getCode", [address, n])
    if code and code_hex != "0x":
        result["code"] = code_hex
    expect = {
        "eth_getBalance": node.call("eth_getBalance", [address, n]),
        "eth_getTransactionCount": node.call("eth_getTransactionCount", [address, n]),
        "eth_getCode": code_hex if code else None,
        "eth_getStorageAt": [node.call("eth_getStorageAt", [address, k, n]) for k in keys],
        "eth_getProof": result["account"],
    }
    write(HERE / "state" / f"{name}.json", {
        "request": {"jsonrpc": "2.0", "id": 1, "method": "willow_getStateProof", "params": params},
        "result": result,
        "expect": expect,
    })


def witness(node, block, want: dict) -> dict:
    """`want` (address → slot keys) proven at `block`, each account with its code."""
    n = block["number"]
    accounts, codes = [], {}
    for address in sorted(want):
        keys = sorted({"0x" + unhex(k).rjust(32, b"\0").hex() for k in want[address]})
        p = node.call("eth_getProof", [address, keys, n])
        a = answer(p, keys)
        accounts.append(a)
        if a["codeHash"] != KECCAK_EMPTY:
            code = node.call("eth_getCode", [address, n])
            assert hx(k256(unhex(code))) == a["codeHash"], f"code of {address}"
            codes[a["codeHash"]] = code
    w = share(accounts)
    w["codes"] = codes
    return w


def touched(node, block, call: dict, extra=()) -> dict:
    """What the call reads, as the node's access list says, with what the list leaves out."""
    n = block["number"]
    listed = node.raw("eth_createAccessList", [call, n])
    if "error" in listed:
        raise SystemExit(f"eth_createAccessList: {listed['error']}")
    want = {}
    for item in listed["result"]["accessList"]:
        want.setdefault(item["address"].lower(), set()).update(item["storageKeys"])
    sender = call.get("from", "0x" + "00" * 20).lower()
    for a in [sender, call.get("to"), block["miner"]] + list(extra):
        if a:
            want.setdefault(a.lower(), set())
    if call.get("to") is None:
        nonce = int(node.call("eth_getTransactionCount", [sender, n]), 16)
        want.setdefault(created(sender, nonce), set())
    return want


def call_fixture(node, version, block, rlp_bytes, name, call, at, hashes=()):
    """`willow_getCallProof` for `call`, its outcome from geth's eth_call. `hashes`: the blocks
    whose hash the call reads."""
    n = int(block["number"], 16)
    extra = {}
    if hashes:
        extra[HISTORY] = {q(h % RING) for h in hashes}
    want = touched(node, block, call, extra)
    for a, ks in extra.items():
        want.setdefault(a, set()).update(ks)
    reference = node.raw("eth_call", [call, block["number"]])
    if "result" in reference:
        outcome, expect = {"returned": reference["result"]}, {"result": reference["result"]}
    elif reference["error"].get("code") == 3:
        data = reference["error"]["data"]
        outcome, expect = {"reverted": data}, {"error": reference["error"]}
    else:
        raise SystemExit(f"{name}: eth_call {reference}: pick a call whose outcome is a result or a revert")
    result = named(1, block, rlp_bytes)
    result["witness"] = witness(node, block, want)
    result["result"] = outcome
    fixture = {
        "request": {"jsonrpc": "2.0", "id": 1, "method": "willow_getCallProof", "params": [call, at]},
        "result": result,
        "expect": expect,
        "reference": {"clientVersion": version, "eth_call": reference},
    }
    if hashes:
        fixture["reference"]["headers"] = {
            str(h): node.call("eth_getBlockByNumber", [q(h), False]) for h in hashes
        }
    write(HERE / "call" / f"call-{name}.json", fixture)


def estimate_fixture(node, version, block, rlp_bytes, name, call, at):
    want = touched(node, block, call)
    reference = node.raw("eth_estimateGas", [call, block["number"]])
    if "result" in reference:
        estimate, expect = {"gas": reference["result"]}, {"result": reference["result"]}
    elif reference["error"].get("code") == 3:
        estimate, expect = {"reverted": reference["error"]["data"]}, {"error": reference["error"]}
    else:
        estimate = {"error": reference["error"]["message"]}
        expect = {"error": {"code": reference["error"]["code"], "message": reference["error"]["message"]}}
    result = named(1, block, rlp_bytes)
    result["witness"] = witness(node, block, want)
    result["algorithm"] = "geth-v1.17.10-estimate"
    result["estimate"] = estimate
    write(HERE / "call" / f"estimate-{name}.json", {
        "request": {"jsonrpc": "2.0", "id": 1, "method": "willow_getEstimateGasProof", "params": [call, at]},
        "result": result,
        "expect": expect,
        "reference": {"clientVersion": version, "eth_estimateGas": reference},
    })


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--geth", default="https://ethereum.public.blockpi.network/v1/rpc/public")
    ap.add_argument("--base", default="https://base.drpc.org")
    args = ap.parse_args()
    (HERE / "state").mkdir(exist_ok=True)
    (HERE / "call").mkdir(exist_ok=True)

    eth = Node(args.geth)
    version = eth.call("web3_clientVersion", [])
    if not version.startswith("Geth/v1.17."):
        raise SystemExit(f"{args.geth} is {version}, not geth v1.17")
    head = int(eth.call("eth_blockNumber", []), 16)
    n = head - 3
    block = eth.call("eth_getBlockByNumber", [q(n), False])
    b = header_rlp(block)
    by_hash = {"blockHash": block["hash"]}
    print(f"Ethereum block {n} {block['hash']} on {version}")

    balance_slot = hx(k256(unhex(addr_word(HOLDER) + word(9))))
    state_fixture(eth, 1, block, b, "usdc-slots-code", USDC, [balance_slot, "0x0"], by_hash, True)
    state_fixture(eth, 1, block, b, "eoa", HOLDER, [], q(n), True)
    state_fixture(eth, 1, block, b, "absent", NOBODY, ["0x0"], by_hash, True)

    def data(selector, *words):
        return "0x" + selector + "".join(words)

    usdc_balance = data("70a08231", addr_word(HOLDER))
    call_fixture(eth, version, block, b, "usdc-balance", {"to": USDC, "data": usdc_balance}, by_hash)
    send_usdc = data("a9059cbb", addr_word(NOBODY2), word(1))
    call_fixture(eth, version, block, b, "revert", {"from": NOBODY, "to": USDC, "data": send_usdc}, by_hash)
    # Creation code returning 0x00 ‖ blockhash(n - 1) ‖ blockhash(n - 256).
    hashes_code = "0x600143034060015261010043034060215260416000f3"
    call_fixture(eth, version, block, b, "blockhash", {"data": hashes_code}, by_hash, hashes=(n - 1, n - 256))

    swap = data("7ff36ab5", word(0), word(0x80), addr_word(NOBODY2), word(0xFFFFFFFF), word(2),
                addr_word(WETH), addr_word(USDC))
    # Creation code deploying 0x602a60005260206000f3 (returns 42).
    deploy = "0x69602a60005260206000f3600052600a6016f3"
    estimates = [
        ("transfer", {"from": HOLDER, "to": NOBODY2, "value": "0x1"}),
        ("usdc-transfer", {"from": USDC_SENDER, "to": USDC, "data": send_usdc}),
        ("weth-deposit", {"from": HOLDER, "to": WETH, "value": q(10**15), "data": "0xd0e30db0"}),
        ("uniswap-swap", {"from": HOLDER, "to": UNISWAP_V2_ROUTER, "value": q(10**16), "data": swap}),
        ("creation", {"from": HOLDER, "data": deploy}),
        ("revert", {"from": NOBODY, "to": USDC, "data": send_usdc}),
        # Creation code that loops until it runs out of gas.
        ("out-of-gas", {"from": HOLDER, "data": "0x5b600056"}),
    ]
    for name, call in estimates:
        estimate_fixture(eth, version, block, b, name, call, by_hash)

    if args.base:
        base = Node(args.base)
        chain = int(base.call("eth_chainId", []), 16)
        assert chain == 8453, f"{args.base} is chain {chain}"
        # reth proves only a short window behind its head.
        bblock = base.call("eth_getBlockByNumber", ["latest", False])
        state_fixture(base, 8453, bblock, header_rlp(bblock), "base-usdc", BASE_USDC, ["0x1"],
                      {"blockHash": bblock["hash"]}, False)
    print(f"{eth.n} requests to {args.geth}")


if __name__ == "__main__":
    main()
