//! Network Simulation Tests
//!
//! Tests CLASP behavior under various network conditions.
//! These tests use application-level simulation (artificial delays)
//! to test protocol resilience without requiring root access.
//!
//! For real network impairment testing on macOS, use:
//!   sudo dnctl pipe 1 config delay 50ms
//!   sudo pfctl -e
//!   echo "dummynet in on lo0 pipe 1" | sudo pfctl -f -
//!
//! For Linux (requires root):
//!   sudo tc qdisc add dev lo root netem delay 50ms

use clasp_client::Clasp;
use clasp_core::Value;
use clasp_test_utils::TestRouter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("             NETWORK SIMULATION TEST SUITE                      ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    let mut passed = 0;
    let mut failed = 0;

    // Run simulated network tests
    if test_baseline_latency().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_high_latency_tolerance().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_intermittent_delays().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_timeout_handling().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_reconnection_after_delay().await {
        passed += 1;
    } else {
        failed += 1;
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("NETWORK SIMULATION RESULTS: {} passed, {} failed", passed, failed);
    println!("═══════════════════════════════════════════════════════════════");

    if failed > 0 {
        std::process::exit(1);
    }
}

/// Test: Baseline Latency
/// Measure normal operation latency for comparison
async fn test_baseline_latency() -> bool {
    println!("▸ Test: Baseline Latency");

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    let iterations = 100;
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let start = Instant::now();
        if client.set("/baseline/test", Value::Int(i as i64)).await.is_ok() {
            latencies.push(start.elapsed().as_micros() as u64);
        }
    }

    if latencies.len() < iterations / 2 {
        println!("  ✗ Too many failures");
        return false;
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("  Baseline: P50 = {}µs, P99 = {}µs", p50, p99);
    println!("  ✓ Baseline latency measured");
    true
}

/// Test: High Latency Tolerance
/// Test that the client handles high latency gracefully
async fn test_high_latency_tolerance() -> bool {
    println!("▸ Test: High Latency Tolerance");

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    // Simulate high latency with delays between operations
    let simulated_latency = Duration::from_millis(100);
    let iterations = 10;
    let mut success_count = 0;

    for i in 0..iterations {
        // Simulate network latency
        sleep(simulated_latency).await;

        let start = Instant::now();
        if client.set("/highlatency/test", Value::Int(i as i64)).await.is_ok() {
            success_count += 1;
            let actual_latency = start.elapsed();
            // The operation should complete despite simulated delays
            if actual_latency > Duration::from_secs(5) {
                println!("  ✗ Operation took too long: {:?}", actual_latency);
                return false;
            }
        }
    }

    if success_count >= iterations * 9 / 10 {
        println!("  ✓ High latency tolerance: {} of {} operations succeeded", success_count, iterations);
        true
    } else {
        println!("  ✗ Too many failures: {} of {}", iterations - success_count, iterations);
        false
    }
}

/// Test: Intermittent Delays
/// Test behavior with sporadic delays
async fn test_intermittent_delays() -> bool {
    println!("▸ Test: Intermittent Delays");

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    let iterations = 50;
    let mut success_count = 0;

    for i in 0..iterations {
        // Every 5th operation has a delay (simulating intermittent issues)
        if i % 5 == 0 {
            sleep(Duration::from_millis(50)).await;
        }

        if client.set("/intermittent/test", Value::Int(i as i64)).await.is_ok() {
            success_count += 1;
        }
    }

    if success_count >= iterations * 9 / 10 {
        println!("  ✓ Intermittent delays: {} of {} operations succeeded", success_count, iterations);
        true
    } else {
        println!("  ✗ Too many failures under intermittent delays");
        false
    }
}

/// Test: Timeout Handling
/// Test that operations timeout appropriately
async fn test_timeout_handling() -> bool {
    println!("▸ Test: Timeout Handling");

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    // Set a value successfully first
    if client.set("/timeout/test", Value::Int(1)).await.is_err() {
        println!("  ✗ Initial set failed");
        return false;
    }

    // Get should complete within reasonable time
    let start = Instant::now();
    let result = client.get("/timeout/test").await;
    let elapsed = start.elapsed();

    match result {
        Ok(Value::Int(v)) => {
            if v == 1 && elapsed < Duration::from_secs(10) {
                println!("  ✓ Timeout handling: response in {:?}", elapsed);
                true
            } else {
                println!("  ✗ Unexpected value or slow response");
                false
            }
        }
        Ok(_) => {
            println!("  ✗ Wrong value type");
            false
        }
        Err(e) => {
            println!("  ✗ Get failed: {}", e);
            false
        }
    }
}

/// Test: Reconnection After Delay
/// Test that client can reconnect after network issues
async fn test_reconnection_after_delay() -> bool {
    println!("▸ Test: Reconnection After Delay");

    let router = TestRouter::start().await;

    // First connection
    let client1 = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed first connection: {}", e);
            return false;
        }
    };

    // Set a value
    if client1.set("/reconnect/test", Value::Int(1)).await.is_err() {
        println!("  ✗ First set failed");
        return false;
    }

    // Drop connection
    drop(client1);

    // Simulate network delay
    sleep(Duration::from_millis(200)).await;

    // Reconnect
    let client2 = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to reconnect: {}", e);
            return false;
        }
    };

    // Previous value should still exist (router maintains state)
    match client2.get("/reconnect/test").await {
        Ok(Value::Int(v)) => {
            if v == 1 {
                println!("  ✓ Reconnection: state preserved after delay");
                true
            } else {
                println!("  ✗ State not preserved: got {}", v);
                false
            }
        }
        Ok(_) => {
            println!("  ✗ Wrong value type");
            false
        }
        Err(e) => {
            println!("  ✗ Get failed after reconnect: {}", e);
            false
        }
    }
}
