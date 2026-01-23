//! CLASP Protocol Conformance Suite
//!
//! Comprehensive protocol conformance testing inspired by the Autobahn test suite.
//! This module provides a framework for validating CLASP router implementations
//! against the protocol specification.
//!
//! ## Test Categories
//!
//! - **Handshake**: HELLO/WELCOME exchange, version negotiation
//! - **Messages**: All 12 message types encode/decode correctly
//! - **State**: LWW, Max, Min, Lock, Merge conflict resolution
//! - **Subscriptions**: Wildcard patterns, unsubscribe, snapshots
//! - **Security**: Token validation, scope enforcement
//! - **Encoding**: Binary frame format validation

pub mod handshake;
pub mod messages;
pub mod state;
pub mod subscription;
pub mod security;
pub mod encoding;

use std::time::Duration;

/// Result of a single conformance test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub spec_reference: Option<String>,
}

impl TestResult {
    pub fn pass(name: &str, category: &str, duration_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
            passed: true,
            duration_ms,
            error: None,
            spec_reference: None,
        }
    }

    pub fn fail(name: &str, category: &str, duration_ms: u64, error: &str) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
            passed: false,
            duration_ms,
            error: Some(error.to_string()),
            spec_reference: None,
        }
    }

    pub fn with_spec_reference(mut self, reference: &str) -> Self {
        self.spec_reference = Some(reference.to_string());
        self
    }
}

/// Summary of conformance test run
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<TestResult>,
    pub duration_ms: u64,
}

impl ConformanceReport {
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            results: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.results.push(result);
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        100.0 * self.passed as f64 / self.total_tests as f64
    }

    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║            CLASP PROTOCOL CONFORMANCE REPORT                 ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║ Result: {}/{} tests passed ({:.1}%)                        ║",
            self.passed,
            self.total_tests,
            self.pass_rate()
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        // Print by category
        let categories = ["Handshake", "Messages", "State", "Subscription", "Security", "Encoding"];

        for category in categories {
            let cat_results: Vec<_> = self.results.iter().filter(|r| r.category == category).collect();
            if cat_results.is_empty() {
                continue;
            }

            let passed = cat_results.iter().filter(|r| r.passed).count();
            let total = cat_results.len();

            println!("═══ {} ({}/{}) ═══", category, passed, total);

            for result in cat_results {
                let status = if result.passed { "✓" } else { "✗" };
                let error_msg = result.error.as_ref().map(|e| format!(" - {}", e)).unwrap_or_default();
                println!("  {} {}{}", status, result.name, error_msg);
            }
            println!();
        }

        // Print failed tests summary
        let failed: Vec<_> = self.results.iter().filter(|r| !r.passed).collect();
        if !failed.is_empty() {
            println!("═══ FAILED TESTS ═══");
            for result in failed {
                println!(
                    "  ✗ [{}] {} - {}",
                    result.category,
                    result.name,
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
            println!();
        }
    }

    pub fn to_json(&self) -> String {
        let results_json: Vec<String> = self
            .results
            .iter()
            .map(|r| {
                format!(
                    r#"    {{
      "name": "{}",
      "category": "{}",
      "passed": {},
      "duration_ms": {},
      "error": {}
    }}"#,
                    r.name,
                    r.category,
                    r.passed,
                    r.duration_ms,
                    r.error
                        .as_ref()
                        .map(|e| format!("\"{}\"", e.replace('"', "\\\"")))
                        .unwrap_or_else(|| "null".to_string())
                )
            })
            .collect();

        format!(
            r#"{{
  "total_tests": {},
  "passed": {},
  "failed": {},
  "pass_rate": {:.2},
  "duration_ms": {},
  "results": [
{}
  ]
}}"#,
            self.total_tests,
            self.passed,
            self.failed,
            self.pass_rate(),
            self.duration_ms,
            results_json.join(",\n")
        )
    }
}

impl Default for ConformanceReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for conformance test runner
pub struct ConformanceConfig {
    pub router_url: String,
    pub timeout: Duration,
    pub verbose: bool,
}

impl Default for ConformanceConfig {
    fn default() -> Self {
        Self {
            router_url: "ws://127.0.0.1:7330".to_string(),
            timeout: Duration::from_secs(5),
            verbose: false,
        }
    }
}

/// Run all conformance tests against a router
pub async fn run_all_tests(config: &ConformanceConfig) -> ConformanceReport {
    let start = std::time::Instant::now();
    let mut report = ConformanceReport::new();

    // Run each test category
    println!("Running handshake tests...");
    handshake::run_tests(config, &mut report).await;

    println!("Running message tests...");
    messages::run_tests(config, &mut report).await;

    println!("Running state tests...");
    state::run_tests(config, &mut report).await;

    println!("Running subscription tests...");
    subscription::run_tests(config, &mut report).await;

    println!("Running security tests...");
    security::run_tests(config, &mut report).await;

    println!("Running encoding tests...");
    encoding::run_tests(config, &mut report).await;

    report.duration_ms = start.elapsed().as_millis() as u64;

    report
}
