//! Every mode of the CLI end to end on saved responses, served by a stand-in Willow on 127.0.0.1:
//! block, fee and fee-history proofs of all seven original chains (blockproof's fixtures, with
//! publicnode's own responses to compare), state, call and estimate proofs (state-verify's, with
//! geth's), transactions, receipts and block receipts anchored by the light client end to end
//! (web/verify/test/items with the saved anchors of their blocks), and logs anchored at their last
//! block. Each is also tampered, and every tampered run must exit 1 with the expected kind.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn guest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn web() -> PathBuf {
    guest().join("../../web/verify/test")
}

fn read(p: PathBuf) -> Value {
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .unwrap()
}

fn listed(dir: PathBuf, suffix: &str) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.ends_with(suffix)
                .then(|| (name.clone(), read(dir.join(&name))))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

type Asked = Arc<Mutex<Vec<(u64, String, Value)>>>;

/// What the stand-in serves: a response by (chain, method, params) first, else by (chain,
/// method); Ethereum's light-client data on chain 1.
#[derive(Default, Clone)]
struct World {
    by_method: HashMap<(u64, String), Value>,
    by_params: HashMap<(u64, String, String), Value>,
    lc: Option<(Value, Vec<Value>)>,
}

impl World {
    fn result(mut self, chain: u64, method: &str, result: Value) -> Self {
        self.by_method
            .insert((chain, method.into()), json!({ "result": result }));
        self
    }

    fn response(mut self, chain: u64, method: &str, response: Value) -> Self {
        self.by_method.insert((chain, method.into()), response);
        self
    }

    fn at(mut self, chain: u64, method: &str, params: Value, result: Value) -> Self {
        self.by_params.insert(
            (chain, method.into(), params.to_string()),
            json!({ "result": result }),
        );
        self
    }

    /// Ethereum's light client as saved in web/verify/test/anchor/`dir`.
    fn light_client(mut self, dir: &str) -> Self {
        let d = web().join("anchor").join(dir);
        self.lc = Some((
            read(d.join("bootstrap.json")),
            read(d.join("updates.json")).as_array().unwrap().clone(),
        ));
        self
    }

    fn respond(&self, chain: u64, method: &str, params: &Value) -> Value {
        if let (1, Some((b, u))) = (chain, &self.lc) {
            match method {
                "willow_lightClientBootstrap" => return json!({ "result": b }),
                "willow_lightClientUpdates" => {
                    let (from, count) = (params[0].as_u64().unwrap(), params[1].as_u64().unwrap());
                    let list: Vec<&Value> = u
                        .iter()
                        .filter(|x| {
                            let slot: u64 = x["data"]["attested_header"]["beacon"]["slot"]
                                .as_str()
                                .unwrap()
                                .parse()
                                .unwrap();
                            (from..from + count).contains(&(slot / 8192))
                        })
                        .collect();
                    return json!({ "result": list });
                }
                _ => {}
            }
        }
        if let Some(r) = self
            .by_params
            .get(&(chain, method.into(), params.to_string()))
        {
            return r.clone();
        }
        self.by_method
            .get(&(chain, method.into()))
            .cloned()
            .unwrap_or_else(|| {
                json!({"error": {"code": -32601, "message": format!("{method} is not in this test's saved data")}})
            })
    }
}

struct Mock {
    base: String,
    asked: Asked,
}

impl Mock {
    fn rpc(&self, chain: u64) -> String {
        format!("{}/rpc/{chain}/test", self.base)
    }

    fn asked(&self, method: &str) -> Vec<(u64, Value)> {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, m, _)| m == method)
            .map(|(c, _, p)| (*c, p.clone()))
            .collect()
    }
}

fn serve(w: World) -> Mock {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let asked: Asked = Arc::default();
    let (w, a) = (Arc::new(w), asked.clone());
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (w, a) = (w.clone(), a.clone());
            std::thread::spawn(move || {
                let _ = handle(s, &w, &a);
            });
        }
    });
    Mock { base, asked }
}

fn handle(mut s: TcpStream, w: &World, asked: &Asked) -> std::io::Result<()> {
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
    let mut len = 0;
    loop {
        let mut h = String::new();
        r.read_line(&mut h)?;
        if h == "\r\n" || h.is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    let chain = path
        .split('/')
        .skip_while(|p| *p != "rpc")
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let (status, out) = match serde_json::from_slice::<Value>(&body) {
        Ok(req) => {
            let m = req["method"].as_str().unwrap_or("").to_string();
            let p = req["params"].clone();
            asked.lock().unwrap().push((chain, m.clone(), p.clone()));
            let mut resp = w.respond(chain, &m, &p);
            resp["jsonrpc"] = json!("2.0");
            resp["id"] = req["id"].clone();
            ("200 OK", resp.to_string())
        }
        Err(_) => ("404 Not Found", "{}".to_string()),
    };
    write!(
        s,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}",
        out.len()
    )?;
    s.flush()
}

