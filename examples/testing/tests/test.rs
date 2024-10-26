//! These are unit tests in a library.
#![feature(custom_test_frameworks, test)]
#![test_runner(entrypoint::test_utils::test_runner)]

#[test]
fn test_integration() {
    assert_eq!(3 + 3, 6);
}
