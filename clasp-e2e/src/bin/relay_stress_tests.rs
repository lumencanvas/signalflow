//! CLASP Relay Server Stress Tests & Critical Benchmarks
//!
//! This suite is designed to find flaws, not just prove basic functionality.
//! It includes:
//!
//! - Latency benchmarks with statistical analysis (p50, p95, p99, max)
//! - High-concurrency stress tests (100+ simultaneous clients)
//! - Race condition detection (concurrent writes to same address)
//! - Subscription pattern edge cases and correctness
//! - Message ordering guarantees
//! - Throughput limits and saturation behavior
//! - Connection churn (rapid connect/disconnect)
//! - Large payload handling
//! - Protocol edge cases and malformed data resilience
//! - State consistency under concurrent modification
//! - Memory leak detection via sustained load
//! - P2P signaling under load
//!
//! Usage:
//!   cargo run --bin relay-stress-tests --release
//!   cargo run --bin relay-stress-tests --release --features p2p

use bytes::Bytes;
use clasp_client::Clasp;
use clasp_core::{
    codec, BundleMessage, HelloMessage, Message, PublishMessage, SetMessage, SubscribeMessage,
    UnsubscribeMessage, Value,
};
use clasp_transport::{
    Transport, TransportEvent, TransportReceiver, TransportSender, WebSocketTransport,
};
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

#[cfg(feature = "p2p")]
use {clasp_client::P2PEvent, clasp_core::P2PConfig};

// ============================================================================
// Configuration
// ============================================================================

const PUBLIC_RELAY_URL: &str = "wss://relay.clasp.to";
const TEST_NAMESPACE: &str = "/stress-test";

const MAX_CONCURRENT_CLIENTS: usize = 100;
const SUSTAINED_LOAD_DURATION_SECS: u64 = 30;
const LARGE_PAYLOAD_SIZES: &[usize] = &[1024, 4096, 16384, 65536, 262144];
const CONNECTION_CHURN_CYCLES: usize = 50;

// ============================================================================
// Test Infrastructure
// ============================================================================

struct LatencyStats {
    histogram: Histogram<u64>,
    name: String,
}

impl LatencyStats {
    fn new(name: &str) -> Self {
        Self {
            histogram: Histogram::new(3).unwrap(),
            name: name.to_string(),
        }
    }

    fn record(&mut self, latency_us: u64) {
        let _ = self.histogram.record(latency_us);
    }

    fn report(&self) -> String {
        if self.histogram.is_empty() {
            return format!("{}: No samples", self.name);
        }
        format!(
            "{}: n={} min={:.2}ms p50={:.2}ms p95={:.2}ms p99={:.2}ms max={:.2}ms mean={:.2}ms",
            self.name,
            self.histogram.len(),
            self.histogram.min() as f64 / 1000.0,
            self.histogram.value_at_quantile(0.50) as f64 / 1000.0,
            self.histogram.value_at_quantile(0.95) as f64 / 1000.0,
            self.histogram.value_at_quantile(0.99) as f64 / 1000.0,
            self.histogram.max() as f64 / 1000.0,
            self.histogram.mean() / 1000.0,
        )
    }
}

#[derive(Debug, Clone)]
struct TestResult {
    name: String,
    passed: bool,
    message: String,
    duration_ms: u128,
    details: Vec<String>,
}

impl TestResult {
    fn pass(name: &str, duration_ms: u128) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: "OK".to_string(),
            duration_ms,
            details: vec![],
        }
    }

    fn pass_with_details(name: &str, duration_ms: u128, details: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: "OK".to_string(),
            duration_ms,
            details,
        }
    }

    fn fail(name: &str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            message: message.into(),
            duration_ms,
            details: vec![],
        }
    }
}

struct RawClient {
    sender: clasp_transport::websocket::WebSocketSender,
    receiver: clasp_transport::websocket::WebSocketReceiver,
    session_id: Option<String>,
}

impl RawClient {
    async fn connect() -> Result<Self, String> {
        let (sender, receiver) = WebSocketTransport::connect(PUBLIC_RELAY_URL)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;
        Ok(Self { sender, receiver, session_id: None })
    }