/// The CLI's exit, its report, and its stderr. No cache unless one is named.
fn cli(args: &[&str]) -> (bool, Value, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_willow-logs-verify"));
    c.args(args);
    if !args.contains(&"--cache-dir") {
        c.arg("--no-cache");
    }
    let out = c.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (
        out.status.success(),
        serde_json::from_str(&stdout).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "wlv-modes-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Changes one hex digit of `s` at `i` from the end (a byte inside a header or a proof node).
fn flip(s: &str, from_end: usize) -> String {
    let i = s.len() - from_end;
    let c = if &s[i..i + 1] == "0" { "1" } else { "0" };
    format!("{}{c}{}", &s[..i], &s[i + 1..])
}

/// Flips a digit of the first long hex string in `v`.
fn flip_first_long(v: &mut Value) -> bool {
    match v {
        Value::String(s) if s.starts_with("0x") && s.len() > 200 => {
            *s = flip(s, 100);
            true
        }
        Value::Array(a) => a.iter_mut().any(flip_first_long),
        Value::Object(o) => o.values_mut().any(flip_first_long),
        _ => false,
    }
}

fn chain_of(v: &Value) -> u64 {
    let s = v
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string());
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).unwrap(),
        None => s.parse().unwrap(),
    }
}

/// The saved light client of web/verify/test/anchor/: its checkpoint and when it was current.
fn original() -> (String, u64) {
    let cp = read(web().join("anchor/checkpoint.json"));
    let slot: u64 = read(web().join("anchor/history-26226600.json"))["update"]["data"]
        ["signature_slot"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    (
        format!("{}:{}", cp["root"].as_str().unwrap(), cp["slot"]),
        1_606_824_023 + 12 * slot + 60,
    )
}

/// A minute after the Ethereum block an L2 anchor reads.
fn l2_now(anchor: &Value) -> u64 {
    anchor["update"]["data"]["finalized_header"]["execution"]["timestamp"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        + 60
}

#[test]
fn blocks_of_every_chain_verify_and_render_as_the_node_does() {
    let all = listed(guest().join("blockproof/fixtures/block"), ".json");
    assert!(all.len() >= 10);
    for (name, f) in all {
        let chain = chain_of(&f["chainId"]);
        let full_block = f["node"]["eth_getBlockByNumber"].clone();
        let hash = f["result"]["blockHash"].as_str().unwrap().to_string();
        let mut plain = full_block.clone();
        if let Some(txs) = plain["transactions"].as_array_mut() {
            for t in txs.iter_mut() {
                if t.is_object() {
                    *t = t["hash"].clone();
                }
            }
        }
        let w = World::default()
            .result(chain, "willow_getBlockProof", f["result"].clone())
            .at(chain, "eth_getBlockByHash", json!([hash, false]), plain)
            .at(chain, "eth_getBlockByHash", json!([hash, true]), full_block);
        let m = serve(w);
        let c = chain.to_string();
        let rpc = m.rpc(chain);
        let (ok, r, err) = cli(&[
            "--rpc",
            &rpc,
            "--chain-id",
            &c,
            "--block",
            &hash,
            "--full",
            "--no-anchor",
            "--compare-rpc",
            &rpc,
            "--print",
        ]);
        assert!(ok, "{name}: {err} {r}");
        assert_eq!(r["verified"], true, "{name}");
        assert_eq!(r["block"]["hash"], json!(hash), "{name}");
        assert_eq!(r["anchor"]["by"], "none", "{name}");
        assert_eq!(
            r["compare"]["eth_getBlockByHash"]["exact"], true,
            "{name}: {r}"
        );
        assert_eq!(
            r["compare"]["eth_getBlockByHash.transactions"]["exact"], true,
            "{name}: {}",
            r["compare"]
        );
        assert!(r["proven"]["transactions"].is_array(), "{name}");
        // The block proof as Willow sent it asked for by this very hash.
        assert_eq!(
            m.asked("willow_getBlockProof")[0].1,
            json!([hash]),
            "{name}"
        );
        // Tampered: a header byte; the proof for another chain.
        let mut bad = f["result"].clone();
        bad["header"] = json!(flip(bad["header"].as_str().unwrap(), 300));
        let m2 = serve(World::default().result(chain, "willow_getBlockProof", bad));
        let (ok, r, _) = cli(&[
            "--rpc",
            &m2.rpc(chain),
            "--chain-id",
            &c,
            "--block",
            &hash,
            "--no-anchor",
        ]);
        assert!(!ok, "{name}: a changed header passed");
        assert_eq!(r["failure"]["kind"], "rejected", "{name}: {r}");
        let other = if chain == 1 { "10" } else { "1" };
        let (ok, r, _) = cli(&[
            "--rpc",
            &rpc,
            "--chain-id",
            other,
            "--block",
            &hash,
            "--no-anchor",
        ]);
        assert!(!ok && r["failure"]["kind"] == "rejected", "{name}: {r}");
    }
}

#[test]
fn fees_and_fee_history_of_every_chain_verify_by_the_formula() {
    for (name, f) in listed(guest().join("blockproof/fixtures/fees"), ".fee.json") {
        let chain = chain_of(&f["chainId"]);
        let n = chain_of(&f["result"]["blockNumber"]).to_string();
        let mut w = World::default().result(chain, "willow_getFeeProof", f["result"].clone());
        for (m, v) in f["node"]["responses"].as_object().unwrap() {
            w = w.result(chain, m, v.clone());
        }
        let m = serve(w);
        let c = chain.to_string();
        let (ok, r, err) = cli(&[
            "--rpc",
            &m.rpc(chain),
            "--chain-id",
            &c,
            "--fees",
            "--at",
            &n,
            "--no-anchor",
            "--compare-rpc",
            &m.rpc(chain),
            "--print",
        ]);
        assert!(ok, "{name}: {err} {r}");
        assert_eq!(r["formula"], "willow-fee-v1");
        for k in [
            "eth_gasPrice",
            "eth_maxPriorityFeePerGas",
            "eth_blobBaseFee",
        ] {
            assert_eq!(r["proven"][k], f["verified"][k], "{name} {k}");
            assert!(r["compare"][k].is_object(), "{name} {k}");
        }
        let mut bad = f["result"].clone();
        let last = bad["headers"].as_array().unwrap().len() - 1;
        bad["headers"][last] = json!(flip(bad["headers"][last].as_str().unwrap(), 300));
        let m2 = serve(World::default().result(chain, "willow_getFeeProof", bad));
        let (ok, r, _) = cli(&[
            "--rpc",
            &m2.rpc(chain),
            "--chain-id",
            &c,
            "--fees",
            "--at",
            &n,
            "--no-anchor",
        ]);
        assert!(!ok && r["failure"]["kind"] == "rejected", "{name}: {r}");
    }
    for (name, f) in listed(guest().join("blockproof/fixtures/fees"), ".history.json") {
        let chain = chain_of(&f["chainId"]);
        let p = &f["request"]["params"];
        let mut arg = format!("{},{}", chain_of(&p[0]), chain_of(&p[1]));
        for x in p[2].as_array().unwrap() {
            arg.push_str(&format!(",{x}"));
        }
        let w = World::default()
            .result(chain, "willow_getFeeHistoryProof", f["result"].clone())
            .result(chain, "eth_feeHistory", f["node"]["eth_feeHistory"].clone());
        let m = serve(w);
        let c = chain.to_string();
        let nitro = chain == 42161 || chain == 4663;
        let mut args = vec![
            "--rpc".to_string(),
            m.rpc(chain),
            "--chain-id".into(),
            c.clone(),
            "--fee-history".into(),
            arg.clone(),
            "--no-anchor".into(),
            "--print".into(),
        ];
        // Nitro's own gas-used ratios need ArbOS state the proof does not carry.
        if !nitro {
            args.extend(["--compare-rpc".to_string(), m.rpc(chain)]);
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, r, err) = cli(&args);
        assert!(ok, "{name}: {err} {r}");
        assert_eq!(
            r["proven"]["baseFeePerGas"], f["verified"]["baseFeePerGas"],
            "{name}"
        );
        assert_eq!(
            r["proven"]["oldestBlock"], f["verified"]["oldestBlock"],
            "{name}"
        );
        if !nitro {
            assert_eq!(r["compare"]["eth_feeHistory"]["exact"], true, "{name}: {r}");
        }
        let mut bad = f["result"].clone();
        bad["headers"][0] = json!(flip(bad["headers"][0].as_str().unwrap(), 300));
        let m2 = serve(World::default().result(chain, "willow_getFeeHistoryProof", bad));
        let (ok, r, _) = cli(&[
            "--rpc",
            &m2.rpc(chain),
            "--chain-id",
            &c,
            "--fee-history",
            &arg,
            "--no-anchor",
        ]);
        assert!(!ok && r["failure"]["kind"] == "rejected", "{name}: {r}");
    }
}

#[test]
fn state_calls_and_estimates_verify_over_proven_state() {
    let mut dirs = listed(guest().join("state-verify/testdata/state"), ".json");
    dirs.extend(listed(guest().join("state-verify/testdata/call"), ".json"));
    assert_eq!(dirs.len(), 14);
    for (name, f) in dirs {
        let method = f["request"]["method"].as_str().unwrap();
        let p = &f["request"]["params"];
        let chain = chain_of(&f["result"]["chainId"]);
        let at = match &p[if method == "willow_getStateProof" {
            2
        } else {
            1
        }] {
            Value::Object(o) => o["blockHash"].as_str().unwrap().to_string(),
            v => v.as_str().unwrap().to_string(),
        };
        let mut w = World::default().result(chain, method, f["result"].clone());
        let mut args: Vec<String> = vec![
            "--rpc".into(),
            String::new(),
            "--chain-id".into(),
            chain.to_string(),
            "--no-anchor".into(),
            "--at".into(),
            at.clone(),
        ];
        let compared: Vec<&str>;
        match method {
            "willow_getStateProof" => {
                args.extend(["--state".into(), p[0].as_str().unwrap().into()]);
                let slots: Vec<&str> = p[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| k.as_str().unwrap())
                    .collect();
                if !slots.is_empty() {
                    args.extend(["--slots".into(), slots.join(",")]);
                }
                if p[3]["code"] == true {
                    args.push("--code".into());
                }
                // The standard responses the proof gives, as another endpoint would send them.
                let e = &f["expect"];
                for m in [
                    "eth_getBalance",
                    "eth_getTransactionCount",
                    "eth_getCode",
                    "eth_getProof",
                ] {
                    w = w.result(chain, m, e[m].clone());
                }
                if let Some(first) = e["eth_getStorageAt"].as_array().and_then(|a| a.first()) {
                    w = w.result(chain, "eth_getStorageAt", first.clone());
                }
                compared = if slots.len() > 1 {
                    vec![]
                } else {
                    vec!["eth_getBalance", "eth_getProof"]
                };
            }
            m => {
                let flag = if m == "willow_getCallProof" {
                    "--call"
                } else {
                    "--estimate"
                };
                args.extend([flag.into(), p[0].to_string()]);
                let standard = if flag == "--call" {
                    "eth_call"
                } else {
                    "eth_estimateGas"
                };
                w = w.response(chain, standard, f["reference"][standard].clone());
                compared = vec![standard];
            }
        }
        let mock = serve(w);
        args[1] = mock.rpc(chain);
        if !compared.is_empty() {
            args.extend(["--compare-rpc".into(), mock.rpc(chain)]);
        }
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, r, err) = cli(&a);
        assert!(ok, "{name}: {err} {r}");
        assert_eq!(r["responses"], f["expect"], "{name}");
        for c in compared {
            assert_eq!(r["compare"][c]["exact"], true, "{name} {c}: {r}");
        }
        // What was asked is what the user gave: the block by its hash, as EIP-1898's object.
        let sent = &mock.asked(method)[0].1;
        if at.len() == 66 {
            let i = if method == "willow_getStateProof" {
                2
            } else {
                1
            };
            assert_eq!(sent[i], json!({"blockHash": at}), "{name}");
        }
        // Tampered: a proof node, the code or the witness; the block header.
        let mut bad = f["result"].clone();
        assert!(flip_first_long(&mut bad), "{name}");
        let m2 = serve(World::default().result(chain, method, bad));
        let mut a2 = args.clone();
        a2[1] = m2.rpc(chain);
        if let Some(i) = a2.iter().position(|x| x == "--compare-rpc") {
            a2.truncate(i);
        }
        let a2: Vec<&str> = a2.iter().map(String::as_str).collect();
        let (ok, r, _) = cli(&a2);
        assert!(
            !ok && r["failure"]["kind"].is_string(),
            "{name}: tampered passed: {r}"
        );
        let mut bad = f["result"].clone();
        bad["header"] = json!(flip(bad["header"].as_str().unwrap(), 300));
        let m3 = serve(World::default().result(chain, method, bad));
        let mut a3: Vec<String> = a2.iter().map(|s| s.to_string()).collect();
        a3[1] = m3.rpc(chain);
        let a3: Vec<&str> = a3.iter().map(String::as_str).collect();
        let (ok, r, _) = cli(&a3);
        assert!(
            !ok && r["failure"]["kind"].is_string(),
            "{name}: changed header passed: {r}"
        );
    }
}

/// web/verify/test/items fixtures whose blocks have saved anchors: (file, anchor file, path).
const ANCHORED_ITEMS: [(&str, &str, &str); 8] = [
    ("eth-tx.json", "beacon-26124863.json", "beacon"),
    ("eth-receipt.json", "beacon-26124863.json", "beacon"),
    ("eth-receipt-1.json", "beacon-26124863.json", "beacon"),
    ("arb-tx.json", "l2-42161-511868572.json", "assertion"),
    ("arb-receipt.json", "l2-42161-511868572.json", "assertion"),
    ("arb-block.json", "l2-42161-511868572.json", "assertion"),
    (
        "base-reverted-receipt.json",
        "l2-8453-52597001.json",
        "game",
    ),
    ("base-swap-receipt.json", "l2-8453-52491359.json", "game"),
];

fn item_world(f: &Value, chain: u64) -> World {
    let mut w = World::default().result(
        chain,
        f["proof"]["method"].as_str().unwrap(),
        f["proof"]["result"].clone(),
    );
    for a in f["answers"].as_array().unwrap() {
        w = w.at(
            chain,
            a["method"].as_str().unwrap(),
            a["params"].clone(),
            a["result"].clone(),
        );
    }
    w
}

fn item_flag(kind: &str) -> &'static str {
    match kind {
        "transaction" => "--tx",
        "receipt" => "--receipt",
        _ => "--block-receipts",
    }
}

