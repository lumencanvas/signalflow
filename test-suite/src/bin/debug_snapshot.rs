//! Debug late-joiner snapshot issue

use clasp_core::{codec, Message, ParamValue, SnapshotMessage, Value};
use std::time::Instant;

fn main() {
    println!("=== Snapshot Encoding Test ===\n");
    
    for count in [10, 100, 500, 1000, 2000, 5000] {
        // Create snapshot with N params
        let params: Vec<ParamValue> = (0..count)
            .map(|i| ParamValue {
                address: format!("/state/{}", i),
                value: Value::Float(i as f64),
                revision: i as u64,
                writer: Some("test".to_string()),
                timestamp: Some(1000000u64 + i as u64),
            })
            .collect();
        
        let snapshot = Message::Snapshot(SnapshotMessage { params });
        
        // Measure encoding
        let start = Instant::now();
        match codec::encode(&snapshot) {
            Ok(bytes) => {
                let encode_time = start.elapsed();
                println!(
                    "{:>5} params: {:>8} bytes | encoded in {:>8?} | {:.1} KB",
                    count,
                    bytes.len(),
                    encode_time,
                    bytes.len() as f64 / 1024.0
                );
                
                // Verify decoding
                let decode_start = Instant::now();
                match codec::decode(&bytes) {
                    Ok((msg, _frame)) => {
                        let decode_time = decode_start.elapsed();
                        if let Message::Snapshot(s) = msg {
                            println!("         decoded {} params in {:?}", s.params.len(), decode_time);
                        }
                    }
                    Err(e) => {
                        println!("         DECODE ERROR: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("{:>5} params: ENCODE ERROR: {}", count, e);
            }
        }
        println!();
    }
}
