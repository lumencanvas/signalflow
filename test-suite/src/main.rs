//! CLASP Test Suite Runner
//!
//! This binary runs all integration tests to prove the CLASP protocol implementation
//! is real and functional. Tests verify:
//!
//! 1. OSC bridge can send/receive with real OSC libraries
//! 2. MIDI bridge works with real MIDI ports (virtual or hardware)
//! 3. Art-Net/DMX bridge communicates with Art-Net devices
//! 4. CLASP-to-CLASP communication works between multiple clients
//! 5. Security model (auth, capability tokens) is enforced
//! 6. System handles load under stress testing

use std::process::ExitCode;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

use clasp_test_suite::{TestSuite, tests};

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    println!(r#"
   _____ _        _    ____  ____
  / ____| |      / \  / ___||  _ \
 | |    | |     / _ \ \___ \| |_) |
 | |____| |___ / ___ \ ___) |  __/
  \_____|_____/_/   \_\____/|_|

  Integration Test Suite v0.1.0
  Proving the protocol is REAL
"#);

    let mut suite = TestSuite::new();

    // Run test categories
    info!("Starting OSC integration tests...");
    tests::osc::run_tests(&mut suite).await;

    info!("Starting MIDI integration tests...");
    tests::midi::run_tests(&mut suite).await;

    info!("Starting Art-Net integration tests...");
    tests::artnet::run_tests(&mut suite).await;

    info!("Starting CLASP-to-CLASP tests...");
    tests::clasp_to_clasp::run_tests(&mut suite).await;

    info!("Starting security model tests...");
    tests::security::run_tests(&mut suite).await;

    info!("Starting load tests...");
    tests::load::run_tests(&mut suite).await;

    // Print final summary
    suite.print_summary();

    if suite.all_passed() {
        info!("All tests passed! The protocol is REAL.");
        ExitCode::SUCCESS
    } else {
        error!("{} tests failed", suite.failed());
        ExitCode::FAILURE
    }
}