#[test]
fn transactions_receipts_and_block_receipts_anchored_by_the_light_client() {
    let (cp, eth_now) = original();
    for (file, anchor_file, path) in ANCHORED_ITEMS {
        let f = read(web().join("items").join(file));
        let chain = chain_of(&f["chainId"]);
        let anchor = read(web().join("anchor").join(anchor_file));
        let now = if chain == 1 { eth_now } else { l2_now(&anchor) };
        let number = f["block"]["number"].as_str().unwrap().to_string();
        let w = item_world(&f, chain).light_client("").at(
            chain,
            "willow_getBlockAnchor",
            json!([number]),
            anchor.clone(),
        );
        let m = serve(w);
        let (c, t, now_s) = (
            chain.to_string(),
            f["target"].to_string().trim_matches('"').to_string(),
            now.to_string(),
        );
        let cache = temp_dir();
        let cache_s = cache.to_str().unwrap();
        let flag = item_flag(f["kind"].as_str().unwrap());
        let (ok, r, err) = cli(&[
            "--rpc",
            &m.rpc(chain),
            "--chain-id",
            &c,
            flag,
            &t,
            "--checkpoint",
            &cp,
            "--now",
            &now_s,
            "--cache-dir",
            cache_s,
            "--compare-rpc",
            &m.rpc(chain),
        ]);
        assert!(ok, "{file}: {err} {r}");
        assert_eq!(
            (
                &r["anchor"]["anchored"],
                &r["anchor"]["path"],
                &r["block"]["canonical"]
            ),
            (&json!(true), &json!(path), &json!(true)),
            "{file}: {}",
            r["anchor"]
        );
        assert_eq!(r["anchor"]["by"], "light client");
        assert_eq!(r["block"]["hash"], f["block"]["hash"], "{file}");
        for (k, v) in r["compare"].as_object().unwrap() {
            assert_eq!(v["exact"], true, "{file} {k}: {v}");
        }
        // Ethereum's light-client data was asked for on chain 1's route, and kept.
        let lc = m.asked("willow_lightClientBootstrap");
        assert!(!lc.is_empty() && lc.iter().all(|(c, _)| *c == 1), "{file}");
        assert!(cache.join("light-client.json").exists(), "{file}");
        // Tampered: another block's anchor; a proof node.
        let other = if chain == 1 {
            read(web().join("anchor/history-26226600.json"))
        } else {
            read(web().join(if chain == 42161 {
                "anchor/l2-42161-516340000.json"
            } else {
                "anchor/l2-8453-52820000.json"
            }))
        };
        let m2 = serve(item_world(&f, chain).light_client("").at(
            chain,
            "willow_getBlockAnchor",
            json!([number]),
            other,
        ));
        let (ok, r, _) = cli(&[
            "--rpc",
            &m2.rpc(chain),
            "--chain-id",
            &c,
            flag,
            &t,
            "--checkpoint",
            &cp,
            "--now",
            &now_s,
        ]);
        assert!(!ok, "{file}: another block's anchor passed");
        assert_eq!(r["anchor"]["anchored"], false, "{file}");
        assert_eq!(r["block"]["canonical"], false, "{file}");
        let mut bad = f.clone();
        assert!(flip_first_long(&mut bad["proof"]["result"]), "{file}");
        let m3 = serve(item_world(&bad, chain));
        let (ok, r, _) = cli(&[
            "--rpc",
            &m3.rpc(chain),
            "--chain-id",
            &c,
            flag,
            &t,
            "--no-anchor",
        ]);
        assert!(!ok && r["failure"]["kind"] == "rejected", "{file}: {r}");
    }
}

