//! JSON-RPC over HTTP, one request per call, and the routes a check reads from: the chain's own
//! (`--rpc`), and chain 1's for Ethereum's light-client data, which every chain anchored on
//! Ethereum needs. Everything a route returns is verified before it counts, so no route is
//! trusted: it only decides where the data comes from.

use serde_json::{json, Value};

/// Where JSON-RPC requests go, by chain: the anchor asks for its light-client data on chain 1's
/// route (Ethereum's), and for everything else on the chain checked.
pub trait Transport {
    /// The response body of `method(params)` on chain `chain`'s route, as the server sent it.
    fn body(&self, chain: u64, method: &str, params: &Value) -> Result<String, String>;
}

/// The HTTP client of every route.
pub fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent(concat!("willow-logs-verify/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

/// The response body of `method(params)` at `url`.
pub fn post(
    http: &reqwest::blocking::Client,
    url: &str,
    method: &str,
    params: &Value,
) -> Result<String, String> {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let r = http
        .post(url)
        .json(&body)
        .send()
        .map_err(|e| format!("{method}: {}", e.without_url()))?;
    let status = r.status();
    let text = r
        .text()
        .map_err(|e| format!("{method}: {}", e.without_url()))?;
    if !status.is_success() && serde_json::from_str::<Value>(&text).is_err() {
        return Err(format!("{method}: HTTP {status}"));
    }
    Ok(text)
}

/// The `result` of a response body, or its JSON-RPC error as text.
pub fn result_of(method: &str, body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| format!("{method}: the response is not JSON: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("{method}: {err}"));
    }
    v.get("result")
        .cloned()
        .ok_or_else(|| format!("{method}: no result"))
}

/// The JSON-RPC error of a response body, if it is one.
pub fn error_of(body: &str) -> Option<Value> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("error")
        .cloned()
}

/// `result_of` at `url`.
pub fn call(
    http: &reqwest::blocking::Client,
    url: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    result_of(method, &post(http, url, method, &params)?)
}

/// The chain in a log RPC URL's `/rpc/<chain>[/<key>]` path.
pub fn url_chain(url: &str) -> Option<u64> {
    let path = url.split_once("://").map_or(url, |(_, r)| r);
    let mut it = path.split(['/', '?', '#']);
    while let Some(seg) = it.next() {
        if seg == "rpc" {
            return it.next()?.parse().ok();
        }
    }
    None
}

/// `url` with its `/rpc/<chain>` segment naming `to` instead: the same server and key, another
/// chain's route. `None` when the URL names no chain.
pub fn route_of(url: &str, to: u64) -> Option<String> {
    let (scheme, rest) = url.split_once("://").unwrap_or(("", url));
    let mut parts: Vec<String> = rest.split('/').map(str::to_string).collect();
    let i = parts.iter().position(|s| s == "rpc")?;
    let seg = parts.get_mut(i + 1)?;
    let (chain, tail) = match seg.find(['?', '#']) {
        Some(j) => seg.split_at(j),
        None => (seg.as_str(), ""),
    };
    chain.parse::<u64>().ok()?;
    *seg = format!("{to}{tail}");
    let joined = parts.join("/");
    Some(if scheme.is_empty() {
        joined
    } else {
        format!("{scheme}://{joined}")
    })
}

/// The routes of one check: the chain's own, and chain 1's (`--light-client-rpc`, or the own
/// URL with its chain replaced).
pub struct Http {
    pub http: reqwest::blocking::Client,
    pub url: String,
    pub chain_id: u64,
    pub ethereum: Option<String>,
}

impl Http {
    fn url_for(&self, chain: u64) -> Result<String, String> {
        if chain == self.chain_id {
            return Ok(self.url.clone());
        }
        if chain == 1 {
            if let Some(u) = &self.ethereum {
                return Ok(u.clone());
            }
        }
        route_of(&self.url, chain).ok_or_else(|| {
            format!(
                "--rpc names no /rpc/<chain> route, so chain {chain}'s cannot be derived from it: \
                 pass --light-client-rpc <a Willow RPC URL of chain {chain}>"
            )
        })
    }
}

impl Transport for Http {
    fn body(&self, chain: u64, method: &str, params: &Value) -> Result<String, String> {
        post(&self.http, &self.url_for(chain)?, method, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chain_comes_from_the_rpc_path_when_not_given() {
        assert_eq!(
            url_chain("https://api.willow.tech/v1/rpc/8453/k3y"),
            Some(8453)
        );
        assert_eq!(url_chain("http://127.0.0.1:8545/rpc/4663"), Some(4663));
        assert_eq!(url_chain("https://api.willow.tech/v1/rpc/health"), None);
        assert_eq!(url_chain("https://eth.example/"), None);
        assert_eq!(url_chain(""), None);
    }

    #[test]
    fn another_chains_route_keeps_the_server_and_the_key() {
        assert_eq!(
            route_of("https://api.willow.tech/v1/rpc/8453/k3y", 1).as_deref(),
            Some("https://api.willow.tech/v1/rpc/1/k3y")
        );
        assert_eq!(
            route_of("https://api.willow.tech/v1/rpc/56/public", 1).as_deref(),
            Some("https://api.willow.tech/v1/rpc/1/public")
        );
        assert_eq!(
            route_of("http://127.0.0.1:9/rpc/4663?x=1", 1).as_deref(),
            Some("http://127.0.0.1:9/rpc/1?x=1")
        );
        assert_eq!(route_of("https://eth.example/", 1), None);
        assert_eq!(route_of("https://api.willow.tech/v1/rpc/health", 1), None);
    }
}
