//! Long-Running Soak Tests
//!
//! Tests CLASP stability over extended periods:
//! - Memory leak detection
//! - Connection stability
//! - Performance consistency
//!
//! Run with: cargo run -p clasp-test-suite --bin soak-tests -- [duration_minutes]
//!
//! Default duration: 5 minutes
//! Example: cargo run -p clasp-test-suite --bin soak-tests -- 60  # 1 hour

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Test Infrastructure
// ============================================================================

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct TestRouter {
    port: u16,
    handle: tokio::task::JoinHandle<()>,
}

impl TestRouter {
    async fn start() -> Self {
        let port = find_port().await;
        let addr = format!("127.0.0.1:{}", port);
        let router = Router::new(RouterConfig {
            name: "Soak Test Router".to_string(),
            max_sessions: 1000,
            session_timeout: 300,
            features: vec!["param".to_string(), "event".to_string()],
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
// Statistics Tracking
// ============================================================================

#[derive(Default)]
struct SoakStats {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    errors: AtomicU64,
    connections: AtomicU64,
    reconnections: AtomicU64,
    max_latency_us: AtomicU64,
    total_latency_us: AtomicU64,
    latency_samples: AtomicU64,
}

impl SoakStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_send(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    fn record_receive(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    fn record_connect(&self) {
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    fn record_reconnect(&self) {
        self.reconnections.fetch_add(1, Ordering::Relaxed);
    }

    fn record_latency(&self, latency_us: u64) {
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.latency_samples.fetch_add(1, Ordering::Relaxed);

        let current_max = self.max_latency_us.load(Ordering::Relaxed);
        if latency_us > current_max {
            self.max_latency_us.store(latency_us, Ordering::Relaxed);
        }
    }

    fn print_summary(&self, elapsed: Duration) {
        let sent = self.messages_sent.load(Ordering::Relaxed);
        let received = self.messages_received.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let conns = self.connections.load(Ordering::Relaxed);
        let reconns = self.reconnections.load(Ordering::Relaxed);
        let max_lat = self.max_latency_us.load(Ordering::Relaxed);
        let total_lat = self.total_latency_us.load(Ordering::Relaxed);
        let lat_samples = self.latency_samples.load(Ordering::Relaxed);

        let avg_lat = if lat_samples > 0 {
            total_lat / lat_samples
        } else {
            0
        };
        let rate = sent as f64 / elapsed.as_secs_f64();
        let loss_pct = if sent > 0 {
            ((sent.saturating_sub(received)) as f64 / sent as f64) * 100.0
        } else {
            0.0
        };

        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║                    SOAK TEST SUMMARY                             ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!(
            "║ Duration:          {:>10.1} minutes                           ║",
            elapsed.as_secs_f64() / 60.0
        );
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ Messages:                                                        ║");
        println!(
            "║   Sent:            {:>15}                              ║",
            sent
        );
        println!(
            "║   Received:        {:>15}                              ║",
            received
        );
        println!(
            "║   Rate:            {:>12.1} msg/s                          ║",
            rate
        );
        println!(
            "║   Loss:            {:>14.2}%                              ║",
            loss_pct
        );
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ Connections:                                                     ║");
        println!(
            "║   Established:     {:>15}                              ║",
            conns
        );
        println!(
            "║   Reconnections:   {:>15}                              ║",
            reconns
        );
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ Latency:                                                         ║");
        println!(
            "║   Average:         {:>12} μs                              ║",
            avg_lat
        );
        println!(
            "║   Maximum:         {:>12} μs                              ║",
            max_lat
        );
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!(
            "║ Errors:            {:>15}                              ║",
            errors
        );
        println!("╚══════════════════════════════════════════════════════════════════╝");
    }
}

// ============================================================================
// Memory Monitoring
// ============================================================================

fn get_memory_usage() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        let rss_str = String::from_utf8_lossy(&output.stdout);
        rss_str.trim().parse::<u64>().ok().map(|kb| kb * 1024)
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<u64>().ok().map(|kb| kb * 1024);
                }
            }
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

// ============================================================================
// Main Soak Test
// ============================================================================

#[tokio::main]
async fn main() {
    // Parse duration from command line
    let args: Vec<String> = std::env::args().collect();
    let duration_minutes: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);

    let duration = Duration::from_secs(duration_minutes * 60);

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Long-Running Soak Test                        ║");
    println!("║                                                                  ║");
    println!(
        "║  Duration: {} minutes                                             ║",
        duration_minutes
    );
    println!("║  Press Ctrl+C to stop early                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Reduce log noise
    tracing_subscriber::fmt().with_env_filter("warn").init();

    // Start router
    println!("Starting router...");
    let router = TestRouter::start().await;
    let url = router.url();
    println!("Router running at {}\n", url);

    // Initialize stats
    let stats = Arc::new(SoakStats::new());
    let running = Arc::new(AtomicBool::new(true));
    let start = Instant::now();
    let _initial_memory = get_memory_usage();

    // Set up Ctrl+C handler
    let running_clone = running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n\nStopping soak test...");
        running_clone.store(false, Ordering::Relaxed);
    });

    // Connect sender and receiver
    let sender = match Clasp::builder(&url).name("soak-sender").connect().await {
        Ok(c) => {
            stats.record_connect();
            c
        }
        Err(e) => {
            println!("Failed to connect sender: {}", e);
            router.stop();
            std::process::exit(1);
        }
    };

    let receiver = match Clasp::builder(&url).name("soak-receiver").connect().await {
        Ok(c) => {
            stats.record_connect();
            c
        }
        Err(e) => {
            println!("Failed to connect receiver: {}", e);
            router.stop();
            std::process::exit(1);
        }
    };

    // Set up receiver subscription
    let stats_clone = stats.clone();
    let _ = receiver
        .subscribe("/soak/**", move |value, _| {
            stats_clone.record_receive();
            if let Some(sent_time) = value.as_f64() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64;
                let latency = now.saturating_sub(sent_time as u64);
                stats_clone.record_latency(latency);
            }
        })
        .await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Main test loop
    let mut msg_id = 0u64;
    let mut last_report = Instant::now();
    let mut last_sent = 0u64;
    let mut memory_samples = Vec::new();

    println!("Running soak test...\n");

    while running.load(Ordering::Relaxed) && start.elapsed() < duration {
        // Send messages in batches
        for _ in 0..100 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;

            let address = format!("/soak/msg/{}", msg_id % 100);
            match sender.set(&address, now as f64).await {
                Ok(()) => {
                    stats.record_send();
                    msg_id += 1;
                }
                Err(_) => {
                    stats.record_error();
                }
            }
        }

        // Small delay between batches
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Progress report every 10 seconds
        if last_report.elapsed() >= Duration::from_secs(10) {
            let elapsed = start.elapsed();
            let sent = stats.messages_sent.load(Ordering::Relaxed);
            let received = stats.messages_received.load(Ordering::Relaxed);
            let errors = stats.errors.load(Ordering::Relaxed);
            let rate = (sent - last_sent) as f64 / 10.0;
            last_sent = sent;

            // Memory sample
            if let Some(mem) = get_memory_usage() {
                memory_samples.push(mem);
            }

            let remaining = duration.saturating_sub(elapsed);
            print!(
                "\r[{:>3}m remaining] sent: {} | recv: {} | rate: {:.0}/s | errors: {}    ",
                remaining.as_secs() / 60,
                sent,
                received,
                rate,
                errors
            );
            std::io::Write::flush(&mut std::io::stdout()).ok();

            last_report = Instant::now();
        }
    }

