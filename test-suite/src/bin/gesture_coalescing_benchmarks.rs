//! Real-World Benchmarks for Gesture Coalescing
//!
//! These benchmarks measure ACTUAL bandwidth reduction with REAL numbers.
//! No assumptions, no guesses - just measured data.

use clasp_client::Clasp;
use clasp_core::{GesturePhase, Value};
use clasp_router::{Router, RouterConfig};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Find an available port
async fn find_port() -> u16 {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::main]
async fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║      Gesture Coalescing - REAL METRICS (No Assumptions)          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Benchmark 1: 120Hz touch input - measure actual reduction
    benchmark_120hz_touch().await;
    
    // Benchmark 2: 240Hz pen input - measure actual reduction
    benchmark_240hz_pen().await;
    
    // Benchmark 3: Fan-out (1→10) - measure total bandwidth
    benchmark_fanout().await;
    
    // Benchmark 4: Multitouch (10 concurrent) - measure reduction
    benchmark_multitouch().await;
}

/// Benchmark 1: 120Hz touch input
/// REAL SCENARIO: Modern touchscreen sends 120 updates/second
/// MEASURES: Actual message count with vs without coalescing
async fn benchmark_120hz_touch() {
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ Benchmark 1: 120Hz Touch Input (Realistic Touchscreen)           │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    
    let port = find_port().await;
    let addr = format!("127.0.0.1:{}", port);
    
    // Test WITH coalescing
    let router_with = Router::new(RouterConfig {
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        ..Default::default()
    });
    
    let router_handle = {
        let addr = addr.clone();
        tokio::spawn(async move {
            let _ = router_with.serve_websocket(&addr).await;
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", addr);
    let sender = Clasp::connect_to(&url).await.unwrap();
    let receiver = Clasp::connect_to(&url).await.unwrap();
    
    let messages_received = Arc::new(AtomicU64::new(0));
    let count = messages_received.clone();
    
    let _ = receiver.subscribe("/touch/**", move |_, _| {
        count.fetch_add(1, Ordering::SeqCst);
    }).await;
    
    sleep(Duration::from_millis(50)).await;
    
    // Send 120 moves over 1 second (120Hz = 8.33ms intervals)
    let start = Instant::now();
    sender.gesture("/touch/1", 1, GesturePhase::Start, Value::Float(0.0)).await.unwrap();
    
    for i in 0..120 {
        if i > 0 {
            sleep(Duration::from_nanos(8_333_333)).await; // 120Hz
        }
        let mut payload = HashMap::new();
        payload.insert("x".to_string(), Value::Float(i as f64 / 120.0));
        payload.insert("y".to_string(), Value::Float(0.5));
        sender.gesture("/touch/1", 1, GesturePhase::Move, Value::Map(payload)).await.unwrap();
    }
    
    sender.gesture("/touch/1", 1, GesturePhase::End, Value::Float(1.0)).await.unwrap();
    
    // Wait for all messages including flushed moves
    sleep(Duration::from_millis(500)).await;
    
    let elapsed = start.elapsed();
    let received_with = messages_received.load(Ordering::SeqCst);
    
    router_handle.abort();
    
    // Test WITHOUT coalescing
    let port2 = find_port().await;
    let addr2 = format!("127.0.0.1:{}", port2);
    
    let router_without = Router::new(RouterConfig {
        gesture_coalescing: false,
        ..Default::default()
    });
    
    let router_handle2 = {
        let addr = addr2.clone();
        tokio::spawn(async move {
            let _ = router_without.serve_websocket(&addr).await;
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    let url2 = format!("ws://{}", addr2);
    let sender2 = Clasp::connect_to(&url2).await.unwrap();
    let receiver2 = Clasp::connect_to(&url2).await.unwrap();
    
    let messages_received2 = Arc::new(AtomicU64::new(0));
    let count2 = messages_received2.clone();
    
    let _ = receiver2.subscribe("/touch/**", move |_, _| {
        count2.fetch_add(1, Ordering::SeqCst);
    }).await;
    
    sleep(Duration::from_millis(50)).await;
    
    let start2 = Instant::now();
    sender2.gesture("/touch/1", 1, GesturePhase::Start, Value::Float(0.0)).await.unwrap();
    
    for i in 0..120 {
        if i > 0 {
            sleep(Duration::from_nanos(8_333_333)).await;
        }
        let mut payload = HashMap::new();
        payload.insert("x".to_string(), Value::Float(i as f64 / 120.0));
        payload.insert("y".to_string(), Value::Float(0.5));
        sender2.gesture("/touch/1", 1, GesturePhase::Move, Value::Map(payload)).await.unwrap();
    }
    
    sender2.gesture("/touch/1", 1, GesturePhase::End, Value::Float(1.0)).await.unwrap();
    
    sleep(Duration::from_millis(500)).await;
    
    let elapsed2 = start2.elapsed();
    let received_without = messages_received2.load(Ordering::SeqCst);
    
    router_handle2.abort();
    
    // Calculate REAL metrics
    let messages_sent = 122; // Start + 120 moves + End
    let reduction = ((received_without as f64 - received_with as f64) / received_without as f64) * 100.0;
    let messages_saved = received_without - received_with;
    
    println!("  Messages sent:        {}", messages_sent);
    println!("  Received (WITH):      {} messages", received_with);
    println!("  Received (WITHOUT):    {} messages", received_without);
    println!("  Messages saved:       {}", messages_saved);
    println!("  Bandwidth reduction:  {:.1}%", reduction);
    println!("  Time (with):          {:?}", elapsed);
    println!("  Time (without):       {:?}", elapsed2);
    
    // Real validation - no assumptions, just report the numbers
    if received_with < received_without {
        println!("  ✅ Coalescing reduces messages by {} ({:.1}%)", messages_saved, reduction);
    } else {
        println!("  ⚠️  Unexpected: coalescing did not reduce messages");
    }
    println!();
}

/// Benchmark 2: 240Hz pen input
/// REAL SCENARIO: High-end pen tablets (Wacom, iPad Pro) send 240 updates/second
async fn benchmark_240hz_pen() {
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ Benchmark 2: 240Hz Pen Input (High-End Tablet)                  │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    
    let port = find_port().await;
    let addr = format!("127.0.0.1:{}", port);
    
    let router = Router::new(RouterConfig {
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        ..Default::default()
    });
    
    let router_handle = {
        let addr = addr.clone();
        tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", addr);
    let sender = Clasp::connect_to(&url).await.unwrap();
    let receiver = Clasp::connect_to(&url).await.unwrap();
    
    let messages_received = Arc::new(AtomicU64::new(0));
    let count = messages_received.clone();
    
    let _ = receiver.subscribe("/pen/**", move |_, _| {
        count.fetch_add(1, Ordering::SeqCst);
    }).await;
    
    sleep(Duration::from_millis(50)).await;
    
    // 240Hz for 1 second = 240 moves
    let start = Instant::now();
    sender.gesture("/pen/1", 1, GesturePhase::Start, Value::Float(0.0)).await.unwrap();
    
    for i in 0..240 {
        if i > 0 {
            sleep(Duration::from_nanos(4_166_666)).await; // 240Hz = 4.17ms
        }
        sender.gesture("/pen/1", 1, GesturePhase::Move, Value::Float(i as f64 / 240.0)).await.unwrap();
    }
    
    sender.gesture("/pen/1", 1, GesturePhase::End, Value::Float(1.0)).await.unwrap();
    
    sleep(Duration::from_millis(500)).await;
    
    let elapsed = start.elapsed();
    let received = messages_received.load(Ordering::SeqCst);
    
    router_handle.abort();
    
    let messages_sent = 242; // Start + 240 moves + End
    
    println!("  Messages sent:        {}", messages_sent);
    println!("  Messages received:    {}", received);
    println!("  Reduction:            {} messages ({:.1}%)", 
             messages_sent - received,
             ((messages_sent as f64 - received as f64) / messages_sent as f64) * 100.0);
    println!("  Time elapsed:         {:?}", elapsed);
    
    if received < messages_sent {
        println!("  ✅ Coalescing working - {} messages reduced", messages_sent - received);
    }
    println!();
}

/// Benchmark 3: Fan-out scenario
/// REAL SCENARIO: One touch input feeds multiple displays (studio setup)
async fn benchmark_fanout() {
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ Benchmark 3: Fan-Out (1 sender → 10 subscribers)                  │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    
    let port = find_port().await;
    let addr = format!("127.0.0.1:{}", port);
    
    let router = Router::new(RouterConfig {
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        ..Default::default()
    });
    
    let router_handle = {
        let addr = addr.clone();
        tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", addr);
    
    // Create 10 subscribers
    let mut subscribers = Vec::new();
    let total_messages = Arc::new(AtomicU64::new(0));
    
    for _ in 0..10 {
        let receiver = Clasp::connect_to(&url).await.unwrap();
        let count = total_messages.clone();
        let _ = receiver.subscribe("/fanout/**", move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
        }).await;
        subscribers.push(receiver);
    }
    
    sleep(Duration::from_millis(100)).await;
    
    // Single sender
    let sender = Clasp::connect_to(&url).await.unwrap();
    
    // Send 120Hz input for 1 second
    let start = Instant::now();
    sender.gesture("/fanout/touch", 1, GesturePhase::Start, Value::Float(0.0)).await.unwrap();
    
    for i in 0..120 {
        if i > 0 {
            sleep(Duration::from_nanos(8_333_333)).await;
        }
        sender.gesture("/fanout/touch", 1, GesturePhase::Move, Value::Float(i as f64 / 120.0)).await.unwrap();
    }
    
    sender.gesture("/fanout/touch", 1, GesturePhase::End, Value::Float(1.0)).await.unwrap();
    
    sleep(Duration::from_millis(500)).await;
    
    let elapsed = start.elapsed();
    let total_received = total_messages.load(Ordering::SeqCst);
    
    router_handle.abort();
    
    // Without coalescing: 122 messages * 10 subscribers = 1220
    // With coalescing: ~20 messages * 10 subscribers = ~200
    let expected_without = 122 * 10;
    let reduction = ((expected_without as f64 - total_received as f64) / expected_without as f64) * 100.0;
    
    println!("  Messages sent:        {} (Start + 120 moves + End)", 122);
    println!("  Subscribers:          10");
    println!("  Total received:       {} messages (all subscribers)", total_received);
    println!("  Expected (no coalesce): {} messages", expected_without);
    println!("  Reduction:            {} messages ({:.1}%)", 
             expected_without - total_received,
             reduction);
    println!("  Time elapsed:         {:?}", elapsed);
    
    if total_received < expected_without {
        println!("  ✅ Fan-out benefits from coalescing");
    }
    println!();
}

/// Benchmark 4: Multitouch (10 concurrent gestures)
/// REAL SCENARIO: Multi-touch screen with 10 simultaneous touches
async fn benchmark_multitouch() {
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ Benchmark 4: Multitouch (10 Concurrent Gestures)                │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    
    let port = find_port().await;
    let addr = format!("127.0.0.1:{}", port);
    
    let router = Router::new(RouterConfig {
        gesture_coalescing: true,
        gesture_coalesce_interval_ms: 16,
        ..Default::default()
    });
    
    let router_handle = {
        let addr = addr.clone();
        tokio::spawn(async move {
            let _ = router.serve_websocket(&addr).await;
        })
    };
    
    sleep(Duration::from_millis(100)).await;
    
    let url = format!("ws://{}", addr);
    let sender = Clasp::connect_to(&url).await.unwrap();
    let receiver = Clasp::connect_to(&url).await.unwrap();
    
    let messages_received = Arc::new(AtomicU64::new(0));
    let count = messages_received.clone();
    
    let _ = receiver.subscribe("/multitouch/**", move |_, _| {
        count.fetch_add(1, Ordering::SeqCst);
    }).await;
    
    sleep(Duration::from_millis(50)).await;
    
    // Start 10 concurrent gestures
    let start = Instant::now();
    for id in 0..10 {
        sender.gesture("/multitouch/touch", id, GesturePhase::Start, Value::Float(0.0)).await.unwrap();
    }
    
    // Send 60 moves for each gesture (1 second at 60Hz)
    for move_idx in 0..60 {
        for id in 0..10 {
            sender.gesture(
                "/multitouch/touch",
                id,
                GesturePhase::Move,
                Value::Float(move_idx as f64 / 60.0)
            ).await.unwrap();
        }
        sleep(Duration::from_millis(16)).await; // 60Hz
    }
    
    // End all gestures
    for id in 0..10 {
        sender.gesture("/multitouch/touch", id, GesturePhase::End, Value::Float(1.0)).await.unwrap();
    }
    
    sleep(Duration::from_millis(500)).await;
    
    let elapsed = start.elapsed();
    let received = messages_received.load(Ordering::SeqCst);
    
    router_handle.abort();
    
    // Without coalescing: 10 starts + (60 moves * 10 gestures) + 10 ends = 620
    let expected_without = 620;
    let reduction = ((expected_without as f64 - received as f64) / expected_without as f64) * 100.0;
    
    println!("  Concurrent gestures:  10");
    println!("  Moves per gesture:    60");
    println!("  Messages sent:         {} (10 starts + 600 moves + 10 ends)", expected_without);
    println!("  Messages received:    {}", received);
    println!("  Reduction:            {} messages ({:.1}%)", 
             expected_without - received,
             reduction);
    println!("  Time elapsed:         {:?}", elapsed);
    
    if received < expected_without {
        println!("  ✅ Multitouch coalescing working");
    }
    println!();
}
