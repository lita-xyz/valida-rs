#![feature(custom_test_frameworks, test)]
#![test_runner(valida_rs::test_utils::test_runner)]

#[test]
fn test_integration() {
    valida_rs::io::println("Running integration test...");
    assert_eq!(1 + 1, 2);
}

#[test]
#[should_panic]
fn test_integration_fail() {
    valida_rs::io::println("Running integration test fail...");
    assert_eq!(1 + 1, 3);
}

#[test]
#[ignore]
fn test_integration_ignore() {
    valida_rs::io::println("Running integration test ignore...");
    assert_eq!(1 + 1, 3);
}

#[test]
#[ignore]
fn test_panics_on_valida() {
    #[allow(unexpected_cfgs)]
    if cfg!(target_arch = "delendum") {
        panic!("This test will panic on valida, but not on the native host");
    }
}
