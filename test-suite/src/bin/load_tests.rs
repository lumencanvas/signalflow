//! Standalone Load Test Runner

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

    println!("Running Load Tests...\n");

    let mut suite = TestSuite::new();
    tests::load::run_tests(&mut suite).await;
    suite.print_summary();

    std::process::exit(if suite.all_passed() { 0 } else { 1 });
}
