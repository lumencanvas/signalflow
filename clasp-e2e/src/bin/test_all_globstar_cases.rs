use clasp_core::{SubscribeOptions, address::Pattern};
use clasp_router::subscription::{Subscription, SubscriptionManager};

fn main() {
    println!("=== Testing all globstar pattern cases ===\n");
    
    let test_cases = vec![
        ("/**", "/a"),
        ("/**", "/test/nested"),
        ("/prefix/**", "/prefix/a"),
        ("/prefix/**", "/prefix/a/b/c"),
        ("/a/*/c/**", "/a/b/c/d/e"),
    ];
    
    for (pattern, address) in test_cases {
        let manager = SubscriptionManager::new();
        let sub = Subscription::new(
            1,
            "session1".to_string(),
            pattern,
            vec![],
            SubscribeOptions::default(),
        ).unwrap();
        
        let direct_match = sub.pattern.matches(address);
        
        manager.add(sub);
        
        let subscribers = manager.find_subscribers(address, None);
        
        let status = if direct_match && subscribers.len() == 1 {
            "PASS"
        } else if !direct_match && subscribers.is_empty() {
            "PASS"
        } else {
            "FAIL"
        };
        
        println!("Pattern: {:15} | Address: {:20} | Pattern.matches(): {} | find_subscribers(): {} | {}", 
                 pattern, address, direct_match, subscribers.len(), status);
    }
}
