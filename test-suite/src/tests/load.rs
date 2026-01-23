//! Load Testing Framework
//!
//! These tests verify that CLASP can handle high-throughput scenarios:
//! 1. High message rates (10k+ messages/second)
//! 2. Multiple concurrent clients
//! 3. Large payloads
//! 4. Sustained load over time
//! 5. Memory and resource usage

use crate::tests::helpers::run_test;
use crate::{TestResult, TestSuite};
use clasp_core::{
    codec::{decode, encode},
    Message, PublishMessage, SetMessage, SignalType, Value,
};
use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub async fn run_tests(suite: &mut TestSuite) {
    suite.add_result(test_encoding_throughput().await);
    suite.add_result(test_decoding_throughput().await);
    suite.add_result(test_roundtrip_throughput().await);
    suite.add_result(test_large_payload().await);
    suite.add_result(test_many_small_messages().await);
    suite.add_result(test_concurrent_encoding().await);
    suite.add_result(test_memory_stability().await);
    suite.add_result(test_latency_distribution().await);
}

/// Test: Message encoding throughput
async fn test_encoding_throughput() -> TestResult {
    run_test(
        "Load: Encoding throughput (10K msgs)",
        Duration::from_secs(30),
        || async {
            let msg = Message::Set(SetMessage {
                address: "/test/throughput".to_string(),
                value: Value::Float(0.5),
                revision: Some(1),
                lock: false,
                unlock: false,
            });

            let count = 10_000;
            let start = Instant::now();

            for _ in 0..count {
                let _ = encode(&msg).map_err(|e| format!("Encode failed: {:?}", e))?;
            }

            let elapsed = start.elapsed();
            let rate = count as f64 / elapsed.as_secs_f64();

            if rate < 50_000.0 {
                // Expect at least 50k/sec
                return Err(format!("Encoding rate too low: {:.0} msg/s", rate));
            }

            tracing::info!("Encoding rate: {:.0} msg/s", rate);
            Ok(())
        },
    )
    .await
}

/// Test: Message decoding throughput
async fn test_decoding_throughput() -> TestResult {
    run_test(
        "Load: Decoding throughput (10K msgs)",
        Duration::from_secs(30),
        || async {
            let msg = Message::Set(SetMessage {
                address: "/test/throughput".to_string(),
                value: Value::Float(0.5),
                revision: Some(1),
                lock: false,
                unlock: false,
            });

            // Pre-encode messages
            let encoded = encode(&msg).map_err(|e| format!("Encode failed: {:?}", e))?;

            let count = 10_000;
            let start = Instant::now();

            for _ in 0..count {
                let _ = decode(&encoded).map_err(|e| format!("Decode failed: {:?}", e))?;
            }

            let elapsed = start.elapsed();
            let rate = count as f64 / elapsed.as_secs_f64();

            if rate < 50_000.0 {
                return Err(format!("Decoding rate too low: {:.0} msg/s", rate));
            }

            tracing::info!("Decoding rate: {:.0} msg/s", rate);
            Ok(())
        },
    )
    .await
}

/// Test: Full roundtrip throughput
async fn test_roundtrip_throughput() -> TestResult {
    run_test(
        "Load: Roundtrip throughput (5K msgs)",
        Duration::from_secs(30),
        || async {
            let count = 5_000;
            let start = Instant::now();

            for i in 0..count {
                let msg = Message::Set(SetMessage {
                    address: format!("/test/roundtrip/{}", i),
                    value: Value::Float(i as f64 / count as f64),
                    revision: Some(i as u64),
                    lock: false,
                    unlock: false,
                });

                let encoded = encode(&msg).map_err(|e| format!("Encode {} failed: {:?}", i, e))?;
                let _ = decode(&encoded).map_err(|e| format!("Decode {} failed: {:?}", i, e))?;
            }

            let elapsed = start.elapsed();
            let rate = count as f64 / elapsed.as_secs_f64();

            if rate < 20_000.0 {
                return Err(format!("Roundtrip rate too low: {:.0} msg/s", rate));
            }

            tracing::info!("Roundtrip rate: {:.0} msg/s", rate);
            Ok(())
        },
    )
    .await
}

