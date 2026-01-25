use clasp_core::address::{Pattern, glob_match};

fn main() {
    // Test the exact failing case from the stress test
    let pattern_str = "/stress-test/1234567890/patterns/**";
    let address_single = "/stress-test/1234567890/patterns/a";
    let address_multi = "/stress-test/1234567890/patterns/a/b/c";
    
    println!("=== Testing glob_match behavior ===");
    println!("Pattern: {}", pattern_str);
    println!("Address (single level): {}", address_single);
    println!("Address (multi level): {}", address_multi);
    println!();
    
    let match_single = glob_match(pattern_str, address_single);
    let match_multi = glob_match(pattern_str, address_multi);
    
    println!("glob_match('{}', '{}') = {}", pattern_str, address_single, match_single);
    println!("glob_match('{}', '{}') = {}", pattern_str, address_multi, match_multi);
    println!();
    
    // Test with Pattern struct
    println!("=== Testing Pattern struct ===");
    let pattern = Pattern::compile(pattern_str).unwrap();
    println!("Pattern struct matches single: {}", pattern.matches(address_single));
    println!("Pattern struct matches multi: {}", pattern.matches(address_multi));
    println!();
    
    // Let's test simpler cases
    println!("=== Testing simpler cases ===");
    let test_cases = vec![
        ("/**", "/a"),
        ("/**", "/a/b"),
        ("/test/**", "/test/a"),
        ("/test/**", "/test/a/b"),
        ("/test/**", "/test"),
    ];
    
    for (pat, addr) in test_cases {
        let result = glob_match(pat, addr);
        println!("glob_match('{}', '{}') = {}", pat, addr, result);
    }
}
