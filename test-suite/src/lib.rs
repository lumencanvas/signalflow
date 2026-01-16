//! CLASP Test Suite Library
//!
//! This library provides comprehensive integration tests for the CLASP protocol.
//! It can be used both as a standalone test runner and as a library for custom tests.

use std::time::Duration;

pub mod tests;

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
        println!("\n{:=<60}", "");
        println!("CLASP TEST SUITE RESULTS");
        println!("{:=<60}\n", "");

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
                if !result.passed {
                    println!("       Error: {}", msg);
                }
            }
        }

        println!("\n{:-<60}", "");
        println!(
            "Total: {} | Passed: {} | Failed: {}",
            self.results.len(),
            self.passed(),
            self.failed()
        );
        println!("{:-<60}\n", "");
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
