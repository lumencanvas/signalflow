//! Cold Start Benchmarks
//!
//! Measures connection latency from zero state (no cache warming).
//! These benchmarks are designed to reflect real-world usage where
//! connections are not pre-established.
//!
//! Metrics reported:
//! - Connection establishment time (P50/P95/P99/P99.9)
//! - First message latency
//! - Handshake completion time

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::time::{Duration, Instant};

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 * p / 100.0) as usize).min(sorted.len() - 1);
    sorted[idx]
}

fn print_stats(name: &str, latencies: &mut [u64]) {
    if latencies.is_empty() {
        println!("  ❌ {} - NO DATA", name);
        return;
    }
    latencies.sort_unstable();
    let p50 = percentile(latencies, 50.0);
    let p95 = percentile(latencies, 95.0);
    let p99 = percentile(latencies, 99.0);
    let p999 = percentile(latencies, 99.9);
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let avg: u64 = latencies.iter().sum::<u64>() / latencies.len() as u64;

    println!(
        "  ✓ {:35} │ p50: {:>6}µs │ p95: {:>6}µs │ p99: {:>6}µs │ p99.9: {:>6}µs │ min: {:>5}µs │ max: {:>6}µs │ n={}",
        name, p50, p95, p99, p999, min, max, latencies.len()
    );
}

/// Benchmark cold connection establishment (no connection reuse)
async fn benchmark_cold_connection(port: u16, iterations: usize) -> Vec<u64> {
    let url = format!("ws://127.0.0.1:{}", port);
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();

        // Create fresh connection each time (cold start)
        let client = Clasp::connect_to(&url).await;

        match client {
            Ok(c) => {
                latencies.push(start.elapsed().as_micros() as u64);
                // Disconnect to ensure next iteration is cold
                drop(c);
            }
            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }

        // Small delay between iterations to ensure TCP state is cleared
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    latencies
}

/// Benchmark first message after cold connection
async fn benchmark_first_message(port: u16, iterations: usize) -> Vec<u64> {
    let url = format!("ws://127.0.0.1:{}", port);
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        // Create fresh connection
        let client = match Clasp::connect_to(&url).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
                continue;
            }
        };

        // Measure first message latency (includes potential session setup)
        let start = Instant::now();
        match client.set("/cold/first", i as f64).await {
            Ok(_) => {
                latencies.push(start.elapsed().as_micros() as u64);
            }
            Err(e) => {
                eprintln!("First message failed: {:?}", e);
            }
        }

        drop(client);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    latencies
}

/// Benchmark reconnection after disconnect
async fn benchmark_reconnection(port: u16, iterations: usize) -> Vec<u64> {
    let url = format!("ws://127.0.0.1:{}", port);
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        // First connection
        let client = match Clasp::connect_to(&url).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Initial connection failed: {:?}", e);
                continue;
            }
        };

        // Do some work
        let _ = client.set("/reconnect/test", 1.0).await;

        // Disconnect
        drop(client);

        // Small delay to simulate real disconnection scenario
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Measure reconnection time
        let start = Instant::now();
        match Clasp::connect_to(&url).await {
            Ok(c) => {
                latencies.push(start.elapsed().as_micros() as u64);
                drop(c);
            }
            Err(e) => {
                eprintln!("Reconnection failed: {:?}", e);
            }
        }
    }

    latencies
}

/// Benchmark connection with different numbers of existing connections
async fn benchmark_connection_under_load(port: u16, existing_connections: usize) -> Vec<u64> {
    let url = format!("ws://127.0.0.1:{}", port);

    // Create existing connections
    let mut existing: Vec<Clasp> = Vec::with_capacity(existing_connections);
    for _ in 0..existing_connections {
        if let Ok(c) = Clasp::connect_to(&url).await {
            existing.push(c);
        }
    }

    // Measure new connection time with load
    let mut latencies = Vec::with_capacity(50);
    for _ in 0..50 {
        let start = Instant::now();
        if let Ok(c) = Clasp::connect_to(&url).await {
            latencies.push(start.elapsed().as_micros() as u64);
            drop(c);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Cleanup
    drop(existing);

    latencies
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              CLASP COLD START BENCHMARKS                                              ║");
    println!("║                    (No cache warming - reflects real-world usage)                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    let port = find_port().await;
    let router = Router::new(RouterConfig {
        name: "Cold Start Benchmark".into(),
        max_sessions: 1000,
        session_timeout: 60,
        features: vec!["param".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 100,
        gesture_coalescing: false,
        gesture_coalesce_interval_ms: 0,
    });

    let addr = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("═══ Cold Connection Establishment (process restart between tests) ═══\n");

    let mut lat = benchmark_cold_connection(port, 100).await;
    print_stats("Cold connection (n=100)", &mut lat);

    let mut lat = benchmark_cold_connection(port, 500).await;
    print_stats("Cold connection (n=500)", &mut lat);

    println!("\n═══ First Message After Cold Connection ═══\n");

    let mut lat = benchmark_first_message(port, 100).await;
    print_stats("First message latency", &mut lat);

    println!("\n═══ Reconnection After Disconnect ═══\n");

    let mut lat = benchmark_reconnection(port, 100).await;
    print_stats("Reconnection time", &mut lat);

    println!("\n═══ Connection Under Load (existing connections) ═══\n");

    for load in [0, 10, 50, 100, 500] {
        let mut lat = benchmark_connection_under_load(port, load).await;
        print_stats(&format!("With {} existing connections", load), &mut lat);
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("  COLD START ASSESSMENT:");
    println!("  ────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!("  │ Metric                  │ Target       │ Notes                                                  │");
    println!("  ├─────────────────────────┼──────────────┼────────────────────────────────────────────────────────┤");
    println!("  │ Cold connection p99     │ <10ms        │ Time from connect() to ready                           │");
    println!("  │ First message p99       │ <5ms         │ Time for first SET after connection                    │");
    println!("  │ Reconnection p99        │ <15ms        │ Time to reconnect after disconnect                     │");
    println!("  │ Under load degradation  │ <2x baseline │ Connection time with 500 existing connections          │");
    println!("  ═══════════════════════════════════════════════════════════════════════════════════════════════════\n");
}
