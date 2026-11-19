


use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::Value;
use stellar_xdr::{Limits, ReadXdr, ScVal, WriteXdr};

use crate::render::parse_contract_strkey;

pub const MAX_FILTERS: usize = 5;
pub const MAX_CONTRACT_IDS: usize = 5;
pub const MAX_TOPIC_FILTERS: usize = 5;
pub const MAX_SEGMENTS: usize = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {

    pub types: Vec<String>,

    pub contract_ids: Vec<String>,


    pub topics: Vec<Vec<String>>,
}

fn canonical_scval(s: &str) -> Result<String, String> {
    let bytes = B64
        .decode(s)
        .map_err(|_| format!("topic segment {s:?} is not base64"))?;
    let v = ScVal::from_xdr(bytes, Limits::depth(64))
        .map_err(|_| format!("topic segment {s:?} is not an ScVal"))?;
    Ok(B64.encode(v.to_xdr(Limits::none()).map_err(|e| e.to_string())?))
}


pub fn parse_filters(v: Option<&Value>) -> Result<Vec<Filter>, String> {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(vec![]);
    };
    let arr = v.as_array().ok_or("filters must be an array")?;
    if arr.len() > MAX_FILTERS {
        return Err(format!("maximum {MAX_FILTERS} filters per request"));
    }
    let mut out = Vec::with_capacity(arr.len());
    for f in arr {
        let o = f.as_object().ok_or("a filter must be an object")?;
        for k in o.keys() {
            if !matches!(k.as_str(), "type" | "contractIds" | "topics") {
                return Err(format!("unknown filter field {k:?}"));
            }
        }
        let mut filter = Filter::default();
        if let Some(t) = o.get("type").filter(|t| !t.is_null()) {
            let t = t.as_str().ok_or("filter type must be a string")?;
            for part in t.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                if !matches!(part, "contract" | "system" | "diagnostic") {
                    return Err(format!("if set, type must be either 'system', 'contract' or 'diagnostic', not {part:?}"));
                }
                filter.types.push(part.to_string());
            }
        }
        if let Some(ids) = o.get("contractIds").filter(|t| !t.is_null()) {
            let ids = ids.as_array().ok_or("contractIds must be an array")?;
            if ids.len() > MAX_CONTRACT_IDS {
                return Err(format!(
                    "maximum {MAX_CONTRACT_IDS} contract IDs per filter"
                ));
            }
            for id in ids {
                let s = id.as_str().ok_or("a contract ID must be a string")?;
                if parse_contract_strkey(s).is_none() {
                    return Err(format!("contract ID {s:?} is not a valid contract strkey"));
                }
                filter.contract_ids.push(s.to_string());
            }
        }
        if let Some(topics) = o.get("topics").filter(|t| !t.is_null()) {
            let topics = topics.as_array().ok_or("topics must be an array")?;
            if topics.len() > MAX_TOPIC_FILTERS {
                return Err(format!(
                    "maximum {MAX_TOPIC_FILTERS} topic filters per filter"
                ));
            }
            for t in topics {
                let segs = t.as_array().ok_or("a topic filter must be an array")?;
                if segs.is_empty() || segs.len() > MAX_SEGMENTS + 1 {
                    return Err(format!("a topic filter has 1 to {MAX_SEGMENTS} segments"));
                }
                let mut tf = Vec::with_capacity(segs.len());
                for (i, s) in segs.iter().enumerate() {
                    let s = s.as_str().ok_or("a topic segment must be a string")?;
                    match s {
                        "*" => tf.push(s.to_string()),
                        "**" if i + 1 == segs.len() => tf.push(s.to_string()),
                        "**" => return Err("'**' may only be the last segment".into()),
                        _ => tf.push(canonical_scval(s)?),
                    }
                }
                let fixed = tf.iter().filter(|s| s.as_str() != "**").count();
                if fixed > MAX_SEGMENTS {
                    return Err(format!(
                        "a topic filter has at most {MAX_SEGMENTS} segments"
                    ));
                }
                filter.topics.push(tf);
            }
        }
        out.push(filter);
    }
    Ok(out)
}

fn topic_matches(tf: &[String], topics: &[Value]) -> bool {
    let open = tf.last().is_some_and(|s| s == "**");
    let fixed = if open { &tf[..tf.len() - 1] } else { tf };
    if topics.len() < fixed.len() || (!open && topics.len() != fixed.len()) {
        return false;
    }
    fixed
        .iter()
        .zip(topics)
        .all(|(seg, t)| seg == "*" || t.as_str() == Some(seg.as_str()))
}

impl Filter {

    pub fn matches(&self, ev: &Value) -> bool {
        let ty = ev["type"].as_str().unwrap_or("");
        if !self.types.is_empty() && !self.types.iter().any(|t| t == ty) {
            return false;
        }
        let cid = ev["contractId"].as_str().unwrap_or("");
        if !self.contract_ids.is_empty() && !self.contract_ids.iter().any(|c| c == cid) {
            return false;
        }
        if self.topics.is_empty() {
            return true;
        }
        let topics = ev["topic"].as_array().map_or(&[][..], |a| a.as_slice());
        self.topics.iter().any(|tf| topic_matches(tf, topics))
    }
}


pub fn matches_any(filters: &[Filter], ev: &Value) -> bool {
    filters.is_empty() || filters.iter().any(|f| f.matches(ev))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topics_types_and_contracts() {
        let fee = "AAAADwAAAANmZWUA";
        let ev = json!({"type": "contract", "contractId": "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
            "topic": [fee, "AAAAEgAAAAAAAAAAH+s1pQp3PLqbLg8QADzg7Ith+AHnfU+Lqmo03hyRBfA="]});
        let f = |v: Value| parse_filters(Some(&v)).unwrap();
        assert!(matches_any(&f(json!([])), &ev));
        assert!(matches_any(&f(json!([{"topics": [[fee, "*"]]}])), &ev));
        assert!(matches_any(&f(json!([{"topics": [[fee, "**"]]}])), &ev));
        assert!(matches_any(&f(json!([{"topics": [["**"]]}])), &ev));
        assert!(!matches_any(&f(json!([{"topics": [[fee]]}])), &ev));
        assert!(!matches_any(
            &f(json!([{"topics": [[fee, "*", "*"]]}])),
            &ev
        ));
        assert!(!matches_any(&f(json!([{"type": "system"}])), &ev));
        assert!(matches_any(
            &f(json!([{"type": "system"}, {"type": "contract"}])),
            &ev
        ));
        assert!(matches_any(
            &f(
                json!([{"contractIds": ["CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"]}])
            ),
            &ev
        ));
        assert!(parse_filters(Some(&json!([{"contractIds": ["Cnope"]}]))).is_err());
        assert!(parse_filters(Some(&json!([{"topics": [["**", fee]]}]))).is_err());
        assert!(parse_filters(Some(&json!([{"bogus": 1}]))).is_err());
    }
}
