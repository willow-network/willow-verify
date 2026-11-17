"""A stand-in for api.willow.tech: serves the built page at /verify/ with the
same headers as the live server, and JSON-RPC from a saved answer and proof store.

    python3 mock_rpc.py <dist dir> <data dir> [port]

<data dir> holds answer.json (a chain-1 willow_getLogsProof result whose parts are all one-block
parts), store/ and optionally tampered.json, forged-pv.json. Transaction and receipt proofs come
from items/*.json next to this file (saved from the live log RPC: the request, the proof, Willow's
plain answers and the block).
RPC paths pick the behaviour, on any served chain id (the saved log answer is chain 1's):
  /v1/rpc/<chain>/<mode>
                     mode: good | tampered | forged | lying (eth_getLogs drops a log) | ahead (tip past the proofs)
                     | noproofs (proof methods not served) | badproof (willow_getProof sends a corrupted proof)
                     | ratelimited (HTTP 429, as the server's per-key limit) | toolarge (-32002 record budget)
                     | tail (-32002 "not in a final 64-block segment" for ranges over one block)
                     | tailonce (-32002 "once proven, ... would be outside a final 64-block segment")
                     | joblimit (-32002 the key's daily proving cap) | queuelimit (-32002 too many blocks to queue)
                     | unavailable (on-demand proving unavailable for ranges past the saved blocks)
                     Proof jobs: the first willow_getLogsProof answers "proof queued" (error.data with the job),
                     and each willow_getProofJob moves the job on; mode.<tag> keeps one job per tag:
                     | queue (queued, then half proven, then done: the same request is then answered)
                     | queuefail (fails on the second poll) | queueslow (never finishes)
                     | queueother (willow_getProofJob does not show the job, as for another key's; the third
                     willow_getLogsProof is answered)
                     | big, big20 (the answer names 30 or 20 more proofs than the store has; any fromBlock)
                     | oomid (proof ids end in " out of memory: allocation failed", and the proofs are corrupted)
                     | badhex (willow_getProof sends "0xzz") | noparts (willow_getLogsProof sends no parts)
                     | badrecords (a block's records are not hex) | logsnotarray (eth_getLogs answers an object)
                     | leaftail (-32002 a fast chain's "each 1024-block leaf is proven on its own" for ranges over one block)
                     | leafonce (-32002 "once proven, 3 leaves of 1024 blocks ... would be outside a final 2048-block segment")
                     | waiting (the first willow_getLogsProof answers a fast chain's "block is in a leaf that ends at
                     block N, which the chain has not made yet" with error.data status "waiting"; mode.<tag> as for queue)
                     Transaction and receipt proofs (willow_getTransactionProof, willow_getReceiptProof,
                     willow_getBlockReceiptsProof and the plain eth_getTransactionByHash, eth_getTransactionReceipt,
                     eth_getBlockReceipts, eth_getBlockByHash, eth_getBlockByNumber) answer from items/ for the saved
                     requests, and refuse others as the log store does (not in the full-block tier, outside the indexed
                     range, no such index): good | tampered (a trie node or item flipped) | lying (the plain answer has
                     another status) | absent (the plain answer is null)
                     A "+<twist>" after the mode nests a value 300,000 levels deep (more than JSON.stringify
                     or toString handle in any browser): +deepproof (a field "x" first in the willow_getLogsProof
                     result) | +deeplogs (a first eth_getLogs entry) | +deeptip (eth_blockNumber's result)
                     | +deeperror (eth_getLogs answers an error whose message is that value)
  /node/<mode>, /node/<chain>/<mode>
                     the user's node for the last-block check (chain 1 unless given): good | wrong | chain10 | unfinal
                     (its finalized block is before the saved ones), and +deepchain (eth_chainId's result nested deep);
                     it knows the saved blocks and the items' blocks, and calls every token "TKN" with 6 decimals
                     Ethereum's light client and block anchors (willow_lightClientBootstrap, willow_lightClientUpdates,
                     willow_getBlockAnchor) answer from anchor/ in every mode; these modes change only them:
                     | anchorforged (the anchor's validator signature changed) | anchorbadupdate (a period update's next
                     committee changed) | anchorgloas (the anchor named as from a later fork) | anchorsepolia (the anchor
                     signed by Sepolia's validators, from anchor/sepolia.json) | anchorwait (the first anchor answer is
                     "waiting": the block is newer than the light client's head) | anchornone (no anchor methods, as a
                     server without the anchor service)
  /v1/rpc/<chain>/proxy
                     forwards to $WILLOW_UPSTREAM (a real log RPC URL, its chain id replaced by <chain>), so the page
                     runs same-origin against live data as it will at /verify/
  /v1/rpc/<chain>/public
                     the keyless path a link uses: proxy when $WILLOW_UPSTREAM is set, else good
  GET /v1/rpc/health/proofs  the proof status of a server without proof jobs: on chain 1, proven from SEG_END - 63
                     to the last saved block, the newest final segment ending at SEG_END (the 64-block window
                     before the first saved block's); the other chains, nothing proven yet, with their geometry;
                     proxied to the upstream's when $WILLOW_UPSTREAM is set
  GET /v1/rpc/health the log store's status: each chain's tip, 10 blocks past its saved items block (proxied too)
"""

