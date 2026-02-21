use std::hint::black_box;
use std::sync::{Arc, Mutex};

use criterion::{Criterion, criterion_group, criterion_main};
use tracing_microjson::JsonLayer;
use tracing_microjson::writer::JsonWriter;
use tracing_microjson::{FormatTime, SystemTimestamp};
use tracing_subscriber::fmt::format::Writer as FmtWriter;
use tracing_subscriber::prelude::*;

// ──────────────────────────────────────────────────────────────────────────────
// TestWriter (same pattern as integration tests)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl TestWriter {
    fn new() -> Self {
        Self::default()
    }

    fn take_output(&self) -> String {
        let mut buf = self.0.lock().unwrap();
        let out = String::from_utf8(buf.clone()).unwrap();
        buf.clear();
        out
    }
}

impl std::io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TestWriter {
    type Writer = TestWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Group 1: JsonWriter micro-benchmarks
// ──────────────────────────────────────────────────────────────────────────────

fn writer_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("writer");

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

    group.bench_function("val_i64", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_i64(black_box(-1_234_567_890));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("val_f64", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_f64(black_box(2.78128));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("val_f64_nan", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_f64(black_box(f64::NAN));
            black_box(jw.into_vec())
        });
    });

    group.bench_function("val_bool", |b| {
        b.iter(|| {
            let mut jw = JsonWriter::new();
            jw.val_bool(black_box(true));
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

// ──────────────────────────────────────────────────────────────────────────────
// Group 2: Full event formatting through the layer
// ──────────────────────────────────────────────────────────────────────────────

fn event_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("event");

    group.bench_function("event_simple", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_fields", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(
                    count = 42u64,
                    flag = true,
                    ratio = 1.5f64,
                    name = "Alice",
                    "msg"
                );
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_nested_spans", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                let outer = tracing::info_span!("outer", req = "r1");
                let _og = outer.enter();
                let inner = tracing::info_span!("inner", step = 2u64);
                let _ig = inner.enter();
                tracing::info!("processing");
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_escape", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(text = "say \"hi\"\nnewline\ttab", "escape test");
            });
            black_box(w.take_output())
        });
    });

    group.finish();
}

// ──────────────────────────────────────────────────────────────────────────────
// Group 3: Head-to-head comparison with tracing-subscriber's JSON layer
// ──────────────────────────────────────────────────────────────────────────────

fn comparison_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");

    // ── event_simple ────────────────────────────────────────────────────────
    group.bench_function("event_simple/microjson", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_simple/tracing-subscriber", |b| {
        let w = TestWriter::new();
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(w.clone())
                .finish(),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
            black_box(w.take_output())
        });
    });

    // ── event_fields ────────────────────────────────────────────────────────
    group.bench_function("event_fields/microjson", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(
                    count = 42u64,
                    flag = true,
                    ratio = 1.5f64,
                    name = "Alice",
                    "msg"
                );
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_fields/tracing-subscriber", |b| {
        let w = TestWriter::new();
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(w.clone())
                .finish(),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(
                    count = 42u64,
                    flag = true,
                    ratio = 1.5f64,
                    name = "Alice",
                    "msg"
                );
            });
            black_box(w.take_output())
        });
    });

    // ── event_nested_spans ──────────────────────────────────────────────────
    group.bench_function("event_nested_spans/microjson", |b| {
        let w = TestWriter::new();
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(JsonLayer::new(w.clone())));
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                let outer = tracing::info_span!("outer", req = "r1");
                let _og = outer.enter();
                let inner = tracing::info_span!("inner", step = 2u64);
                let _ig = inner.enter();
                tracing::info!("processing");
            });
            black_box(w.take_output())
        });
    });

    group.bench_function("event_nested_spans/tracing-subscriber", |b| {
        let w = TestWriter::new();
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(w.clone())
                .finish(),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                let outer = tracing::info_span!("outer", req = "r1");
                let _og = outer.enter();
                let inner = tracing::info_span!("inner", step = 2u64);
                let _ig = inner.enter();
                tracing::info!("processing");
            });
            black_box(w.take_output())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    writer_benchmarks,
    event_benchmarks,
    comparison_benchmarks
);
criterion_main!(benches);
