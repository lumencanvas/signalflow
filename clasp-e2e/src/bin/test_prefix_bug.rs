use clasp_core::{SubscribeOptions};
use clasp_router::subscription::{Subscription, SubscriptionManager};

fn main() {
    println!("=== Testing prefix indexing bug ===\n");
    
    // Test case: Subscribe to /** pattern
    let manager = SubscriptionManager::new();
    
    let pattern = "/**";
    let sub = Subscription::new(
        1,
        "session1".to_string(),
        pattern,
        vec![],
        SubscribeOptions::default(),
    ).unwrap();
    
    println!("Adding subscription with pattern: {}", pattern);
    println!("Pattern segments: {:?}", sub.pattern.address().segments());
    println!("First segment: {:?}", sub.pattern.address().segments().first());
    
    manager.add(sub);
    
    // Now try to find subscribers for /a
    let test_address = "/a";
    println!("\nLooking up address: {}", test_address);
    
    // Manually extract what the code does
    let address_prefix = test_address
        .split('/')
        .nth(1)
        .map(|s| format!("/{}", s))
        .unwrap_or_else(|| "/".to_string());
    println!("Address prefix extracted: {}", address_prefix);
    
    let subscribers = manager.find_subscribers(test_address, None);
    println!("Subscribers found: {}", subscribers.len());
    if !subscribers.is_empty() {
        for sub in subscribers {
            println!("  - {}", sub);
        }
    } else {
        println!("  (none) <- BUG!");
    }
    
    println!("\n=== Testing with /root/** pattern ===\n");
    
    let manager2 = SubscriptionManager::new();
    let pattern2 = "/root/**";
    let sub2 = Subscription::new(
        1,
        "session2".to_string(),
        pattern2,
        vec![],
        SubscribeOptions::default(),
    ).unwrap();
    
    println!("Adding subscription with pattern: {}", pattern2);
    println!("Pattern segments: {:?}", sub2.pattern.address().segments());
    
    manager2.add(sub2);
    
    let test_address2 = "/root/a";
    println!("\nLooking up address: {}", test_address2);
    
    let address_prefix2 = test_address2
        .split('/')
        .nth(1)
        .map(|s| format!("/{}", s))
        .unwrap_or_else(|| "/".to_string());
    println!("Address prefix extracted: {}", address_prefix2);
    
    let subscribers2 = manager2.find_subscribers(test_address2, None);
    println!("Subscribers found: {}", subscribers2.len());
}
