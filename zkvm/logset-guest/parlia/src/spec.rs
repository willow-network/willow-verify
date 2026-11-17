



pub const BSC_MAINNET: u64 = 56;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    pub chain_id: u64,

    pub luban_block: u64,

    pub plato_block: u64,

    pub london_block: u64,

    pub cancun_time: u64,

    pub bohr_time: u64,

    pub prague_time: u64,

    pub lorentz_time: u64,

    pub maxwell_time: u64,

    pub fermi_time: u64,

    pub amsterdam_time: Option<u64>,
}

pub const MAINNET: Spec = Spec {
    chain_id: BSC_MAINNET,
    luban_block: 29_020_050,
    plato_block: 30_720_096,
    london_block: 31_302_048,
    cancun_time: 1_718_863_500,
    bohr_time: 1_727_317_200,
    prague_time: 1_742_436_600,
    lorentz_time: 1_745_903_100,
    maxwell_time: 1_751_250_600,
    fermi_time: 1_768_357_800,
    amsterdam_time: None,
};




pub const MAXWELL_EPOCH: u64 = 1000;







impl Spec {
    pub fn is_luban(&self, number: u64) -> bool {
        number >= self.luban_block
    }










    pub fn is_bohr(&self, time: u64) -> bool {
        time >= self.bohr_time
    }

































}
