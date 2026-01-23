//! Real-World CLASP Benchmarks
//!
//! These benchmarks measure actual system throughput, not just codec speed.
//! They include:
//! - End-to-end latency (pub → router → sub)
//! - Fanout curves (1 to 1000 subscribers)
//! - Wildcard routing costs
//! - State overhead
//! - Address table scaling

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Test Infrastructure
// ============================================================================

async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

struct TestRouter {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestRouter {
    async fn start() -> Self {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let router = Router::new(RouterConfig {
            name: "Benchmark Router".to_string(),
            max_sessions: 2000,
            session_timeout: 60,
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
            ],
            security_mode: SecurityMode::Open,
            max_subscriptions_per_session: 1000,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        });

        let handle = tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        Self { port, handle }
    }

    fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    fn stop(self) {
        self.handle.abort();
    }
}

// ============================================================================
// SCENARIO A: End-to-End Single Hop
// ============================================================================

async fn benchmark_e2e_single_hop(transport: &str) -> BenchmarkResult {
    let name = format!("E2E single hop ({})", transport);
    let router = TestRouter::start().await;
    
    // Create subscriber
    let received = Arc::new(AtomicU64::new(0));
    let recv_counter = received.clone();
    
    let subscriber = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => return BenchmarkResult::error(&name, format!("Sub connect failed: {}", e)),
    };
    
    let _ = subscriber.subscribe("/bench/**", move |_, _| {
        recv_counter.fetch_add(1, Ordering::Relaxed);
    }).await;
    
    // Small delay for subscription to register
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Create publisher
    let publisher = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => return BenchmarkResult::error(&name, format!("Pub connect failed: {}", e)),
    };
    
    // Warmup
    for i in 0..100 {
        let _ = publisher.set("/bench/warmup", i as f64).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    received.store(0, Ordering::SeqCst);
    
    // Benchmark
    let msg_count = 10_000u64;
    let start = Instant::now();
    
    for i in 0..msg_count {
        let _ = publisher.set("/bench/value", i as f64).await;
    }
    
    // Wait for delivery
    let deadline = Instant::now() + Duration::from_secs(10);
    while received.load(Ordering::Relaxed) < msg_count && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let delivered = received.load(Ordering::Relaxed);
    
    router.stop();
    
    BenchmarkResult {
        name,
        throughput: delivered as f64 / elapsed.as_secs_f64(),
        latency_avg_us: (elapsed.as_micros() as f64) / (delivered as f64),
        messages_sent: msg_count,
        messages_received: delivered,
        elapsed,
        error: None,
    }
}

// ============================================================================
// SCENARIO B: Fanout Curve
// ============================================================================

async fn benchmark_fanout(subscriber_count: usize) -> BenchmarkResult {
    let name = format!("Fanout to {} subscribers", subscriber_count);
    let router = TestRouter::start().await;
    
    // Create counters for each subscriber
    let counters: Vec<Arc<AtomicU64>> = (0..subscriber_count)
        .map(|_| Arc::new(AtomicU64::new(0)))
        .collect();
    
    // Create subscribers
    let mut subscribers = Vec::with_capacity(subscriber_count);
    for i in 0..subscriber_count {
        let counter = counters[i].clone();
        match Clasp::connect_to(&router.url()).await {
            Ok(client) => {
                let _ = client.subscribe("/fanout/**", move |_, _| {
                    counter.fetch_add(1, Ordering::Relaxed);
                }).await;
                subscribers.push(client);
            }
            Err(e) => {
                router.stop();
                return BenchmarkResult::error(&name, format!("Sub {} connect failed: {}", i, e));
            }
        }
    }
    
    // Let subscriptions settle
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Publisher
    let publisher = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Pub connect failed: {}", e));
        }
    };
    
    // Reset counters
    for c in &counters {
        c.store(0, Ordering::SeqCst);
    }
    
    // Benchmark
    let msg_count = 1000u64;
    let expected_total = msg_count * subscriber_count as u64;
    let start = Instant::now();
    
    for i in 0..msg_count {
        let _ = publisher.set("/fanout/value", i as f64).await;
    }
    
    // Wait for all deliveries
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let total: u64 = counters.iter().map(|c| c.load(Ordering::Relaxed)).sum();
        if total >= expected_total || Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let total_delivered: u64 = counters.iter().map(|c| c.load(Ordering::Relaxed)).sum();
    
    router.stop();
    
    BenchmarkResult {
        name,
        throughput: total_delivered as f64 / elapsed.as_secs_f64(),
        latency_avg_us: (elapsed.as_micros() as f64) / (msg_count as f64),
        messages_sent: msg_count,
        messages_received: total_delivered,
        elapsed,
        error: None,
    }
}

// ============================================================================
// SCENARIO C: Address Table Scale
// ============================================================================

