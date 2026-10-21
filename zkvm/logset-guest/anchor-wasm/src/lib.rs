//! The browser verifier's block confirmation, compiled to wasm32 for the page's Web Worker: the
//! JS face of `anchor_verify::Anchor` (`web/verify/TRUSTLESS-ANCHOR.md`).

use wasm_bindgen::prelude::*;

/// `new Anchor(checkpointRoot, checkpointSlot)`, then `restore`, `bootstrap`, `updates` and
/// `verify` as the page fetches what `verify` says it needs. Every method returns JSON.
#[wasm_bindgen]
pub struct Anchor(anchor_verify::Anchor);

#[wasm_bindgen]
impl Anchor {
    #[wasm_bindgen(constructor)]
    pub fn new(checkpoint_root: &str, checkpoint_slot: f64) -> Anchor {
        Anchor(anchor_verify::Anchor::new(checkpoint_root, checkpoint_slot))
    }

    pub fn restore(&mut self, saved: &str, now: f64) -> String {
        self.0.restore(saved, now)
    }

    pub fn bootstrap(&mut self, response: &str, now: f64) -> String {
        self.0.bootstrap(response, now)
    }

    pub fn updates(&mut self, response: &str) -> String {
        self.0.updates(response)
    }

    pub fn verify(
        &mut self,
        chain_id: u32,
        number: f64,
        hash: &str,
        response: &str,
        now: f64,
    ) -> String {
        self.0.verify(chain_id, number, hash, response, now)
    }

    pub fn saved(&self) -> String {
        self.0.saved()
    }

    pub fn reset(&mut self) {
        self.0.reset()
    }
}
