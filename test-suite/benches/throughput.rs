//! Throughput Benchmarks for CLASP Protocol

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use clasp_core::{Message, Value, SignalType, encode, decode};

fn bench_encode_set(c: &mut Criterion) {
    let msg = Message::Set {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(0.5),
        revision: Some(1),
        meta: None,
    };

    let mut group = c.benchmark_group("encode");
    group.throughput(Throughput::Elements(1));

    group.bench_function("set_float", |b| {
        b.iter(|| {
            black_box(encode(&msg).unwrap())
        })
    });

    group.finish();
}

fn bench_decode_set(c: &mut Criterion) {
    let msg = Message::Set {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(0.5),
        revision: Some(1),
        meta: None,
    };
    let encoded = encode(&msg).unwrap();

    let mut group = c.benchmark_group("decode");
    group.throughput(Throughput::Elements(1));

    group.bench_function("set_float", |b| {
        b.iter(|| {
            black_box(decode(&encoded).unwrap())
        })
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let msg = Message::Set {
        address: "/test/benchmark/value".to_string(),
        value: Value::Float(0.5),
        revision: Some(1),
        meta: None,
    };

    let mut group = c.benchmark_group("roundtrip");
    group.throughput(Throughput::Elements(1));

    group.bench_function("set_float", |b| {
        b.iter(|| {
            let encoded = encode(&msg).unwrap();
            black_box(decode(&encoded).unwrap())
        })
    });

    group.finish();
}

fn bench_publish_stream(c: &mut Criterion) {
    let msg = Message::Publish {
        address: "/stream/test".to_string(),
        signal_type: SignalType::Stream,
        value: Value::Float(0.5),
        meta: None,
    };

    let mut group = c.benchmark_group("stream");
    group.throughput(Throughput::Elements(1));

    group.bench_function("publish_roundtrip", |b| {
        b.iter(|| {
            let encoded = encode(&msg).unwrap();
            black_box(decode(&encoded).unwrap())
        })
    });

    group.finish();
}

fn bench_bundle(c: &mut Criterion) {
    let bundle = Message::Bundle {
        timestamp: Some(1704067200000000),
        messages: vec![
            Message::Set {
                address: "/bundle/1".to_string(),
                value: Value::Float(1.0),
                revision: None,
                meta: None,
            },
            Message::Set {
                address: "/bundle/2".to_string(),
                value: Value::Float(0.5),
                revision: None,
                meta: None,
            },
            Message::Set {
                address: "/bundle/3".to_string(),
                value: Value::Float(0.25),
                revision: None,
                meta: None,
            },
        ],
    };

    let mut group = c.benchmark_group("bundle");
    group.throughput(Throughput::Elements(3)); // 3 messages in bundle

    group.bench_function("three_sets_roundtrip", |b| {
        b.iter(|| {
            let encoded = encode(&bundle).unwrap();
            black_box(decode(&encoded).unwrap())
        })
    });

    group.finish();
}

fn bench_large_array(c: &mut Criterion) {
    let large_array: Vec<Value> = (0..512)
        .map(|i| Value::Float(i as f64 / 512.0))
        .collect();

    let msg = Message::Set {
        address: "/large/array".to_string(),
        value: Value::Array(large_array),
        revision: Some(1),
        meta: None,
    };

    let mut group = c.benchmark_group("large_payload");
    group.throughput(Throughput::Elements(512));

    group.bench_function("512_floats_roundtrip", |b| {
        b.iter(|| {
            let encoded = encode(&msg).unwrap();
            black_box(decode(&encoded).unwrap())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_set,
    bench_decode_set,
    bench_roundtrip,
    bench_publish_stream,
    bench_bundle,
    bench_large_array,
);

criterion_main!(benches);
