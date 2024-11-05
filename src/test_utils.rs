//! This module contains a test runner that runs tests on both the host and the Valida VM.
//!
//! # Usage
//! For a full example see the `examples/testing` directory.
//!
//! Make sure you have the `valida` command in your `$PATH`.
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

#![allow(unexpected_cfgs)]

extern crate test;

use std::io::Read;
#[cfg_attr(target_arch = "delendum", allow(unused_imports))]
use std::{
    env,
    io::{self, BufRead, Seek, Write},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[cfg_attr(target_arch = "delendum", allow(unused_imports))]
use test::{ShouldPanic, TestDescAndFn, TestFn};

/// A random sentinel value is printed by the panic hook.
/// This is used to detect if a test running in valida has panicked.
pub const MAGIC_TERMINATOR: &str = "valida_rs_terminator_YMYGE2otWHIAZ5IKtvTkCnt7B/aNTisJtmkNu9/H0C2pZp7XTeGIO2RZypwus7wvKyG9f4/nwrEP1vEy+YJJqS6ulJqks25EgHbZXQIZIWVfVK+HgmFvaINl49axeKZgk2SNIDAayGhmO5a0okHc9qFzOZhDIblXdybCoVCVaZfX/5G9T4FbbX8ktLV0nLI/nns1fakApi2eHTxP/+lWlXFznl+eipFNQg9h3ZS7VX6i3EGTOYO86TJmAUyLAfqKWuQFTvNHeFFofd4nhUiek2FuI939T3L5uFc7A9oQClGmLTSaGytDNT8slxuaRvQM99ntk+CLK+X8eNVQdKh0xA";

pub fn test_runner(tests: &[&TestDescAndFn]) {
    #[allow(unexpected_cfgs)]
    if cfg!(target_arch = "delendum") | cfg!(target = "valida") {
        run_single_test_in_valida(tests);
    } else {
        #[cfg(not(target_arch = "delendum"))]
        host_runner(tests);
    }
}

#[cfg(not(target_arch = "delendum"))]
fn host_runner(tests: &[&TestDescAndFn]) {
    let run_tests_on_valida = env::var("VALIDA_TEST").map(|s| s.to_lowercase());
    let run_tests_on_valida = match run_tests_on_valida {
        Ok(val) => val == "1" || val == "true" || val == "yes" || val == "on",
        Err(_) => true,
    };

    let test_paths = if run_tests_on_valida {
        println!("Building tests for valida");
        build_tests_for_valida()
    } else {
        vec![]
    };

    let mut passed = 0;
    let mut valida_passed = 0;
    let mut ignored = 0;
    let mut failed = 0;
    let mut valida_failed = 0;
    let mut unsupported = 0;

    // Get test filter from environment variable, just like rustc does
    let filter = env::args().skip(1).find(|arg| !arg.starts_with('-'));

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
        print!("test {} on native ... ", t.desc.name);

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

                if run_tests_on_valida {
                    print!("test {} on valida ... ", t.desc.name);
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
                eprintln!("\ntest {} on native failure message: {}", t.desc.name, msg);
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
            {unsupported} unsupported\n\n",
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

#[cfg(not(target_arch = "delendum"))]
fn run_test_on_host(test: &TestDescAndFn) -> TestOutcome {
    use std::os::fd::AsRawFd;

    match &test.testfn {
        TestFn::StaticTestFn(f) => {
            let start_time = Instant::now();

            let mut tempfile = tempfile::tempfile().expect("Failed to create tempfile");

            let g1 =
                gag::Redirect::stdout(tempfile.as_raw_fd()).expect("Failed to redirect stdout");
            let g2 =
                gag::Redirect::stderr(tempfile.as_raw_fd()).expect("Failed to redirect stderr");

            let result = panic::catch_unwind(AssertUnwindSafe(f));

            drop(g1);
            drop(g2);

            let log_test_failure = move || {
                eprintln!("\n\nTest {} failed on native, output:\n\n", test.desc.name);

                tempfile.seek(std::io::SeekFrom::Start(0)).unwrap();
                std::io::BufReader::new(tempfile)
                    .lines()
                    .for_each(|line| eprintln!("{}", line.expect("Failed to read line")));
            };

            let duration = start_time.elapsed();

            match (result, &test.desc.should_panic) {
                // Test succeeded and wasn't supposed to panic
                (Ok(Ok(())), ShouldPanic::No) => TestOutcome::Passed(duration),

                // Test panicked and was supposed to panic
                (Err(_), ShouldPanic::Yes) => TestOutcome::Passed(duration),

                // Test panicked and was supposed to panic with specific message
                (Err(e), ShouldPanic::YesWithMessage(msg)) => {
                    let panic_msg = e
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| e.downcast_ref::<&str>().copied());

                    if panic_msg.map(|s| s.contains(msg)).unwrap_or(false) {
                        TestOutcome::Passed(duration)
                    } else {
                        log_test_failure();
                        TestOutcome::Failed(format!(
                            "Expected panic message containing '{}', got '{}'",
                            msg,
                            panic_msg.unwrap_or("Non string panic value")
                        ))
                    }
                }

                // Test panicked but shouldn't have
                (Err(_e), ShouldPanic::No) => {
                    log_test_failure();
                    TestOutcome::Failed("Test panicked unexpectedly".to_string())
                }

                // Test succeeded but should have panicked
                (Ok(Ok(())), ShouldPanic::Yes | ShouldPanic::YesWithMessage(_)) => {
                    log_test_failure();
                    TestOutcome::ShouldPanicButPassed
                }

                // Test returned Err - this means the test function itself failed
                (Ok(Err(_e)), _) => {
                    log_test_failure();
                    TestOutcome::Failed("Test returned error: {:?}".to_string())
                }
            }
        }

        _ => TestOutcome::Unsupported,
    }
}

/// Build tests for valida and return the test program paths.
///
/// # Panics
/// This function will panic if the cargo cannot build the tests.
#[cfg(not(target_arch = "delendum"))]
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

#[cfg(not(target_arch = "delendum"))]
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
///
/// # Panics
/// If the `valida` command cannot be found in the `$PATH`.
/// Or if the `valida` command cannot be started.
#[cfg(not(target_arch = "delendum"))]
fn run_test_on_valida_inner(
    test: &TestDescAndFn,
    test_path: &Path,
    host_test_time: Duration,
) -> Result<bool, String> {
    let temp_log = tempfile::NamedTempFile::new().expect("Failed to create temp log file");
    let temp_log_path = temp_log.path();

    // We call try_wait() the process in a loop or kill it after a timeout, so this warning is erroneous.
    #[allow(clippy::zombie_processes)]
    let mut child = Command::new("valida")
        .arg("run")
        .arg(test_path)
        .arg(temp_log_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            panic!("Are you sure `valida` is in your `$PATH`?\nFailed to start test process: {e}")
        })
        .unwrap();

    let mut valida_stdin = child.stdin.take().unwrap();

    // The pipe may break if the process exits before we write to it.
    // This can happen if the test name/filename is not found in this test binary.
    let _ = writeln!(valida_stdin, "{}", test.desc.name);
    let _ = writeln!(valida_stdin, "{}", test.desc.source_file);

    let mut valida_stdout = io::BufReader::new(child.stdout.take().unwrap());

    // contains a list of available tests
    let mut first_line = String::new();
    let Ok(_) = valida_stdout.read_line(&mut first_line) else {
        return Err(format!(
            "Failed to read available tests from first line of stdout for test executable {}",
            test_path.display()
        ));
    };

    // the test you selected
    let mut second_line = String::new();
    let Ok(_) = valida_stdout.read_line(&mut second_line) else {
        return Err(format!(
            "Failed to read test name from second line of stdout for test executable {}",
            test_path.display()
        ));
    };

    if second_line.trim() != valida_test_second_line_stdout(test) {
        return Ok(false);
    }

    let timeout = std::cmp::max(host_test_time * 20, Duration::from_secs(10));
    let start_time = Instant::now();

    let mut stdout_buffer: Vec<u8> = Vec::with_capacity(1024);
    let mut searched_cursor = 0;

    loop {
        // Read stdout the valida child process
        read_extend_vec(&mut stdout_buffer, &mut valida_stdout)
            .expect("Failed to read child stdout");

        let search_end = stdout_buffer
            .len()
            .checked_sub(MAGIC_TERMINATOR.len())
            .unwrap_or(searched_cursor);

        let paniced_with_magic_terminator = (searched_cursor..search_end)
            .any(|i| &stdout_buffer[i..i + MAGIC_TERMINATOR.len()] == MAGIC_TERMINATOR.as_bytes());
        searched_cursor = search_end;

        if paniced_with_magic_terminator {
            let _ = child.kill();

            if let ShouldPanic::No = test.desc.should_panic {
                return Err(format!(
                    "Test panicked unexpectedly.\n\n{}\n\n",
                    String::from_utf8_lossy(&stdout_buffer)
                ));
            } else {
                return Ok(true);
            }
        }

        match (child.try_wait(), &test.desc.should_panic) {
            (Ok(None), _) => {}
            (Ok(Some(status)), ShouldPanic::No) => {
                if status.success() {
                    return Ok(true);
                } else {
                    return Err(format!(
                        "Test failed with exit code: {:?}\n\n{}\n\n",
                        status,
                        String::from_utf8_lossy(&stdout_buffer)
                    ));
                }
            }
            (Ok(Some(status)), ShouldPanic::Yes | ShouldPanic::YesWithMessage(_)) => {
                if status.success() {
                    return Err(format!(
                        "Test did not panic as expected.\n\n{}\n\n",
                        String::from_utf8_lossy(&stdout_buffer)
                    ));
                } else {
                    return Ok(true);
                }
            }
            (Err(e), _) => {
                return Err(format!(
                    "Failed to wait for cargo process: {}\n\n{}",
                    e,
                    String::from_utf8_lossy(&stdout_buffer)
                ))
            }
        }

        if start_time.elapsed() >= timeout {
            let _ = child.kill();

            match &test.desc.should_panic {
                ShouldPanic::No => {
                    return Err(format!(
                        "Test timed out after {:?}\n\n{}",
                        timeout,
                        String::from_utf8_lossy(&stdout_buffer)
                    ));
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
    std::panic::set_hook(Box::new(|panic_info| {
        crate::io::println(&format!("Panic: {:?}", panic_info));

        crate::io::println(MAGIC_TERMINATOR);
    }));

    crate::io::print("Available tests:");
    for t in tests.iter() {
        crate::io::print(&format!(" ({}, {})", t.desc.name, t.desc.source_file))
    }
    crate::io::println("");

    let Ok(test_name) = crate::io::read_line::<String>() else {
        // If no test name is provided the program will hang.
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
        // If the test takes 20x longer than expected, we can assume it has panicked.
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

/// Read from a reader extending the vec until the reader would block
pub fn read_extend_vec(vec: &mut Vec<u8>, reader: &mut impl Read) -> io::Result<usize> {
    let mut total_read = 0;
    loop {
        if vec.len() == vec.capacity() {
            let new_capacity = std::cmp::max(vec.capacity() + vec.capacity() / 4, 1048);
            vec.reserve(new_capacity - vec.capacity());
        }

        let start = vec.len();
        vec.resize(vec.capacity(), 0);

        match reader.read(&mut vec[start..]) {
            Ok(0) => {
                // No more data available
                vec.truncate(start);
                break;
            }
            Ok(n) => {
                // Truncate to actual read size
                vec.truncate(start + n);
                total_read += n;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                vec.truncate(start);
                break;
            }
            Err(e) => {
                vec.truncate(start);
                return Err(e);
            }
        }
    }
    Ok(total_read)
}

#[test]
fn test_unit_test_in_lib() {
    assert_eq!(1, 1);
}
