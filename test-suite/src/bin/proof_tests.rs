//! Comprehensive proof tests for CLASP
//!
//! This binary runs all proof tests to validate CLASP's claims:
//! - Performance comparisons (CLASP vs OSC)
//! - Security model validation
//! - Bridge data transformation visualization
//! - Stress tests

use clasp_test_suite::tests::proof;

fn main() {
    let suite = proof::run_all_proof_tests();
    suite.print_verbose();

    // Exit with error code if any tests failed
    if suite.failed() > 0 {
        std::process::exit(1);
    }
}
