use clasp_core::address::Pattern;

fn main() {
    println!("=== Testing segment parsing ===\n");
    
    let patterns = vec![
        "/stress-test/patterns/**",
        "/test/**",
        "/**",
        "/a/b/c/**",
    ];
    
    for pattern_str in patterns {
        match Pattern::compile(pattern_str) {
            Ok(pattern) => {
                let addr = pattern.address();
                println!("Pattern: {}", pattern_str);
                println!("  Raw: {}", addr.as_str());
                println!("  Segments: {:?}", addr.segments());
                println!("  First segment: {:?}", addr.segments().first());
                println!("  Namespace: {:?}", addr.namespace());
                println!();
            }
            Err(e) => println!("Error parsing '{}': {}\n", pattern_str, e),
        }
    }
}
