//! Debug late-joiner snapshot delivery with detailed tracing

use clasp_client::Clasp;
use clasp_core::{codec, Message, ParamValue, SecurityMode, SnapshotMessage, Value};
use clasp_router::{Router, RouterConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn find_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap()
        .local_addr().unwrap().port()
}

async fn test_late_joiner(param_count: usize) -> (u64, Duration, String) {
    let port = find_port().await;
    let url = format!("ws://127.0.0.1:{}", port);
    
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
    
    let addr = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let _ = router.serve_websocket(&addr).await;
    });
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Pre-populate state
    let setter = Clasp::connect_to(&url).await.unwrap();
    for i in 0..param_count {
        let _ = setter.set(&format!("/state/{}", i), i as f64).await;
    }
    
    // Wait for state to settle
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Late joiner connects and subscribes
    let received = Arc::new(AtomicU64::new(0));
    let counter = received.clone();
    
    let start = Instant::now();
    let late_joiner = match Clasp::connect_to(&url).await {
        Ok(c) => c,
        Err(e) => return (0, start.elapsed(), format!("Connect error: {}", e)),
    };
    
    match late_joiner.subscribe("/state/**", move |_, _| {
        counter.fetch_add(1, Ordering::Relaxed);
    }).await {
        Ok(_) => {},
        Err(e) => return (0, start.elapsed(), format!("Subscribe error: {}", e)),
    };
    
    // Wait for snapshot with timeout
    let deadline = Instant::now() + Duration::from_secs(5);
    while received.load(Ordering::Relaxed) < param_count as u64 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let count = received.load(Ordering::Relaxed);
    
    (count, elapsed, String::new())
}

#[tokio::main]
async fn main() {
    println!("=== Late Joiner Snapshot Chunking Test ===\n");
    
    // Test sizes that will require chunking (> 800 params)
    for count in [500, 1000, 2000, 5000, 10000] {
        print!("{:>6} params: ", count);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let (received, elapsed, err) = test_late_joiner(count).await;
        
        if !err.is_empty() {
            println!("ERROR: {}", err);
        } else if received >= count as u64 {
            println!("✓ {} received in {:?} ({:.0} params/s)", 
                received, elapsed, received as f64 / elapsed.as_secs_f64());
        } else {
            println!("✗ only {} of {} received in {:?}", received, count, elapsed);
        }
    }
}
