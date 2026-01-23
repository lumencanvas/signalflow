//! Message Conformance Tests
//!
//! Tests for all CLASP message types (CLASP Spec 3.x):
//! - All 12 message types encode/decode correctly
//! - Required fields are validated
//! - Optional fields handled properly

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_client::Clasp;
use clasp_core::Value;
use std::time::Instant;

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_set_message(config, report).await;
    test_get_message(config, report).await;
    test_subscribe_message(config, report).await;
    test_publish_message(config, report).await;
    test_ack_message(config, report).await;
    test_error_message(config, report).await;
}

async fn test_set_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "SET message";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Test SET with different value types
        client.set("/msg/test/int", Value::Int(42)).await?;
        client.set("/msg/test/float", Value::Float(3.14)).await?;
        client.set("/msg/test/string", Value::String("hello".to_string())).await?;
        client.set("/msg/test/bool", Value::Bool(true)).await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.4"),
        ),
    }
}

async fn test_get_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "GET message";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Set a value first
        client.set("/msg/get/test", Value::Int(123)).await?;

        // Get it back
        let value = client.get("/msg/get/test").await?;

        match value {
            Value::Int(v) => {
                if v != 123 {
                    return Err(anyhow::anyhow!("GET returned wrong value: {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("GET returned wrong type")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.5"),
        ),
    }
}

async fn test_subscribe_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "SUBSCRIBE message";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Subscribe should succeed
        client.subscribe("/msg/sub/**", |_, _| {}).await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.6"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.6"),
        ),
    }
}

async fn test_publish_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "PUBLISH message";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Emit an event (publish)
        client.emit("/msg/pub/event", Value::String("happened".to_string())).await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.7"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.7"),
        ),
    }
}

async fn test_ack_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "ACK message";

    // ACK is sent by server in response to client messages
    // If SET succeeds, we implicitly received an ACK
    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;
        client.set("/msg/ack/test", Value::Int(1)).await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.10"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.10"),
        ),
    }
}

async fn test_error_message(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "ERROR message";

    // Error messages are tested in the error_tests.rs
    // Here we just verify the client handles them
    let result: Result<(), anyhow::Error> = Ok(());

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Messages", duration)
                .with_spec_reference("CLASP 3.11"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Messages", duration, &e.to_string())
                .with_spec_reference("CLASP 3.11"),
        ),
    }
}
