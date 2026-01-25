//! Memory Benchmarks
//!
//! Measures memory usage and leak detection:
//! - KB per connection
//! - Memory growth over time
//! - Leak detection after disconnects
//!
//! This benchmark requires running with `--release` for accurate measurements.

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
fn get_process_memory_kb() -> Option<u64> {
    use std::fs;
    // Read from /proc/self/statm
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let parts: Vec<&str> = statm.split_whitespace().collect();
    // Second field is RSS (resident set size) in pages
    let rss_pages: u64 = parts.get(1)?.parse().ok()?;
    // Page size is typically 4KB on Linux
    Some(rss_pages * 4)
}

#[cfg(target_os = "macos")]
fn get_process_memory_kb() -> Option<u64> {
    use std::process::Command;
    // Use ps to get RSS
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let rss_str = String::from_utf8_lossy(&output.stdout);
    rss_str.trim().parse().ok()
}

#[cfg(target_os = "windows")]
fn get_process_memory_kb() -> Option<u64> {
    // Windows implementation would use GetProcessMemoryInfo
    // For simplicity, return None (skip memory measurements)
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn get_process_memory_kb() -> Option<u64> {
    None
}

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Measure memory per connection
async fn measure_memory_per_connection(
    port: u16,
    connection_count: usize,
) -> Option<(u64, u64, f64)> {
    let url = format!("ws://127.0.0.1:{}", port);

    // Measure baseline memory
    let baseline = get_process_memory_kb()?;
    println!("    Baseline memory: {} KB", baseline);

    // Create connections
    let mut clients: Vec<Clasp> = Vec::with_capacity(connection_count);
    for _ in 0..connection_count {
        match Clasp::connect_to(&url).await {
            Ok(c) => clients.push(c),
            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }
    }

    // Force a small delay for memory to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Measure memory after connections
    let after_connect = get_process_memory_kb()?;
    let memory_used = after_connect.saturating_sub(baseline);
    let per_connection = if clients.len() > 0 {
        memory_used as f64 / clients.len() as f64
    } else {
        0.0
    };

    println!(
        "    After {} connections: {} KB (+{} KB, {:.2} KB/connection)",
        clients.len(),
        after_connect,
        memory_used,
        per_connection
    );

    Some((baseline, after_connect, per_connection))
}

/// Measure memory leak after disconnect/reconnect cycles
async fn measure_memory_leak(
    port: u16,
    cycles: usize,
    connections_per_cycle: usize,
) -> Option<i64> {
    let url = format!("ws://127.0.0.1:{}", port);

    // Baseline
    let baseline = get_process_memory_kb()?;

    for cycle in 0..cycles {
        // Create connections
        let mut clients: Vec<Clasp> = Vec::with_capacity(connections_per_cycle);
        for _ in 0..connections_per_cycle {
            if let Ok(c) = Clasp::connect_to(&url).await {
                // Do some work
                let _ = c.set("/leak/test", cycle as f64).await;
                clients.push(c);
            }
        }

        // Disconnect all
        drop(clients);

        // Small delay
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Final memory
    let final_memory = get_process_memory_kb()?;
    let growth = final_memory as i64 - baseline as i64;

    println!(
        "    After {} cycles ({} connections each): {} KB (growth: {} KB)",
        cycles, connections_per_cycle, final_memory, growth
    );

    Some(growth)
}

/// Measure memory with subscriptions
async fn measure_subscription_memory(port: u16, subscription_count: usize) -> Option<(u64, f64)> {
    let url = format!("ws://127.0.0.1:{}", port);

    let baseline = get_process_memory_kb()?;

    // Create a client
    let client = Clasp::connect_to(&url).await.ok()?;

    // Create many subscriptions
    for i in 0..subscription_count {
        let pattern = format!("/sub/{}/value", i);
        let _ = client.subscribe(&pattern, |_, _| {}).await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let after = get_process_memory_kb()?;
    let memory_used = after.saturating_sub(baseline);
    let per_subscription = memory_used as f64 / subscription_count as f64;

    println!(
        "    {} subscriptions: {} KB (+{} KB, {:.2} KB/subscription)",
        subscription_count, after, memory_used, per_subscription
    );

    Some((memory_used, per_subscription))
}

/// Measure memory with stored state
async fn measure_state_memory(port: u16, state_count: usize) -> Option<(u64, f64)> {
    let url = format!("ws://127.0.0.1:{}", port);

    let baseline = get_process_memory_kb()?;

    let client = Clasp::connect_to(&url).await.ok()?;

    // Create many state values
    for i in 0..state_count {
        let address = format!("/state/{}/value", i);
        let _ = client.set(&address, i as f64).await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let after = get_process_memory_kb()?;
    let memory_used = after.saturating_sub(baseline);
    let per_state = memory_used as f64 / state_count as f64;

    println!(
        "    {} state values: {} KB (+{} KB, {:.3} KB/value)",
        state_count, after, memory_used, per_state
    );

    Some((memory_used, per_state))
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                  CLASP MEMORY BENCHMARKS                                              ║");
    println!("║                    (Run with --release for accurate measurements)                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Check if memory measurement is available
    if get_process_memory_kb().is_none() {
        println!("⚠️  Memory measurement not available on this platform.");
        println!("   Supported platforms: Linux, macOS\n");
        return;
    }

    let port = find_port().await;
    let router = Router::new(RouterConfig {
        name: "Memory Benchmark".into(),
        max_sessions: 10000,
        session_timeout: 60,
        features: vec!["param".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 10000,
        gesture_coalescing: false,
        gesture_coalesce_interval_ms: 0,
            max_messages_per_second: 0,
            rate_limiting_enabled: false,
    });

    let addr = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("═══ Memory Per Connection ═══\n");

    for count in [10, 50, 100, 500, 1000] {
        println!("  Testing {} connections:", count);
        measure_memory_per_connection(port, count).await;

        // Allow cleanup
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n═══ Memory Leak Detection (connect/disconnect cycles) ═══\n");

    println!("  Testing 10 cycles of 100 connections each:");
    let leak = measure_memory_leak(port, 10, 100).await;
    if let Some(growth) = leak {
        if growth > 1000 {
            println!("  ⚠️  Potential memory leak detected: {} KB growth", growth);
        } else {
            println!("  ✓  No significant leak: {} KB growth", growth);
        }
    }

    println!("\n═══ Subscription Memory ═══\n");

    for count in [100, 500, 1000] {
        measure_subscription_memory(port, count).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n═══ State Storage Memory ═══\n");

    for count in [100, 1000, 10000] {
        measure_state_memory(port, count).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n═══════════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("  MEMORY TARGETS:");
    println!("  ────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!("  │ Metric                  │ Target       │ Notes                                                  │");
    println!("  ├─────────────────────────┼──────────────┼────────────────────────────────────────────────────────┤");
    println!("  │ Memory per connection   │ <50 KB       │ Base overhead per WebSocket session                    │");
    println!("  │ Memory per subscription │ <1 KB        │ Pattern matching overhead                              │");
    println!("  │ Memory per state value  │ <0.5 KB      │ State storage overhead                                 │");
    println!("  │ Leak after 1000 cycles  │ <100 KB      │ Should not grow unbounded                              │");
    println!("  ═══════════════════════════════════════════════════════════════════════════════════════════════════\n");
}
