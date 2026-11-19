
































use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, VerifyingKey};
use stellar_xdr::{
    GeneralizedTransactionSet, LedgerHeader, Limits, NodeId, PublicKey, ScpEnvelope, ScpQuorumSet,
    ScpStatementPledges, StellarValue, WriteXdr,
};

use crate::blob::AnchorWitness;
use crate::hash::sha256;
use crate::{decode, Error, NETWORK_ID};




const ENVELOPE_TYPE_SCP: i32 = 1;

pub struct AnchorOutput {

    pub ledger: u32,

    pub ledger_hash: [u8; 32],
    pub qset_hash: [u8; 32],
    pub next_qset_hash: [u8; 32],

    pub signed_entries: u64,
    pub total_entries: u64,

}

fn node_key(n: &NodeId) -> [u8; 32] {
    let PublicKey::PublicKeyTypeEd25519(k) = &n.0;
    k.0
}



pub fn satisfies(q: &ScpQuorumSet, nodes: &BTreeSet<[u8; 32]>) -> bool {
    if q.threshold == 0 {
        return false;
    }
    let mut n: u32 = 0;
    for v in q.validators.iter() {
        if nodes.contains(&node_key(v)) {
            n += 1;
        }
    }
    for inner in q.inner_sets.iter() {
        if satisfies(inner, nodes) {
            n += 1;
        }
    }
    n >= q.threshold
}


pub fn satisfied_entries(q: &ScpQuorumSet, nodes: &BTreeSet<[u8; 32]>) -> u64 {
    let v = q
        .validators
        .iter()
        .filter(|v| nodes.contains(&node_key(v)))
        .count();
    let i = q.inner_sets.iter().filter(|s| satisfies(s, nodes)).count();
    (v + i) as u64
}



fn quorum_core(
    declared: &BTreeMap<[u8; 32], [u8; 32]>,
    known: &BTreeMap<[u8; 32], &ScpQuorumSet>,
) -> BTreeSet<[u8; 32]> {
    let mut u: BTreeSet<[u8; 32]> = declared
        .iter()
        .filter(|(_, h)| known.contains_key(*h))
        .map(|(n, _)| *n)
        .collect();
    loop {
        let drop: Vec<[u8; 32]> = u
            .iter()
            .filter(|n| !satisfies(known[&declared[*n]], &u))
            .copied()
            .collect();
        if drop.is_empty() {
            return u;
        }
        for n in drop {
            u.remove(&n);
        }
    }
}


const MAX_QSET_DEPTH: u32 = 4;
const MAX_QSET_NODES: usize = 1000;





pub fn sanity(q: &ScpQuorumSet) -> Result<(), Error> {
    fn walk(q: &ScpQuorumSet, depth: u32, seen: &mut BTreeSet<[u8; 32]>) -> Result<(), Error> {
        if depth > MAX_QSET_DEPTH {
            return Err(Error::QuorumSetInsane("nesting"));
        }
        let entries = q.validators.len() + q.inner_sets.len();
        if q.threshold == 0 || q.threshold as usize > entries {
            return Err(Error::QuorumSetInsane("threshold"));
        }
        for v in q.validators.iter() {
            if !seen.insert(node_key(v)) {
                return Err(Error::QuorumSetInsane("duplicate validator"));
            }
        }
        if seen.len() > MAX_QSET_NODES {
            return Err(Error::QuorumSetInsane("too many validators"));
        }
        for inner in q.inner_sets.iter() {
            walk(inner, depth + 1, seen)?;
        }
        Ok(())
    }
    walk(q, 0, &mut BTreeSet::new())
}

