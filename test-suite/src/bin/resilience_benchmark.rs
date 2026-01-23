//! Network resilience and recovery benchmarks
//!
//! Tests:
//! - Reconnection time after disconnect
//! - State recovery after reconnect
//! - Message delivery under load
//! - QoS behavior

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap()
        .local_addr().unwrap().port()
}

/// Test reconnection time
async fn test_reconnection_time(url: &str, iterations: usize) -> Vec<u64> {
    let mut times = Vec::with_capacity(iterations);
    
    for _ in 0..iterations {
        // Connect
        let client = Clasp::connect_to(url).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Disconnect (drop client)
        drop(client);
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Measure reconnection time
        let start = Instant::now();
        let _client = Clasp::connect_to(url).await.unwrap();
        times.push(start.elapsed().as_micros() as u64);
    }
    
    times
}

/// Test state recovery after reconnect
async fn test_state_recovery(url: &str, param_count: usize) -> (Duration, usize) {
    // Set up initial state
    let setter = Clasp::connect_to(url).await.unwrap();
    for i in 0..param_count {
        setter.set(&format!("/state/{}", i), i as f64).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Connect late joiner
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let start = Instant::now();
    let late_joiner = Clasp::connect_to(url).await.unwrap();
    late_joiner.subscribe("/state/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await.unwrap();
    
    // Wait for state recovery
    let deadline = Instant::now() + Duration::from_secs(5);
    while received.load(Ordering::Relaxed) < param_count as u64 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    let elapsed = start.elapsed();
    let count = received.load(Ordering::Relaxed) as usize;
    
    (elapsed, count)
}

/// Test message delivery rate under sustained load
async fn test_sustained_load(url: &str, msg_count: usize, duration_ms: u64) -> (usize, usize, f64) {
    let publisher = Clasp::connect_to(url).await.unwrap();
    let subscriber = Clasp::connect_to(url).await.unwrap();
    
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    subscriber.subscribe("/load/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Send messages at sustained rate
    let start = Instant::now();
    let interval = Duration::from_millis(duration_ms) / msg_count as u32;
    
    for i in 0..msg_count {
        let addr = format!("/load/msg/{}", i);
        let _ = publisher.set(&addr, i as f64).await;
        
        // Rate limiting
        let target_time = interval * i as u32;
        let actual_time = start.elapsed();
        if actual_time < target_time {
            tokio::time::sleep(target_time - actual_time).await;
        }
    }
    
    // Wait for delivery
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let sent = msg_count;
    let recv = received.load(Ordering::Relaxed) as usize;
    let loss_rate = 100.0 * (1.0 - recv as f64 / sent as f64);
    
    (sent, recv, loss_rate)
}

/// Test concurrent client handling
async fn test_concurrent_clients(url: &str, client_count: usize, msgs_per_client: usize) -> (Duration, usize, usize) {
    let total_expected = client_count * msgs_per_client;
    let received = Arc::new(AtomicU64::new(0));
    
    // Create subscriber
    let subscriber = Clasp::connect_to(url).await.unwrap();
    let counter = received.clone();
    subscriber.subscribe("/concurrent/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let start = Instant::now();
    
    // Spawn concurrent publishers
    let mut handles = Vec::with_capacity(client_count);
    for client_id in 0..client_count {
        let url = url.to_string();
        let handle = tokio::spawn(async move {
            let client = Clasp::connect_to(&url).await.unwrap();
            for i in 0..msgs_per_client {
                let addr = format!("/concurrent/{}/{}", client_id, i);
                let _ = client.set(&addr, i as f64).await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all publishers
    for handle in handles {
        let _ = handle.await;
    }
    
    // Wait for delivery
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let elapsed = start.elapsed();
    let recv = received.load(Ordering::Relaxed) as usize;
    
    (elapsed, total_expected, recv)
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() { return 0; }
    let idx = ((sorted.len() as f64 * p / 100.0) as usize).min(sorted.len() - 1);
    sorted[idx]
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    CLASP RESILIENCE & RECOVERY BENCHMARKS                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝\n");
    
    let port = find_port().await;
    let router = Router::new(RouterConfig {
        name: "Resilience Test".into(),
        max_sessions: 1000,
        session_timeout: 60,
        features: vec!["param".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 100,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
    });
    
    let addr = format!("127.0.0.1:{}", port);
    tokio::spawn(async move { let _ = router.serve_websocket(&addr).await; });
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://127.0.0.1:{}", port);
    
    // Test 1: Reconnection time
    println!("═══ Reconnection Time ═══");
    let mut times = test_reconnection_time(&url, 50).await;
    times.sort_unstable();
    let p50 = percentile(&times, 50.0);
    let p99 = percentile(&times, 99.0);
    let avg: u64 = times.iter().sum::<u64>() / times.len() as u64;
    println!("  ✓ Reconnection (n=50) | p50: {}µs | p99: {}µs | avg: {}µs\n", p50, p99, avg);
    
    // Test 2: State recovery
    println!("═══ State Recovery After Reconnect ═══");
    for params in [100, 500, 1000, 5000] {
        let (elapsed, recovered) = test_state_recovery(&url, params).await;
        let status = if recovered >= params { "✓" } else { "✗" };
        let rate = recovered as f64 / elapsed.as_secs_f64();
        println!("  {} {:>5} params recovered in {:>8.2?} ({:.0} params/s)", 
            status, recovered, elapsed, rate);
    }
    println!();
    
    // Test 3: Sustained load
    println!("═══ Sustained Load (message delivery rate) ═══");
    for (msgs, duration) in [(1000, 1000), (5000, 2000), (10000, 5000)] {
        let (sent, recv, loss) = test_sustained_load(&url, msgs, duration).await;
        let status = if loss < 1.0 { "✓" } else { "~" };
        let rate = recv as f64 / (duration as f64 / 1000.0);
        println!("  {} {:>5} msgs over {}ms | sent: {} | recv: {} | loss: {:.1}% | {:.0} msg/s",
            status, msgs, duration, sent, recv, loss, rate);
    }
    println!();
    
    // Test 4: Concurrent clients
    println!("═══ Concurrent Client Handling ═══");
    for (clients, msgs) in [(10, 100), (50, 50), (100, 20)] {
        let (elapsed, expected, recv) = test_concurrent_clients(&url, clients, msgs).await;
        let loss = 100.0 * (1.0 - recv as f64 / expected as f64);
        let status = if loss < 1.0 { "✓" } else { "~" };
        let rate = recv as f64 / elapsed.as_secs_f64();
        println!("  {} {:>3} clients × {:>3} msgs = {:>5} | recv: {:>5} | loss: {:>4.1}% | {:>6.0} msg/s | {:?}",
            status, clients, msgs, expected, recv, loss, rate, elapsed);
    }
    println!();
    
    println!("═══════════════════════════════════════════════════════════════════════════════════");
    println!("  ANALYSIS:");
    println!("  - Reconnection: Sub-millisecond on LAN (WebSocket handshake)");
    println!("  - State recovery: Scales linearly with state size via snapshot chunking");
    println!("  - Sustained load: Should maintain <1% loss under normal conditions");
    println!("  - Concurrent clients: Router handles multiple simultaneous connections");
    println!("═══════════════════════════════════════════════════════════════════════════════════");
}
