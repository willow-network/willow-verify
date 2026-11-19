



use op_revm::precompiles::{karst, OpPrecompiles};
use op_revm::OpSpecId;
use revm::context::Cfg;
use revm::context_interface::ContextTr;
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::interpreter::{CallInputs, InterpreterResult};
use revm::precompile::{
    bn254, eth_precompile_fn, EthPrecompileResult, Precompile, PrecompileHalt, PrecompileId,
    Precompiles,
};
use revm::primitives::hardfork::SpecId;
use revm::primitives::{Address, OnceLock};


pub const KARST_PAIRING_MAX_INPUT_SIZE: usize = 57_600;

fn run_pair_karst(input: &[u8], gas_limit: u64) -> EthPrecompileResult {
    if input.len() > KARST_PAIRING_MAX_INPUT_SIZE {
        return Err(PrecompileHalt::Bn254PairLength);
    }
    bn254::run_pair(
        input,
        bn254::pair::ISTANBUL_PAIR_PER_POINT,
        bn254::pair::ISTANBUL_PAIR_BASE,
        gas_limit,
    )
}

eth_precompile_fn!(karst_pair, run_pair_karst);


fn karst_set() -> &'static Precompiles {
    static SET: OnceLock<Precompiles> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = karst().clone();
        let mut pairing = Precompiles::default();
        pairing.extend([op_revm::precompiles::bn254_pair::JOVIAN]);
        set.difference(&pairing);
        set.extend([Precompile::new(
            PrecompileId::Bn254Pairing,
            bn254::pair::ADDRESS,
            karst_pair,
        )]);
        set
    })
}


#[derive(Clone, Debug)]
pub struct OpPrecompileSet {
    inner: EthPrecompiles,
    spec: OpSpecId,
}

impl OpPrecompileSet {
    pub fn new(spec: OpSpecId) -> Self {
        let precompiles = match spec {
            OpSpecId::KARST | OpSpecId::INTEROP => karst_set(),
            s => OpPrecompiles::new_with_spec(s).precompiles(),
        };
        Self {
            inner: EthPrecompiles {
                precompiles,
                spec: SpecId::default(),
            },
            spec,
        }
    }


    pub fn holds(&self, address: &Address) -> bool {
        self.inner.contains(address)
    }
}

impl<CTX> PrecompileProvider<CTX> for OpPrecompileSet
where
    CTX: ContextTr<Cfg: Cfg<Spec = OpSpecId>>,
{
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: OpSpecId) -> bool {
        if spec == self.spec {
            return false;
        }
        *self = Self::new(spec);
        true
    }

    fn run(
        &mut self,
        context: &mut CTX,
        inputs: &CallInputs,
    ) -> Result<Option<InterpreterResult>, String> {
        self.inner.run(context, inputs)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        self.inner.warm_addresses()
    }

    fn contains(&self, address: &Address) -> bool {
        self.inner.contains(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn karst_caps_the_pairing_and_keeps_the_rest() {
        let capped = OpPrecompileSet::new(OpSpecId::KARST);
        let jovian = OpPrecompileSet::new(OpSpecId::JOVIAN);
        let pair = |s: &OpPrecompileSet, n: usize| {
            let p = s.inner.precompiles.get(&bn254::pair::ADDRESS).unwrap();
            p.execute(&vec![0u8; n], u64::MAX, 0).unwrap()
        };

        assert!(pair(&capped, 57_600).halt_reason().is_none());
        assert!(pair(&capped, 57_792).halt_reason().is_some());
        assert!(pair(&jovian, 57_792).halt_reason().is_none());
        assert_eq!(
            capped.inner.precompiles.addresses().count(),
            karst().addresses().count()
        );
        for a in karst().addresses() {
            assert!(capped.holds(a), "{a}");
        }
    }
}
