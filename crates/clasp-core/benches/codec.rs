//! Codec benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use clasp_core::{codec, Message, SetMessage, Value};

fn encode_benchmark(c: &mut Criterion) {
    let msg = Message::Set(SetMessage {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(3.14159),
        revision: Some(1),
        lock: false,
        unlock: false,
    });

    c.bench_function("encode_set_message", |b| {
        b.iter(|| {
            black_box(codec::encode(&msg).unwrap())
        })
    });
}

fn decode_benchmark(c: &mut Criterion) {
    let msg = Message::Set(SetMessage {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(3.14159),
        revision: Some(1),
        lock: false,
        unlock: false,
    });
    let encoded = codec::encode(&msg).unwrap();

    c.bench_function("decode_set_message", |b| {
        b.iter(|| {
            black_box(codec::decode::<Message>(&encoded).unwrap())
        })
    });
}

fn roundtrip_benchmark(c: &mut Criterion) {
    let msg = Message::Set(SetMessage {
        address: "/test/benchmark/value".to_string(),
        value: Value::Map(
            vec![
                ("key1".to_string(), Value::Int(42)),
                ("key2".to_string(), Value::String("value".to_string())),
                ("key3".to_string(), Value::Array(vec![
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                ])),
            ]
            .into_iter()
            .collect(),
        ),
        revision: Some(1),
        lock: false,
        unlock: false,
    });

    c.bench_function("roundtrip_complex_message", |b| {
        b.iter(|| {
            let encoded = codec::encode(&msg).unwrap();
            black_box(codec::decode::<Message>(&encoded).unwrap())
        })
    });
}

criterion_group!(benches, encode_benchmark, decode_benchmark, roundtrip_benchmark);
criterion_main!(benches);
