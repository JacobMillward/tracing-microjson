use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::Throughput;
use criterion::measurement::{Measurement, ValueFormatter};

// ──────────────────────────────────────────────────────────────────────────────
// Counting allocator
// ──────────────────────────────────────────────────────────────────────────────

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
}

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|c| {
            if c.get() {
                ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
                ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

// ──────────────────────────────────────────────────────────────────────────────
// AllocCount measurement
// ──────────────────────────────────────────────────────────────────────────────

pub struct AllocCount;

struct AllocCountFormatter;

impl Measurement for AllocCount {
    type Intermediate = u64;
    type Value = u64;

    fn start(&self) -> Self::Intermediate {
        COUNTING.with(|c| c.set(true));
        ALLOC_COUNT.load(Ordering::Relaxed)
    }

    fn end(&self, start: Self::Intermediate) -> Self::Value {
        let end = ALLOC_COUNT.load(Ordering::Relaxed);
        COUNTING.with(|c| c.set(false));
        end.saturating_sub(start)
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        v1 + v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &AllocCountFormatter
    }
}

impl ValueFormatter for AllocCountFormatter {
    fn scale_values(&self, typical: f64, values: &mut [f64]) -> &'static str {
        if typical >= 1_000_000.0 {
            for v in values {
                *v /= 1_000_000.0;
            }
            "mallocs"
        } else if typical >= 1_000.0 {
            for v in values {
                *v /= 1_000.0;
            }
            "kallocs"
        } else {
            "allocs"
        }
    }

    fn scale_throughputs(
        &self,
        _typical: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        let divisor = match *throughput {
            Throughput::Elements(n)
            | Throughput::Bytes(n)
            | Throughput::BytesDecimal(n)
            | Throughput::Bits(n) => n as f64,
            Throughput::ElementsAndBytes { elements, .. } => elements as f64,
        };
        for v in values {
            *v /= divisor;
        }
        "allocs/elem"
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "allocs"
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// AllocBytes measurement
// ──────────────────────────────────────────────────────────────────────────────

pub struct AllocBytes;

struct AllocBytesFormatter;

impl Measurement for AllocBytes {
    type Intermediate = u64;
    type Value = u64;

    fn start(&self) -> Self::Intermediate {
        COUNTING.with(|c| c.set(true));
        ALLOC_BYTES.load(Ordering::Relaxed)
    }

    fn end(&self, start: Self::Intermediate) -> Self::Value {
        let end = ALLOC_BYTES.load(Ordering::Relaxed);
        COUNTING.with(|c| c.set(false));
        end.saturating_sub(start)
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        v1 + v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &AllocBytesFormatter
    }
}

impl ValueFormatter for AllocBytesFormatter {
    fn scale_values(&self, typical: f64, values: &mut [f64]) -> &'static str {
        let (factor, unit) = if typical >= 1024.0 * 1024.0 * 1024.0 {
            (1.0 / (1024.0 * 1024.0 * 1024.0), "GiB")
        } else if typical >= 1024.0 * 1024.0 {
            (1.0 / (1024.0 * 1024.0), "MiB")
        } else if typical >= 1024.0 {
            (1.0 / 1024.0, "KiB")
        } else {
            (1.0, "B")
        };
        for v in values {
            *v *= factor;
        }
        unit
    }

    fn scale_throughputs(
        &self,
        _typical: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        let divisor = match *throughput {
            Throughput::Elements(n)
            | Throughput::Bytes(n)
            | Throughput::BytesDecimal(n)
            | Throughput::Bits(n) => n as f64,
            Throughput::ElementsAndBytes { elements, .. } => elements as f64,
        };
        for v in values {
            *v /= divisor;
        }
        "B/elem"
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "B"
    }
}