import copy
import json
import os
import random
import re
import sys
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DIST, DATA = sys.argv[1], sys.argv[2]
PORT = int(sys.argv[3]) if len(sys.argv) > 3 else 8787

HEADERS_STATIC = {
    "Content-Security-Policy": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; "
    "connect-src 'self' https: http://127.0.0.1:*; img-src 'self' data:; style-src 'self'; "
    "worker-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
}
HASHED = re.compile(r"[a-z]+\.[0-9a-f]{12}\.(js|css|wasm|svg|woff2)")
TYPES = {".html": "text/html; charset=utf-8", ".js": "application/javascript", ".css": "text/css",
         ".wasm": "application/wasm", ".svg": "image/svg+xml", ".txt": "text/plain; charset=utf-8",
         ".woff2": "font/woff2"}


def load(name):
    p = os.path.join(DATA, name)
    return json.load(open(p)) if os.path.exists(p) else None


ANSWERS = {"good": load("answer.json"), "tampered": load("tampered.json"), "forged": load("forged-pv.json")}
# Without a saved answer (proxy-only use) the mock modes answer nothing.
GOOD = ANSWERS["good"] or {"fromBlock": "0x1", "toBlock": "0x0", "parts": []}
FROM, TO = int(GOOD["fromBlock"], 16), int(GOOD["toBlock"], 16)
HASHES = {int(p["proof"].split("/")[1].split("-")[0]): "0x" + p["proof"].split("-")[1] for p in GOOD["parts"]}
OOM = " out of memory: allocation failed"
DEEP = "[" * 300000 + "]" * 300000
# twist -> the method whose result it nests deep.
DEEPEN = {"deepproof": "willow_getLogsProof", "deeplogs": "eth_getLogs", "deeptip": "eth_blockNumber", "deepchain": "eth_chainId"}


def deepen(text):
    """`text` (JSON) with DEEP first: a field "x" of an object, the first entry of an array, or in place of anything else."""
    if text[0] in "{[":
        close = "}" if text[0] == "{" else "]"
        first = '"x":' + DEEP if text[0] == "{" else DEEP
        return text[0] + first + ("" if text[1:].strip() == close else ",") + text[1:]
    return DEEP


def decode_records(block, hexrec):
    """Plain eth_getLogs entries from a block's record stream (tx 4 | log 4 | emitter 20 | n 1 | topics | len 4 | data)."""
    r, out, p = bytes.fromhex(hexrec[2:]), [], 0
    while p < len(r):
        tx, log = int.from_bytes(r[p:p + 4], "big"), int.from_bytes(r[p + 4:p + 8], "big")
        emitter, n = r[p + 8:p + 28], r[p + 28]
        topics = [r[p + 29 + 32 * i:p + 61 + 32 * i] for i in range(n)]
        q = p + 29 + 32 * n
        dl = int.from_bytes(r[q:q + 4], "big")
        data = r[q + 4:q + 4 + dl]
        p = q + 4 + dl
        out.append({
            "address": "0x" + emitter.hex(), "topics": ["0x" + t.hex() for t in topics], "data": "0x" + data.hex(),
            "blockNumber": hex(block), "transactionHash": "0x" + "ab" * 32, "transactionIndex": hex(tx),
            "blockHash": HASHES[block], "logIndex": hex(log), "removed": False,
        })
    return out


