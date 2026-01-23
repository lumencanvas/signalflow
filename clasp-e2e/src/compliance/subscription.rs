//! Subscription Conformance Tests
//!
//! Tests for CLASP subscription system (CLASP Spec 4.3):
//! - Single-level wildcard (*)
//! - Multi-level wildcard (**)
//! - Exact address matching
//! - Unsubscribe functionality
//! - Subscription snapshots

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_client::Clasp;
use clasp_core::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_exact_subscription(config, report).await;
    test_single_wildcard(config, report).await;
    test_multi_wildcard(config, report).await;
    test_unsubscribe(config, report).await;
    test_subscription_snapshot(config, report).await;
    test_wildcard_no_match(config, report).await;
    test_multiple_subscriptions(config, report).await;
}

async fn test_exact_subscription(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Exact address subscription";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let address = "/sub/exact/test";
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        // Subscribe to exact address
        subscriber
            .subscribe(address, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        // Small delay for subscription to register
        sleep(Duration::from_millis(50)).await;

        // Publish to exact address
        publisher.set(address, Value::Int(42)).await?;

        // Wait for delivery
        let got_message = timeout(config.timeout, async {
            while received.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;

        if got_message.is_err() {
            return Err(anyhow::anyhow!("Did not receive subscribed message"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.1"),
        ),
    }
}

async fn test_single_wildcard(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Single-level wildcard (*)";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let pattern = "/sub/single/*/value";
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        // Subscribe with single wildcard
        subscriber
            .subscribe(pattern, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        sleep(Duration::from_millis(50)).await;

        // Publish to matching addresses
        publisher.set("/sub/single/a/value", Value::Int(1)).await?;
        publisher.set("/sub/single/b/value", Value::Int(2)).await?;

        // Wait for both messages
        let got_messages = timeout(config.timeout, async {
            while received.load(Ordering::SeqCst) < 2 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;

        if got_messages.is_err() {
            let count = received.load(Ordering::SeqCst);
            return Err(anyhow::anyhow!(
                "Expected 2 messages, received {}",
                count
            ));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.2"),
        ),
    }
}

async fn test_multi_wildcard(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Multi-level wildcard (**)";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let pattern = "/sub/multi/**";
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        // Subscribe with multi-level wildcard
        subscriber
            .subscribe(pattern, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        sleep(Duration::from_millis(50)).await;

        // Publish to various depths
        publisher.set("/sub/multi/a", Value::Int(1)).await?;
        publisher.set("/sub/multi/a/b", Value::Int(2)).await?;
        publisher.set("/sub/multi/a/b/c", Value::Int(3)).await?;

        // Wait for all messages
        let got_messages = timeout(config.timeout, async {
            while received.load(Ordering::SeqCst) < 3 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;

        if got_messages.is_err() {
            let count = received.load(Ordering::SeqCst);
            return Err(anyhow::anyhow!(
                "Expected 3 messages from ** wildcard, received {}",
                count
            ));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.3"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.3"),
        ),
    }
}

async fn test_unsubscribe(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Unsubscribe";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let address = "/sub/unsub/test";
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        // Subscribe
        let sub_id = subscriber
            .subscribe(address, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        sleep(Duration::from_millis(50)).await;

        // Send first message (should be received)
        publisher.set(address, Value::Int(1)).await?;

        // Wait for first message
        timeout(config.timeout, async {
            while received.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;

        // Unsubscribe
        subscriber.unsubscribe(sub_id).await?;

        sleep(Duration::from_millis(50)).await;

        // Send second message (should NOT be received)
        let before_count = received.load(Ordering::SeqCst);
        publisher.set(address, Value::Int(2)).await?;

        // Wait a bit and verify no new messages
        sleep(Duration::from_millis(200)).await;
        let after_count = received.load(Ordering::SeqCst);

        if after_count > before_count {
            return Err(anyhow::anyhow!(
                "Received message after unsubscribe"
            ));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.4"),
        ),
    }
}

async fn test_subscription_snapshot(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Subscription snapshot";

    let result = async {
        let setup = Clasp::connect_to(&config.router_url).await?;
        let subscriber = Clasp::connect_to(&config.router_url).await?;

        let address = "/sub/snapshot/test";

        // Set value BEFORE subscribing
        setup.set(address, Value::Int(100)).await?;

        sleep(Duration::from_millis(50)).await;

        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        // Subscribe - should receive snapshot of existing value
        subscriber
            .subscribe(address, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        // Wait for snapshot delivery
        let got_snapshot = timeout(config.timeout, async {
            while received.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;

        if got_snapshot.is_err() {
            return Err(anyhow::anyhow!(
                "Did not receive subscription snapshot"
            ));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.5"),
        ),
    }
}

async fn test_wildcard_no_match(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Wildcard non-matching";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let pattern = "/sub/nomatch/*/specific";
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();

        subscriber
            .subscribe(pattern, move |_addr, _value| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        sleep(Duration::from_millis(50)).await;

        // Publish to NON-matching addresses
        publisher
            .set("/sub/nomatch/a/b/specific", Value::Int(1))
            .await?; // Too deep
        publisher
            .set("/sub/nomatch/different", Value::Int(2))
            .await?; // Wrong ending
        publisher
            .set("/other/nomatch/a/specific", Value::Int(3))
            .await?; // Wrong prefix

        // Wait a bit
        sleep(Duration::from_millis(200)).await;

        let count = received.load(Ordering::SeqCst);
        if count > 0 {
            return Err(anyhow::anyhow!(
                "Received {} messages for non-matching addresses",
                count
            ));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.2"),
        ),
    }
}

async fn test_multiple_subscriptions(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Multiple subscriptions";

    let result = async {
        let subscriber = Clasp::connect_to(&config.router_url).await?;
        let publisher = Clasp::connect_to(&config.router_url).await?;

        let received1 = Arc::new(AtomicUsize::new(0));
        let received2 = Arc::new(AtomicUsize::new(0));
        let r1 = received1.clone();
        let r2 = received2.clone();

        // Subscribe to two different patterns
        subscriber
            .subscribe("/sub/multi1/**", move |_addr, _value| {
                r1.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        subscriber
            .subscribe("/sub/multi2/**", move |_addr, _value| {
                r2.fetch_add(1, Ordering::SeqCst);
            })
            .await?;

        sleep(Duration::from_millis(50)).await;

        // Publish to both patterns
        publisher.set("/sub/multi1/a", Value::Int(1)).await?;
        publisher.set("/sub/multi2/b", Value::Int(2)).await?;

        // Wait for both
        timeout(config.timeout, async {
            while received1.load(Ordering::SeqCst) == 0 || received2.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Subscription", duration)
                .with_spec_reference("CLASP 4.3.6"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Subscription", duration, &e.to_string())
                .with_spec_reference("CLASP 4.3.6"),
        ),
    }
}
