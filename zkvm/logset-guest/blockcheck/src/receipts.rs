












use anyhow::{bail, ensure, Context, Result};





#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}





pub use logset_chains::{
    DEPOSIT_TX_TYPE, FEE_CURRENCY_TX_TYPE, NITRO_TX_TYPES, STATE_SYNC_TX_TYPE,
};

pub fn deposits_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).deposits
}

pub fn nitro_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).nitro
}

pub fn fee_currency_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).fee_currency
}


pub fn state_sync_allowed(chain_id: u64) -> bool {
    logset_chains::STATE_SYNC_CHAIN_IDS.contains(&chain_id)
}



pub fn pol_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).pol
}






































































































































































































































































































pub fn receipts_root(values: &[Vec<u8>]) -> Result<[u8; 32]> {
    crate::trie::ordered_root(values)
}






pub fn rlp_item(b: &[u8]) -> Result<(&[u8], usize, bool)> {
    let first = *b.first().context("RLP: empty input")?;
    let (hdr, len, is_list) = match first {
        0x00..=0x7f => (0usize, 1usize, false),
        0x80..=0xb7 => (1, (first - 0x80) as usize, false),
        0xb8..=0xbf => long(b, first - 0xb7, false)?,
        0xc0..=0xf7 => (1, (first - 0xc0) as usize, true),
        0xf8..=0xff => long(b, first - 0xf7, true)?,
    };
    ensure!(
        len <= b.len().saturating_sub(hdr),
        "RLP: item runs past the end"
    );
    Ok((&b[hdr..hdr + len], hdr + len, is_list))
}

fn long(b: &[u8], nlen: u8, is_list: bool) -> Result<(usize, usize, bool)> {
    let nlen = nlen as usize;
    ensure!(b.len() > nlen, "RLP: truncated length prefix");
    let lb = &b[1..1 + nlen];
    ensure!(lb[0] != 0, "RLP: non-minimal length prefix");
    let mut len = 0usize;
    for &x in lb {
        len = len
            .checked_mul(256)
            .and_then(|v| v.checked_add(x as usize))
            .context("RLP: length overflow")?;
    }
    ensure!(len >= 56, "RLP: non-minimal long form");
    Ok((1 + nlen, len, is_list))
}


pub fn rlp_list_items(payload: &[u8]) -> Result<Vec<(&[u8], bool)>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < payload.len() {
        let (p, total, is_list) = rlp_item(&payload[at..])?;
        out.push((p, is_list));
        at += total;
    }
    Ok(out)
}

fn fixed<const N: usize>(b: &[u8], what: &str) -> Result<[u8; N]> {
    ensure!(b.len() == N, "{what}: expected {N} bytes, got {}", b.len());
    Ok(b.try_into().expect("length checked"))
}









pub fn logs_from_value(value: &[u8], chain_id: u64) -> Result<Vec<Log>> {
    let first = *value.first().context("empty receipt value")?;
    let deposit = first == DEPOSIT_TX_TYPE && deposits_allowed(chain_id);
    let nitro = NITRO_TX_TYPES.contains(&first) && nitro_allowed(chain_id);
    let fee_currency = first == FEE_CURRENCY_TX_TYPE && fee_currency_allowed(chain_id);
    let pol = first == DEPOSIT_TX_TYPE && pol_allowed(chain_id);
    let state_sync = first == STATE_SYNC_TX_TYPE && state_sync_allowed(chain_id);
    let body = match first {
        0x01..=0x04 => &value[1..],
        _ if deposit || nitro || fee_currency || pol || state_sync => &value[1..],
        0x00 => bail!("receipt value starts with a 0x00 type byte"),
        0x05..=0x7f => bail!("unsupported EIP-2718 receipt type 0x{first:02x}"),
        _ => value,
    };
    let (payload, total, is_list) = rlp_item(body)?;
    ensure!(is_list, "receipt body is not an RLP list");
    ensure!(
        total == body.len(),
        "receipt body has {} trailing bytes",
        body.len() - total
    );
    let fields = rlp_list_items(payload)?;
    if deposit {
        ensure!(
            (4..=6).contains(&fields.len()),
            "deposit receipt has {} fields, expected 4..=6",
            fields.len()
        );
        for (f, is_list) in &fields[4..] {
            ensure!(
                !is_list && f.len() <= 8 && f.first() != Some(&0),
                "deposit nonce/version is not a canonical u64"
            );
        }
    } else if fee_currency {
        ensure!(
            (4..=5).contains(&fields.len()),
            "fee-currency receipt has {} fields, expected 4 or 5",
            fields.len()
        );
        for (f, is_list) in &fields[4..] {
            ensure!(
                !is_list && f.len() <= 32 && f.first() != Some(&0),
                "a fee-currency receipt's baseFee is not a canonical scalar"
            );
        }
    } else {
        ensure!(
            fields.len() == 4,
            "receipt body has {} fields, expected 4",
            fields.len()
        );
    }
    let status = fields[0].0;

    ensure!(
        (!fields[0].1 && status.len() <= 1) || status.len() == 32,
        "receipt field 0 is neither a status scalar nor a 32-byte postState"
    );
    let _ = fixed::<256>(fields[2].0, "logsBloom")?;
    ensure!(fields[3].1, "receipt logs field is not a list");

    let mut out = Vec::new();
    for (log_payload, is_list) in rlp_list_items(fields[3].0)? {
        ensure!(is_list, "log is not an RLP list");
        let l = rlp_list_items(log_payload)?;
        ensure!(l.len() == 3, "log has {} fields, expected 3", l.len());
        let address = fixed::<20>(l[0].0, "log address")?;
        ensure!(l[1].1, "log topics field is not a list");
        let mut topics = Vec::new();
        for (t, tl) in rlp_list_items(l[1].0)? {
            ensure!(!tl, "a topic is an RLP list");
            topics.push(fixed::<32>(t, "topic")?);
        }
        ensure!(
            topics.len() <= 4,
            "log has {} topics, at most 4 allowed",
            topics.len()
        );
        out.push(Log {
            address,
            topics,
            data: l[2].0.to_vec(),
        });
    }
    Ok(out)
}






















































































































































































































































































































































































