async fn benchmark_address_scale(address_count: usize) -> BenchmarkResult {
    let name = format!("Address scale ({} addresses)", address_count);
    let router = TestRouter::start().await;
    
    let client = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    // Populate addresses
    let populate_start = Instant::now();
    for i in 0..address_count {
        let _ = client.set(&format!("/addr/{}/value", i), i as f64).await;
    }
    let populate_time = populate_start.elapsed();
    
    // Measure read back time (one at a time - simulates random access)
    let read_start = Instant::now();
    let mut read_count = 0usize;
    for i in (0..address_count).step_by(address_count / 100.max(1)) {
        if client.get(&format!("/addr/{}/value", i)).await.is_ok() {
            read_count += 1;
        }
    }
    let read_time = read_start.elapsed();
    
    router.stop();
    
    BenchmarkResult {
        name: format!("{} (write: {:?}, read: {:?})", name, populate_time, read_time),
        throughput: address_count as f64 / populate_time.as_secs_f64(),
        latency_avg_us: (populate_time.as_micros() as f64) / (address_count as f64),
        messages_sent: address_count as u64,
        messages_received: read_count as u64,
        elapsed: populate_time + read_time,
        error: None,
    }
}

// ============================================================================
// SCENARIO D: Wildcard Routing Cost
// ============================================================================

async fn benchmark_wildcard_cost(pattern_type: &str) -> BenchmarkResult {
    let name = format!("Wildcard routing ({})", pattern_type);
    let router = TestRouter::start().await;
    
    // Create subscriber with specific pattern FIRST
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let subscriber = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Sub connect failed: {}", e));
        }
    };
    
    // Pattern and expected match count
    // Based on 1000 msgs: zone = i % 100 (0-99), fixture = (i / 100) % 10 (0-9)
    let (pattern, expected_matches) = match pattern_type {
        "exact" => ("/lights/zone50/fixture5/brightness", 1u64),       // Only i=550 matches
        "single" => ("/lights/zone50/*/brightness", 10u64),            // zone50, fixtures 0-9
        "globstar" => ("/lights/**", 1000u64),                         // All 1000 messages
        "complex" => ("/lights/zone5*/fixture*/brightness", 110u64),   // zone5 + zone50-59 = 11 zones × 10 fixtures
        _ => ("/lights/**", 1000u64),
    };
    
    let _ = subscriber.subscribe(pattern, move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Publisher
    let sender = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    // Benchmark message delivery
    let msg_count = 1000u64;
    let start = Instant::now();
    
    // Send to 100 zones x 10 fixtures = 1000 unique addresses
    for i in 0..msg_count {
        let zone = i % 100;
        let fixture = (i / 100) % 10;
        let _ = sender.set(&format!("/lights/zone{}/fixture{}/brightness", zone, fixture), i as f64).await;
    }
    
    // Wait for expected deliveries (or timeout)
    let deadline = Instant::now() + Duration::from_secs(5);
    while received.load(Ordering::Relaxed) < expected_matches && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let delivered = received.load(Ordering::Relaxed);
    
    router.stop();
    
    BenchmarkResult {
        name: format!("{} (expect {})", name, expected_matches),
        throughput: delivered as f64 / elapsed.as_secs_f64(),
        latency_avg_us: (elapsed.as_micros() as f64) / (delivered.max(1) as f64),
        messages_sent: msg_count,
        messages_received: delivered,
        elapsed,
        error: None,
    }
}

// ============================================================================
// SCENARIO E: State Overhead
// ============================================================================

