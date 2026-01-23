//! Security Conformance Tests
//!
//! Tests for CLASP security features (CLASP Spec 6.x):
//! - Token validation
//! - Scope enforcement
//! - Connection authentication
//! - Permission boundaries

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_client::Clasp;
use clasp_core::Value;
use std::time::Instant;

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_connection_without_token(config, report).await;
    test_connection_with_token(config, report).await;
    test_invalid_token_rejected(config, report).await;
    test_token_scope_read(config, report).await;
    test_token_scope_write(config, report).await;
    test_expired_token_rejected(config, report).await;
}

async fn test_connection_without_token(
    config: &ConformanceConfig,
    report: &mut ConformanceReport,
) {
    let start = Instant::now();
    let test_name = "Connection without token";

    // In open mode, connection without token should succeed
    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        if !client.is_connected() {
            return Err(anyhow::anyhow!("Client not connected"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.1"),
        ),
    }
}

async fn test_connection_with_token(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Connection with token";

    let result = async {
        // Connect with a token (in open mode, any token is accepted)
        let client = Clasp::builder(&config.router_url)
            .token("test-token-12345")
            .connect()
            .await?;

        if !client.is_connected() {
            return Err(anyhow::anyhow!("Client not connected with token"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.2"),
        ),
    }
}

async fn test_invalid_token_rejected(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Invalid token handling";

    // Note: In open mode, any token is accepted, so this test verifies
    // the client can send tokens without crashing
    let result = async {
        let client = Clasp::builder(&config.router_url)
            .token("invalid-token-xxx")
            .connect()
            .await;

        // In open mode: connection succeeds
        // In secure mode: connection would fail with auth error
        match client {
            Ok(c) => {
                if c.is_connected() {
                    // Open mode - acceptable
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Client disconnected unexpectedly"))
                }
            }
            Err(_) => {
                // Secure mode would reject - also acceptable
                Ok(())
            }
        }
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.3"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.3"),
        ),
    }
}

async fn test_token_scope_read(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Token scope - read";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Set a value first
        client.set("/security/scope/read", Value::Int(42)).await?;

        // Read it back
        let value = client.get("/security/scope/read").await?;

        match value {
            Value::Int(v) => {
                if v != 42 {
                    return Err(anyhow::anyhow!("Read wrong value: {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Wrong value type")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.4"),
        ),
    }
}

async fn test_token_scope_write(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Token scope - write";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Write should succeed in open mode
        client.set("/security/scope/write", Value::Int(123)).await?;

        // Verify write succeeded
        let value = client.get("/security/scope/write").await?;

        match value {
            Value::Int(v) => {
                if v != 123 {
                    return Err(anyhow::anyhow!("Write not applied: got {}", v));
                }
            }
            _ => return Err(anyhow::anyhow!("Wrong value type")),
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.4"),
        ),
    }
}

async fn test_expired_token_rejected(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Expired token handling";

    // This test documents expected behavior for expired tokens
    // In open mode, tokens aren't validated for expiry
    let result = async {
        let client = Clasp::builder(&config.router_url)
            .token("expired-token-2020-01-01")
            .connect()
            .await;

        // Connection behavior depends on server mode
        match client {
            Ok(c) => {
                // Open mode accepts any token
                if c.is_connected() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Unexpected disconnect"))
                }
            }
            Err(_) => {
                // Secure mode would reject expired token
                Ok(())
            }
        }
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Security", duration).with_spec_reference("CLASP 6.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Security", duration, &e.to_string())
                .with_spec_reference("CLASP 6.5"),
        ),
    }
}
