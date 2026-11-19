



use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use stellar_xdr::{
    ContractEvent, ContractEventBody, ContractEventType, Limits, TransactionResult,
    TransactionResultResult, WriteXdr,
};



pub const TX_AFTER_ALL: u32 = (1 << 20) - 1;


pub const OP_AFTER_TX: u32 = (1 << 12) - 1;


pub fn toid(ledger: u32, tx: u32, op: u32) -> i64 {
    ((ledger as i64) << 32) | (((tx & TX_AFTER_ALL) as i64) << 12) | (op & OP_AFTER_TX) as i64
}


pub fn event_id(ledger: u32, tx: u32, op: u32, index: u32) -> String {
    format!("{:019}-{:010}", toid(ledger, tx, op), index)
}





















pub fn rfc3339(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let rem = unix % 86_400;

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem / 60) % 60,
        rem % 60
    )
}

fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn base32(data: &[u8]) -> String {
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity(data.len() * 8 / 5 + 1);
    let (mut buf, mut bits) = (0u32, 0u32);
    for &b in data {
        buf = (buf << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(A[((buf >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(A[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn strkey(version: u8, payload: &[u8; 32]) -> String {
    let mut b = Vec::with_capacity(35);
    b.push(version);
    b.extend_from_slice(payload);
    let crc = crc16_xmodem(&b);
    b.extend_from_slice(&crc.to_le_bytes());
    base32(&b)
}


pub fn contract_strkey(id: &[u8; 32]) -> String {
    strkey(2 << 3, id)
}


pub fn parse_contract_strkey(s: &str) -> Option<[u8; 32]> {
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    if s.len() != 56 {
        return None;
    }
    let mut out = Vec::with_capacity(35);
    let (mut buf, mut bits) = (0u64, 0u32);
    for c in s.bytes() {
        let v = A.iter().position(|&a| a == c)? as u64;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    if out.len() != 35 || out[0] != 2 << 3 {
        return None;
    }
    let crc = crc16_xmodem(&out[..33]);
    if out[33..35] != crc.to_le_bytes() {
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&out[1..33]);

    (contract_strkey(&id) == s).then_some(id)
}





pub fn b64_xdr<T: WriteXdr>(v: &T) -> String {
    B64.encode(v.to_xdr(Limits::none()).expect("XDR of a decoded value"))
}


pub fn event_type(t: ContractEventType) -> &'static str {
    match t {
        ContractEventType::Contract => "contract",
        ContractEventType::System => "system",
        ContractEventType::Diagnostic => "diagnostic",
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventAt {
    pub ledger: u32,


    pub tx: u32,

    pub op: u32,
    pub index: u32,
}

impl EventAt {
    pub fn id(&self) -> String {
        event_id(self.ledger, self.tx, self.op, self.index)
    }
}


pub fn event_json(
    at: EventAt,
    close_time: u64,
    tx_hash: &[u8; 32],
    successful: bool,
    ev: &ContractEvent,
) -> Value {
    let ContractEventBody::V0(body) = &ev.body;
    json!({
        "type": event_type(ev.type_),
        "ledger": at.ledger,
        "ledgerClosedAt": rfc3339(close_time),
        "contractId": ev.contract_id.as_ref().map_or(String::new(), |c| contract_strkey(&c.0 .0)),
        "id": at.id(),
        "operationIndex": at.op,
        "transactionIndex": at.tx,
        "txHash": hex::encode(tx_hash),
        "inSuccessfulContractCall": successful,
        "topic": body.topics.iter().map(b64_xdr).collect::<Vec<_>>(),
        "value": b64_xdr(&body.data),
    })
}


pub fn tx_successful(r: &TransactionResult) -> bool {
    matches!(
        r.result,
        TransactionResultResult::TxSuccess(_) | TransactionResultResult::TxFeeBumpInnerSuccess(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;























    #[test]
    fn times_and_strkeys() {
        assert_eq!(rfc3339(1_794_923_907), "2026-11-17T13:58:27Z");
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");

        let s = "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA";
        let id = parse_contract_strkey(s).unwrap();
        assert_eq!(contract_strkey(&id), s);
        assert_eq!(
            parse_contract_strkey("CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMB"),
            None
        );
        assert_eq!(
            parse_contract_strkey("GAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"),
            None
        );
    }
}