/// Test: Large payload handling (tests near the 64KB limit)
async fn test_large_payload() -> TestResult {
    run_test(
        "Load: Large payload (near 64KB limit)",
        Duration::from_secs(10),
        || async {
            // Create a large array - 6000 floats is ~54KB which is under the 64KB limit
            // (Each MessagePack float is ~9 bytes with overhead)
            const ARRAY_SIZE: usize = 6000;
            let large_array: Vec<Value> = (0..ARRAY_SIZE).map(|i| Value::Float(i as f64)).collect();

            let msg = Message::Set(SetMessage {
                address: "/test/large".to_string(),
                value: Value::Array(large_array),
                revision: Some(1),
                lock: false,
                unlock: false,
            });

            let start = Instant::now();
            let encoded = encode(&msg).map_err(|e| format!("Large encode failed: {:?}", e))?;
            let encode_time = start.elapsed();

            let size = encoded.len();
            if size > 65535 {
                return Err(format!("Payload too large: {} bytes (max is 65535)", size));
            }

            let start = Instant::now();
            let (decoded, _) =
                decode(&encoded).map_err(|e| format!("Large decode failed: {:?}", e))?;
            let decode_time = start.elapsed();

            match decoded {
                Message::Set(set) => match set.value {
                    Value::Array(arr) => {
                        if arr.len() != ARRAY_SIZE {
                            return Err(format!("Array size mismatch: {}", arr.len()));
                        }
                    }
                    _ => return Err("Expected Array value".to_string()),
                },
                _ => return Err("Expected Set message".to_string()),
            }

            tracing::info!(
                "Large payload: {} bytes (~{}KB), encode {:.2}ms, decode {:.2}ms",
                size,
                size / 1024,
                encode_time.as_secs_f64() * 1000.0,
                decode_time.as_secs_f64() * 1000.0
            );

            Ok(())
        },
    )
    .await
}

