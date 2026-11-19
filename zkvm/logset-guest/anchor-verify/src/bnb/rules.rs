

























pub fn check_len(validators: usize, turn_length: u64) -> u64 {
    (validators as u64 / 2 + 1) * turn_length - 1
}
