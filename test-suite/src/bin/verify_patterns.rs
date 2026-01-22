//! Verify pattern matching expectations for benchmarks

use clasp_core::address::Pattern;

fn main() {
    println!("=== Pattern Matching Verification ===\n");
    
    // Generate all 1000 addresses from benchmark
    let mut addrs = Vec::with_capacity(1000);
    for i in 0..1000 {
        let zone = i % 100;
        let fixture = (i / 100) % 10;
        addrs.push(format!("/lights/zone{}/fixture{}/brightness", zone, fixture));
    }
    
    // Test each pattern
    let patterns = [
        ("/lights/zone50/fixture5/brightness", "exact"),
        ("/lights/zone50/*/brightness", "single"),
        ("/lights/**", "globstar"),
        ("/lights/zone5*/fixture*/brightness", "complex"),
    ];
    
    for (pattern_str, name) in patterns {
        let pattern = Pattern::compile(pattern_str).unwrap();
        let matches: Vec<_> = addrs.iter()
            .filter(|a| pattern.matches(a))
            .collect();
        
        println!("{} pattern '{}': {} matches", name, pattern_str, matches.len());
        if matches.len() <= 10 {
            for m in &matches {
                println!("  - {}", m);
            }
        } else {
            println!("  First 5: {:?}", &matches[..5]);
            println!("  Last 5: {:?}", &matches[matches.len()-5..]);
        }
        println!();
    }
    
    // Also test with glob_match crate (what client uses)
    println!("=== Using glob_match crate (client-side matching) ===\n");
    for (pattern_str, name) in patterns {
        let matches: Vec<_> = addrs.iter()
            .filter(|a| clasp_core::address::glob_match(pattern_str, a))
            .collect();
        
        println!("{} pattern '{}': {} matches", name, pattern_str, matches.len());
        println!();
    }
}
