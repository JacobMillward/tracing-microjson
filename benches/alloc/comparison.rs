use criterion::{Criterion, measurement::Measurement};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

pub fn benchmarks<M: Measurement>(c: &mut Criterion<M>, prefix: &str) {
    let mut group = c.benchmark_group(format!("{prefix}/comparison"));

    // ── event_simple ────────────────────────────────────────────────────────
    group.bench_function("event_simple/microjson", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry()
                .with(JsonLayer::new(std::io::sink).without_time().with_target(false)),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
        });
    });

    group.bench_function("event_simple/tracing-subscriber", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(std::io::sink)
                .without_time()
                .with_target(false)
                .finish(),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
        });
    });

    // ── event_fields ─────────────────────────────────────────────────────────
    group.bench_function("event_fields/microjson", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry()
                .with(JsonLayer::new(std::io::sink).without_time().with_target(false)),
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
        });
    });

    group.bench_function("event_fields/tracing-subscriber", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(std::io::sink)
                .without_time()
                .with_target(false)
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
        });
    });

    // ── event_nested_spans ──────────────────────────────────────────────────
    group.bench_function("event_nested_spans/microjson", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry()
                .with(JsonLayer::new(std::io::sink).without_time().with_target(false)),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                let outer = tracing::info_span!("outer", req = "r1");
                let _og = outer.enter();
                let inner = tracing::info_span!("inner", step = 2u64);
                let _ig = inner.enter();
                tracing::info!("processing");
            });
        });
    });

    group.bench_function("event_nested_spans/tracing-subscriber", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::fmt()
                .json()
                .with_writer(std::io::sink)
                .without_time()
                .with_target(false)
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
        });
    });

    group.finish();
}
