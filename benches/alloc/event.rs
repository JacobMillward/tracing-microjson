use criterion::{Criterion, measurement::Measurement};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

pub fn benchmarks<M: Measurement>(c: &mut Criterion<M>, prefix: &str) {
    let mut group = c.benchmark_group(format!("{prefix}/event"));

    group.bench_function("event_simple", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry().with(
                JsonLayer::new(std::io::sink)
                    .without_time()
                    .with_target(false),
            ),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!("hello");
            });
        });
    });

    group.bench_function("event_fields", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry().with(
                JsonLayer::new(std::io::sink)
                    .without_time()
                    .with_target(false),
            ),
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

    group.bench_function("event_nested_spans", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry().with(
                JsonLayer::new(std::io::sink)
                    .without_time()
                    .with_target(false),
            ),
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

    group.bench_function("event_escape", |b| {
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry().with(
                JsonLayer::new(std::io::sink)
                    .without_time()
                    .with_target(false),
            ),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(text = "say \"hi\"\nnewline\ttab", "escape test");
            });
        });
    });

    group.bench_function("event_debug_fields", |b| {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Point {
            x: f64,
            y: f64,
        }
        let point = Point { x: 1.0, y: 2.0 };
        let dispatch = tracing::Dispatch::new(
            tracing_subscriber::registry().with(
                JsonLayer::new(std::io::sink)
                    .without_time()
                    .with_target(false),
            ),
        );
        b.iter(|| {
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(val = ?point, "msg");
            });
        });
    });

    group.finish();
}
