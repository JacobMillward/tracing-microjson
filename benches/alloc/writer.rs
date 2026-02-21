use std::hint::black_box;

use criterion::{Criterion, measurement::Measurement};
use tracing_microjson::writer::JsonWriter;
use tracing_microjson::{FormatTime, SystemTimestamp};
use tracing_subscriber::fmt::format::Writer as FmtWriter;

pub fn benchmarks<M: Measurement>(c: &mut Criterion<M>, prefix: &str) {
    let mut group = c.benchmark_group(format!("{prefix}/writer"));

    group.bench_function("val_str_plain", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_str(black_box("hello world"));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("val_str_escape", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_str(black_box("say \"hi\"\nline2"));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("val_u64", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_u64(black_box(1_234_567_890));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("timestamp", |b| {
        b.iter(|| {
            let mut buf = String::new();
            let mut w = FmtWriter::new(&mut buf);
            SystemTimestamp.format_time(&mut w).unwrap();
            black_box(buf)
        });
    });

    group.finish();
}
