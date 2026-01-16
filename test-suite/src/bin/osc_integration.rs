//! Standalone OSC Integration Test Runner

use clasp_test_suite::tests;
use clasp_test_suite::TestSuite;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    println!("Running OSC Integration Tests...\n");

    let mut suite = TestSuite::new();
    tests::osc::run_tests(&mut suite).await;
    suite.print_summary();

    std::process::exit(if suite.all_passed() { 0 } else { 1 });
}