# Transaction and receipt proofs: (chain, method, params as JSON) -> result, from items/*.json; each
# chain's saved blocks (number -> hash) and the full blocks saved with them.
HERE = os.path.dirname(os.path.abspath(__file__))
ITEMS, ITEM_BLOCKS, FULL_BLOCKS = {}, {}, {}
for _name in sorted(os.listdir(os.path.join(HERE, "items"))):
    _c = json.load(open(os.path.join(HERE, "items", _name)))
    _chain = _c["chainId"]
    for _call in [_c["proof"]] + _c["answers"]:
        ITEMS[(_chain, _call["method"], json.dumps(_call["params"]))] = _call["result"]
        if _call["method"] == "eth_getBlockByHash":
            FULL_BLOCKS[(_chain, int(_call["result"]["number"], 16))] = _call["result"]
    ITEM_BLOCKS.setdefault(_chain, {})[int(_c["block"]["number"], 16)] = _c["block"]["hash"]
ITEM_METHODS = {"willow_getTransactionProof", "willow_getReceiptProof", "willow_getBlockReceiptsProof",
                "eth_getTransactionByHash", "eth_getTransactionReceipt", "eth_getBlockReceipts", "eth_getBlockByHash"}
PLAIN_ITEM_METHODS = ITEM_METHODS - {"willow_getTransactionProof", "willow_getReceiptProof", "willow_getBlockReceiptsProof"}
# Ethereum's light client data and block anchors (test/save_anchor_data.py): the checkpoint's bootstrap,
# the updates after it, and per block its willow_getBlockAnchor result (history-<n>.json, beacon-<n>.json
# on Ethereum, l2-<chain>-<n>.json on the L2s), keyed by (chain, block).
ANCHOR_DIR = os.path.join(HERE, "anchor")
ANCHORS = {}
for _name in sorted(os.listdir(ANCHOR_DIR)):
    _m = re.fullmatch(r"(?:history|beacon)-(\d+)\.json", _name)
    _l2 = re.fullmatch(r"l2-(\d+)-(\d+)\.json", _name)
    if _m:
        ANCHORS[(1, int(_m.group(1)))] = json.load(open(os.path.join(ANCHOR_DIR, _name)))
    elif _l2:
        ANCHORS[(int(_l2.group(1)), int(_l2.group(2)))] = json.load(open(os.path.join(ANCHOR_DIR, _name)))
CHECKPOINT = json.load(open(os.path.join(ANCHOR_DIR, "checkpoint.json")))
BOOTSTRAP = json.load(open(os.path.join(ANCHOR_DIR, "bootstrap.json")))
UPDATES = json.load(open(os.path.join(ANCHOR_DIR, "updates.json")))
SEPOLIA = json.load(open(os.path.join(ANCHOR_DIR, "sepolia.json")))
ANCHOR_METHODS = {"willow_lightClientBootstrap", "willow_lightClientUpdates", "willow_getBlockAnchor"}
ANCHOR_ASKS = {}


