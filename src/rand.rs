use std::cell::OnceCell;

use rand::{rngs::StdRng, Rng, SeedableRng};

/// The random number generator seed for the zkVM.
///
/// In the future, we can try and take this seed from the input tape.
const PRNG_SEED: u64 = 0xdeadbeefdeadbeef;

/// A mutable static to generate a global random number generator.
static mut RNG: OnceCell<StdRng> = OnceCell::new();

/// Generates random bytes.
pub fn delendum_rand(s: &mut [u8]) -> Result<(), getrandom::Error> {
    unsafe {
        let rng = RNG.get_mut_or_init(|| StdRng::seed_from_u64(PRNG_SEED));
        for i in 0..s.len() {
            s[i] = rng.gen();
        }
    }

    Ok(())
}
