//! Sustained Load Benchmarks
//!
//! Measures throughput and latency degradation over extended periods.
//! Default duration: 5 minutes. Set CLASP_SUSTAIN_DURATION_SECS to customize.
//!
//! Metrics reported:
//! - Messages per second over time
//! - Latency percentiles per interval
//! - Throughput degradation ratio

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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

struct IntervalStats {
    interval_num: usize,
    messages_sent: u64,
    messages_received: u64,
    latencies: Vec<u64>,
    throughput_per_sec: f64,
}

impl IntervalStats {
    fn print(&self) {
        let mut lat = self.latencies.clone();
        lat.sort_unstable();

        let p50 = percentile(&lat, 50.0);
        let p95 = percentile(&lat, 95.0);
        let p99 = percentile(&lat, 99.0);
        let p999 = percentile(&lat, 99.9);

        let loss_rate = if self.messages_sent > 0 {
            100.0 * (1.0 - self.messages_received as f64 / self.messages_sent as f64)
        } else {
            0.0
        };

        println!(
            "  Interval {:3} │ {:>7.0} msg/s │ p50: {:>5}µs │ p95: {:>5}µs │ p99: {:>5}µs │ p99.9: {:>6}µs │ loss: {:>5.2}%",
            self.interval_num,
            self.throughput_per_sec,
            p50,
            p95,
            p99,
            p999,
            loss_rate
        );
    }
}

async fn run_sustained_load_test(
    port: u16,
    duration_secs: u64,
    client_count: usize,
    messages_per_second_target: usize,
) -> Vec<IntervalStats> {
    let url = format!("ws://127.0.0.1:{}", port);
    let interval_duration = Duration::from_secs(10); // Report every 10 seconds

    // Set up publisher
    let publisher = Clasp::connect_to(&url)
        .await
        .expect("Publisher connect failed");

    // Set up subscribers
    let received_count = Arc::new(AtomicU64::new(0));
    let mut subscribers = Vec::with_capacity(client_count);

    for _ in 0..client_count {
        let sub = Clasp::connect_to(&url)
            .await
            .expect("Subscriber connect failed");
        let counter = received_count.clone();
        sub.subscribe("/sustain/**", move |_, _| {
            counter.fetch_add(1, Ordering::Relaxed);
        })
        .await
        .expect("Subscribe failed");
        subscribers.push(sub);
    }

    // Wait for subscriptions to be active
    tokio::time::sleep(Duration::from_millis(200)).await;

    let test_start = Instant::now();
    let mut interval_stats = Vec::new();
    let mut interval_num = 0;
    let mut total_sent: u64 = 0;
    let delay_between_messages =
        Duration::from_micros(1_000_000 / messages_per_second_target as u64);

    println!("\n  Starting sustained load test:");
    println!("  - Duration: {} seconds", duration_secs);
    println!("  - Target: {} msg/sec", messages_per_second_target);
    println!("  - Subscribers: {}", client_count);
    println!();

    while test_start.elapsed().as_secs() < duration_secs {
        let interval_start = Instant::now();
        let start_received = received_count.load(Ordering::Relaxed);
        let mut interval_sent: u64 = 0;
        let mut latencies: Vec<u64> = Vec::with_capacity(messages_per_second_target * 10);

        // (tx, rx) for measuring roundtrip latency
        let (tx, mut rx) = mpsc::channel::<Instant>(1000);

        // Spawn receiver task to measure latency
        let counter = received_count.clone();
        let receiver_handle = tokio::spawn(async move {
            let mut lat = Vec::new();
            while let Ok(sent_at) = rx.try_recv() {
                lat.push(sent_at.elapsed().as_micros() as u64);
            }
            lat
        });

        // Send messages for this interval
        while interval_start.elapsed() < interval_duration {
            let msg_start = Instant::now();
            let address = format!("/sustain/test/{}", total_sent % 100);

            if publisher.set(&address, total_sent as f64).await.is_ok() {
                interval_sent += 1;
                total_sent += 1;
                let _ = tx.send(msg_start).await;
            }

            // Pace the messages
            tokio::time::sleep(delay_between_messages).await;
        }

        drop(tx);

        // Wait a bit for messages to be received
        tokio::time::sleep(Duration::from_millis(100)).await;

        let end_received = received_count.load(Ordering::Relaxed);
        let interval_received = (end_received - start_received) / client_count as u64;

        // Collect latencies from receiver task
        let measured_latencies = receiver_handle.await.unwrap_or_default();

        let elapsed_secs = interval_start.elapsed().as_secs_f64();
        let throughput = interval_sent as f64 / elapsed_secs;

        let stats = IntervalStats {
            interval_num,
            messages_sent: interval_sent,
            messages_received: interval_received,
            latencies: measured_latencies,
            throughput_per_sec: throughput,
        };
        stats.print();
        interval_stats.push(stats);

        interval_num += 1;
    }

    interval_stats
}