#[test]
fn a_block_with_no_saved_anchor_is_compared_with_a_trusted_node_on_request() {
    for file in ["uni-block.json", "uni-receipt.json", "uni-deposit.json"] {
        let f = read(web().join("items").join(file));
        let chain = chain_of(&f["chainId"]);
        let number = f["block"]["number"].as_str().unwrap().to_string();
        let flag = item_flag(f["kind"].as_str().unwrap());
        let t = f["target"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| f["target"].to_string());
        let c = chain.to_string();
        for (hash, want) in [
            (f["block"]["hash"].clone(), true),
            (json!(format!("0x{}", "ab".repeat(32))), false),
        ] {
            let m = serve(item_world(&f, chain).at(
                chain,
                "eth_getBlockByNumber",
                json!([number, false]),
                json!({ "hash": hash }),
            ));
            let (ok, r, err) = cli(&[
                "--rpc",
                &m.rpc(chain),
                "--chain-id",
                &c,
                flag,
                &t,
                "--anchor-rpc",
                &m.rpc(chain),
            ]);
            assert_eq!(ok, want, "{file}: {err} {r}");
            assert_eq!(r["anchor"]["by"], "--anchor-rpc");
            assert_eq!(r["block"]["canonical"], want);
        }
    }
}

#[test]
fn logs_are_anchored_at_their_last_proven_block() {
    let (cp, now) = original();
    let dir = web().join("logs/eth-26226600");
    let saved = read(dir.join("answer.json"));
    let filter = json!({"fromBlock": saved["fromBlock"], "toBlock": saved["toBlock"]}).to_string();
    let file = temp_dir().join("logs.json");
    std::fs::write(&file, saved.to_string()).unwrap();
    let store = dir.join("store");
    let run = |anchor: &str, extra: &[&str]| {
        let m = serve(World::default().light_client("").at(
            1,
            "willow_getBlockAnchor",
            json!(["0x1902fa9"]),
            read(web().join("anchor").join(anchor)),
        ));
        let now_s = now.to_string();
        let mut a = vec![
            "--rpc".to_string(),
            m.rpc(1),
            "--proof".into(),
            file.to_str().unwrap().into(),
            "--proofs-dir".into(),
            store.to_str().unwrap().into(),
            "--filter".into(),
            filter.clone(),
            "--chain-id".into(),
            "1".into(),
            "--checkpoint".into(),
            cp.clone(),
            "--now".into(),
            now_s,
            "--openvm-verifier".into(),
            "/nonexistent".into(),
        ];
        a.extend(extra.iter().map(|s| s.to_string()));
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        cli(&a)
    };
    let (ok, r, err) = run("history-26226601.json", &[]);
    assert!(ok, "{err} {r}");
    assert_eq!(
        (
            &r["anchor"]["path"],
            &r["anchor"]["level"],
            &r["lastProvenBlock"]["canonical"]
        ),
        (&json!("history"), &json!("final"), &json!(true)),
        "{r}"
    );
    assert_eq!(r["proofs"], json!({"sp1": 2, "openvm": 0}));
    // The anchor of the block before, served for the last one: not anchored, exit 1.
    let (ok, r, _) = run("history-26226600.json", &[]);
    assert!(!ok);
    assert_eq!(
        (&r["anchor"]["kind"], &r["anchor"]["disproven"]),
        (&json!("rejected"), &json!(true))
    );
    // --min-level above what the anchor shows is not enough either way; final is.
    let (ok, _, _) = run("history-26226601.json", &["--min-level", "final"]);
    assert!(ok);
    // Offline with no --rpc: verified, and the report says the block was not anchored.
    let (ok, r, err) = cli(&[
        "--proof",
        file.to_str().unwrap(),
        "--proofs-dir",
        store.to_str().unwrap(),
        "--filter",
        &filter,
        "--chain-id",
        "1",
        "--openvm-verifier",
        "/nonexistent",
    ]);
    assert!(ok, "{err}");
    assert_eq!(r["anchor"]["by"], "none");
}

