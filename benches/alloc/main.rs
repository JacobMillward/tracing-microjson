mod comparison;
mod event;
mod measurement;
mod writer;

use criterion::{Criterion, criterion_group, criterion_main};

use measurement::{AllocBytes, AllocCount};

fn writer_allocs(c: &mut Criterion<AllocCount>) {
    writer::benchmarks(c, "allocs");
}

fn writer_bytes(c: &mut Criterion<AllocBytes>) {
    writer::benchmarks(c, "bytes");
}

fn event_allocs(c: &mut Criterion<AllocCount>) {
    event::benchmarks(c, "allocs");
}

fn event_bytes(c: &mut Criterion<AllocBytes>) {
    event::benchmarks(c, "bytes");
}

fn comparison_allocs(c: &mut Criterion<AllocCount>) {
    comparison::benchmarks(c, "allocs");
}

fn comparison_bytes(c: &mut Criterion<AllocBytes>) {
    comparison::benchmarks(c, "bytes");
}

criterion_group! {
    name = alloc_benches;
    config = Criterion::default().with_measurement(AllocCount);
    targets = writer_allocs, event_allocs, comparison_allocs
}

criterion_group! {
    name = bytes_benches;
    config = Criterion::default().with_measurement(AllocBytes);
    targets = writer_bytes, event_bytes, comparison_bytes
}

// Both groups share a single thread-local COUNTING flag, so they must run sequentially
// (which criterion_main! guarantees).
criterion_main!(alloc_benches, bytes_benches);
