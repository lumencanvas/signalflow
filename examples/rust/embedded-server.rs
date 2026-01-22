//! Example: Embedding CLASP Server in Your Application
//!
//! This demonstrates how to run a CLASP router alongside your own code.
//! Useful for:
//! - Custom servers that need CLASP functionality
//! - Applications that want to expose state via CLASP
//! - IoT hubs that collect sensor data
//!
//! # Run
//!
//! ```bash
//! cargo run --example embedded-server
//! ```
//!
//! Then connect with:
//! ```bash
//! wscat -c ws://localhost:7330 -s clasp
//! ```

use clasp_core::SecurityMode;
use clasp_router::{Router, RouterConfig};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("info,embedded_server=debug")
        .init();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║      CLASP Server Embedded in Your Application          ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // Create router configuration
    let config = RouterConfig {
        name: "Embedded Server".to_string(),
        security_mode: SecurityMode::Open,
        max_sessions: 100,
        session_timeout: 60,
        features: vec!["param".to_string(), "event".to_string()],
        max_subscriptions_per_session: 50,
    };

    // Create the router
    let router = Arc::new(Router::new(config));
    let router_clone = router.clone();

    // Spawn your application logic that publishes to CLASP
    tokio::spawn(async move {
        // Example: Simulate sensor readings
        let mut tick = 0u64;
        loop {
            // Your business logic here
            let cpu_usage = simulate_cpu_reading();
            let memory_usage = simulate_memory_reading();
            let temperature = simulate_temperature();

            // Publish to CLASP - any connected client will receive these
            router_clone.state().set_value(
                "/system/cpu",
                clasp_core::Value::Float(cpu_usage),
                "server",
            );
            router_clone.state().set_value(
                "/system/memory", 
                clasp_core::Value::Float(memory_usage),
                "server",
            );
            router_clone.state().set_value(
                "/sensors/temperature",
                clasp_core::Value::Float(temperature),
                "server",
            );
            router_clone.state().set_value(
                "/system/uptime",
                clasp_core::Value::Int(tick as i64),
                "server",
            );

            tracing::debug!(
                "Published: cpu={:.1}%, mem={:.1}%, temp={:.1}°C, uptime={}s",
                cpu_usage * 100.0,
                memory_usage * 100.0,
                temperature,
                tick
            );

            tick += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Spawn a task that responds to external events
    // (In real app, this might be HTTP webhooks, database changes, etc.)
    tokio::spawn(async {
        // Example: React to some external trigger
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("External event occurred - you could publish this to CLASP");
        }
    });

    println!("\nServer starting on ws://localhost:7330");
    println!("Connect and subscribe to:");
    println!("  /system/cpu       - CPU usage (0.0-1.0)");
    println!("  /system/memory    - Memory usage (0.0-1.0)");
    println!("  /sensors/temperature - Temperature (°C)");
    println!("  /system/uptime    - Uptime (seconds)");
    println!("\nExample: wscat -c ws://localhost:7330 -s clasp");
    println!();

    // Run the WebSocket server (this blocks)
    router.serve_websocket("0.0.0.0:7330").await?;

    Ok(())
}

// Simulated sensor readings
fn simulate_cpu_reading() -> f64 {
    0.2 + (rand::random::<f64>() * 0.6) // 20-80%
}

fn simulate_memory_reading() -> f64 {
    0.4 + (rand::random::<f64>() * 0.3) // 40-70%
}

fn simulate_temperature() -> f64 {
    20.0 + (rand::random::<f64>() * 15.0) // 20-35°C
}

// Needed for rand::random
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    static mut SEED: u64 = 0;
    
    pub fn random<T: RandomValue>() -> T {
        T::random()
    }
    
    pub trait RandomValue {
        fn random() -> Self;
    }
    
    impl RandomValue for f64 {
        fn random() -> Self {
            unsafe {
                SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
                if SEED == 0 {
                    SEED = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                }
                (SEED as f64) / (u64::MAX as f64)
            }
        }
    }
}