#[test]
fn the_newest_block_is_anchored_before_its_proof_is_asked_for() {
    let latest = read(web().join("anchor/tags/latest-1.json"));
    let cp = read(web().join("anchor/tags/checkpoint.json"));
    let cp = format!("{}:{}", cp["root"].as_str().unwrap(), cp["slot"]);
    let other_block = read(guest().join("blockproof/fixtures/block/ethereum-26391511.json"));
    let w = World::default()
        .light_client("tags")
        .at(
            1,
            "willow_getBlockAnchor",
            json!(["latest"]),
            latest["response"]["result"].clone(),
        )
        .result(1, "willow_getBlockProof", other_block["result"].clone());
    let m = serve(w);
    let now = latest["savedAt"].as_u64().unwrap().to_string();
    let (ok, r, _) = cli(&[
        "--rpc",
        &m.rpc(1),
        "--block",
        "latest",
        "--checkpoint",
        &cp,
        "--now",
        &now,
    ]);
    // Asked for by the anchored head's hash; a proof of another block is refused.
    assert_eq!(
        m.asked("willow_getBlockProof")[0].1,
        json!([latest["expect"]["hash"]])
    );
    assert!(!ok && r["failure"]["kind"] == "rejected", "{r}");
    // An L2's newest block at --min-level final: refused before anything is asked for.
    let m = serve(World::default());
    let (ok, r, _) = cli(&[
        "--rpc",
        &m.rpc(8453),
        "--block",
        "latest",
        "--min-level",
        "final",
        "--checkpoint",
        &cp,
        "--now",
        &now,
    ]);
    assert!(!ok);
    assert_eq!(r["anchor"]["kind"], "below-min-level", "{r}");
    assert!(
        m.asked("willow_getBlockAnchor").is_empty() && m.asked("willow_getBlockProof").is_empty()
    );
}