fn analyze_degradation(stats: &[IntervalStats]) {
    if stats.len() < 2 {
        println!("\n  Insufficient data for degradation analysis");
        return;
    }

    // Compare first interval to last interval
    let first = &stats[0];
    let last = &stats[stats.len() - 1];

    let throughput_ratio = last.throughput_per_sec / first.throughput_per_sec;

    let mut first_lat = first.latencies.clone();
    let mut last_lat = last.latencies.clone();
    first_lat.sort_unstable();
    last_lat.sort_unstable();

    let first_p99 = percentile(&first_lat, 99.0);
    let last_p99 = percentile(&last_lat, 99.0);

    let latency_ratio = if first_p99 > 0 {
        last_p99 as f64 / first_p99 as f64
    } else {
        1.0
    };

    println!("\n  ═══ DEGRADATION ANALYSIS ═══");
    println!();
    println!(
        "  Throughput: {:.1}% of initial ({:.0} -> {:.0} msg/s)",
        throughput_ratio * 100.0,
        first.throughput_per_sec,
        last.throughput_per_sec
    );
    println!(
        "  P99 Latency: {:.1}x initial ({}µs -> {}µs)",
        latency_ratio, first_p99, last_p99
    );
    println!();

    if throughput_ratio < 0.9 {
        println!("  ⚠️  Throughput degraded by more than 10%");
    } else {
        println!("  ✓  Throughput stable");
    }

    if latency_ratio > 2.0 {
        println!("  ⚠️  P99 latency increased by more than 2x");
    } else {
        println!("  ✓  Latency stable");
    }
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              CLASP SUSTAINED LOAD BENCHMARKS                                          ║");
    println!("║                    (Measures throughput degradation over time)                                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    let duration_secs: u64 = std::env::var("CLASP_SUSTAIN_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300); // Default 5 minutes

    let port = find_port().await;
    let router = Router::new(RouterConfig {
        name: "Sustained Load Benchmark".into(),
        max_sessions: 1000,
        session_timeout: 60,
        features: vec!["param".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 1000,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 0,
            max_messages_per_second: 0,
            rate_limiting_enabled: false,
    });

    let addr = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("═══ Moderate Load (1000 msg/sec, 10 subscribers) ═══");
    let stats = run_sustained_load_test(port, duration_secs.min(60), 10, 1000).await;
    analyze_degradation(&stats);

    println!("\n═══ High Load (5000 msg/sec, 10 subscribers) ═══");
    let stats = run_sustained_load_test(port, duration_secs.min(60), 10, 5000).await;
    analyze_degradation(&stats);

    println!("\n═══ Many Subscribers (1000 msg/sec, 100 subscribers) ═══");
    let stats = run_sustained_load_test(port, duration_secs.min(60), 100, 1000).await;
    analyze_degradation(&stats);

    println!("\n═══════════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("  SUSTAINED LOAD TARGETS:");
    println!("  ────────────────────────────────────────────────────────────────────────────────────────────────────");
    println!("  │ Metric                  │ Target       │ Notes                                                  │");
    println!("  ├─────────────────────────┼──────────────┼────────────────────────────────────────────────────────┤");
    println!("  │ Throughput degradation  │ <10%         │ After 1 hour of sustained load                         │");
    println!("  │ P99 latency increase    │ <2x          │ Compared to initial interval                           │");
    println!("  │ Message loss rate       │ <0.1%        │ At target throughput                                   │");
    println!("  ═══════════════════════════════════════════════════════════════════════════════════════════════════\n");
}
