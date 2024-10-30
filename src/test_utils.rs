//! This module contains a test runner that runs tests on both the host and the Valida VM.
//!
//! # Usage
//! For a full example see the `examples/testing` directory.
//!
//! Set the `test_runner` attribute in the root of each crate (`lib.rs`, `main.rs`, test.rs).
//! ```rust,ignore
//! #![feature(custom_test_frameworks, test)]
//! #![test_runner(valida_rs::test_utils::test_runner)]
//! // If your testing a binary crate, you will also need to add this:
//! #![cfg_attr(not(test), no_main)]
//! valida_rs::entrypoint!(main);
//! ```
//!
//! You can run tests on only the host by setting the `VALIDA_TEST` environment variable to `0`.
//!
//! # Caveats
//! Testing examples, benchmarks, or any dynamic tests are not supported yet.
extern crate test;
use std::{
    env,
    io::{self, BufRead, Write},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use test::{ShouldPanic, TestDescAndFn, TestFn};

pub fn test_runner(tests: &[&TestDescAndFn]) {
    #[allow(unexpected_cfgs)]
    if cfg!(target_arch = "delendum") | cfg!(target = "valida") {
        run_single_test_in_valida(tests);
    } else {
        host_runner(tests);
    }
}

fn host_runner(tests: &[&TestDescAndFn]) {
    let run_tests_on_valida = env::var("VALIDA_TEST").map(|s| s.to_lowercase());
    let run_tests_on_valida = match run_tests_on_valida {
        Ok(val) => val == "1" || val == "true" || val == "yes" || val == "on",
        Err(_) => true,
    };

    let test_paths = if run_tests_on_valida {
        crate::io::println("Building tests for valida");
        build_tests_for_valida()
    } else {
        vec![]
    };

    crate::io::println("Running tests host.");

    let mut passed = 0;
    let mut valida_passed = 0;
    let mut ignored = 0;
    let mut failed = 0;
    let mut valida_failed = 0;
    let mut unsupported = 0;

    // Get test filter from environment variable, just like rustc does
    let filter = env::args().nth(1);
    let filtered_tests: Vec<&&TestDescAndFn> = match &filter {
        Some(f) => tests
            .iter()
            .filter(|t| t.desc.name.as_slice().contains(f))
            .collect(),
        None => tests.iter().collect(),
    };

    if let Some(f) = &filter {
        println!("Running tests matching '{}'", f);
    }
    println!("running {} tests", filtered_tests.len());

    for t in filtered_tests.iter() {
        print!("test {} ... ", t.desc.name);

        if t.desc.ignore {
            println!("ignored");
            ignored += 1;
            continue;
        }

        let r = run_test_on_host(t);
        match r {
            TestOutcome::Passed(test_time) => {
                println!("ok");
                passed += 1;

                print!("test {} on valida ... ", t.desc.name);
                if run_tests_on_valida {
                    match run_test_on_valida(t, &test_paths, test_time) {
                        Ok(()) => {
                            println!("ok");
                            valida_passed += 1;
                        }
                        Err(msg) => {
                            println!("FAILED");
                            eprintln!("\n\ntest {} failure message: {}\n\n", t.desc.name, msg);
                            valida_failed += 1;
                        }
                    }
                }
            }
            TestOutcome::Failed(msg) => {
                println!("FAILED");
                eprintln!("\ntest {} on host failure message: {}", t.desc.name, msg);
                failed += 1;
            }
            TestOutcome::ShouldPanicButPassed => {
                println!("FAILED");
                eprintln!("\nfailure message: test did not panic as expected");
                failed += 1;
            }
            TestOutcome::Unsupported => {
                println!("unsupported");
                unsupported += 1;
            }
        }
    }

    if filtered_tests.is_empty() && filter.is_some() {
        println!("\nno tests matched filter '{}'", filter.unwrap());
    } else {
        println!(
            "\ntest result: {}\n\
            on native:      {passed} passed; {failed} failed\n\
            on valida:      {valida_passed} passed; {valida_failed} failed\n\
            {ignored} ignored;\n\
            {unsupported} unsupported",
            if failed == 0 { "ok" } else { "FAILED" },
        );
    }

    if failed > 0 || valida_failed > 0 {
        std::process::exit(1);
    }
}

#[derive(Debug)]
pub enum TestOutcome {
    Passed(Duration),
    Failed(String),
    ShouldPanicButPassed,
    Unsupported,
}

fn run_test_on_host(test: &TestDescAndFn) -> TestOutcome {
    match &test.testfn {
        TestFn::StaticTestFn(f) => {
            let start_time = Instant::now();

            // TODO instead of catching panics, run the tests in seprate processes
            // That would allow us to capture output by default like cargo test (see --nocapture).
            let result = panic::catch_unwind(AssertUnwindSafe(f));
            let duration = start_time.elapsed();

            match (result, &test.desc.should_panic) {
                // Test succeeded and wasn't supposed to panic
                (Ok(Ok(())), ShouldPanic::No) => TestOutcome::Passed(duration),

                // Test panicked and was supposed to panic
                (Err(_), ShouldPanic::Yes) => TestOutcome::Passed(duration),

                // Test panicked and was supposed to panic with specific message
                (Err(e), ShouldPanic::YesWithMessage(msg)) => {
                    let panic_msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.as_str()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s
                    } else {
                        "Unknown panic payload"
                    };

                    if panic_msg.contains(msg) {
                        TestOutcome::Passed(duration)
                    } else {
                        TestOutcome::Failed(format!(
                            "Expected panic message containing '{}', got '{}'",
                            msg, panic_msg
                        ))
                    }
                }

                // Test panicked but shouldn't have
                (Err(e), ShouldPanic::No) => {
                    TestOutcome::Failed(format!("Test panicked unexpectedly: {:?}", e))
                }

                // Test succeeded but should have panicked
                (Ok(Ok(())), ShouldPanic::Yes | ShouldPanic::YesWithMessage(_)) => {
                    TestOutcome::ShouldPanicButPassed
                }

                // Test returned Err - this means the test function itself failed
                (Ok(Err(e)), _) => TestOutcome::Failed(format!("Test returned error: {:?}", e)),
            }
        }

        _ => TestOutcome::Unsupported,
    }
}

