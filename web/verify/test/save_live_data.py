"""Saves the browser test's data dir from a live log RPC: two Ethereum blocks proven on their own
(outside a final segment, so each comes as a one-block part with every log it holds). The first block
must be 40 blocks into its 64-block window (first % 64 == 40): browser.mjs's download cases count the
blocks after the last final segment.

    WILLOW_RPC=<log rpc url for chain 1, with key> python3 save_live_data.py <data dir> <first block>

Writes answer.json (the willow_getLogsProof result), store/leaf/<block>-<hash>.proof (each SP1
proof, as bytes), tampered.json (the second block's last record byte changed) and forged-pv.json
(the first block's parent hash in its public values changed). Never prints the URL.
"""

import json
import os
import sys
import urllib.request


def call(method, params):
    req = urllib.request.Request(os.environ["WILLOW_RPC"], data=json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode(),
        headers={"content-type": "application/json", "user-agent": "willow-verify-test"})
    with urllib.request.urlopen(req, timeout=120) as r:
        body = json.load(r)
    if "error" in body:
        sys.exit(f"{method}: {body['error'].get('message')}")
    return body["result"]


out, first = sys.argv[1], int(sys.argv[2])
if first % 64 != 40:
    sys.exit(f"block {first} is {first % 64} blocks into its 64-block window; pick one 40 blocks in")
answer = call("willow_getLogsProof", [{"fromBlock": hex(first), "toBlock": hex(first + 1)}])
if any(p.get("type") != "block" for p in answer["parts"]):
    sys.exit("these blocks are in a final segment; pick two blocks proven on their own")
os.makedirs(os.path.join(out, "store", "leaf"), exist_ok=True)
json.dump(answer, open(os.path.join(out, "answer.json"), "w"))
for p in answer["parts"]:
    proof = call("willow_getProof", [p["proof"]])["proof"]
    open(os.path.join(out, "store", p["proof"] + ".proof"), "wb").write(bytes.fromhex(proof[2:]))
tampered = json.loads(json.dumps(answer))
records = tampered["parts"][1]["records"]
tampered["parts"][1]["records"] = records[:-2] + "%02x" % (int(records[-2:], 16) ^ 1)
json.dump(tampered, open(os.path.join(out, "tampered.json"), "w"))
# The first block's parent hash (word 5 of its statement): nothing before the SP1 check reads it.
forged = json.loads(json.dumps(answer))
pv = forged["parts"][0]["publicValues"]
i = 2 + 5 * 64 + 10
forged["parts"][0]["publicValues"] = pv[:i] + ("1" if pv[i] == "0" else "0") + pv[i + 1:]
json.dump(forged, open(os.path.join(out, "forged-pv.json"), "w"))
print(f"saved blocks {first}-{first + 1}: {len(answer['parts'])} parts")