/// Test: Many small messages
async fn test_many_small_messages() -> TestResult {
    run_test(
        "Load: Many small messages (50K)",
        Duration::from_secs(60),
        || async {
            let count = 50_000;
            let mut total_bytes = 0usize;
            let start = Instant::now();

            for i in 0..count {
                let msg = Message::Publish(PublishMessage {
                    address: "/s".to_string(), // Minimal address
                    signal: Some(SignalType::Stream),
                    value: Some(Value::Float(i as f64)),
                    payload: None,
                    samples: None,
                    rate: None,
                    id: None,
                    phase: None,
                    timestamp: None,
                    timeline: None,
                });

                let encoded = encode(&msg).map_err(|e| format!("Encode {} failed: {:?}", i, e))?;
                total_bytes += encoded.len();

                // Only decode every 100th to speed up
                if i % 100 == 0 {
                    let _ = decode(&encoded).map_err(|e| format!("Decode failed: {:?}", e))?;
                }
            }

            let elapsed = start.elapsed();
            let rate = count as f64 / elapsed.as_secs_f64();
            let throughput_mb = (total_bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64();

            if rate < 100_000.0 {
                return Err(format!("Small message rate too low: {:.0} msg/s", rate));
            }

            tracing::info!(
                "Small messages: {:.0} msg/s, {:.2} MB/s",
                rate,
                throughput_mb
            );

            Ok(())
        },
    )
    .await
}

/// Test: Concurrent encoding (multi-threaded)
async fn test_concurrent_encoding() -> TestResult {
    run_test(
        "Load: Concurrent encoding (4 threads)",
        Duration::from_secs(30),
        || async {
            let count_per_thread = 10_000;
            let thread_count = 4;
            let total_count = Arc::new(AtomicU64::new(0));

            let start = Instant::now();

            let mut handles = Vec::new();

            for thread_id in 0..thread_count {
                let counter = total_count.clone();

                let handle = tokio::task::spawn_blocking(move || {
                    for i in 0..count_per_thread {
                        let msg = Message::Set(SetMessage {
                            address: format!("/thread/{}/value", thread_id),
                            value: Value::Float(i as f64),
                            revision: Some(i as u64),
                            lock: false,
                            unlock: false,
                        });

                        if encode(&msg).is_ok() {
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle
                    .await
                    .map_err(|e| format!("Thread failed: {:?}", e))?;
            }

            let elapsed = start.elapsed();
            let total = total_count.load(Ordering::Relaxed);
            let rate = total as f64 / elapsed.as_secs_f64();

            let expected = (thread_count * count_per_thread) as u64;
            if total != expected {
                return Err(format!("Missing messages: {} / {}", total, expected));
            }

            if rate < 100_000.0 {
                return Err(format!("Concurrent rate too low: {:.0} msg/s", rate));
            }

            tracing::info!(
                "Concurrent rate: {:.0} msg/s across {} threads",
                rate,
                thread_count
            );
            Ok(())
        },
    )
    .await
}

/// Test: Memory stability under load
async fn test_memory_stability() -> TestResult {
    run_test(
        "Load: Memory stability (100K msgs)",
        Duration::from_secs(60),
        || async {
            let count = 100_000;

            // Encode/decode many messages without keeping references
            for batch in 0..100 {
                for i in 0..1000 {
                    let idx = batch * 1000 + i;
                    let msg = Message::Set(SetMessage {
                        address: format!("/mem/test/{}", idx),
                        value: Value::Array(vec![
                            Value::Float(idx as f64),
                            Value::String(format!("item-{}", idx)),
                        ]),
                        revision: Some(idx as u64),
                        lock: false,
                        unlock: false,
                    });

                    let encoded =
                        encode(&msg).map_err(|e| format!("Encode {} failed: {:?}", idx, e))?;
                    let _ =
                        decode(&encoded).map_err(|e| format!("Decode {} failed: {:?}", idx, e))?;

                    // Don't keep any references - let memory be freed
                }

                // Yield to allow GC/cleanup
                tokio::task::yield_now().await;
            }

            // If we get here without OOM, the test passes
            tracing::info!("Memory stability: {} messages processed", count);
            Ok(())
        },
    )
    .await
}

/// Test: Latency distribution
async fn test_latency_distribution() -> TestResult {
    run_test(
        "Load: Latency distribution (1K samples)",
        Duration::from_secs(30),
        || async {
            let mut histogram = Histogram::<u64>::new(3)
                .map_err(|e| format!("Failed to create histogram: {:?}", e))?;

            let count = 1_000;

            for i in 0..count {
                let msg = Message::Set(SetMessage {
                    address: "/latency/test".to_string(),
                    value: Value::Float(i as f64),
                    revision: Some(i as u64),
                    lock: false,
                    unlock: false,
                });

                let start = Instant::now();
                let encoded = encode(&msg).map_err(|e| format!("Encode failed: {:?}", e))?;
                let _ = decode(&encoded).map_err(|e| format!("Decode failed: {:?}", e))?;
                let elapsed = start.elapsed();

                // Record in microseconds
                let _ = histogram.record(elapsed.as_micros() as u64);
            }

            let p50 = histogram.value_at_percentile(50.0);
            let p95 = histogram.value_at_percentile(95.0);
            let p99 = histogram.value_at_percentile(99.0);
            let max = histogram.max();

            tracing::info!(
                "Latency: p50={}us, p95={}us, p99={}us, max={}us",
                p50,
                p95,
                p99,
                max
            );

            // Sanity check - roundtrip should be under 1ms at p99
            if p99 > 1000 {
                return Err(format!("P99 latency too high: {}us", p99));
            }

            Ok(())
        },
    )
    .await
}
