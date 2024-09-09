#![no_main]

use entrypoint::rand::delendum_rand;
use getrandom::register_custom_getrandom;

register_custom_getrandom!(delendum_rand);

extern { fn write_stdout(n: u32);}

#[no_mangle]
fn main() {
}