async fn benchmark_state_overhead(signal_type: &str) -> BenchmarkResult {
    let name = format!("Signal type: {}", signal_type);
    let router = TestRouter::start().await;
    
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let subscriber = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    // Subscribe to pattern
    let _ = subscriber.subscribe("/bench/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await;
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let publisher = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    let msg_count = 10_000u64;
    let start = Instant::now();
    
    for i in 0..msg_count {
        match signal_type {
            "param" => { let _ = publisher.set("/bench/value", i as f64).await; }
            "event" => { let _ = publisher.emit("/bench/event", i as f64).await; }
            "stream" => { let _ = publisher.stream("/bench/stream", i as f64).await; }
            _ => { let _ = publisher.set("/bench/value", i as f64).await; }
        }
    }
    
    let deadline = Instant::now() + Duration::from_secs(10);
    while received.load(Ordering::Relaxed) < msg_count && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let delivered = received.load(Ordering::Relaxed);
    
    router.stop();
    
    BenchmarkResult {
        name,
        throughput: delivered as f64 / elapsed.as_secs_f64(),
        latency_avg_us: (elapsed.as_micros() as f64) / (delivered as f64),
        messages_sent: msg_count,
        messages_received: delivered,
        elapsed,
        error: None,
    }
}

// ============================================================================
// SCENARIO F: Late Joiner Replay
// ============================================================================

async fn benchmark_late_joiner(param_count: usize) -> BenchmarkResult {
    let name = if param_count >= 1000 {
        format!("Late joiner replay ({}k params)", param_count / 1000)
    } else {
        format!("Late joiner replay ({} params)", param_count)
    };
    let router = TestRouter::start().await;
    
    let setter = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    // Pre-populate state
    for i in 0..param_count {
        let _ = setter.set(&format!("/state/{}", i), i as f64).await;
    }
    
    // Give router time to process all state
    let settle_time = if param_count > 1000 { 500 } else { 100 };
    tokio::time::sleep(Duration::from_millis(settle_time)).await;
    
    // Late joiner connects and subscribes
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let start = Instant::now();
    
    let late_joiner = match Clasp::connect_to(&router.url()).await {
        Ok(c) => c,
        Err(e) => {
            router.stop();
            return BenchmarkResult::error(&name, format!("Connect failed: {}", e));
        }
    };
    
    let _ = late_joiner.subscribe("/state/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await;
    
    // Wait for snapshot with reasonable timeout
    let timeout_secs = if param_count > 5000 { 30 } else { 10 };
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    
    // Check more frequently for small param counts
    let check_interval = if param_count < 100 { 5 } else { 10 };
    
    while received.load(Ordering::Relaxed) < param_count as u64 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(check_interval)).await;
    }
    
    let elapsed = start.elapsed();
    let delivered = received.load(Ordering::Relaxed);
    
    router.stop();
    
    // Determine if this is a timeout or success
    let timed_out = delivered < param_count as u64 && elapsed.as_secs() >= timeout_secs;
    
    BenchmarkResult {
        name: if timed_out { 
            format!("{} (TIMEOUT after {}s)", name, timeout_secs)
        } else { 
            name 
        },
        throughput: delivered as f64 / elapsed.as_secs_f64(),
        latency_avg_us: if delivered > 0 { elapsed.as_micros() as f64 / delivered as f64 } else { 0.0 },
        messages_sent: param_count as u64,
        messages_received: delivered,
        elapsed,
        error: None,
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    throughput: f64,
    latency_avg_us: f64,
    messages_sent: u64,
    messages_received: u64,
    elapsed: Duration,
    error: Option<String>,
}

impl BenchmarkResult {
    fn error(name: &str, msg: String) -> Self {
        Self {
            name: name.to_string(),
            throughput: 0.0,
            latency_avg_us: 0.0,
            messages_sent: 0,
            messages_received: 0,
            elapsed: Duration::ZERO,
            error: Some(msg),
        }
    }
    
    fn print(&self) {
        if let Some(ref err) = self.error {
            println!("❌ {} - ERROR: {}", self.name, err);
        } else {
            // Show delivery rate (received vs expected for wildcard tests, or sent vs received for throughput tests)
            let status = if self.messages_received >= self.messages_sent {
                "✓"
            } else if self.messages_received > 0 {
                "~" // Partial delivery
            } else {
                "✗"
            };
            
            println!(
                "{} {} | {:.0} msg/s | {:.0}µs avg | recv {}/{} | {:?}",
                status,
                self.name,
                self.throughput,
                self.latency_avg_us,
                self.messages_received,
                self.messages_sent,
                self.elapsed
            );
        }
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    CLASP Real-World Benchmarks                       ║");
    println!("║                                                                      ║");
    println!("║  These measure SYSTEM throughput, not just codec speed.              ║");
    println!("║  Includes: routing, state, fanout, wildcards, late-joiner replay     ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    
    // Scenario A: End-to-End
    println!("═══ Scenario A: End-to-End Single Hop ═══");
    benchmark_e2e_single_hop("websocket").await.print();
    println!();
    
    // Scenario B: Fanout Curve
    println!("═══ Scenario B: Fanout Curve ═══");
    for n in [1, 10, 50, 100, 500] {
        benchmark_fanout(n).await.print();
    }
    println!();
    
    // Scenario C: Address Scale
    println!("═══ Scenario C: Address Table Scale ═══");
    for n in [100, 1_000, 10_000] {
        benchmark_address_scale(n).await.print();
    }
    println!();
    
    // Scenario D: Wildcard Cost
    println!("═══ Scenario D: Wildcard Routing Cost ═══");
    for pattern in ["exact", "single", "globstar", "complex"] {
        benchmark_wildcard_cost(pattern).await.print();
    }
    println!();
    
    // Scenario E: Signal Type Comparison
    println!("═══ Scenario E: Signal Type Comparison ═══");
    benchmark_state_overhead("param").await.print();
    benchmark_state_overhead("event").await.print();
    benchmark_state_overhead("stream").await.print();
    println!();
    
    // Scenario F: Late Joiner
    println!("═══ Scenario F: Late Joiner Replay ═══");
    for n in [10, 100, 500, 1_000] {
        benchmark_late_joiner(n).await.print();
    }
    println!();
    
    println!("═══════════════════════════════════════════════════════════════════════");
    println!("Note: These numbers reflect real system behavior including:");
    println!("  - Network I/O (localhost WebSocket)");
    println!("  - Router message processing");
    println!("  - State management");
    println!("  - Subscription matching");
    println!("  - Message fanout");
    println!();
    println!("Codec micro-benchmarks (8M/11M msg/s) are the THEORETICAL CEILING.");
    println!("System throughput is typically 10-100x lower depending on features.");
    println!("═══════════════════════════════════════════════════════════════════════");
}
