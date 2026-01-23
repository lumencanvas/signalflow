//! Debug the benchmark issue with more verbosity

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = find_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let url = format!("ws://{}", addr);
    
    // Start router
    let router = Router::new(RouterConfig {
        name: "Bench Router".into(),
        max_sessions: 2000,
        session_timeout: 60,
        features: vec!["param".into(), "event".into(), "stream".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 1000,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
    });
    
    let addr_clone = addr.clone();
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr_clone).await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Router started on {}", addr);
    
    // Test EXACT pattern like the benchmark
    println!("\n=== Testing EXACT pattern benchmark scenario ===");
    
    let exact_received = Arc::new(AtomicU64::new(0));
    let counter = exact_received.clone();
    
    let subscriber = Clasp::connect_to(&url).await?;
    println!("Subscriber connected");
    
    // Exact pattern from benchmark
    subscriber.subscribe("/lights/zone50/fixture5/brightness", move |val, addr| {
        println!("  RECEIVED: {} = {:?}", addr, val);
        counter.fetch_add(1, Ordering::Relaxed);
    }).await?;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Subscription registered");
    
    let publisher = Clasp::connect_to(&url).await?;
    println!("Publisher connected");
    
    // Send like the benchmark does
    let msg_count = 100u64; // Smaller count for testing
    let start = Instant::now();
    
    for i in 0..msg_count {
        let zone = i % 100;
        let fixture = (i / 100) % 10;
        let addr = format!("/lights/zone{}/fixture{}/brightness", zone, fixture);
        if zone == 50 && fixture == 5 {
            println!("  SENDING TARGET: {} (i={})", addr, i);
        }
        let _ = publisher.set(&addr, i as f64).await;
    }
    
    let send_time = start.elapsed();
    println!("Sent {} messages in {:?}", msg_count, send_time);
    
    // Wait for delivery
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let received = exact_received.load(Ordering::Relaxed);
    println!("Exact received: {} (expected: 1 for 100 msgs)", received);
    
    // Now test with 1000 messages
    println!("\n=== Testing with 1000 messages ===");
    exact_received.store(0, Ordering::SeqCst);
    
    for i in 0..1000u64 {
        let zone = i % 100;
        let fixture = (i / 100) % 10;
        let addr = format!("/lights/zone{}/fixture{}/brightness", zone, fixture);
        if zone == 50 && fixture == 5 {
            println!("  SENDING TARGET: {} (i={})", addr, i);
        }
        let _ = publisher.set(&addr, i as f64).await;
    }
    
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    let received2 = exact_received.load(Ordering::Relaxed);
    println!("Exact received after 1000 msgs: {} (expected: 1)", received2);
    
    // Verify pattern match analysis
    println!("\n=== Pattern match analysis ===");
    println!("For zone 50, fixture 5:");
    for i in 0u64..1000 {
        let zone = i % 100;
        let fixture = (i / 100) % 10;
        if zone == 50 && fixture == 5 {
            println!("  Match at i={}: zone{}, fixture{}", i, zone, fixture);
        }
    }
    
    Ok(())
}