def anchor_rpc(chain, mode, key, method, params):
    """Ethereum's light client and block anchors, from anchor/, changed as the mode says."""
    if mode == "anchornone":
        raise RpcError(-32601, f"the method {method} does not exist/is not available")
    if method == "willow_lightClientBootstrap":
        if params != [CHECKPOINT["root"]]:
            raise RpcError(-32000, "not found: no bootstrap for that block root")
        return BOOTSTRAP
    if method == "willow_lightClientUpdates":
        start, count = int(params[0]), int(params[1])
        out = [u for u in UPDATES if start <= int(u["data"]["attested_header"]["beacon"]["slot"]) // 8192 < start + count]
        if mode == "anchorbadupdate" and out:
            out = copy.deepcopy(out)
            keys = out[0]["data"]["next_sync_committee"]["pubkeys"]
            keys[5] = flip(keys[5], 40)
        return out
    n = int(params[0], 16) if params and isinstance(params[0], str) and params[0].startswith("0x") else None
    if mode == "anchorwait":
        with JOBS_LOCK:
            ANCHOR_ASKS[key] = ANCHOR_ASKS.get(key, 0) + 1
            first = ANCHOR_ASKS[key] == 1
        if first:
            raise RpcError(-32000, f"block {n} is newer than the newest block Ethereum's light client has verified",
                           {"status": "waiting", "retryAfterSeconds": 4})
    # L2 anchors only where a case asks for them: elsewhere the page's clock decides whether they are
    # fresh, and the L2 cases written for public nodes should not change with it.
    a = ANCHORS.get((chain, n)) if chain == 1 or mode in ("anchorl2", "anchorforged") else None
    if a is None:
        raise RpcError(-32000, f"not found in this store: no anchor for block {n}")
    a = copy.deepcopy(a)
    if mode == "anchorforged":
        agg = a["update"]["data"]["sync_aggregate"]
        agg["sync_committee_signature"] = SEPOLIA["update"]["data"]["sync_aggregate"]["sync_committee_signature"]
    if mode == "anchorgloas":
        a["update"]["version"] = "gloas"
    if mode == "anchorsepolia":
        # Named as the fork mainnet had at its slots, so only the validators differ.
        a["update"] = {**SEPOLIA["update"], "version": "deneb"}
    return a


# Each served chain's leaf and segment sizes, as its proof status reports them.
GEOMETRY = {1: (1, 64), 8453: (64, 128), 42161: (2048, 4096), 4663: (1024, 2048), 10: (128, 256), 130: (1024, 2048),
            57073: (1024, 2048)}


def item_tip(chain):
    """The newest block of a chain's log store: 10 past its newest saved items block."""
    return max(ITEM_BLOCKS.get(chain, {0: None})) + 10


def flip(h, at):
    """`h` (0x hex) with one digit changed."""
    i = 2 + at % (len(h) - 2)
    return h[:i] + ("1" if h[i] == "0" else "0") + h[i + 1:]


def items_rpc(chain, mode, method, params):
    """A transaction or receipt proof, or a plain answer, from items/; else refused as the log store does."""
    if method == "eth_getBlockByNumber":
        b = FULL_BLOCKS.get((chain, int(params[0], 16))) if params and isinstance(params[0], str) and params[0].startswith("0x") else None
        if b is None:
            raise RpcError(-32000, "not found in this store: block is not in the full-block tier")
        return {**b, "transactions": [t["hash"] for t in b["transactions"]]} if params[1] is False else b
    r = ITEMS.get((chain, method, json.dumps(params)))
    if r is None:
        known = ITEM_BLOCKS.get(chain, {})
        lo, hi = (min(known) - 100, item_tip(chain)) if known else (1, 2)
        first = params[0] if params else None
        if isinstance(first, str) and len(first) == 66 and len(params) == 1 and "Block" not in method:
            raise RpcError(-32000, f"not found in this store: transaction {first} is not in the full-block tier")
        if isinstance(first, str) and first.startswith("0x") and len(first) < 20:
            n = int(first, 16)
            b = FULL_BLOCKS.get((chain, n))
            if b is not None and len(params) == 2:
                raise RpcError(-32602, f"invalid params: block {n} has {len(b['transactions'])} transactions; there is no "
                                       f"transaction {int(params[1], 16)}")
            if not lo <= n <= hi:
                raise RpcError(-32000, f"not found in this store: blocks {n}-{n} are outside the indexed range {lo}-{hi}")
        raise RpcError(-32000, f"not found in this store: {method} {json.dumps(params)} is not saved in this mock")
    r = copy.deepcopy(r)
    if mode == "tampered" and method.startswith("willow_"):
        if r.get("receipts"):
            r["receipts"][-1] = flip(r["receipts"][-1], 20)
        else:
            item = (r.get("receiptProofs") or r["transactionProofs"])[-1]
            item["proof"][0] = flip(item["proof"][0], 40)
    if method in PLAIN_ITEM_METHODS:
        if mode == "absent":
            return None
        if mode == "lying":
            target = r[0] if isinstance(r, list) else r["transactions"][0] if method == "eth_getBlockByHash" else r
            if "status" in target:
                target["status"] = "0x0" if target["status"] == "0x1" else "0x1"
            else:
                target["value"] = hex(int(target["value"], 16) + 1)
    return r


# The newest final segment in the proof status: the 64-block window before the first saved block's.
SEG_END = FROM // 64 * 64 - 1
# Proof jobs by endpoint mode (with its tag), and the modes that queue them.
JOBS = {}
JOBS_LOCK = threading.Lock()
QUEUE_MODES = ("queue", "queuefail", "queueslow", "queueother")
ALL_LOGS = [l for p in GOOD["parts"] for l in decode_records(int(p["proof"].split("/")[1].split("-")[0]), p["records"])]


def matches(f, l):
    b = int(l["blockNumber"], 16)
    if not (int(f["fromBlock"], 16) <= b <= int(f["toBlock"], 16)):
        return False
    a = f.get("address")
    if a is not None:
        a = [a] if isinstance(a, str) else a
        if l["address"] not in [x.lower() for x in a]:
            return False
    for i, want in enumerate(f.get("topics") or []):
        if want is None or want == []:
            continue
        want = [want] if isinstance(want, str) else want
        if i >= len(l["topics"]) or l["topics"][i] not in [w.lower() for w in want]:
            return False
    return True


class RpcError(Exception):
    def __init__(self, code, msg, data=None):
        self.code, self.msg, self.data = code, msg, data


def job_state(mode, job):
    """(status, proved blocks, error) of a job after its polls so far."""
    total, polls = job["to"] - job["from"] + 1, job["polls"]
    if polls <= 1:
        return "queued", 0, None
    if mode == "queuefail":
        return "failed", 0, "the prover stopped: out of GPU memory"
    if mode == "queueslow":
        return "running", min(polls - 1, total - 1), None
    if polls == 2:
        return "running", (total + 1) // 2, None
    return "done", total, None


def queue(key, mode, lo, hi):
    """Raises "proof queued" for blocks lo..hi until the job for `key` is done (queueother: the third ask)."""
    with JOBS_LOCK:
        job = JOBS.setdefault(key, {"id": f"j-{int(time.time())}-{random.getrandbits(32):08x}", "from": lo, "to": hi,
                                    "polls": 0, "asks": 0, "created": int(time.time())})
        job["asks"] += 1
        status = job_state(mode, job)[0]
        if status == "done" or (mode == "queueother" and job["asks"] >= 3):
            return
        data = {"status": "running" if status == "running" else "queued", "jobId": job["id"], "jobIds": [job["id"]],
                "from": job["from"], "to": job["to"], "estimatedSeconds": 240, "retryAfterSeconds": 1}
    raise RpcError(-32000, f"proof not available yet: queued for blocks {job['from']}-{job['to']}; retry in about 4 minutes",
                   data)


def proof_job(key, mode, params):
    jid = params[0] if params and isinstance(params[0], str) else None
    with JOBS_LOCK:
        job = JOBS.get(key)
        if jid is None or job is None or job["id"] != jid or mode == "queueother":
            raise RpcError(-32000, f"proof job {jid} not found")
        job["polls"] += 1
        status, proved, error = job_state(mode, job)
        return {"id": job["id"], "chain": 1, "from": job["from"], "to": job["to"], "createdAt": job["created"],
                "status": status, "provedBlocks": proved, "totalBlocks": job["to"] - job["from"] + 1,
                "startedAt": job["created"] + 1 if status != "queued" else None,
                "finishedAt": job["created"] + 2 if status in ("done", "failed") else None, "error": error}


def rpc(chain, key, method, params):
    mode = key.split(".")[0]
    if method in ANCHOR_METHODS:
        return anchor_rpc(chain, mode, key, method, params)
    if mode.startswith("anchor"):
        mode = "good"
    if method == "eth_chainId":
        return hex(chain)
    if method in ITEM_METHODS or (method == "eth_getBlockByNumber" and chain in ITEM_BLOCKS):
        return items_rpc(chain, mode, method, params)
    if method == "eth_blockNumber":
        if chain != 1:
            return hex(item_tip(chain))
        return hex(TO + 1) if mode == "ahead" else hex(TO)
    if method in ("willow_getLogsProof", "willow_getProof") and mode == "noproofs":
        raise RpcError(-32000, "not found in this store: proofs for chain 1")
    if method == "willow_getLogsProof":
        f = params[0]
        lo, hi = int(f["fromBlock"], 16), int(f["toBlock"], 16)
        if mode == "toolarge":
            raise RpcError(-32002, "range too large: the proof would carry more than 10000 records (every log of each segment "
                                   "it spans with the filter's address and topic0; the client applies the other topics); "
                                   "narrow the block range")
        if mode == "tail" and hi > lo:
            raise RpcError(-32002, f"range too large: blocks from {lo + 1} are not in a final 64-block segment, so each is "
                                   "proven on its own with every log it holds, and this answer would need more than 256 such blocks "
                                   "or 16 MiB of their logs; narrow the range; on this chain, blocks with no proof yet are "
                                   "proven on request, each complete 64-block window (starting at a multiple of 64) as "
                                   "one segment")
        if mode == "tailonce":
            raise RpcError(-32002, "range too large: once proven, 300 blocks of this range would be outside a final 64-block "
                                   "segment, "
                                   "more than the 256 one answer can carry; narrow the range")
        if mode == "joblimit":
            raise RpcError(-32002, "proof jobs not available for this key until 00:00 UTC: it has queued 20000 blocks for "
                                   "proving today and 2 more would pass its daily cap of 20000")
        if mode == "queuelimit":
            raise RpcError(-32002, "range too large: 2100 blocks in this range have no proof yet; one request can queue at "
                                   "most 2048; "
                                   "narrow the range")
        if mode == "unavailable" and (lo < FROM or hi > TO):
            raise RpcError(-32000, "on-demand proving is not available right now")
        if mode == "leaftail" and hi > lo:
            raise RpcError(-32002, f"range too large: blocks from {lo + 1} are not in a final 2048-block segment, so each "
                                   "1024-block leaf is proven on its own with every log it holds, and this answer would need "
                                   "more than 1 such leaves or 16 MiB of their logs; narrow the range")
        if mode == "leafonce":
            raise RpcError(-32002, "range too large: once proven, 3 leaves of 1024 blocks in this range would be outside a "
                                   "final 2048-block segment, more than the 1 one answer can carry; narrow the range")
        if mode == "waiting":
            with JOBS_LOCK:
                job = JOBS.setdefault(key, {"asks": 0})
                job["asks"] += 1
                first = job["asks"] == 1
            if first:
                raise RpcError(-32000, f"proof not available yet: block {hi} is in a leaf that ends at block {hi + 5}, which "
                                       "the chain has not made yet; retry in about 1 minute",
                               {"status": "waiting", "from": hi, "segmentEnd": hi + 5, "retryAfterSeconds": 1})
        if mode in QUEUE_MODES:
            queue(key, mode, lo, hi)
        if hi > TO:
            raise RpcError(-32000, f"proof not found for block {max(lo, TO + 1)}: it has no proof yet")
        if lo < FROM and not mode.startswith("big"):
            raise RpcError(-32000, f"proof not found for block {lo}: it has no proof yet")
        a = dict(ANSWERS.get(mode) or GOOD)
        a["parts"] = [p for p in a["parts"] if lo <= int(p["proof"].split("/")[1].split("-")[0]) <= hi]
        if mode.startswith("big"):
            extra = int(mode[3:] or 30)
            a["parts"] = a["parts"] + [{**GOOD["parts"][0], "proof": f"leaf/{lo}-{i:064x}"} for i in range(extra)]
        if mode == "oomid":
            a["parts"] = [{**p, "proof": p["proof"] + OOM} for p in a["parts"]]
        if mode == "badrecords":
            a["parts"] = [{**p, "records": "0xzz"} for p in a["parts"]]
        a["fromBlock"], a["toBlock"] = hex(lo), hex(hi)
        if mode == "noparts":
            del a["parts"]
        return a
    if method == "willow_getProofJob":
        return proof_job(key, mode, params)
    if method == "willow_getProof":
        pid = params[0][:-len(OOM)] if mode == "oomid" and params[0].endswith(OOM) else params[0]
        if not re.fullmatch(r"leaf/\d+-[0-9a-f]{64}|l1/\d+-\d+", pid):
            raise RpcError(-32602, "bad proof id")
        path = os.path.join(DATA, "store", pid + ".proof")
        if not os.path.exists(path):
            raise RpcError(-32000, f"stored proof {pid} not found")
        if mode == "badhex":
            return {"id": params[0], "proof": "0xzz"}
        b = bytearray(open(path, "rb").read())
        if mode in ("badproof", "oomid"):
            b[len(b) // 2] ^= 1
        return {"id": params[0], "proof": "0x" + b.hex()}
    if method == "eth_getLogs":
        if mode == "logsnotarray":
            return {"logs": []}
        logs = [l for l in ALL_LOGS if matches(params[0], l)]
        return logs[1:] if mode == "lying" else logs
    raise RpcError(-32601, f"the method {method} does not exist/is not available")


def node(chain, mode, method, params):
    if method == "eth_chainId":
        return "0xa" if mode == "chain10" else hex(chain)
    known = sorted(HASHES if chain == 1 else ITEM_BLOCKS.get(chain, {}))
    if method == "eth_getBlockByNumber" and params[0] == "finalized":
        # Past every saved block, or (unfinal) before them.
        n = known[0] - 1 if mode == "unfinal" else known[-1] + 100
        return {"number": hex(n), "hash": "0x" + "11" * 32}
    if method == "eth_call":
        # Every token is "TKN" with 6 decimals: symbol() and decimals().
        data = params[0].get("data")
        if data == "0x95d89b41":
            return "0x" + "%064x" % 32 + "%064x" % 3 + b"TKN".hex().ljust(64, "0")
        if data == "0x313ce567":
            return "0x" + "%064x" % 6
        raise RpcError(-32000, "execution reverted")
    if method == "eth_getBlockByNumber":
        n = int(params[0], 16)
        h = HASHES.get(n) if chain == 1 and n in HASHES else ITEM_BLOCKS.get(chain, {}).get(n)
        if h is None:
            return None
        return {"number": params[0], "hash": "0x" + "00" * 32 if mode == "wrong" else h}
    raise RpcError(-32601, "not served")


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def send(self, code, body, ctype, extra=None):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = self.path.split("?")[0]
        if path in ("/v1/rpc/health/proofs", "/v1/rpc/health"):
            up = os.environ.get("WILLOW_UPSTREAM")
            if up:
                url = re.sub(r"/rpc/.*$", path[len("/v1"):], up)
                try:
                    with urllib.request.urlopen(urllib.request.Request(url, headers={"user-agent": "willow-verify-test"}), timeout=30) as r:
                        return self.send(r.status, r.read(), "application/json", {"Cache-Control": "no-store"})
                except urllib.error.HTTPError as e:
                    return self.send(e.code, e.read(), "application/json", {"Cache-Control": "no-store"})
            if path == "/v1/rpc/health":
                body = {"healthy": True, "chains": {str(c): {"tip": item_tip(c), "healthy": True} for c in GEOMETRY}}
                return self.send(200, json.dumps(body).encode(), "application/json", {"Cache-Control": "no-store"})
            chain = {"tip": TO + 1, "firstProvenBlock": SEG_END - 63, "newestProvenBlock": TO,
                     "provenRanges": [[SEG_END - 63, TO]], "newestProvenNearTip": TO, "lastSegmentEnd": SEG_END,
                     "leafBlocks": 1, "windowBlocks": 64, "healthy": True}
            chains = {str(c): {"tip": item_tip(c), "firstProvenBlock": None, "newestProvenBlock": None, "provenRanges": [],
                               "newestProvenNearTip": None, "lastSegmentEnd": None, "leafBlocks": g[0], "windowBlocks": g[1],
                               "healthy": True} for c, g in GEOMETRY.items() if c != 1}
            body = {"healthy": True, "chains": {"1": chain, **chains}}
            return self.send(200, json.dumps(body).encode(), "application/json", {"Cache-Control": "no-store"})
        if path == "/verify":
            self.send_response(301)
            self.send_header("Location", "/verify/")
            self.end_headers()
            return
        if not path.startswith("/verify/"):
            return self.send(404, b"not found", "text/plain")
        name = path[len("/verify/"):] or "index.html"
        full = os.path.join(DIST, name)
        if "/" in name or not os.path.isfile(full):
            return self.send(404, b"not found", "text/plain")
        ext = os.path.splitext(name)[1]
        # Same rule as the live server: only content-hashed names are immutable.
        cache = "public, max-age=31536000, immutable" if HASHED.fullmatch(name) else "no-cache"
        self.send(200, open(full, "rb").read(), TYPES.get(ext, "application/octet-stream"),
                  {**HEADERS_STATIC, "Cache-Control": cache})

    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        parts = self.path.strip("/").split("/")
        # The keyless path is the live one's when an upstream is set, else the good mock.
        live = parts[3:] == ["public"] and os.environ.get("WILLOW_UPSTREAM")
        if len(parts) == 4 and parts[:2] == ["v1", "rpc"] and parts[2].isdigit() and (parts[3] == "proxy" or live):
            upstream = re.sub(r"/rpc/\d+/", f"/rpc/{parts[2]}/", os.environ["WILLOW_UPSTREAM"])
            req = urllib.request.Request(upstream, data=raw,
                                         headers={"content-type": "application/json", "user-agent": "willow-verify-test"})
            try:
                with urllib.request.urlopen(req, timeout=120) as r:
                    return self.send(r.status, r.read(), "application/json")
            except urllib.error.HTTPError as e:
                return self.send(e.code, e.read(), "text/plain")
        body = json.loads(raw or b"{}")
        twist = ""
        if "+" in parts[-1]:
            parts[-1], twist = parts[-1].split("+", 1)
        method = body.get("method")
        if twist == "deeperror" and method == "eth_getLogs":
            out = '{"jsonrpc":"2.0","id":%s,"error":{"code":-32000,"message":%s}}' % (json.dumps(body.get("id")), DEEP)
            return self.send(200, out.encode(), "application/json")
        willow = len(parts) == 4 and parts[:2] == ["v1", "rpc"] and parts[2].isdigit()
        if willow and parts[3] == "ratelimited":
            out = {"jsonrpc": "2.0", "id": None, "error": {"code": -32005, "message": "rate limit exceeded"}}
            return self.send(429, json.dumps(out).encode(), "application/json")
        try:
            if willow:
                result = rpc(int(parts[2]), "good" if parts[3] == "public" else parts[3], method, body.get("params", []))
            elif parts[0] == "node" and len(parts) in (2, 3):
                chain = int(parts[1]) if len(parts) == 3 else 1
                result = node(chain, parts[-1], method, body.get("params", []))
            else:
                return self.send(404, b'{"error":"not found"}', "application/json")
        except RpcError as e:
            error = {"code": e.code, "message": e.msg}
            if e.data is not None:
                error["data"] = e.data
            out = {"jsonrpc": "2.0", "id": body.get("id"), "error": error}
            return self.send(200, json.dumps(out).encode(), "application/json")
        text = json.dumps(result)
        if DEEPEN.get(twist) == method:
            text = deepen(text)
        out = '{"jsonrpc":"2.0","id":%s,"result":%s}' % (json.dumps(body.get("id")), text)
        self.send(200, out.encode(), "application/json")


ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
