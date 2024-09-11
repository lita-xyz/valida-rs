#![no_main]

use entrypoint::rand::delendum_rand;
use getrandom::register_custom_getrandom;

register_custom_getrandom!(delendum_rand);

#[no_mangle]
fn main() {
}