    // Cleanup
    sender.close().await;
    receiver.close().await;

    let elapsed = start.elapsed();
    stats.print_summary(elapsed);

    // Memory analysis
    if memory_samples.len() >= 2 {
        let first = memory_samples[0];
        let last = memory_samples[memory_samples.len() - 1];
        let max = *memory_samples.iter().max().unwrap();
        let growth = last as i64 - first as i64;

        println!("\n┌──────────────────────────────────────────────────────────────────┐");
        println!("│ MEMORY ANALYSIS                                                  │");
        println!("├──────────────────────────────────────────────────────────────────┤");
        println!(
            "│ Initial:          {:>12} KB                                │",
            first / 1024
        );
        println!(
            "│ Final:            {:>12} KB                                │",
            last / 1024
        );
        println!(
            "│ Peak:             {:>12} KB                                │",
            max / 1024
        );
        println!(
            "│ Growth:           {:>+12} KB                                │",
            growth / 1024
        );

        if growth > (first as i64 / 10) {
            println!("│ ⚠ WARNING: Significant memory growth detected                    │");
        } else {
            println!("│ ✓ Memory usage stable                                            │");
        }
        println!("└──────────────────────────────────────────────────────────────────┘");
    }

    router.stop();

    // Determine pass/fail
    let errors = stats.errors.load(Ordering::Relaxed);
    let sent = stats.messages_sent.load(Ordering::Relaxed);
    let received = stats.messages_received.load(Ordering::Relaxed);

    let error_rate = if sent > 0 {
        (errors as f64 / sent as f64) * 100.0
    } else {
        0.0
    };
    let loss_rate = if sent > 0 {
        ((sent.saturating_sub(received)) as f64 / sent as f64) * 100.0
    } else {
        0.0
    };

    println!();
    if error_rate < 1.0 && loss_rate < 5.0 {
        println!("\x1b[32m✓ SOAK TEST PASSED\x1b[0m");
        println!("  Error rate: {:.2}% (threshold: 1%)", error_rate);
        println!("  Message loss: {:.2}% (threshold: 5%)", loss_rate);
    } else {
        println!("\x1b[31m✗ SOAK TEST FAILED\x1b[0m");
        if error_rate >= 1.0 {
            println!("  Error rate: {:.2}% exceeds 1% threshold", error_rate);
        }
        if loss_rate >= 5.0 {
            println!("  Message loss: {:.2}% exceeds 5% threshold", loss_rate);
        }
        std::process::exit(1);
    }
}
