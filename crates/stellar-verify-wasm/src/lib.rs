














use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use stellar_rpc_verify::{
    filter, render, verified_json, verify_bundle, LedgerBundle, VerifyError, MAX_LEDGERS,
    PINNED_QUORUM_SET, PUBNET_PASSPHRASE,
};
use wasm_bindgen::prelude::*;

fn fail(kind: &str, message: impl Into<String>) -> String {
    json!({"ok": false, "kind": kind, "message": message.into()}).to_string()
}












#[wasm_bindgen(js_name = verifyLedgerBundle)]
pub fn verify_ledger_bundle(response: &str, trusted_quorum_set: Option<String>) -> String {
    match verify(response, trusted_quorum_set.as_deref()) {
        Ok(v) => json!({"ok": true, "verified": v}).to_string(),
        Err((kind, m)) => fail(kind, m),
    }
}


pub fn verify(
    response: &str,
    trusted_qset_b64: Option<&str>,
) -> Result<Value, (&'static str, String)> {
    let v: Value = serde_json::from_str(response)
        .map_err(|e| ("invalid", format!("the proof is not JSON: {e}")))?;
    let bundle = match v {
        Value::Object(mut o) if o.contains_key("jsonrpc") => match o.remove("result") {
            Some(r) => r,
            None => return Err(("invalid", "the proof response has no result".into())),
        },
        other => other,
    };
    let b: LedgerBundle =
        serde_json::from_value(bundle).map_err(|e| ("invalid", format!("bundle: {e}")))?;
    let trusted = match trusted_qset_b64 {
        Some(s) if !s.is_empty() => Some(B64.decode(s).map_err(|_| {
            (
                "invalid-request",
                "the trusted quorum set is not base64".to_string(),
            )
        })?),
        _ => None,
    };
    let verified = verify_bundle(&b, trusted.as_deref()).map_err(|e| match e {
        VerifyError::Pending(m) => ("pending", m),
        VerifyError::Invalid(m) => ("invalid", m),
    })?;
    let mut out = verified_json(&verified);
    if let Some(ledgers) = out["ledgers"].as_array_mut() {
        for (l, j) in verified.ledgers.iter().zip(ledgers.iter_mut()) {
            let invocations: Vec<Value> = l
                .invocations
                .iter()
                .map(|i| {
                    json!({
                        "tx": i.tx + 1,
                        "op": i.op,
                        "eventsXdr": i.preimage.events.iter().map(render::b64_xdr).collect::<Vec<_>>(),
                    })
                })
                .collect();
            j["invocations"] = json!(invocations);
        }
    }
    Ok(out)
}





#[wasm_bindgen(js_name = matchEvents)]
pub fn match_events(events: &str, filters: &str) -> String {
    let filters: Value = match serde_json::from_str(filters) {
        Ok(v) => v,
        Err(e) => return fail("invalid-request", format!("filters are not JSON: {e}")),
    };
    let parsed = match filter::parse_filters(Some(&filters)) {
        Ok(f) => f,
        Err(m) => return fail("invalid-request", m),
    };
    let events: Vec<Value> = match serde_json::from_str(events) {
        Ok(v) => v,
        Err(e) => {
            return fail(
                "invalid-request",
                format!("events are not a JSON array: {e}"),
            )
        }
    };
    let matches: Vec<bool> = events
        .iter()
        .map(|e| filter::matches_any(&parsed, e))
        .collect();
    json!({"ok": true, "matches": matches}).to_string()
}




#[wasm_bindgen(js_name = pinnedQuorumSet)]
pub fn pinned_quorum_set() -> String {
    json!({
        "ok": true,
        "xdr": B64.encode(PINNED_QUORUM_SET),
        "hash": hex::encode(Sha256::digest(PINNED_QUORUM_SET)),
        "network": PUBNET_PASSPHRASE,
        "maxLedgers": MAX_LEDGERS,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn the_pinned_set_is_the_tier1_set() {
        let p = parse(&pinned_quorum_set());
        assert_eq!(
            p["hash"],
            "040355b75766e6799d1ef69ce03f114530a06d922dd64b5e885da907a453841d"
        );
        assert_eq!(p["network"], PUBNET_PASSPHRASE);
        assert_eq!(p["maxLedgers"], 64);
    }

    #[test]
    fn an_unanchored_bundle_is_pending_and_a_wrong_network_invalid() {
        let bundle = |network: &str| {
            json!({"jsonrpc": "2.0", "id": 1, "result": {
                "network": network, "startLedger": 5, "endLedger": 5,
                "ledgers": [{"sequence": 5, "header": "", "results": "", "preimages": []}],
                "anchor": null, "pending": "checkpoint 127 is not published yet",
            }})
            .to_string()
        };
        let p = parse(&verify_ledger_bundle(&bundle(PUBNET_PASSPHRASE), None));
        assert_eq!(p["ok"], false);
        assert_eq!(p["kind"], "pending");
        let p = parse(&verify_ledger_bundle(
            &bundle("Test SDF Network ; September 2015"),
            None,
        ));
        assert_eq!(p["kind"], "invalid");
        let p = parse(&verify_ledger_bundle(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{}}",
            None,
        ));
        assert_eq!(p["kind"], "invalid");
        let p = parse(&verify_ledger_bundle(
            &bundle(PUBNET_PASSPHRASE),
            Some("%%".into()),
        ));
        assert_eq!(p["kind"], "invalid-request");
    }

    #[test]
    fn events_match_as_the_server_filters() {
        let fee = "AAAADwAAAANmZWUA";
        let ev = json!([{"type": "contract", "contractId": "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
            "topic": [fee, "AAAAEgAAAAAAAAAAH+s1pQp3PLqbLg8QADzg7Ith+AHnfU+Lqmo03hyRBfA="]}])
        .to_string();
        let m = |f: Value| parse(&match_events(&ev, &f.to_string()));
        assert_eq!(m(json!(null))["matches"], json!([true]));
        assert_eq!(
            m(json!([{"topics": [[fee, "*"]]}]))["matches"],
            json!([true])
        );
        assert_eq!(m(json!([{"type": "system"}]))["matches"], json!([false]));
        assert_eq!(m(json!([{"bogus": 1}]))["kind"], "invalid-request");
    }
}
