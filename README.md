# tracing-microjson

[![Crates.io](https://img.shields.io/crates/v/tracing-microjson)](https://crates.io/crates/tracing-microjson)
[![docs.rs](https://img.shields.io/docsrs/tracing-microjson)](https://docs.rs/tracing-microjson)
[![CI](https://github.com/JacobMillward/tracing-microjson/actions/workflows/ci.yml/badge.svg)](https://github.com/JacobMillward/tracing-microjson/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/crates/l/tracing-microjson)](https://www.gnu.org/licenses/gpl-3.0)

A [`tracing`] layer that outputs JSON-formatted logs **without pulling in serde, serde_json, or tracing-serde**.

[`tracing`]: https://docs.rs/tracing

## Why?

Enabling the `json` feature on `tracing-subscriber` pulls in 7 additional crates
(serde, serde_json, tracing-serde, and their transitive dependencies).
`tracing-microjson` produces the same output format using a hand-written JSON
formatter with zero serialization framework dependencies.

## Who is this for?

- Projects where **compile time** and **binary size** matter
- Environments with strict dependency auditing requirements
- Anyone who wants structured JSON logging with a **minimal dependency footprint**

## Usage

```toml
[dependencies]
tracing-microjson = "0.1"
```

```rust
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

tracing_subscriber::registry()
    .with(JsonLayer::new(std::io::stderr))
    .init();
```

## Configuration

```rust
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

tracing_subscriber::registry()
    .with(
        JsonLayer::new(std::io::stderr)
            .with_target(true)          // include event target (default: true)
            .with_file(true)            // include source filename (default: false)
            .with_line_number(true)     // include source line number (default: false)
            .with_thread_ids(true)      // include thread ID (default: false)
            .with_thread_names(true)    // include thread name (default: false)
            .flatten_event(true)        // flatten fields to top level (default: false)
            .without_time(),            // disable timestamps (default: SystemTimestamp)
    )
    .init();
```

## Comparisons

All comparisons are against `tracing-subscriber` with its `json` feature enabled.

### Features

| Feature                 |     tracing-subscriber `json`      | tracing-microjson |
| ----------------------- | :--------------------------------: | :---------------: |
| JSON event output       |               ✅ Yes               |      ✅ Yes       |
| Span fields & nesting   |               ✅ Yes               |      ✅ Yes       |
| Target, file, line      |               ✅ Yes               |      ✅ Yes       |
| `flatten_event`         |               ✅ Yes               |      ✅ Yes       |
| Custom timestamps       |               ✅ Yes               |      ✅ Yes       |
| Thread ID / name        |               ✅ Yes               |      ✅ Yes       |
| Custom field formatters |               ✅ Yes               |       ❌ No       |
| Serialization deps      | serde + serde_json + tracing-serde |      ✅ None      |

### Dependencies

Both configurations start from `tracing` + `tracing-subscriber` with the `fmt` + `registry` features (14 crates in common).

| Approach                            |  Additional crates   | Total  |
| ----------------------------------- | :------------------: | :----: |
| `tracing-microjson`                 | **+1** (this crate)  | **15** |
| `tracing-subscriber` `json` feature | +7 (serde ecosystem) | **21** |

### Binary size

Minimal "hello world" JSON logging binary (release, LTO, stripped):

| Approach                                 |                      Size |
| ---------------------------------------- | ------------------------: |
| `tracing-microjson`                      | **377 KiB (23% smaller)** |
| `tracing-subscriber` with `json` feature |                   490 KiB |

<sub>aarch64-apple-darwin, Rust 1.93, `strip = true`, `lto = true`.</sub>

### Performance

Head-to-head benchmarks on the same workload (lower is better):

| Scenario          | tracing-microjson | tracing-subscriber | Speedup |
| ----------------- | ----------------: | -----------------: | :-----: |
| Simple event      |            705 ns |             749 ns |  1.06x  |
| Event with fields |            851 ns |           1,042 ns |  1.22x  |
| Nested spans      |          1,281 ns |           2,456 ns |  1.92x  |

<sub>Apple M1 Max, Rust 1.93, criterion 0.8. Run `cargo bench --features _bench_internals` to reproduce.</sub>

## MSRV

The minimum supported Rust version is **1.86** (edition 2024).

## License

Licensed under the [GNU General Public License v3.0 or later](https://www.gnu.org/licenses/gpl-3.0).
