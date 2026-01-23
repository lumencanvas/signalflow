//! Debug subscription matching issue

use clasp_client::Clasp;
use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
        name: "Test Router".into(),
        max_sessions: 10,
        session_timeout: 60,
        features: vec!["param".into()],
        security_mode: SecurityMode::Open,
        max_subscriptions_per_session: 100,
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
    });
    
    let addr_clone = addr.clone();
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr_clone).await;
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Router started on {}", addr);
    
    // Create subscriber
    let exact_count = Arc::new(AtomicU64::new(0));
    let single_count = Arc::new(AtomicU64::new(0));
    let glob_count = Arc::new(AtomicU64::new(0));
    
    let subscriber = Clasp::connect_to(&url).await?;
    println!("Subscriber connected");
    
    // Subscribe with exact pattern
    let c1 = exact_count.clone();
    subscriber.subscribe("/lights/zone50/fixture5/brightness", move |_, addr| {
        println!("EXACT match: {}", addr);
        c1.fetch_add(1, Ordering::Relaxed);
    }).await?;
    
    // Subscribe with single wildcard
    let c2 = single_count.clone();
    subscriber.subscribe("/lights/zone50/*/brightness", move |_, addr| {
        println!("SINGLE match: {}", addr);
        c2.fetch_add(1, Ordering::Relaxed);
    }).await?;
    
    // Subscribe with globstar
    let c3 = glob_count.clone();
    subscriber.subscribe("/lights/**", move |_, addr| {
        println!("GLOB match: {}", addr);
        c3.fetch_add(1, Ordering::Relaxed);
    }).await?;
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Subscriptions registered");
    
    // Create publisher
    let publisher = Clasp::connect_to(&url).await?;
    println!("Publisher connected");
    
    // Send test messages
    println!("\n--- Sending /lights/zone50/fixture5/brightness ---");
    publisher.set("/lights/zone50/fixture5/brightness", 1.0).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("\n--- Sending /lights/zone50/fixture0/brightness ---");
    publisher.set("/lights/zone50/fixture0/brightness", 2.0).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("\n--- Sending /lights/zone0/fixture0/brightness ---");
    publisher.set("/lights/zone0/fixture0/brightness", 3.0).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    println!("\n=== RESULTS ===");
    println!("Exact count:  {} (expected: 1)", exact_count.load(Ordering::Relaxed));
    println!("Single count: {} (expected: 2)", single_count.load(Ordering::Relaxed));
    println!("Glob count:   {} (expected: 3)", glob_count.load(Ordering::Relaxed));
    
    Ok(())
}