#[test]
fn a_comparison_that_could_not_be_made_is_shown_not_counted() {
    let f = read(web().join("items/eth-receipt.json"));
    let t = f["target"].as_str().unwrap().to_string();
    let base = World::default().result(1, "willow_getReceiptProof", f["proof"]["result"].clone());
    let refused = json!({"error": {"code": -32602, "message": "archive requests need a token"}});
    let m = serve(
        base.clone()
            .response(1, "eth_getTransactionReceipt", refused),
    );
    let (ok, r, err) = cli(&[
        "--rpc",
        &m.rpc(1),
        "--receipt",
        &t,
        "--no-anchor",
        "--compare-rpc",
        &m.rpc(1),
    ]);
    assert!(ok, "{err} {r}");
    assert!(
        r["compare"]["eth_getTransactionReceipt"]["problem"].is_object(),
        "{r}"
    );
    // The other endpoint has no such receipt: that is a difference, counted.
    let m = serve(base.result(1, "eth_getTransactionReceipt", Value::Null));
    let (ok, r, _) = cli(&[
        "--rpc",
        &m.rpc(1),
        "--receipt",
        &t,
        "--no-anchor",
        "--compare-rpc",
        &m.rpc(1),
    ]);
    assert!(
        !ok && r["compare"]["eth_getTransactionReceipt"]["absent"] == true,
        "{r}"
    );
}