/// Build tests for valida and return the test program paths.
///
/// # Panics
/// This function will panic if the cargo cannot build the tests.
fn build_tests_for_valida() -> Vec<PathBuf> {
    let mut command = Command::new("cargo");

    command
    .arg("+valida")
    .arg("test")
    .arg("--target=delendum-unknown-baremetal-gnu")
    .arg("--config")
    .arg("build.target=\"delendum-unknown-baremetal-gnu\"")
    .arg("--config")
    .arg("target.delendum-unknown-baremetal-gnu.runner=\"echo\"")
    .arg("--config") 
    .arg("target.delendum-unknown-baremetal-gnu.linker=\"/valida-toolchain/bin/ld.lld\"")
    .arg("--config")
     .arg(concat!(
        "target.delendum-unknown-baremetal-gnu.rustflags=[",
        "\"-C\",\"link-arg=/valida-toolchain/DelendumEntryPoint.o\",",
        "\"-C\",\"link-arg=--script=/valida-toolchain/valida.ld\",",
        "\"-C\",\"link-arg=/valida-toolchain/lib/delendum-unknown-baremetal-gnu/libc.a\",",
        "\"-C\",\"link-arg=/valida-toolchain/lib/delendum-unknown-baremetal-gnu/libm.a\",",
        "\"-C\",\"link-arg=--noinhibit-exec\"",
        "]"
    ))
    .arg("--config")
    .arg("env.CC_delendum_unknown_baremetal_gnu=\"/valida-toolchain/bin/clang\"")
    .arg("--config")
    .arg("env.CFLAGS_delendum_unknown_baremetal_gnu=\"--sysroot=/valida-toolchain/ -isystem /valida-toolchain/include\"")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // if the current exe path contains release use add --release arg
    if let Ok(path) = env::current_exe() {
        if path.to_str().unwrap().contains("/release/") {
            command.arg("--release");
        }
    }

    let output = command.spawn().unwrap().wait_with_output().unwrap();

    let paths = output
        .stdout
        .lines()
        .map(|line| line.unwrap())
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .collect();

    if output.status.success() {
        paths
    } else {
        panic!(
            "Failed to build tests for valida: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_test_on_valida(
    test: &TestDescAndFn,
    test_paths: &[PathBuf],
    host_test_time: Duration,
) -> Result<(), String> {
    if test_paths.is_empty() {
        return Err("No test binaries found for valida".to_string());
    }

    // Try to run the test on each of the test exes
    for test_path in test_paths.iter() {
        if run_test_on_valida_inner(test, test_path, host_test_time)? {
            return Ok(());
        }
    }

    Err(format!(
        "Test {} not found in any test binary\n looked in: {:?}",
        test.desc.name, test_paths
    ))
}

/// Run a single test on the Valida VM.
/// # Arguments
/// * `test` - The test to run.
/// * `test_path` - The path to the test binary to look for the test in.
/// * `host_test_time` - The time taken to run the test on the host.
///
/// # Returns
/// Err if the test did not have the expected outcome.
/// Ok(true) if the test passed.
/// Ok(false) if the test was not found in the provided test binary.
fn run_test_on_valida_inner(
    test: &TestDescAndFn,
    test_path: &Path,
    host_test_time: Duration,
) -> Result<bool, String> {
    let timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();

    let mut child = Command::new("valida")
        .arg("run")
        .arg(test_path)
        .arg(format!("/tmp/valida-test-log-{timestamp}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start test process: {}", e))?;

    let mut valida_stdin = child.stdin.take().unwrap();

    // The pipe may break if the process exits before we write to it.
    // This can happen if the test name/filename is not found in this test binary.
    let _ = writeln!(valida_stdin, "{}", test.desc.name);
    let _ = writeln!(valida_stdin, "{}", test.desc.source_file);

    let mut valida_stdout = io::BufReader::new(child.stdout.take().unwrap()).lines();

    // contains a list of available tests
    let first_line = valida_stdout.next();
    // the test you selected
    let second_line = valida_stdout.next();

    match (first_line, second_line) {
        (_first_line, Some(Ok(second_line))) => {
            if second_line != valida_test_second_line_stdout(test) {
                return Ok(false);
            }
        }
        (Some(Err(e)), _) | (_, Some(Err(e))) => {
            return Err(format!("Failed to read from test process: {}", e))
        }
        (_, None) => return Ok(false),
    };

    let timeout = std::cmp::max(host_test_time * 10, Duration::from_secs(10));
    let start_time = Instant::now();

    loop {
        match (child.try_wait(), &test.desc.should_panic) {
            (Ok(None), _) => {}
            (Ok(Some(status)), ShouldPanic::No) => {
                if status.success() {
                    return Ok(true);
                } else {
                    return Err(format!("Test failed with exit code: {:?}", status));
                }
            }
            (Ok(Some(status)), ShouldPanic::Yes | ShouldPanic::YesWithMessage(_)) => {
                if status.success() {
                    return Err("Test did not panic as expected".to_string());
                } else {
                    return Ok(true);
                }
            }
            (Err(e), _) => return Err(format!("Failed to wait for cargo process: {}", e)),
        }

        // Modified timeout handling code
        if start_time.elapsed() >= timeout {
            child
                .kill()
                .map_err(|e| format!("Failed to kill test process: {}", e))?;

            match &test.desc.should_panic {
                ShouldPanic::No => {
                    return Err(format!("Test timed out after {:?}", timeout));
                }
                ShouldPanic::Yes | ShouldPanic::YesWithMessage(_) => {
                    return Ok(true);
                }
            }
        }
    }
}

// Run's a single specified test.
// Get's the test name from first line of input.
fn run_single_test_in_valida(tests: &[&TestDescAndFn]) {
    crate::io::print("Available tests:");
    for t in tests.iter() {
        crate::io::print(&format!(" ({}, {})", t.desc.name, t.desc.source_file))
    }
    crate::io::println("");

    let Ok(test_name) = crate::io::read_line::<String>() else {
        // If no test name is provided the program will hang.
        // This is an upstream issue with read_line.
        return;
    };

    let Ok(test_file) = crate::io::read_line::<String>() else {
        return;
    };

    let test_name = test_name.trim();
    let test = tests
        .iter()
        .find(|t| t.desc.name.as_slice() == test_name && t.desc.source_file == test_file);

    if let Some(test) = test {
        crate::io::println(valida_test_second_line_stdout(test).as_str());
        // TODO catch panics once that works on valida
        // return 0 or 1 exit code based on test outcome
        // there's no point in doing this now since panic will cause an infinite loop
        // The loop has to be detected by the host test runner.
        // If the test takes 10x longer than expected, we can assume it has panicked.
        //
        // TODO support other test types
        if let TestFn::StaticTestFn(f) = test.testfn {
            let _ = f();
        }
    }
}

fn valida_test_second_line_stdout(test: &TestDescAndFn) -> String {
    format!("Running test: {} in valida vm", test.desc.name)
}

#[test]
fn test_unit_test_in_lib() {
    assert_eq!(1, 1);
}
