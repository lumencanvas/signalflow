use clasp_core::{SignalType, SubscribeOptions};
use clasp_router::subscription::{Subscription, SubscriptionManager};

fn main() {
    println!("=== Testing SubscriptionManager with ** patterns ===\n");
    
    let manager = SubscriptionManager::new();
    
    // Test case 1: Subscribe to /stress-test/patterns/**
    let pattern = "/stress-test/patterns/**";
    let sub = Subscription::new(
        1,
        "session1".to_string(),
        pattern,
        vec![],
        SubscribeOptions::default(),
    ).unwrap();
    
    println!("Subscription pattern: {}", pattern);
    println!("Subscription created with id={}, session={}\n", sub.id, sub.session_id);
    
    manager.add(sub);
    
    // Test address matching
    let test_addresses = vec![
        "/stress-test/patterns/a",
        "/stress-test/patterns/a/b/c",
        "/stress-test/other/value",
        "/different/patterns/a",
    ];
    
    println!("Testing subscriber lookup for various addresses:");
    for addr in test_addresses {
        let subscribers = manager.find_subscribers(addr, None);
        println!("  Address '{}': {} subscribers found", addr, subscribers.len());
        if !subscribers.is_empty() {
            for sub in subscribers {
                println!("    - {}", sub);
            }
        }
    }
    
    println!("\n=== Testing subscription with exact pattern ===\n");
    
    let manager2 = SubscriptionManager::new();
    let pattern2 = "/test/**";
    let sub2 = Subscription::new(
        1,
        "session2".to_string(),
        pattern2,
        vec![],
        SubscribeOptions::default(),
    ).unwrap();
    
    println!("Subscription pattern: {}", pattern2);
    manager2.add(sub2);
    
    let test_addresses2 = vec![
        "/test/a",
        "/test/a/b",
        "/other/test/a",
    ];
    
    println!("Testing subscriber lookup:");
    for addr in test_addresses2 {
        let subscribers = manager2.find_subscribers(addr, None);
        println!("  Address '{}': {} subscribers found", addr, subscribers.len());
    }
}