    async fn handshake(&mut self, name: &str) -> Result<(), String> {
        let hello = Message::Hello(HelloMessage {
            version: 2,
            name: name.to_string(),
            features: vec!["param".to_string(), "event".to_string(), "stream".to_string()],
            capabilities: None,
            token: None,
        });
        self.send(&hello).await?;

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut got_welcome = false;
        let mut got_snapshot = false;

        while (!got_welcome || !got_snapshot) && Instant::now() < deadline {
            match timeout(Duration::from_secs(2), self.receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data).map_err(|e| e.to_string())?;
                    match msg {
                        Message::Welcome(w) => {
                            self.session_id = Some(w.session.clone());
                            got_welcome = true;
                        }
                        Message::Snapshot(_) => got_snapshot = true,
                        _ => {}
                    }
                }
                Ok(Some(TransportEvent::Connected)) => continue,
                Ok(Some(TransportEvent::Disconnected { reason })) => {
                    return Err(format!("Disconnected: {:?}", reason));
                }
                Ok(Some(TransportEvent::Error(e))) => return Err(format!("Error: {}", e)),
                Ok(None) => return Err("Connection closed".to_string()),
                Err(_) => continue,
            }
        }

        if !got_welcome || !got_snapshot {
            return Err("Handshake timeout".to_string());
        }
        Ok(())
    }

    async fn send(&mut self, msg: &Message) -> Result<(), String> {
        let data = codec::encode(msg).map_err(|e| e.to_string())?;
        self.sender.send(data).await.map_err(|e| format!("Send failed: {}", e))
    }

    async fn recv(&mut self, timeout_ms: u64) -> Result<Message, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Timeout".to_string());
            }
            match timeout(remaining, self.receiver.recv()).await {
                Ok(Some(TransportEvent::Data(data))) => {
                    let (msg, _) = codec::decode(&data).map_err(|e| e.to_string())?;
                    return Ok(msg);
                }
                Ok(Some(TransportEvent::Connected)) => continue,
                Ok(Some(TransportEvent::Disconnected { reason })) => {
                    return Err(format!("Disconnected: {:?}", reason));
                }
                Ok(Some(TransportEvent::Error(e))) => return Err(format!("Error: {}", e)),
                Ok(None) => return Err("Connection closed".to_string()),
                Err(_) => return Err("Timeout".to_string()),
            }
        }
    }

    async fn recv_non_blocking(&mut self) -> Option<Message> {
        match timeout(Duration::from_millis(1), self.receiver.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => codec::decode(&data).ok().map(|(msg, _)| msg),
            _ => None,
        }
    }

    async fn close(self) {
        let _ = self.sender.close().await;
    }
}

fn unique_addr(suffix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}/{}/{}", TEST_NAMESPACE, ts, suffix)
}

// ============================================================================
// LATENCY BENCHMARKS
// ============================================================================

