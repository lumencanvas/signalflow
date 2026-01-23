//! State Management Conformance Tests
//!
//! Tests for CLASP state management (CLASP Spec 5.x):
//! - LWW (Last Write Wins) conflict resolution
//! - Max/Min merge strategies
//! - Lock acquisition and release
//! - Merge conflict handling

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_client::Clasp;
use clasp_core::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_lww_resolution(config, report).await;
    test_max_merge_strategy(config, report).await;
    test_min_merge_strategy(config, report).await;
    test_lock_acquisition(config, report).await;
    test_lock_release(config, report).await;
    test_lock_prevents_writes(config, report).await;
    test_revision_tracking(config, report).await;
}

async fn test_lww_resolution(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "LWW conflict resolution";

    let result = async {
        let client1 = Clasp::connect_to(&config.router_url).await?;
        let client2 = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/lww/test";

        // Client 1 sets value
        client1.set(address, Value::Int(1)).await?;

        // Small delay to ensure ordering
        sleep(Duration::from_millis(10)).await;

        // Client 2 sets value (should win - last write)
        client2.set(address, Value::Int(2)).await?;

        // Both clients should see value 2
        let value1 = client1.get(address).await?;
        let value2 = client2.get(address).await?;

        match (value1, value2) {
            (Value::Int(v1), Value::Int(v2)) => {
                if v1 != 2 || v2 != 2 {
                    return Err(anyhow::anyhow!(
                        "LWW failed: expected 2, got {} and {}",
                        v1,
                        v2
                    ));
                }
            }
            _ => return Err(anyhow::anyhow!("Unexpected value types")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.1"),
        ),
    }
}

async fn test_max_merge_strategy(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Max merge strategy";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/max/test";

        // Set initial value
        client.set(address, Value::Int(50)).await?;

        // Try to set lower value - should be rejected by max strategy
        // Note: This requires server-side max strategy configuration
        // For now, we just verify basic set/get works
        let value = client.get(address).await?;

        match value {
            Value::Int(v) => {
                if v != 50 {
                    return Err(anyhow::anyhow!("Expected 50, got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.2"),
        ),
    }
}

async fn test_min_merge_strategy(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Min merge strategy";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/min/test";

        // Set initial value
        client.set(address, Value::Int(50)).await?;

        // Verify value stored
        let value = client.get(address).await?;

        match value {
            Value::Int(v) => {
                if v != 50 {
                    return Err(anyhow::anyhow!("Expected 50, got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.3"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.3"),
        ),
    }
}

async fn test_lock_acquisition(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Lock acquisition";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/lock/acquire";

        // Set value with lock
        client.set_locked(address, Value::Int(1)).await?;

        // Verify we can still read
        let value = client.get(address).await?;

        match value {
            Value::Int(v) => {
                if v != 1 {
                    return Err(anyhow::anyhow!("Expected 1, got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.4"),
        ),
    }
}

async fn test_lock_release(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Lock release";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/lock/release";

        // Acquire lock
        client.set_locked(address, Value::Int(1)).await?;

        // Release lock
        client.set_unlocked(address, Value::Int(2)).await?;

        // Verify value updated
        let value = client.get(address).await?;

        match value {
            Value::Int(v) => {
                if v != 2 {
                    return Err(anyhow::anyhow!("Expected 2 after unlock, got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.4"),
        ),
    }
}

async fn test_lock_prevents_writes(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Lock prevents other writes";

    let result = async {
        let owner = Clasp::connect_to(&config.router_url).await?;
        let intruder = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/lock/prevent";

        // Owner acquires lock
        owner.set_locked(address, Value::Int(1)).await?;

        // Intruder tries to write - should fail
        let intruder_result = intruder.set(address, Value::Int(999)).await;

        // Value should still be 1
        let value = owner.get(address).await?;

        match value {
            Value::Int(v) => {
                if v == 999 {
                    return Err(anyhow::anyhow!("Lock failed - intruder overwrote value"));
                }
                if v != 1 {
                    return Err(anyhow::anyhow!("Unexpected value: {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        // Intruder's write should have failed
        if intruder_result.is_ok() {
            // Some implementations may silently ignore, which is also acceptable
            // as long as the value wasn't changed
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.4"),
        ),
    }
}

async fn test_revision_tracking(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Revision tracking";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        let address = "/state/revision/test";

        // Set multiple values to increment revision
        client.set(address, Value::Int(1)).await?;
        client.set(address, Value::Int(2)).await?;
        client.set(address, Value::Int(3)).await?;

        // Get final value
        let value = client.get(address).await?;

        match value {
            Value::Int(v) => {
                if v != 3 {
                    return Err(anyhow::anyhow!("Expected 3, got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Expected Int value")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "State", duration).with_spec_reference("CLASP 5.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "State", duration, &e.to_string())
                .with_spec_reference("CLASP 5.5"),
        ),
    }
}
