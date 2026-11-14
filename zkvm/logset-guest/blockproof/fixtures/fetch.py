#!/usr/bin/env python3
"""Raw public-node data for logset-blockproof's block and fee fixtures (I1, I4) and its live runs.

Every byte comes from free public endpoints (publicnode first). Writes into OUT (an empty
directory); `cargo test -p logset-blockproof -- --ignored make_fixtures` turns the `fixtures/`
part into the committed fixtures, and the live tests read the `live/` part.

  python3 -I fetch.py OUT fixtures      the fixture blocks, heads and fee histories
  python3 -I fetch.py OUT headers N     N+1 consecutive headers per chain (nextBaseFee runs)
  python3 -I fetch.py OUT opheads K     K heads per OP chain with the node's tip at that head
  python3 -I fetch.py OUT ethtips N     N+19 consecutive Ethereum blocks, compact (tip runs)
  python3 -I fetch.py OUT ethfull K      K+3 consecutive full Ethereum blocks (Rust against Python)
  python3 -I fetch.py OUT history       eth_feeHistory comparisons over longer ranges
"""
import json, os, sys, time, urllib.request

CHAINS = {
    "ethereum": (1, "https://ethereum-rpc.publicnode.com"),
    "op": (10, "https://optimism-rpc.publicnode.com"),
    "base": (8453, "https://base-rpc.publicnode.com"),
    "unichain": (130, "https://unichain-rpc.publicnode.com"),
    "ink": (57073, "https://ink-rpc.publicnode.com"),
    "arbitrum-one": (42161, "https://arbitrum-one-rpc.publicnode.com"),
    "robinhood": (4663, "https://robinhood-rpc.publicnode.com"),
}
OP = ("op", "base", "unichain", "ink")
NITRO = ("arbitrum-one", "robinhood")


def post(url, body, tries=6):
    data = json.dumps(body).encode()
    for i in range(tries):
        try:
            req = urllib.request.Request(url, data=data, headers={"content-type": "application/json", "user-agent": "curl/8.7"})
            with urllib.request.urlopen(req, timeout=90) as r:
                return json.loads(r.read())
        except Exception as e:  # noqa: BLE001 - retried, then raised
            if i == tries - 1:
                raise
            time.sleep(2 + 3 * i)


def call(url, method, params):
    r = post(url, {"jsonrpc": "2.0", "id": 1, "method": method, "params": params})
    if "error" in r:
        raise RuntimeError(f"{url} {method} {params}: {r['error']}")
    return r["result"]


def batch(url, calls, size=40):
    out = []
    for i in range(0, len(calls), size):
        part = calls[i:i + size]
        body = [{"jsonrpc": "2.0", "id": j, "method": m, "params": p} for j, (m, p) in enumerate(part)]
        for attempt in range(6):
            r = post(url, body)
            if isinstance(r, list) and len(r) == len(part) and all("result" in x for x in r):
                break
            time.sleep(2 + 3 * attempt)
        else:
            raise RuntimeError(f"{url}: batch failed: {str(r)[:300]}")
        out.extend(x["result"] for x in sorted(r, key=lambda x: x["id"]))
    return out


