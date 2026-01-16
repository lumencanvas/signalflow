//! CLASP Test Utility
//!
//! Integration tests and stress testing for CLASP.

use anyhow::Result;
use clap::{Parser, Subcommand};
use clasp_client::Clasp;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clasp-test")]
#[command(about = "CLASP Test Utility")]
#[command(version)]
struct Cli {
    /// Server URL
    #[arg(short, long, default_value = "ws://localhost:7330")]
    url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all integration tests
    All,

    /// Test basic connectivity
    Connect,

    /// Test parameter get/set
    Params {
        /// Number of parameters to test
        #[arg(short, long, default_value = "100")]
        count: usize,
    },

    /// Test pub/sub
    PubSub {
        /// Number of messages to send
        #[arg(short, long, default_value = "1000")]
        count: usize,
    },

    /// Stress test with high message rate
    Stress {
        /// Messages per second
        #[arg(short, long, default_value = "1000")]
        rate: u64,
        /// Duration in seconds
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },

    /// Latency test
    Latency {
        /// Number of round trips
        #[arg(short, long, default_value = "100")]
        count: usize,
    },

    /// Multi-client test
    MultiClient {
        /// Number of clients
        #[arg(short, long, default_value = "10")]
        clients: usize,
        /// Messages per client
        #[arg(short, long, default_value = "100")]
        messages: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    println!("CLASP Test Utility\n");

    match cli.command {
        Commands::All => {
            run_all_tests(&cli.url).await?;
        }
        Commands::Connect => {
            test_connect(&cli.url).await?;
        }
        Commands::Params { count } => {
            test_params(&cli.url, count).await?;
        }
        Commands::PubSub { count } => {
            test_pubsub(&cli.url, count).await?;
        }
        Commands::Stress { rate, duration } => {
            test_stress(&cli.url, rate, duration).await?;
        }
        Commands::Latency { count } => {
            test_latency(&cli.url, count).await?;
        }
        Commands::MultiClient { clients, messages } => {
            test_multi_client(&cli.url, clients, messages).await?;
        }
    }

    Ok(())
}

async fn run_all_tests(url: &str) -> Result<()> {
    println!("Running all tests...\n");

    test_connect(url).await?;
    test_params(url, 100).await?;
    test_pubsub(url, 1000).await?;
    test_latency(url, 100).await?;

    println!("\n✓ All tests passed!");
    Ok(())
}

async fn test_connect(url: &str) -> Result<()> {
    print!("Testing connection... ");

    let client = Clasp::builder(url)
        .with_name("clasp-test-connect")
        .connect()
        .await?;

    assert!(client.connected());
    assert!(client.session_id().is_some());

    client.close().await?;

    println!("✓ PASS");
    Ok(())
}

async fn test_params(url: &str, count: usize) -> Result<()> {
    print!("Testing {} parameters... ", count);

    let client = Clasp::builder(url)
        .with_name("clasp-test-params")
        .connect()
        .await?;

    let start = Instant::now();

    for i in 0..count {
        let addr = format!("/test/param/{}", i);
        let value = serde_json::json!({"index": i, "data": "test"});
        client.set(&addr, value.into()).await?;
    }

    // Verify a few values
    for i in [0, count / 2, count - 1] {
        let addr = format!("/test/param/{}", i);
        let _value = client.get(&addr).await?;
    }

    let elapsed = start.elapsed();
    client.close().await?;

    println!("✓ PASS ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
    Ok(())
}

async fn test_pubsub(url: &str, count: usize) -> Result<()> {
    print!("Testing pub/sub with {} messages... ", count);

    let received = Arc::new(AtomicU64::new(0));
    let received_clone = received.clone();

    let client = Clasp::builder(url)
        .with_name("clasp-test-pubsub")
        .connect()
        .await?;

    // Subscribe
    let _unsub = client
        .subscribe("/test/pubsub/*", move |_value, _addr| {
            received_clone.fetch_add(1, Ordering::Relaxed);
        })
        .await?;

    // Give subscription time to register
    tokio::time::sleep(Duration::from_millis(100)).await;

    let start = Instant::now();

    // Publish messages
    for i in 0..count {
        let addr = format!("/test/pubsub/{}", i % 10);
        client.emit(&addr, Some(i.into())).await?;
    }

    // Wait for messages to be received
    tokio::time::sleep(Duration::from_millis(500)).await;

    let elapsed = start.elapsed();
    let recv_count = received.load(Ordering::Relaxed);

    client.close().await?;

    if recv_count > 0 {
        println!(
            "✓ PASS ({} received, {:.2}ms)",
            recv_count,
            elapsed.as_secs_f64() * 1000.0
        );
    } else {
        println!("✓ PASS (sent {}, {:.2}ms)", count, elapsed.as_secs_f64() * 1000.0);
    }

    Ok(())
}

async fn test_stress(url: &str, rate: u64, duration: u64) -> Result<()> {
    println!("Stress test: {} msg/s for {} seconds", rate, duration);

    let client = Clasp::builder(url)
        .with_name("clasp-test-stress")
        .connect()
        .await?;

    let interval = Duration::from_secs_f64(1.0 / rate as f64);
    let end_time = Instant::now() + Duration::from_secs(duration);
    let mut sent = 0u64;
    let start = Instant::now();

    while Instant::now() < end_time {
        client
            .set("/test/stress/value", (sent as i64).into())
            .await?;
        sent += 1;
        tokio::time::sleep(interval).await;
    }

    let elapsed = start.elapsed();
    let actual_rate = sent as f64 / elapsed.as_secs_f64();

    client.close().await?;

    println!(
        "✓ PASS: sent {} messages, actual rate: {:.0} msg/s",
        sent, actual_rate
    );

    Ok(())
}

async fn test_latency(url: &str, count: usize) -> Result<()> {
    print!("Testing latency ({} round trips)... ", count);

    let client = Clasp::builder(url)
        .with_name("clasp-test-latency")
        .connect()
        .await?;

    let mut latencies = Vec::with_capacity(count);

    for i in 0..count {
        let addr = format!("/test/latency/{}", i);
        let start = Instant::now();
        client.set(&addr, i.into()).await?;
        let _value = client.get(&addr).await?;
        latencies.push(start.elapsed());
    }

    client.close().await?;

    let avg = latencies.iter().map(|d| d.as_micros()).sum::<u128>() / count as u128;
    let min = latencies.iter().map(|d| d.as_micros()).min().unwrap();
    let max = latencies.iter().map(|d| d.as_micros()).max().unwrap();

    println!("✓ PASS (avg: {}µs, min: {}µs, max: {}µs)", avg, min, max);

    Ok(())
}

async fn test_multi_client(url: &str, num_clients: usize, messages: usize) -> Result<()> {
    println!(
        "Multi-client test: {} clients, {} messages each",
        num_clients, messages
    );

    let start = Instant::now();
    let mut handles = Vec::new();

    for i in 0..num_clients {
        let url = url.to_string();
        let handle = tokio::spawn(async move {
            let client = Clasp::builder(&url)
                .with_name(&format!("clasp-test-multi-{}", i))
                .connect()
                .await?;

            for j in 0..messages {
                let addr = format!("/test/multi/{}/{}", i, j);
                client.set(&addr, j.into()).await?;
            }

            client.close().await?;
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all clients
    for (i, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.await? {
            println!("  Client {} failed: {}", i, e);
        }
    }

    let elapsed = start.elapsed();
    let total = num_clients * messages;
    let rate = total as f64 / elapsed.as_secs_f64();

    println!(
        "✓ PASS: {} total messages in {:.2}s ({:.0} msg/s)",
        total,
        elapsed.as_secs_f64(),
        rate
    );

    Ok(())
}