pub fn run_anchor(blob: &[u8]) -> Result<AnchorOutput, Error> {
    let w = AnchorWitness::decode(blob)?;
    let net = w.network_id;
    if net != NETWORK_ID {
        return Err(Error::Network);
    }
    let slot = (w.ledger as u64) + 1;

    let qh = sha256(&w.qset);
    let q: ScpQuorumSet = decode(&w.qset, "ScpQuorumSet")?;
    sanity(&q)?;
    let next: Option<([u8; 32], ScpQuorumSet)> = if w.next_qset.is_empty() {
        None
    } else {
        let nq: ScpQuorumSet = decode(&w.next_qset, "ScpQuorumSet")?;
        sanity(&nq)?;
        Some((sha256(&w.next_qset), nq))
    };


    let ledger_hash = sha256(&w.header);
    let hdr: LedgerHeader = decode(&w.header, "LedgerHeader")?;
    if hdr.ledger_seq != w.ledger {
        return Err(Error::AnchorLink("header ledger"));
    }
    let ts_hash = sha256(&w.tx_set_next);
    let ts: GeneralizedTransactionSet = decode(&w.tx_set_next, "GeneralizedTransactionSet")?;
    let GeneralizedTransactionSet::V1(v1) = &ts;
    if v1.previous_ledger_hash.0 != ledger_hash {
        return Err(Error::AnchorLink("tx set does not name ledger A"));
    }



    let mut value: Option<Vec<u8>> = None;
    let mut declared: BTreeMap<[u8; 32], [u8; 32]> = BTreeMap::new();
    for (i, bytes) in w.envelopes.iter().enumerate() {
        let env: ScpEnvelope = decode(bytes, "ScpEnvelope")?;
        let st = &env.statement;
        if st.slot_index != slot {
            return Err(Error::ScpStatement("slot"));
        }
        let ScpStatementPledges::Externalize(ext) = &st.pledges else {
            return Err(Error::ScpStatement("not EXTERNALIZE"));
        };
        let v = ext.commit.value.0.to_vec();
        match &value {
            None => {
                let sv: StellarValue = decode(&v, "StellarValue")?;
                if sv.tx_set_hash.0 != ts_hash {
                    return Err(Error::AnchorLink("value names another tx set"));
                }
                value = Some(v);
            }
            Some(first) if *first != v => return Err(Error::ScpStatement("two values")),
            Some(_) => {}
        }
        let mut msg = Vec::with_capacity(32 + 4 + 256);
        msg.extend_from_slice(&net);
        msg.extend_from_slice(&ENVELOPE_TYPE_SCP.to_be_bytes());
        msg.extend_from_slice(
            &st.to_xdr(Limits::none())
                .map_err(|_| Error::Xdr("ScpStatement"))?,
        );
        let key = node_key(&st.node_id);
        let sig_bytes: [u8; 64] = env
            .signature
            .0
            .as_slice()
            .try_into()
            .map_err(|_| Error::ScpSignature { envelope: i as u32 })?;
        let vk = VerifyingKey::from_bytes(&key)
            .map_err(|_| Error::ScpSignature { envelope: i as u32 })?;
        vk.verify_strict(&msg, &Signature::from_bytes(&sig_bytes))
            .map_err(|_| Error::ScpSignature { envelope: i as u32 })?;
        if declared.insert(key, ext.commit_quorum_set_hash.0).is_some() {
            return Err(Error::ScpStatement("duplicate signer"));
        }
    }
    if value.is_none() {
        return Err(Error::NoQuorum);
    }


    let mut known: BTreeMap<[u8; 32], &ScpQuorumSet> = BTreeMap::new();
    known.insert(qh, &q);
    if let Some((nh, nq)) = &next {
        known.insert(*nh, nq);
    }
    let core = quorum_core(&declared, &known);
    if !satisfies(&q, &core) {
        return Err(Error::NoQuorum);
    }


    let mut next_hash = qh;
    if let Some((nh, nq)) = &next {
        let only: BTreeMap<[u8; 32], [u8; 32]> = declared
            .iter()
            .filter(|(_, h)| *h == nh)
            .map(|(n, h)| (*n, *h))
            .collect();
        let mut kn: BTreeMap<[u8; 32], &ScpQuorumSet> = BTreeMap::new();
        kn.insert(*nh, nq);
        let c = quorum_core(&only, &kn);
        if satisfies(&q, &c) && satisfies(nq, &c) {
            next_hash = *nh;
        }
    }

    let signed_entries = satisfied_entries(&q, &core);
    let total_entries = (q.validators.len() + q.inner_sets.len()) as u64;

















    Ok(AnchorOutput {

        ledger: w.ledger,

        ledger_hash,
        qset_hash: qh,
        next_qset_hash: next_hash,
        signed_entries,
        total_entries,

    })
}