def save(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        json.dump(value, f, separators=(",", ":"))


def head(url):
    return int(call(url, "eth_blockNumber", []), 16)


def meta(name):
    cid, url = CHAINS[name]
    return {"chain": name, "chainId": cid, "endpoint": url, "client": call(url, "web3_clientVersion", [])}


def block_full(url, n):
    b = call(url, "eth_getBlockByNumber", [hex(n), True])
    return b, call(url, "eth_getBlockReceipts", [b["hash"]])


def at_same_head(url, methods, tries=200, max_size=None):
    """The node's responses to `methods`, read in one batch between two eth_blockNumber calls that agree.
    With `max_size`, only a head whose block is at most that many bytes (fixtures stay small)."""
    for _ in range(tries):
        body = [{"jsonrpc": "2.0", "id": 0, "method": "eth_blockNumber", "params": []}]
        body += [{"jsonrpc": "2.0", "id": i + 1, "method": m, "params": []} for i, m in enumerate(methods)]
        body += [{"jsonrpc": "2.0", "id": len(methods) + 1, "method": "eth_blockNumber", "params": []}]
        r = sorted(post(url, body), key=lambda x: x["id"])
        before, after = r[0].get("result"), r[-1].get("result")
        if before is not None and before == after:
            at = int(before, 16)
            if max_size is not None:
                size = int(call(url, "eth_getBlockByNumber", [hex(at), False])["size"], 16)
                if size > max_size:
                    time.sleep(0.5)
                    continue
            return at, {m: (x["result"] if "result" in x else {"error": x.get("error")}) for m, x in zip(methods, r[1:-1])}
        time.sleep(0.05)
    raise RuntimeError(f"{url}: no head held still (or was small) on any try")


def smallest_window(url, n_head, width, scan=240):
    calls = [("eth_getBlockByNumber", [hex(n), False]) for n in range(n_head - scan, n_head + 1)]
    sizes = {int(b["number"], 16): int(b["size"], 16) for b in batch(url, calls)}
    best = min(range(n_head - scan + width - 1, n_head + 1), key=lambda h: sum(sizes[h - k] for k in range(width)))
    return best, sum(sizes[best - k] for k in range(width))


def fixtures(out):
    d = os.path.join(out, "fixtures")
    for name in CHAINS:
        cid, url = CHAINS[name]
        if os.path.exists(f"{d}/{name}-history.json"):
            continue
        m = meta(name)
        methods = ["eth_maxPriorityFeePerGas", "eth_gasPrice", "eth_blobBaseFee"]
        if name == "ethereum":
            # The fee window H-3..H with the fewest bytes among recent blocks; then the node's
            # responses at its own head (reth: recorded, not compared).
            h, size = smallest_window(url, head(url) - 8, 4)
            print(f"ethereum: fee window ends {h}, {size} bytes")
            for n in range(h - 3, h + 1):
                b, r = block_full(url, n)
                save(f"{d}/ethereum-{n}.block.json", b)
                save(f"{d}/ethereum-{n}.receipts.json", r)
            at, got = at_same_head(url, methods)
            save(f"{d}/ethereum-head.json", {**m, "head": at, "responses": got, "fee": h})
            hist = {"count": 2, "newest": h, "percentiles": [10, 50, 90]}
        else:
            at, got = at_same_head(url, methods, max_size=24_000)
            b, r = block_full(url, at)
            save(f"{d}/{name}-{at}.block.json", b)
            save(f"{d}/{name}-{at}.receipts.json", r)
            save(f"{d}/{name}-head.json", {**m, "head": at, "responses": got, "fee": at})
            h = at
            hist = {"count": 4, "newest": h, "percentiles": [10, 50, 90]}
            if name in OP:
                for n in range(h - hist["count"] + 1, h):
                    b, r = block_full(url, n)
                    save(f"{d}/{name}-{n}.block.json", b)
                    save(f"{d}/{name}-{n}.receipts.json", r)
            print(f"{name}: head {at}: {got}")
        # Fee history over the same range, with and without percentiles, from the same node.
        rewards = call(url, "eth_feeHistory", [hex(hist["count"]), hex(h), hist["percentiles"]])
        plain = call(url, "eth_feeHistory", [hex(16), hex(h), []])
        headers = batch(url, [("eth_getBlockByNumber", [hex(n), False]) for n in range(h - 15, h + 1)])
        for x in headers:
            x.pop("transactions", None)
        save(f"{d}/{name}-history.json", {**m, "request": hist, "eth_feeHistory": rewards,
                                           "plain": {"count": 16, "newest": h, "eth_feeHistory": plain},
                                           "headers": headers})
    # Ethereum's first Shanghai block (withdrawals) and a pre-merge block with an uncle.
    url = CHAINS["ethereum"][1]
    for n in (17034870,):
        b = call(url, "eth_getBlockByNumber", [hex(n), True])
        save(f"{d}/ethereum-{n}.block.json", b)
    for n in (1500000,):
        b = call(url, "eth_getBlockByNumber", [hex(n), True])
        uncles = [call(url, "eth_getUncleByBlockHashAndIndex", [b["hash"], hex(i)]) for i in range(len(b["uncles"]))]
        save(f"{d}/ethereum-{n}.block.json", b)
        save(f"{d}/ethereum-{n}.uncles.json", uncles)
    save(f"{d}/meta.json", {k: meta(k) for k in CHAINS})


def headers(out, n):
    for name in ("ethereum",) + OP:
        cid, url = CHAINS[name]
        h = head(url) - 4
        got = batch(url, [("eth_getBlockByNumber", [hex(k), False]) for k in range(h - n, h + 1)], size=50)
        with open(os.path.join(out, "live", f"{name}-headers.jsonl"), "w") as f:
            for x in got:
                x.pop("transactions", None)
                f.write(json.dumps(x, separators=(",", ":")) + "\n")
        print(f"{name}: headers {h - n}..={h}")


def opheads(out, k):
    for name in OP:
        cid, url = CHAINS[name]
        for i in range(k):
            at, got = at_same_head(url, ["eth_maxPriorityFeePerGas", "eth_gasPrice"])
            b, r = block_full(url, at)
            save(os.path.join(out, "live", "opheads", f"{name}-{at}.json"),
                 {"chainId": cid, "head": at, "responses": got, "block": b, "receipts": r})
            time.sleep(1.0)
        print(f"{name}: {k} heads")


def ethtips(out, n):
    """Per block: number, miner, baseFee, and each transaction's (type, from, gasPrice, maxFee, maxTip)."""
    url = CHAINS["ethereum"][1]
    h = head(url) - 4
    path = os.path.join(out, "live", "ethtips.jsonl")
    with open(path, "w") as f:
        for k in range(h - n - 18, h + 1):
            b = call(url, "eth_getBlockByNumber", [hex(k), True])
            txs = [[t["type"], t["from"], t.get("gasPrice"), t.get("maxFeePerGas"), t.get("maxPriorityFeePerGas")]
                   for t in b["transactions"]]
            f.write(json.dumps({"n": k, "hash": b["hash"], "miner": b["miner"], "base": b["baseFeePerGas"], "txs": txs},
                               separators=(",", ":")) + "\n")
    print(f"ethereum: compact blocks {h - n - 18}..={h}")


def ethfull(out, k):
    """K+3 consecutive full Ethereum blocks: K heads of the willow-fee-v1 tip, in Rust and here."""
    url = CHAINS["ethereum"][1]
    h = head(url) - 4
    for n in range(h - k - 2, h + 1):
        save(os.path.join(out, "live", "ethfull", f"{n}.block.json"), call(url, "eth_getBlockByNumber", [hex(n), True]))
    print(f"ethereum: full blocks {h - k - 2}..={h}")


def history(out):
    for name in CHAINS:
        cid, url = CHAINS[name]
        h = head(url) - 4
        plain = call(url, "eth_feeHistory", [hex(1024), hex(h), []])
        oldest = int(plain["oldestBlock"], 16)
        hs = batch(url, [("eth_getBlockByNumber", [hex(k), False]) for k in range(oldest, h + 1)], size=50)
        for x in hs:
            x.pop("transactions", None)
        latest = call(url, "eth_feeHistory", [hex(8), "latest", []])
        save(os.path.join(out, "live", "history", f"{name}.json"),
             {"chainId": cid, "newest": h, "eth_feeHistory": plain, "headers": hs, "latest": latest})
        if name in OP or name == "ethereum":
            count = 16 if name in OP else 4
            rew = call(url, "eth_feeHistory", [hex(count), hex(h), [5, 25, 50, 75, 95]])
            blocks = [block_full(url, k) for k in range(h - count + 1, h + 1)]
            save(os.path.join(out, "live", "history", f"{name}-rewards.json"),
                 {"chainId": cid, "newest": h, "count": count, "percentiles": [5, 25, 50, 75, 95],
                  "eth_feeHistory": rew, "blocks": [b for b, _ in blocks], "receipts": [r for _, r in blocks]})
        print(f"{name}: history to {h}")


if __name__ == "__main__":
    out, what = sys.argv[1], sys.argv[2]
    os.makedirs(os.path.join(out, "live"), exist_ok=True)
    if what == "fixtures":
        fixtures(out)
    elif what == "headers":
        headers(out, int(sys.argv[3]))
    elif what == "opheads":
        opheads(out, int(sys.argv[3]))
    elif what == "ethtips":
        ethtips(out, int(sys.argv[3]))
    elif what == "ethfull":
        ethfull(out, int(sys.argv[3]))
    elif what == "history":
        history(out)
