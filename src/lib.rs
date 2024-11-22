#![feature(once_cell_get_mut)]
#![feature(custom_test_frameworks, test)]
#![test_runner(test_utils::test_runner)]

pub use getrandom;

pub mod io;
pub mod macros;
pub mod rand;
pub mod test_utils;
