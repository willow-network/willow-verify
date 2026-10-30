//! The verify page's OpenVM log module: [`OpenVmVerifier`] checks one OpenVM proof under the
//! programs pinned in `willow_logs_verify_openvm::pinned` (the canonical mint's leaf, L1 and fold,
//! under `pins/vk/`), and nothing else. The page's SP1 module decides which key each proof is
//! checked under and whether its system is right for its blocks, and calls [`OpenVmVerifier::check`]
//! through its `checkOpenVm` callback.

use logset_query::ProofCheck;
use serde_json::json;
use wasm_bindgen::prelude::*;
use willow_logs_verify_openvm::{
    pinned::{self, Kind},
    OvmCheck,
};

/// The JS face: `new OpenVmVerifier()` loads the pinned verifying keys once.
#[wasm_bindgen]
pub struct OpenVmVerifier {
    check: Result<OvmCheck, String>,
}

impl Default for OpenVmVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl OpenVmVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new() -> OpenVmVerifier {
        OpenVmVerifier {
            check: pinned::check(),
        }
    }

    /// The minted program keys it verifies under: `{"ok": true, "leaf", "l1", "fold"}` (hex),
    /// or `{"ok": false, "message"}` when the pinned keys did not load.
    pub fn keys(&self) -> String {
        let keys = || -> Result<serde_json::Value, String> {
            self.check.as_ref().map_err(Clone::clone)?;
            let k = |kind| pinned::key(kind).map(hex::encode);
            Ok(
                json!({"ok": true, "leaf": k(Kind::Leaf)?, "l1": k(Kind::L1)?, "fold": k(Kind::Fold)?}),
            )
        };
        keys()
            .unwrap_or_else(|e| json!({"ok": false, "message": e}))
            .to_string()
    }

    /// `undefined` when `proof` (a stored OpenVM `.proof`) proves exactly `public_values` under
    /// the pinned program whose key is `key`; else why not.
    pub fn check(&mut self, key: &[u8], proof: &[u8], public_values: &[u8]) -> Option<String> {
        let check = match &mut self.check {
            Ok(c) => c,
            Err(e) => return Some(format!("the pinned OpenVM keys did not load: {e}")),
        };
        let Ok(key) = <[u8; 32]>::try_from(key) else {
            return Some("a program key is 32 bytes".into());
        };
        check.check(&key, proof, public_values).err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_under_the_minted_keys_only() {
        let mut v = OpenVmVerifier::new();
        let k: serde_json::Value = serde_json::from_str(&v.keys()).unwrap();
        assert_eq!(k["ok"], json!(true));
        assert_eq!(
            k["leaf"],
            json!("28b819642cb4cbd26da3b3762408c71656423fd4267bb71e12a0c76e08490119")
        );
        assert_eq!(
            k["l1"],
            json!("3b585e4468e56c4e4ee629bb67c85e26328301473b4e0d611dde318e47585789")
        );
        let leaf = pinned::key(Kind::Leaf).unwrap();
        assert!(v
            .check(&leaf, b"OVM2", b"")
            .unwrap()
            .starts_with("proof bytes"));
        assert!(v
            .check(&[7; 32], b"OVM2", b"")
            .unwrap()
            .starts_with("no OpenVM program"));
        assert_eq!(
            v.check(&leaf[..31], b"OVM2", b"").unwrap(),
            "a program key is 32 bytes"
        );
        assert_eq!(
            v.check(&leaf, &[1, 0, 0, 0], b"").unwrap(),
            "not an OpenVM proof"
        );
    }
}
