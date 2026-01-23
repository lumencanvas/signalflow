//! CLASP Protocol Conformance Report Generator
//!
//! Runs the full conformance test suite against a CLASP router and generates
//! a comprehensive report in both human-readable and machine-parseable formats.
//!
//! Usage:
//!   cargo run -p clasp-e2e --bin conformance-report
//!   cargo run -p clasp-e2e --bin conformance-report -- --url ws://localhost:7330
//!   cargo run -p clasp-e2e --bin conformance-report -- --json > report.json

use clasp_e2e::compliance::{run_all_tests, ConformanceConfig};
use clasp_e2e::TestRouter;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse arguments
    let mut url: Option<String> = None;
    let mut json_output = false;
    let mut verbose = false;
    let mut timeout_secs = 5u64;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" | "-u" => {
                if i + 1 < args.len() {
                    url = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--json" | "-j" => {
                json_output = true;
            }
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--timeout" | "-t" => {
                if i + 1 < args.len() {
                    timeout_secs = args[i + 1].parse().unwrap_or(5);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
        i += 1;
    }

    // Start embedded router if no URL provided
    let _embedded_router;
    let router_url = if let Some(u) = url {
        u
    } else {
        if !json_output {
            println!("Starting embedded CLASP router...");
        }
        _embedded_router = TestRouter::start().await;
        _embedded_router.url()
    };

    let config = ConformanceConfig {
        router_url: router_url.clone(),
        timeout: Duration::from_secs(timeout_secs),
        verbose,
    };

    if !json_output {
        println!("Running CLASP Protocol Conformance Suite");
        println!("Router: {}", router_url);
        println!("Timeout: {}s", timeout_secs);
        println!();
    }

    // Run all conformance tests
    let report = run_all_tests(&config).await;

    if json_output {
        // Output JSON for CI/CD integration
        println!("{}", report.to_json());
    } else {
        // Print human-readable summary
        report.print_summary();

        // Exit with non-zero status if tests failed
        if report.failed > 0 {
            println!(
                "\n⚠️  {} test(s) failed - see details above",
                report.failed
            );
            std::process::exit(1);
        } else {
            println!("\n✓ All conformance tests passed!");
        }
    }
}

fn print_help() {
    println!("CLASP Protocol Conformance Report Generator");
    println!();
    println!("USAGE:");
    println!("    conformance-report [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -u, --url <URL>       Router URL (default: start embedded router)");
    println!("    -j, --json            Output JSON report (for CI/CD)");
    println!("    -v, --verbose         Verbose output");
    println!("    -t, --timeout <SECS>  Timeout per test in seconds (default: 5)");
    println!("    -h, --help            Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Run against embedded router");
    println!("    conformance-report");
    println!();
    println!("    # Run against external router");
    println!("    conformance-report --url ws://192.168.1.100:7330");
    println!();
    println!("    # Generate JSON report for CI");
    println!("    conformance-report --json > conformance-report.json");
}