#[test]
fn refusals_absences_and_usage() {
    let m = serve(
        World::default()
            .result(1, "willow_getTransactionProof", Value::Null)
            .response(
                1,
                "willow_getReceiptProof",
                json!({"error": {"code": -32000, "message": "not found in this store: block 1"}}),
            ),
    );
    let h = format!("0x{}", "ab".repeat(32));
    let (ok, r, _) = cli(&["--rpc", &m.rpc(1), "--tx", &h, "--no-anchor"]);
    assert!(!ok && r["absent"] == true, "{r}");
    let (ok, r, _) = cli(&["--rpc", &m.rpc(1), "--receipt", &h, "--no-anchor"]);
    assert!(!ok && r["refused"]["code"] == -32000, "{r}");
    for (args, says) in [
        (
            vec!["--rpc", "http://127.0.0.1:9/rpc/1/x", "--trace", "x"],
            "--trace:",
        ),
        (
            vec![
                "--rpc",
                "http://127.0.0.1:9/rpc/1/x",
                "--fees",
                "--block-overrides",
                "{}",
            ],
            "go with --call or --estimate",
        ),
        (
            vec![
                "--rpc",
                "http://127.0.0.1:9/rpc/1/x",
                "--fees",
                "--block",
                "1",
            ],
            "one mode at a time",
        ),
        (
            vec!["--rpc", "http://127.0.0.1:9/rpc/1/x", "--fees", "--full"],
            "--full goes with --block",
        ),
        (vec!["--rpc", "http://127.0.0.1:9/rpc/1/x"], "no mode given"),
        (
            vec![
                "--rpc",
                "http://127.0.0.1:9/rpc/1/x",
                "--block",
                "1",
                "--no-anchor",
                "--anchor-rpc",
                "x",
            ],
            "choose one",
        ),
        (
            vec!["--rpc", "http://127.0.0.1:9/rpc/1/x", "--fee-history", "4"],
            "no newest block",
        ),
    ] {
        let (ok, _, err) = cli(&args);
        assert!(!ok && err.contains(says), "{args:?}: {err}");
    }
    let (ok, _, err) = cli(&["--help"]);
    assert!(ok, "{err}");
}

/// The calls and estimates the RPC served (state-verify's testdata/served): state and block
/// overrides, gas prices, Unichain's and Robinhood Chain's calls and estimates. Each verifies here
/// to the very response the RPC served, its overrides sent as the user gave them (`--overrides`,
/// `--block-overrides`), and fails when tampered.
#[test]
fn served_calls_and_estimates_with_overrides_verify_on_every_chain() {
    let all = listed(guest().join("state-verify/testdata/served"), ".json");
    assert!(all.len() >= 20, "{}", all.len());
    let mut chains = std::collections::BTreeSet::new();
    for (name, f) in all {
        let method = f["request"]["method"].as_str().unwrap();
        let p = &f["request"]["params"];
        let chain = chain_of(&f["result"]["chainId"]);
        chains.insert(chain);
        let at = match &p[1] {
            Value::Object(o) => o["blockHash"].as_str().unwrap().to_string(),
            v => v.as_str().unwrap().to_string(),
        };
        let flag = match method {
            "willow_getCallProof" => "--call",
            _ => "--estimate",
        };
        let present = |v: &Value| !(v.is_null() || v.as_object().is_some_and(|o| o.is_empty()));
        let w = World::default().result(chain, method, f["result"].clone());
        let mock = serve(w);
        let rpc = mock.rpc(chain);
        let mut args: Vec<String> = vec![
            "--rpc".into(),
            rpc,
            "--chain-id".into(),
            chain.to_string(),
            "--no-anchor".into(),
            "--at".into(),
            at,
            flag.into(),
            p[0].to_string(),
        ];
        if present(&p[2]) {
            args.extend(["--overrides".into(), p[2].to_string()]);
        }
        if present(&p[3]) {
            args.extend(["--block-overrides".into(), p[3].to_string()]);
        }
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, r, err) = cli(&a);
        assert!(ok, "{name}: {err} {r}");
        assert_eq!(r["responses"], f["expect"], "{name}");
        // The overrides went out as given: third and fourth.
        let sent = &mock.asked(method)[0].1;
        let given: Vec<&Value> = p.as_array().unwrap()[2..]
            .iter()
            .filter(|v| present(v))
            .collect();
        let gone: Vec<&Value> = sent.as_array().unwrap()[2..]
            .iter()
            .filter(|v| present(v))
            .collect();
        assert_eq!(gone, given, "{name}");
        // Tampered: a proof node.
        let mut bad = f["result"].clone();
        assert!(flip_first_long(&mut bad), "{name}");
        let m2 = serve(World::default().result(chain, method, bad));
        args[1] = m2.rpc(chain);
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, r, _) = cli(&a);
        assert!(
            !ok && r["failure"]["kind"].is_string(),
            "{name}: tampered passed: {r}"
        );
    }
    assert_eq!(chains, [1, 130, 4663].into());
}

