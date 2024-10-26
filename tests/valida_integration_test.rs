#![feature(custom_test_frameworks, test)]
#![test_runner(entrypoint::test_utils::test_runner)]

#[test]
fn test_integration() {
    entrypoint::io::println("Running integration test...");
    assert_eq!(1 + 1, 2);
}

#[test]
#[should_panic]
fn test_integration_fail() {
    entrypoint::io::println("Running integration test fail...");
    assert_eq!(1 + 1, 3);
}

#[test]
#[ignore]
fn test_integration_ignore() {
    entrypoint::io::println("Running integration test ignore...");
    assert_eq!(1 + 1, 2);
}
