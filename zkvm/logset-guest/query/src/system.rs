//! Which proof system made a stored `.proof`.
//!
//! Proof ids and store names say nothing about the system, so the bytes do. An SP1 `.proof`
//! is the bincode of `SP1ProofWithPublicValues`, whose first field is the `SP1Proof` enum: it
//! starts with the variant index as a u32 LE, 0 to 3 (a compressed proof starts `01 00 00 00`).
//! An OpenVM `.proof` starts with [`OPENVM_PROOF_MAGIC`], which read as a u32 LE is far above
//! 3, and then holds the zstd of the `VmStarkProof` encoding, the bytes
//! `openvm_verify_stark_host::verify_vm_stark_proof` takes.

/// The first four bytes of every OpenVM `.proof`.
pub const OPENVM_PROOF_MAGIC: [u8; 4] = *b"OVM2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofSystem {
    Sp1,
    OpenVm,
}

impl ProofSystem {
    /// The system a stored `.proof` comes from, or `None` for bytes that are neither.
    pub fn of(proof: &[u8]) -> Option<Self> {
        let head: [u8; 4] = proof.get(..4)?.try_into().ok()?;
        if head == OPENVM_PROOF_MAGIC {
            Some(Self::OpenVm)
        } else if u32::from_le_bytes(head) <= 3 {
            Some(Self::Sp1)
        } else {
            None
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sp1 => "sp1",
            Self::OpenVm => "openvm",
        }
    }

    /// The name people know it by: `SP1` or `OpenVM`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sp1 => "SP1",
            Self::OpenVm => "OpenVM",
        }
    }
}

impl std::fmt::Display for ProofSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The OpenVM proof body after its magic, or `None` if `proof` is not an OpenVM `.proof`.
pub fn openvm_body(proof: &[u8]) -> Option<&[u8]> {
    proof.strip_prefix(&OPENVM_PROOF_MAGIC[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_the_two_systems_apart_and_refuses_anything_else() {
        assert_eq!(ProofSystem::of(&[1, 0, 0, 0, 9, 9]), Some(ProofSystem::Sp1));
        assert_eq!(ProofSystem::of(&[3, 0, 0, 0]), Some(ProofSystem::Sp1));
        assert_eq!(ProofSystem::of(b"OVM2\x28\xb5\x2f\xfd"), Some(ProofSystem::OpenVm));
        assert_eq!(ProofSystem::of(&[4, 0, 0, 0]), None);
        assert_eq!(ProofSystem::of(b"OVM1...."), None);
        assert_eq!(ProofSystem::of(&[1, 0, 0]), None);
        // The magic itself is not a valid SP1 variant index.
        assert!(u32::from_le_bytes(OPENVM_PROOF_MAGIC) > 3);
        assert_eq!(openvm_body(b"OVM2abc"), Some(&b"abc"[..]));
        assert_eq!(openvm_body(&[1, 0, 0, 0]), None);
    }
}
