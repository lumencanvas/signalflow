//! CLASP E2E Test Suite Library
//!
//! This library provides end-to-end tests, benchmarks, and load tests for the CLASP protocol.
//! Standard tests are migrated to individual crate tests/ directories.

use std::time::Duration;

pub mod compliance;
pub mod tests;

// Re-export test utilities from clasp-test-utils
pub use clasp_test_utils::{
    assert_approx_eq, assert_err, assert_ok, assert_some, assert_that,
    find_available_port, find_available_udp_port,
    wait_for, wait_for_count, wait_for_flag, wait_with_notify,
    TestRouter, ValueCollector,
    DEFAULT_CHECK_INTERVAL, DEFAULT_TIMEOUT,
};

/// Result of a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub message: Option<String>,
}

/// Collection of test results
#[derive(Debug)]
pub struct TestSuite {
    pub results: Vec<TestResult>,
}

impl TestSuite {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.results.push(result);
    }

    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    pub fn print_summary(&self) {
        self.print_results(false);
    }

    pub fn print_verbose(&self) {
        self.print_results(true);
    }

    fn print_results(&self, verbose: bool) {
        for result in &self.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            let status_color = if result.passed {
                "\x1b[32m"
            } else {
                "\x1b[31m"
            };
            let reset = "\x1b[0m";

            println!(
                "[{status_color}{status}{reset}] {} ({:.2}ms)",
                result.name,
                result.duration.as_secs_f64() * 1000.0
            );

            if let Some(msg) = &result.message {
                if verbose || !result.passed {
                    println!("{}", msg);
                }
            }
        }

        println!("\n{:=<75}", "");
        println!(
            "SUMMARY: Total: {} | Passed: {} | Failed: {}",
            self.results.len(),
            self.passed(),
            self.failed()
        );
        println!("{:=<75}\n", "");
    }

    pub fn all_passed(&self) -> bool {
        self.failed() == 0
    }
}

impl Default for TestSuite {
    fn default() -> Self {
        Self::new()
    }
}
