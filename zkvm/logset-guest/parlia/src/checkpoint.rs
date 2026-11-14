//! Pinned checkpoints: a block hash the light client starts from, 200 blocks past an epoch
//! block (geth's trusted-snapshot offset), whose named set is then in force.

use crate::header::Header;
use crate::snapshot::Snapshot;
use crate::spec::Spec;
use crate::Error;
use alloy_primitives::{b256, B256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub epoch_length: u64,
}

impl Checkpoint {
    /// The epoch block whose set is in force at the checkpoint.
    pub fn epoch(&self) -> u64 {
        self.number - self.number % self.epoch_length
    }

    /// The snapshot at the checkpoint from its headers `epoch()..=number`, oldest first, each
    /// the parent of the next and the last hashing to the pinned hash.
    pub fn snapshot(&self, spec: &Spec, headers: &[(Header, B256)]) -> Result<Snapshot, Error> {
        let (first, last) = match (headers.first(), headers.last()) {
            (Some(f), Some(l)) => (f, l),
            _ => return Err(Error::Checkpoint("no headers".into())),
        };
        if spec.chain_id != self.chain_id
            || first.0.number != self.epoch()
            || last.0.number != self.number
            || last.1 != self.hash
        {
            return Err(Error::Checkpoint(format!(
                "headers {}..={} do not lead to checkpoint {} {}",
                first.0.number, last.0.number, self.number, self.hash
            )));
        }
        for w in headers.windows(2) {
            if w[1].0.number != w[0].0.number + 1 || w[1].0.parent_hash != w[0].1 {
                return Err(Error::Checkpoint(format!(
                    "block {} is not the parent of the next",
                    w[0].0.number
                )));
            }
        }
        Snapshot::trusted(spec, &first.0, &last.0, last.1, self.epoch_length)
    }
}

/// BNB Smart Chain mainnet, block 133,195,200 (2026-11-12), the same hash at four endpoints of
/// three operators (publicnode, defibit, ninicoin), 200 blocks past epoch block 133,195,000
/// (0x676b2974…3d37ca).
pub const MAINNET: Checkpoint = Checkpoint {
    chain_id: crate::spec::BSC_MAINNET,
    number: 133_195_200,
    hash: b256!("836ab32951e654537ae9a6c708d581bfb694225fa96d93f124bb624ef5c96adc"),
    epoch_length: crate::spec::MAXWELL_EPOCH,
};
