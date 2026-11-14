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

    /// Where light client `light_client` (`"bnb"`) starts, in place of the pin built in.
    pub fn start(&mut self, light_client: &str, start: &str) -> String {
        self.0.start(light_client, start)
    }

    #[wasm_bindgen(js_name = bootstrapFor)]
    pub fn bootstrap_for(&mut self, light_client: &str, response: &str, now: f64) -> String {
        self.0.bootstrap_for(light_client, response, now)
    }

    #[wasm_bindgen(js_name = updatesFor)]
    pub fn updates_for(&mut self, light_client: &str, response: &str) -> String {
        self.0.updates_for(light_client, response)
    }

    #[wasm_bindgen(js_name = restoreFor)]
    pub fn restore_for(&mut self, light_client: &str, saved: &str, now: f64) -> String {
        self.0.restore_for(light_client, saved, now)
    }

    #[wasm_bindgen(js_name = savedFor)]
    pub fn saved_for(&self, light_client: &str) -> String {
        self.0.saved_for(light_client)
    }

    #[wasm_bindgen(js_name = resetFor)]
    pub fn reset_for(&mut self, light_client: &str) {
        self.0.reset_for(light_client)
    }
}