/// trace-verify's recorded trace proofs, as a case's proof: a block's (its pool) or a call's.
fn trace_cases() -> Vec<(String, u64, Value, Value, Value)> {
    let dir = guest().join("trace-verify/testdata");
    let mut out = Vec::new();
    for (name, f) in listed(dir.clone(), ".json") {
        let chain = chain_of(&f["block"]["chainId"]);
        for c in f["cases"].as_array().unwrap() {
            let mut proof = f["block"].clone();
            proof["ancestors"] = c["ancestors"].clone();
            let pick = |field: &str| -> Value {
                let at = |i: &Value| f["pool"][field][i.as_u64().unwrap() as usize].clone();
                Value::Array(
                    c["witness"][field]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(at)
                        .collect(),
                )
            };
            proof["witness"] = json!({"nodes": pick("nodes"), "codes": pick("codes")});
            proof["algorithm"] = json!("willow-trace-v1");
            out.push((
                name.clone(),
                chain,
                c["params"].clone(),
                proof,
                c["expect"].clone(),
            ));
        }
    }
    for (name, f) in listed(dir.join("calls"), ".json") {
        for c in f["cases"].as_array().unwrap() {
            out.push((
                name.clone(),
                1,
                c["params"].clone(),
                c["result"].clone(),
                c["expect"].clone(),
            ));
        }
    }
    out
}

/// `--trace`: every recorded trace proof (Ethereum's and Unichain's blocks and transactions under
/// every tracer, Ethereum's calls under their overrides) verifies here, the block or the call run
/// again, to the response the trace method sent; compared exact with another endpoint's; and a
/// tampered proof fails with its kind.
#[test]
fn traces_of_blocks_transactions_and_calls_verify() {
    let all = trace_cases();
    assert!(all.len() >= 30, "{}", all.len());
    let mut methods = std::collections::BTreeSet::new();
    for (name, chain, params, proof, expect) in all {
        let method = params[0].as_str().unwrap().to_string();
        methods.insert(method.clone());
        let w = World::default()
            .result(chain, "willow_getTraceProof", proof.clone())
            .result(chain, &method, expect.clone());
        let mock = serve(w);
        let rpc = mock.rpc(chain);
        let args = [
            "--rpc",
            &rpc,
            "--chain-id",
            &chain.to_string(),
            "--no-anchor",
            "--trace",
            &params.to_string(),
            "--compare-rpc",
            &rpc,
        ]
        .map(str::to_string);
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, r, err) = cli(&a);
        assert!(ok, "{name} {method}: {err} {r}");
        assert_eq!(r["responses"]["result"], expect, "{name} {method}");
        assert_eq!(r["traced"], json!(method), "{name}");
        assert_eq!(r["compare"][&method]["exact"], true, "{name} {method}: {r}");
        // What was asked: the trace method and its params, a call's overrides among them.
        let sent = &mock.asked("willow_getTraceProof")[0].1;
        assert_eq!(sent[0], params[0], "{name}");
        // Tampered: the header.
        let mut bad = proof.clone();
        bad["header"] = json!(flip(bad["header"].as_str().unwrap(), 300));
        let m2 = serve(World::default().result(chain, "willow_getTraceProof", bad));
        let rpc2 = m2.rpc(chain);
        let a2: Vec<&str> = a[..7]
            .iter()
            .map(|x| if *x == rpc { rpc2.as_str() } else { *x })
            .collect();
        let (ok, r, _) = cli(&a2);
        assert!(
            !ok && r["failure"]["kind"] == "header",
            "{name} {method}: tampered passed: {r}"
        );
    }
    for m in [
        "debug_traceCall",
        "trace_call",
        "debug_traceTransaction",
        "trace_block",
    ] {
        assert!(methods.contains(m), "{m}");
    }
    // Usage: one [method, params].
    let (ok, _, err) = cli(&["--rpc", "http://127.0.0.1:9", "--trace", "\"trace_block\""]);
    assert!(!ok && err.contains("[method, params]"), "{err}");
}
