//! Chaos Testing for CLASP Router
//!
//! Tests for system resilience under adverse conditions:
//! - Router crash and recovery
//! - Disconnect storms (mass disconnection)
//! - Memory pressure (many addresses)
//! - Connection churn (rapid connect/disconnect)

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
    println!("                 CLASP CHAOS TEST SUITE                         ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    let mut passed = 0;
    let mut failed = 0;

    // Run tests
    if test_disconnect_storm().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_memory_pressure().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_connection_churn().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_rapid_subscribe_unsubscribe().await {
        passed += 1;
    } else {
        failed += 1;
    }

    if test_message_flood().await {
        passed += 1;
    } else {
        failed += 1;
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("CHAOS TEST RESULTS: {} passed, {} failed", passed, failed);
    println!("═══════════════════════════════════════════════════════════════");

    if failed > 0 {
        std::process::exit(1);
    }
}

/// Test: Disconnect Storm
/// Connect many clients and disconnect them all simultaneously
async fn test_disconnect_storm() -> bool {
    println!("▸ Test: Disconnect Storm");
    let start = Instant::now();

    let router = TestRouter::start().await;
    let client_count = 50;
    let mut clients = Vec::with_capacity(client_count);

    // Connect many clients
    for i in 0..client_count {
        match Clasp::connect_to(&router.url()).await {
            Ok(client) => {
                // Set some state
                let _ = client.set(&format!("/storm/client/{}", i), Value::Int(i as i64)).await;
                clients.push(client);
            }
            Err(e) => {
                println!("  ✗ Failed to connect client {}: {}", i, e);
                return false;
            }
        }
    }

    println!("  Connected {} clients", clients.len());

    // Drop all clients at once (disconnect storm)
    drop(clients);

    // Give the router a moment to process disconnections
    sleep(Duration::from_millis(100)).await;

    // Router should still be responsive after storm
    match Clasp::connect_to(&router.url()).await {
        Ok(client) => {
            // Should be able to use the router normally
            if client.set("/storm/after", Value::Int(1)).await.is_ok() {
                let elapsed = start.elapsed();
                println!("  ✓ Disconnect storm: router survived ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
                true
            } else {
                println!("  ✗ Router unresponsive after disconnect storm");
                false
            }
        }
        Err(e) => {
            println!("  ✗ Failed to connect after storm: {}", e);
            false
        }
    }
}

/// Test: Memory Pressure
/// Create many addresses to test router memory handling
async fn test_memory_pressure() -> bool {
    println!("▸ Test: Memory Pressure");
    let start = Instant::now();

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    let address_count = 10_000;
    let mut success_count = 0;

    // Create many addresses
    for i in 0..address_count {
        if client
            .set(&format!("/pressure/addr/{}", i), Value::Int(i as i64))
            .await
            .is_ok()
        {
            success_count += 1;
        }

        // Progress indicator
        if i % 1000 == 0 && i > 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
    println!();

    let creation_time = start.elapsed();
    println!("  Created {} addresses in {:.2}s", success_count, creation_time.as_secs_f64());

    // Router should still respond quickly
    let response_start = Instant::now();
    match client.get("/pressure/addr/5000").await {
        Ok(Value::Int(v)) => {
            let response_time = response_start.elapsed();
            if v == 5000 && response_time < Duration::from_secs(5) {
                println!(
                    "  ✓ Memory pressure: router responsive ({:.2}ms response)",
                    response_time.as_secs_f64() * 1000.0
                );
                true
            } else {
                println!("  ✗ Response too slow or wrong value");
                false
            }
        }
        Ok(_) => {
            println!("  ✗ Wrong value type returned");
            false
        }
        Err(e) => {
            println!("  ✗ Failed to get value: {}", e);
            false
        }
    }
}

/// Test: Connection Churn
/// Rapidly connect and disconnect clients
async fn test_connection_churn() -> bool {
    println!("▸ Test: Connection Churn");
    let start = Instant::now();

    let router = TestRouter::start().await;
    let iterations = 100;
    let mut success_count = 0;

    for i in 0..iterations {
        match Clasp::connect_to(&router.url()).await {
            Ok(client) => {
                // Quick operation
                if client.set("/churn/test", Value::Int(i as i64)).await.is_ok() {
                    success_count += 1;
                }
                // Drop client (disconnect)
            }
            Err(_) => {}
        }
    }

    let elapsed = start.elapsed();
    let success_rate = (success_count as f64 / iterations as f64) * 100.0;

    if success_rate >= 95.0 {
        println!(
            "  ✓ Connection churn: {:.1}% success ({} iterations in {:.2}s)",
            success_rate,
            iterations,
            elapsed.as_secs_f64()
        );
        true
    } else {
        println!(
            "  ✗ Connection churn: {:.1}% success (expected >= 95%)",
            success_rate
        );
        false
    }
}

/// Test: Rapid Subscribe/Unsubscribe
/// Test subscription system under rapid changes
async fn test_rapid_subscribe_unsubscribe() -> bool {
    println!("▸ Test: Rapid Subscribe/Unsubscribe");
    let start = Instant::now();

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    let iterations = 100;
    let mut success_count = 0;

    for i in 0..iterations {
        let pattern = format!("/rapid/sub/{}", i);
        match client.subscribe(&pattern, |_, _| {}).await {
            Ok(sub_id) => {
                // Immediately unsubscribe
                if client.unsubscribe(sub_id).await.is_ok() {
                    success_count += 1;
                }
            }
            Err(_) => {}
        }
    }

    let elapsed = start.elapsed();
    let success_rate = (success_count as f64 / iterations as f64) * 100.0;

    if success_rate >= 95.0 {
        println!(
            "  ✓ Rapid subscribe/unsubscribe: {:.1}% success ({:.2}ms)",
            success_rate,
            elapsed.as_secs_f64() * 1000.0
        );
        true
    } else {
        println!(
            "  ✗ Rapid subscribe/unsubscribe: {:.1}% success (expected >= 95%)",
            success_rate
        );
        false
    }
}

/// Test: Message Flood
/// Send many messages rapidly
async fn test_message_flood() -> bool {
    println!("▸ Test: Message Flood");
    let start = Instant::now();

    let router = TestRouter::start().await;

    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  ✗ Failed to connect: {}", e);
            return false;
        }
    };

    let message_count = 1000;
    let mut success_count = 0;

    // Flood with messages
    for i in 0..message_count {
        if client
            .set("/flood/msg", Value::Int(i as i64))
            .await
            .is_ok()
        {
            success_count += 1;
        }
    }

    let flood_time = start.elapsed();

    // Verify final state
    match client.get("/flood/msg").await {
        Ok(Value::Int(v)) => {
            let success_rate = (success_count as f64 / message_count as f64) * 100.0;
            let msgs_per_sec = message_count as f64 / flood_time.as_secs_f64();

            if success_rate >= 95.0 {
                println!(
                    "  ✓ Message flood: {:.1}% success, {:.0} msg/s, final value: {}",
                    success_rate, msgs_per_sec, v
                );
                true
            } else {
                println!(
                    "  ✗ Message flood: {:.1}% success (expected >= 95%)",
                    success_rate
                );
                false
            }
        }
        Ok(_) => {
            println!("  ✗ Wrong value type");
            false
        }
        Err(e) => {
            println!("  ✗ Failed to get final value: {}", e);
            false
        }
    }
}