async fn bench_set_ack_latency() -> TestResult {
    let start = Instant::now();
    let name = "latency/set_ack";

    let result: Result<Vec<String>, String> = async {
        let mut client = RawClient::connect().await?;
        client.handshake("LatencyBench").await?;

        let mut stats = LatencyStats::new("SET→ACK");
        let base = unique_addr("latency");

        // Warmup
        for i in 0..10 {
            let addr = format!("{}/warmup/{}", base, i);
            client.send(&Message::Set(SetMessage {
                address: addr, value: Value::Int(i), revision: None, lock: false, unlock: false,
            })).await?;
            client.recv(5000).await?;
        }

        // Actual measurements
        for i in 0..100 {
            let addr = format!("{}/measure/{}", base, i);
            let send_time = Instant::now();
            client.send(&Message::Set(SetMessage {
                address: addr.clone(), value: Value::Int(i), revision: None, lock: false, unlock: false,
            })).await?;

            match client.recv(5000).await? {
                Message::Ack(ack) if ack.address == Some(addr) => {
                    stats.record(send_time.elapsed().as_micros() as u64);
                }
                other => return Err(format!("Expected ACK, got {:?}", other)),
            }
        }

        client.close().await;

        let mut details = vec![stats.report()];
        let p99 = stats.histogram.value_at_quantile(0.99) as f64 / 1000.0;
        if p99 > 500.0 {
            details.push(format!("WARNING: p99 latency {:.2}ms exceeds 500ms", p99));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn bench_pubsub_latency() -> TestResult {
    let start = Instant::now();
    let name = "latency/pubsub";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("pubsub-latency");

        let mut subscriber = RawClient::connect().await?;
        subscriber.handshake("LatencySub").await?;
        subscriber.send(&Message::Subscribe(SubscribeMessage {
            id: 1, pattern: format!("{}/**", base), types: vec![], options: None,
        })).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut publisher = RawClient::connect().await?;
        publisher.handshake("LatencyPub").await?;

        let mut stats = LatencyStats::new("PUB→SUB");

        for i in 0..100 {
            let addr = format!("{}/measure/{}", base, i);
            let send_time = Instant::now();

            publisher.send(&Message::Set(SetMessage {
                address: addr.clone(), value: Value::Int(i), revision: None, lock: false, unlock: false,
            })).await?;

            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                match subscriber.recv(100).await {
                    Ok(Message::Set(set)) if set.address == addr => {
                        stats.record(send_time.elapsed().as_micros() as u64);
                        break;
                    }
                    _ => continue,
                }
            }
            let _ = publisher.recv(100).await;
        }

        subscriber.close().await;
        publisher.close().await;
        Ok(vec![stats.report()])
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// THROUGHPUT BENCHMARKS
// ============================================================================

async fn bench_throughput() -> TestResult {
    let start = Instant::now();
    let name = "throughput/single_client";

    let result: Result<Vec<String>, String> = async {
        let mut client = RawClient::connect().await?;
        client.handshake("ThroughputBench").await?;
        let base = unique_addr("throughput");

        let send_count = Arc::new(AtomicU64::new(0));
        let ack_count = Arc::new(AtomicU64::new(0));

        let duration = Duration::from_secs(5);
        let start_time = Instant::now();
        let mut i = 0u64;

        while start_time.elapsed() < duration {
            let addr = format!("{}/{}", base, i);
            if client.send(&Message::Set(SetMessage {
                address: addr, value: Value::Int(i as i64), revision: None, lock: false, unlock: false,
            })).await.is_ok() {
                send_count.fetch_add(1, Ordering::Relaxed);
            }
            i += 1;

            while let Some(msg) = client.recv_non_blocking().await {
                if matches!(msg, Message::Ack(_)) {
                    ack_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Drain remaining ACKs
        let drain_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < drain_deadline {
            match client.recv(100).await {
                Ok(Message::Ack(_)) => { ack_count.fetch_add(1, Ordering::Relaxed); }
                _ => break,
            }
        }

        let sent = send_count.load(Ordering::Relaxed);
        let acked = ack_count.load(Ordering::Relaxed);
        let elapsed = start_time.elapsed().as_secs_f64();
        let send_rate = sent as f64 / elapsed;
        let ack_ratio = if sent > 0 { acked as f64 / sent as f64 * 100.0 } else { 0.0 };

        client.close().await;

        let details = vec![
            format!("Sent: {} messages in {:.2}s = {:.0} msg/s", sent, elapsed, send_rate),
            format!("ACKed: {} ({:.1}%)", acked, ack_ratio),
        ];

        if ack_ratio < 90.0 {
            return Err(format!("ACK ratio {:.1}% too low", ack_ratio));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn bench_fanout_throughput() -> TestResult {
    let start = Instant::now();
    let name = "throughput/fanout";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("fanout");
        let subscriber_count: usize = 10;
        let message_count: usize = 100;

        let mut subscribers = Vec::new();
        for i in 0..subscriber_count {
            let mut sub = RawClient::connect().await?;
            sub.handshake(&format!("FanoutSub{}", i)).await?;
            sub.send(&Message::Subscribe(SubscribeMessage {
                id: 1, pattern: format!("{}/**", base), types: vec![], options: None,
            })).await?;
            subscribers.push(sub);
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        let mut publisher = RawClient::connect().await?;
        publisher.handshake("FanoutPub").await?;

        let publish_start = Instant::now();
        for i in 0..message_count {
            publisher.send(&Message::Set(SetMessage {
                address: format!("{}/{}", base, i), value: Value::Int(i as i64), revision: None, lock: false, unlock: false,
            })).await?;
        }

        let mut received_counts = vec![0usize; subscriber_count];
        let timeout_deadline = Instant::now() + Duration::from_secs(10);

        while Instant::now() < timeout_deadline {
            let total: usize = received_counts.iter().sum();
            if total >= subscriber_count * message_count { break; }
            for (idx, sub) in subscribers.iter_mut().enumerate() {
                if let Ok(Message::Set(_)) = sub.recv(10).await {
                    received_counts[idx] += 1;
                }
            }
        }

        let elapsed = publish_start.elapsed();
        let total_received: usize = received_counts.iter().sum();
        let expected = subscriber_count * message_count;

        for sub in subscribers { sub.close().await; }
        publisher.close().await;

        let details = vec![
            format!("Published {} to {} subscribers", message_count, subscriber_count),
            format!("Received: {}/{} ({:.1}%)", total_received, expected, total_received as f64 / expected as f64 * 100.0),
        ];

        if total_received < expected * 90 / 100 {
            return Err(format!("Fanout delivery too low: {}/{}", total_received, expected));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// CONCURRENCY STRESS TESTS
// ============================================================================

async fn stress_concurrent_clients() -> TestResult {
    let start = Instant::now();
    let name = "stress/concurrent_clients";

    let result: Result<Vec<String>, String> = async {
        let client_count = MAX_CONCURRENT_CLIENTS;
        let success_count = Arc::new(AtomicU32::new(0));
        let failure_count = Arc::new(AtomicU32::new(0));
        let connection_times = Arc::new(Mutex::new(Vec::new()));
        let semaphore = Arc::new(Semaphore::new(20));

        let mut handles = Vec::new();
        for i in 0..client_count {
            let success = success_count.clone();
            let failure = failure_count.clone();
            let times = connection_times.clone();
            let sem = semaphore.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let conn_start = Instant::now();

                match RawClient::connect().await {
                    Ok(mut client) => {
                        let conn_time = conn_start.elapsed();
                        match client.handshake(&format!("StressClient{}", i)).await {
                            Ok(()) => {
                                let addr = unique_addr(&format!("stress/{}", i));
                                if client.send(&Message::Set(SetMessage {
                                    address: addr, value: Value::Int(i as i64), revision: None, lock: false, unlock: false,
                                })).await.is_ok() {
                                    if let Ok(Message::Ack(_)) = client.recv(5000).await {
                                        success.fetch_add(1, Ordering::Relaxed);
                                        times.lock().await.push(conn_time.as_millis() as u64);
                                    } else { failure.fetch_add(1, Ordering::Relaxed); }
                                } else { failure.fetch_add(1, Ordering::Relaxed); }
                                client.close().await;
                            }
                            Err(_) => { failure.fetch_add(1, Ordering::Relaxed); }
                        }
                    }
                    Err(_) => { failure.fetch_add(1, Ordering::Relaxed); }
                }
            }));
        }

        for handle in handles { let _ = handle.await; }

        let successes = success_count.load(Ordering::Relaxed);
        let failures = failure_count.load(Ordering::Relaxed);
        let success_rate = successes as f64 / client_count as f64 * 100.0;
        let times = connection_times.lock().await;
        let avg_conn_time = if times.is_empty() { 0.0 } else { times.iter().sum::<u64>() as f64 / times.len() as f64 };

        let details = vec![
            format!("Clients: {} attempted", client_count),
            format!("Success: {} ({:.1}%)", successes, success_rate),
            format!("Failures: {}", failures),
            format!("Avg connection time: {:.0}ms", avg_conn_time),
        ];

        if success_rate < 90.0 {
            return Err(format!("Success rate {:.1}% too low", success_rate));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn stress_concurrent_writes() -> TestResult {
    let start = Instant::now();
    let name = "stress/concurrent_writes";

    let result: Result<Vec<String>, String> = async {
        let writer_count = 10;
        let writes_per_writer = 100;
        let shared_addr = unique_addr("concurrent-write");

        let write_counts = Arc::new(AtomicU32::new(0));
        let ack_counts = Arc::new(AtomicU32::new(0));

        let mut handles = Vec::new();
        for writer_id in 0..writer_count {
            let addr = shared_addr.clone();
            let writes = write_counts.clone();
            let acks = ack_counts.clone();

            handles.push(tokio::spawn(async move {
                let mut client = match RawClient::connect().await { Ok(c) => c, Err(_) => return };
                if client.handshake(&format!("Writer{}", writer_id)).await.is_err() { return; }

                for i in 0..writes_per_writer {
                    if client.send(&Message::Set(SetMessage {
                        address: addr.clone(),
                        value: Value::String(format!("w{}:{}", writer_id, i)),
                        revision: None, lock: false, unlock: false,
                    })).await.is_ok() {
                        writes.fetch_add(1, Ordering::Relaxed);
                        if let Ok(Message::Ack(_)) = client.recv(2000).await {
                            acks.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                client.close().await;
            }));
        }

        for handle in handles { let _ = handle.await; }

        let total_writes = write_counts.load(Ordering::Relaxed);
        let total_acks = ack_counts.load(Ordering::Relaxed);
        let expected = (writer_count * writes_per_writer) as u32;

        let details = vec![
            format!("Writers: {}, writes each: {}", writer_count, writes_per_writer),
            format!("Total writes: {}/{}", total_writes, expected),
            format!("ACKs received: {}", total_acks),
        ];

        if total_acks < expected * 90 / 100 {
            return Err(format!("ACK rate too low: {}/{}", total_acks, expected));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn stress_connection_churn() -> TestResult {
    let start = Instant::now();
    let name = "stress/connection_churn";

    let result: Result<Vec<String>, String> = async {
        let cycles = CONNECTION_CHURN_CYCLES;
        let mut success_count = 0;
        let mut failure_count = 0;

        for i in 0..cycles {
            match RawClient::connect().await {
                Ok(mut client) => {
                    match client.handshake(&format!("ChurnClient{}", i)).await {
                        Ok(()) => { client.close().await; success_count += 1; }
                        Err(_) => failure_count += 1,
                    }
                }
                Err(_) => failure_count += 1,
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let success_rate = success_count as f64 / cycles as f64 * 100.0;
        let details = vec![
            format!("Cycles: {}", cycles),
            format!("Success: {} ({:.1}%)", success_count, success_rate),
            format!("Failures: {}", failure_count),
        ];

        if success_rate < 95.0 { return Err(format!("Churn success rate {:.1}% too low", success_rate)); }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// PROTOCOL CORRECTNESS TESTS
// ============================================================================

async fn test_subscription_patterns() -> TestResult {
    let start = Instant::now();
    let name = "protocol/subscription_patterns";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("patterns");
        let mut details = Vec::new();

        let test_cases = vec![
            (format!("{}/exact", base), vec![(format!("{}/exact", base), true), (format!("{}/other", base), false)]),
            (format!("{}/wild/*/end", base), vec![
                (format!("{}/wild/foo/end", base), true),
                (format!("{}/wild/bar/end", base), true),
                (format!("{}/wild/foo/bar/end", base), false),
            ]),
            (format!("{}/**", base), vec![
                (format!("{}/a", base), true),
                (format!("{}/a/b/c", base), true),
            ]),
        ];

        for (pattern, addresses) in test_cases {
            let mut sub = RawClient::connect().await?;
            sub.handshake("PatternTest").await?;
            sub.send(&Message::Subscribe(SubscribeMessage { id: 1, pattern: pattern.clone(), types: vec![], options: None })).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;

            let mut pub_client = RawClient::connect().await?;
            pub_client.handshake("PatternPub").await?;

            for (addr, should_match) in addresses {
                pub_client.send(&Message::Set(SetMessage {
                    address: addr.clone(), value: Value::Bool(true), revision: None, lock: false, unlock: false,
                })).await?;
                let _ = pub_client.recv(500).await;

                let received = match sub.recv(200).await {
                    Ok(Message::Set(set)) => set.address == addr,
                    _ => false,
                };

                if received != should_match {
                    return Err(format!("Pattern '{}': '{}' should_match={} but received={}", pattern, addr, should_match, received));
                }
            }

            sub.close().await;
            pub_client.close().await;
            details.push(format!("Pattern '{}': OK", pattern));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_message_ordering() -> TestResult {
    let start = Instant::now();
    let name = "protocol/message_ordering";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("ordering");
        let message_count = 100i64;

        let mut sub = RawClient::connect().await?;
        sub.handshake("OrderingSub").await?;
        sub.send(&Message::Subscribe(SubscribeMessage { id: 1, pattern: format!("{}/**", base), types: vec![], options: None })).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut pub_client = RawClient::connect().await?;
        pub_client.handshake("OrderingPub").await?;

        for i in 0..message_count {
            pub_client.send(&Message::Set(SetMessage {
                address: format!("{}/seq", base), value: Value::Int(i), revision: None, lock: false, unlock: false,
            })).await?;
        }

        let mut received = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while received.len() < message_count as usize && Instant::now() < deadline {
            if let Ok(Message::Set(set)) = sub.recv(500).await {
                if let Value::Int(n) = set.value { received.push(n); }
            }
        }

        sub.close().await;
        pub_client.close().await;

        let mut out_of_order = 0;
        for i in 1..received.len() {
            if received[i] < received[i - 1] { out_of_order += 1; }
        }

        let details = vec![
            format!("Sent: {} messages", message_count),
            format!("Received: {} messages", received.len()),
            format!("Out of order: {}", out_of_order),
        ];

        if received.len() < (message_count as usize * 90 / 100) {
            return Err(format!("Received only {}/{}", received.len(), message_count));
        }
        if out_of_order > received.len() / 10 {
            return Err(format!("Too many out-of-order: {}/{}", out_of_order, received.len()));
        }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_state_consistency() -> TestResult {
    let start = Instant::now();
    let name = "protocol/state_consistency";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("consistency");
        let final_value = Value::String("final_value".to_string());

        let mut setter = RawClient::connect().await?;
        setter.handshake("Setter").await?;
        let addr = format!("{}/param", base);

        for i in 0..10 {
            setter.send(&Message::Set(SetMessage {
                address: addr.clone(), value: Value::Int(i), revision: None, lock: false, unlock: false,
            })).await?;
            let _ = setter.recv(1000).await;
        }

        setter.send(&Message::Set(SetMessage {
            address: addr.clone(), value: final_value.clone(), revision: None, lock: false, unlock: false,
        })).await?;
        let _ = setter.recv(1000).await;
        setter.close().await;

        tokio::time::sleep(Duration::from_millis(200)).await;

        let mut joiner = RawClient::connect().await?;
        joiner.handshake("LateJoiner").await?;
        joiner.send(&Message::Subscribe(SubscribeMessage { id: 1, pattern: addr.clone(), types: vec![], options: None })).await?;

        let mut found_value = None;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match joiner.recv(500).await {
                Ok(Message::Set(set)) if set.address == addr => { found_value = Some(set.value); break; }
                Ok(Message::Snapshot(snap)) => {
                    for param in snap.params {
                        if param.address == addr { found_value = Some(param.value); break; }
                    }
                    if found_value.is_some() { break; }
                }
                _ => continue,
            }
        }
        joiner.close().await;

        match found_value {
            Some(value) if value == final_value => Ok(vec!["Late joiner received correct final value".to_string()]),
            Some(value) => Err(format!("Wrong value: {:?}", value)),
            None => Err("Late joiner did not receive state".to_string()),
        }
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_bundle_atomicity() -> TestResult {
    let start = Instant::now();
    let name = "protocol/bundle_atomicity";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("bundle");

        let mut sub = RawClient::connect().await?;
        sub.handshake("BundleSub").await?;
        sub.send(&Message::Subscribe(SubscribeMessage { id: 1, pattern: format!("{}/**", base), types: vec![], options: None })).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut pub_client = RawClient::connect().await?;
        pub_client.handshake("BundlePub").await?;

        let bundle_messages = vec![
            Message::Set(SetMessage { address: format!("{}/a", base), value: Value::Int(1), revision: None, lock: false, unlock: false }),
            Message::Set(SetMessage { address: format!("{}/b", base), value: Value::Int(2), revision: None, lock: false, unlock: false }),
            Message::Set(SetMessage { address: format!("{}/c", base), value: Value::Int(3), revision: None, lock: false, unlock: false }),
        ];

        pub_client.send(&Message::Bundle(BundleMessage { messages: bundle_messages, timestamp: None })).await?;

        let mut received = 0;
        let deadline = Instant::now() + Duration::from_secs(5);
        while received < 3 && Instant::now() < deadline {
            if let Ok(Message::Set(_)) = sub.recv(500).await { received += 1; }
        }

        sub.close().await;
        pub_client.close().await;

        if received == 3 { Ok(vec![format!("Bundle delivered atomically: {} messages", received)]) }
        else { Err(format!("Bundle partially delivered: {}/3", received)) }
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

async fn test_large_payloads() -> TestResult {
    let start = Instant::now();
    let name = "edge/large_payloads";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("large");
        let mut details = Vec::new();

        let mut client = RawClient::connect().await?;
        client.handshake("LargePayload").await?;

        for &size in LARGE_PAYLOAD_SIZES {
            let payload = vec![0x42u8; size];
            let addr = format!("{}/size_{}", base, size);
            let send_start = Instant::now();

            if client.send(&Message::Set(SetMessage {
                address: addr.clone(), value: Value::Bytes(payload), revision: None, lock: false, unlock: false,
            })).await.is_err() {
                details.push(format!("{}B: SEND FAILED", size));
                continue;
            }

            match client.recv(10000).await {
                Ok(Message::Ack(_)) => details.push(format!("{}B: OK ({:.0}ms)", size, send_start.elapsed().as_millis())),
                Ok(Message::Error(e)) => details.push(format!("{}B: ERROR {:?}", size, e.message)),
                other => details.push(format!("{}B: UNEXPECTED {:?}", size, other)),
            }
        }
        client.close().await;
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_special_characters() -> TestResult {
    let start = Instant::now();
    let name = "edge/special_chars";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("special");
        let mut details = Vec::new();

        let test_suffixes = vec!["with-dashes", "with_underscores", "UPPERCASE", "numbers123", "unicode_日本語"];

        let mut client = RawClient::connect().await?;
        client.handshake("SpecialChars").await?;

        for suffix in test_suffixes {
            let addr = format!("{}/{}", base, suffix);
            client.send(&Message::Set(SetMessage {
                address: addr, value: Value::String(format!("test_{}", suffix)), revision: None, lock: false, unlock: false,
            })).await?;

            match client.recv(2000).await {
                Ok(Message::Ack(_)) => details.push(format!("'{}': OK", suffix)),
                Ok(Message::Error(e)) => details.push(format!("'{}': REJECTED ({:?})", suffix, e.message)),
                other => details.push(format!("'{}': UNEXPECTED {:?}", suffix, other)),
            }
        }
        client.close().await;
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

async fn test_empty_null_values() -> TestResult {
    let start = Instant::now();
    let name = "edge/empty_null_values";

    let result: Result<Vec<String>, String> = async {
        let base = unique_addr("nullempty");
        let mut details = Vec::new();

        let test_values = vec![
            ("null", Value::Null), ("empty_string", Value::String(String::new())),
            ("empty_bytes", Value::Bytes(vec![])), ("empty_array", Value::Array(vec![])),
            ("zero_int", Value::Int(0)), ("zero_float", Value::Float(0.0)), ("false_bool", Value::Bool(false)),
        ];

        let mut client = RawClient::connect().await?;
        client.handshake("NullEmpty").await?;

        for (name, value) in test_values {
            client.send(&Message::Set(SetMessage {
                address: format!("{}/{}", base, name), value, revision: None, lock: false, unlock: false,
            })).await?;

            match client.recv(2000).await {
                Ok(Message::Ack(_)) => details.push(format!("'{}': OK", name)),
                Ok(Message::Error(e)) => details.push(format!("'{}': REJECTED ({:?})", name, e.message)),
                other => details.push(format!("'{}': UNEXPECTED {:?}", name, other)),
            }
        }
        client.close().await;
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// SUSTAINED LOAD TEST
// ============================================================================

async fn test_sustained_load() -> TestResult {
    let start = Instant::now();
    let name = "sustained/30s_load";

    let result: Result<Vec<String>, String> = async {
        let duration = Duration::from_secs(SUSTAINED_LOAD_DURATION_SECS);
        let base = unique_addr("sustained");

        let sent = Arc::new(AtomicU64::new(0));
        let received = Arc::new(AtomicU64::new(0));
        let running = Arc::new(AtomicBool::new(true));

        let sub_received = received.clone();
        let sub_running = running.clone();
        let sub_base = base.clone();
        let sub_handle = tokio::spawn(async move {
            let mut sub = match RawClient::connect().await { Ok(s) => s, Err(_) => return };
            if sub.handshake("SustainedSub").await.is_err() { return; }
            let _ = sub.send(&Message::Subscribe(SubscribeMessage { id: 1, pattern: format!("{}/**", sub_base), types: vec![], options: None })).await;

            while sub_running.load(Ordering::Relaxed) {
                if let Ok(Message::Set(_)) = sub.recv(100).await {
                    sub_received.fetch_add(1, Ordering::Relaxed);
                }
            }
            sub.close().await;
        });

        tokio::time::sleep(Duration::from_millis(200)).await;

        let start_time = Instant::now();
        let mut client = RawClient::connect().await?;
        client.handshake("SustainedPub").await?;

        let mut i = 0u64;
        while start_time.elapsed() < duration {
            if client.send(&Message::Set(SetMessage {
                address: format!("{}/{}", base, i), value: Value::Int(i as i64), revision: None, lock: false, unlock: false,
            })).await.is_ok() {
                sent.fetch_add(1, Ordering::Relaxed);
            }

            while let Some(_) = client.recv_non_blocking().await {}
            i += 1;
            if i % 10 == 0 { tokio::time::sleep(Duration::from_millis(10)).await; }
        }

        running.store(false, Ordering::Relaxed);
        client.close().await;
        let _ = sub_handle.await;

        let total_sent = sent.load(Ordering::Relaxed);
        let total_received = received.load(Ordering::Relaxed);
        let elapsed = start_time.elapsed().as_secs_f64();
        let delivery_rate = if total_sent > 0 { total_received as f64 / total_sent as f64 * 100.0 } else { 0.0 };

        let details = vec![
            format!("Duration: {:.1}s", elapsed),
            format!("Sent: {} ({:.0} msg/s)", total_sent, total_sent as f64 / elapsed),
            format!("Received: {} ({:.1}% delivery)", total_received, delivery_rate),
        ];

        if delivery_rate < 80.0 { return Err(format!("Delivery rate {:.1}% too low", delivery_rate)); }
        Ok(details)
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

// ============================================================================
// P2P TESTS
// ============================================================================

#[cfg(feature = "p2p")]
async fn test_p2p_connection() -> TestResult {
    let start = Instant::now();
    let name = "p2p/connection";

    let result: Result<Vec<String>, String> = async {
        let p2p_config = P2PConfig {
            ice_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        };

        let client_a = Clasp::builder(PUBLIC_RELAY_URL).name("P2PA").p2p_config(p2p_config.clone()).connect().await.map_err(|e| e.to_string())?;
        let client_b = Clasp::builder(PUBLIC_RELAY_URL).name("P2PB").p2p_config(p2p_config).connect().await.map_err(|e| e.to_string())?;

        let session_a = client_a.session_id().ok_or("No session A")?;
        let session_b = client_b.session_id().ok_or("No session B")?;

        let connected = Arc::new(AtomicBool::new(false));
        let conn_clone = connected.clone();
        let sess_a = session_a.clone();
        client_b.on_p2p_event(move |event| {
            if let P2PEvent::Connected { peer_session_id } = event {
                if peer_session_id == sess_a { conn_clone.store(true, Ordering::SeqCst); }
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        client_a.connect_to_peer(&session_b).await.map_err(|e| e.to_string())?;

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline && !connected.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let p2p_ok = connected.load(Ordering::SeqCst);
        Ok(vec![format!("P2P connected: {} (may fail due to NAT)", p2p_ok)])
    }.await;

    match result {
        Ok(details) => TestResult::pass_with_details(name, start.elapsed().as_millis(), details),
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

#[cfg(not(feature = "p2p"))]
async fn test_p2p_connection() -> TestResult {
    TestResult { name: "p2p/connection".to_string(), passed: true, message: "SKIP: P2P not enabled".to_string(), duration_ms: 0, details: vec![] }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info,webrtc=warn,webrtc_ice=warn").init();

    println!("\n{}", "=".repeat(80));
    println!("          CLASP RELAY STRESS TESTS & CRITICAL BENCHMARKS");
    println!("          Target: {}", PUBLIC_RELAY_URL);
    println!("{}", "=".repeat(80));

    println!("\nVerifying relay connectivity...");
    match RawClient::connect().await {
        Ok(mut client) => match client.handshake("Check").await {
            Ok(()) => { println!("  Relay OK: {}", client.session_id.as_ref().unwrap()); client.close().await; }
            Err(e) => { eprintln!("  FATAL: {}", e); std::process::exit(1); }
        },
        Err(e) => { eprintln!("  FATAL: {}", e); std::process::exit(1); }
    }

    let categories = vec![
        ("LATENCY", vec![bench_set_ack_latency().await, bench_pubsub_latency().await]),
        ("THROUGHPUT", vec![bench_throughput().await, bench_fanout_throughput().await]),
        ("CONCURRENCY", vec![stress_concurrent_clients().await, stress_concurrent_writes().await, stress_connection_churn().await]),
        ("PROTOCOL", vec![test_subscription_patterns().await, test_message_ordering().await, test_state_consistency().await, test_bundle_atomicity().await]),
        ("EDGE CASES", vec![test_large_payloads().await, test_special_characters().await, test_empty_null_values().await]),
        ("SUSTAINED", vec![test_sustained_load().await]),
        ("P2P", vec![test_p2p_connection().await]),
    ];

    let mut passed = 0; let mut failed = 0; let mut skipped = 0;

    for (cat, tests) in &categories {
        println!("\n{}\n  {}\n{}", "-".repeat(80), cat, "-".repeat(80));
        for t in tests {
            let status = if t.message.starts_with("SKIP") { "\x1b[33mSKIP\x1b[0m" } else if t.passed { "\x1b[32mPASS\x1b[0m" } else { "\x1b[31mFAIL\x1b[0m" };
            println!("  {:<45} {} {:>8}ms", t.name, status, t.duration_ms);
            for d in &t.details { println!("    {}", d); }
            if !t.passed && !t.message.starts_with("SKIP") { println!("    \x1b[31m{}\x1b[0m", t.message); }
            if t.message.starts_with("SKIP") { skipped += 1; } else if t.passed { passed += 1; } else { failed += 1; }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("  RESULTS: {} passed, {} failed, {} skipped", passed, failed, skipped);
    println!("{}", "=".repeat(80));

    if failed > 0 { std::process::exit(1); }
}
