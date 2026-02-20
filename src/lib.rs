//! A tracing JSON layer with zero serialization framework dependencies.
//!
//! Drop-in replacement for tracing-subscriber's `json` feature, producing
//! identical output format without pulling in serde/serde_json/tracing-serde.
//!
//! # Example
//!
//! ```rust
//! use tracing_microjson::JsonLayer;
//! use tracing_subscriber::prelude::*;
//!
//! tracing_subscriber::registry()
//!     .with(JsonLayer::new(std::io::stderr))
//!     .init();
//! ```

use std::io::Write;
use std::time::SystemTime;
use tracing_core::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

mod writer;
mod visitor;

use visitor::JsonVisitor;
use writer::JsonWriter;

// Extension type stored in span data
struct SpanFields(String);

/// A [`tracing_subscriber::Layer`] that formats events as JSON lines.
pub struct JsonLayer<W> {
    make_writer: W,
    display_target: bool,
    display_filename: bool,
    display_line_number: bool,
    flatten_event: bool,
}

impl<W> JsonLayer<W>
where
    W: for<'w> tracing_subscriber::fmt::MakeWriter<'w> + 'static,
{
    /// Create a new `JsonLayer` writing to the given writer.
    pub fn new(make_writer: W) -> Self {
        Self {
            make_writer,
            display_target: true,
            display_filename: false,
            display_line_number: false,
            flatten_event: false,
        }
    }

    /// Whether to emit the `target` field. Default: `true`.
    pub fn with_target(mut self, display_target: bool) -> Self {
        self.display_target = display_target;
        self
    }

    /// Whether to emit the `filename` field. Default: `false`.
    pub fn with_file(mut self, display_filename: bool) -> Self {
        self.display_filename = display_filename;
        self
    }

    /// Whether to emit the `line_number` field. Default: `false`.
    pub fn with_line_number(mut self, display_line: bool) -> Self {
        self.display_line_number = display_line;
        self
    }

    /// Whether to flatten event fields to the top level instead of nesting
    /// them under `"fields"`. Default: `false`.
    pub fn flatten_event(mut self, flatten: bool) -> Self {
        self.flatten_event = flatten;
        self
    }
}

impl<S, W> Layer<S> for JsonLayer<W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> tracing_subscriber::fmt::MakeWriter<'w> + 'static,
{
    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: Context<'_, S>,
    ) {
        let span = match ctx.span(id) {
            Some(s) => s,
            None => return,
        };
        let mut jw = JsonWriter::new();
        let mut visitor = JsonVisitor::new(&mut jw);
        attrs.record(&mut visitor);
        span.extensions_mut().insert(SpanFields(jw.finish()));
    }

    fn on_record(
        &self,
        id: &tracing_core::span::Id,
        values: &tracing_core::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        let span = match ctx.span(id) {
            Some(s) => s,
            None => return,
        };
        let mut ext = span.extensions_mut();
        if let Some(fields) = ext.get_mut::<SpanFields>() {
            let has_existing = !fields.0.is_empty();
            let mut jw = JsonWriter::continuing(&fields.0);
            let mut visitor = if has_existing {
                JsonVisitor::continuing(&mut jw)
            } else {
                JsonVisitor::new(&mut jw)
            };
            values.record(&mut visitor);
            fields.0 = jw.finish();
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut jw = JsonWriter::new();

        // timestamp
        jw.obj_start();
        jw.key("timestamp");
        jw.val_str(&format_timestamp(SystemTime::now()));

        // level
        jw.comma();
        jw.key("level");
        jw.val_str(&event.metadata().level().to_string());

        if self.flatten_event {
            // Event fields flattened to top level
            let mut visitor = JsonVisitor::continuing(&mut jw);
            event.record(&mut visitor);
        } else {
            // Event fields nested under "fields"
            jw.comma();
            jw.key("fields");
            jw.obj_start();
            let mut visitor = JsonVisitor::new(&mut jw);
            event.record(&mut visitor);
            jw.obj_end();
        }

        // target
        if self.display_target {
            jw.comma();
            jw.key("target");
            jw.val_str(event.metadata().target());
        }

        // filename
        if self.display_filename {
            if let Some(file) = event.metadata().file() {
                jw.comma();
                jw.key("filename");
                jw.val_str(file);
            }
        }

        // line_number
        if self.display_line_number {
            if let Some(line) = event.metadata().line() {
                jw.comma();
                jw.key("line_number");
                jw.val_u64(line as u64);
            }
        }

        // current span and spans list
        if let Some(scope) = ctx.event_scope(event) {
            let spans: Vec<_> = scope.collect();

            // "span" = innermost (first in iterator = closest to current)
            if let Some(leaf) = spans.first() {
                jw.comma();
                jw.key("span");
                jw.obj_start();
                jw.key("name");
                jw.val_str(leaf.name());
                let ext = leaf.extensions();
                if let Some(fields) = ext.get::<SpanFields>() {
                    if !fields.0.is_empty() {
                        jw.comma();
                        jw.raw(&fields.0);
                    }
                }
                jw.obj_end();
            }

            // "spans" = all spans from root to leaf
            jw.comma();
            jw.key("spans");
            jw.arr_start();
            for (i, span) in spans.iter().rev().enumerate() {
                if i > 0 {
                    jw.comma();
                }
                jw.obj_start();
                jw.key("name");
                jw.val_str(span.name());
                let ext = span.extensions();
                if let Some(fields) = ext.get::<SpanFields>() {
                    if !fields.0.is_empty() {
                        jw.comma();
                        jw.raw(&fields.0);
                    }
                }
                jw.obj_end();
            }
            jw.arr_end();
        }

        jw.obj_end();
        jw.finish_line();

        let line = jw.into_string();
        let mut writer = self.make_writer.make_writer();
        let _ = writer.write_all(line.as_bytes());
    }
}

/// Format a `SystemTime` as RFC 3339 with microsecond precision in UTC.
/// e.g. "2026-02-20T12:00:00.000000Z"
fn format_timestamp(t: SystemTime) -> String {
    let dur = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let micros = dur.subsec_micros();

    // Decompose Unix seconds into date/time components
    let (year, month, day, hour, min, sec) = secs_to_datetime(secs);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        year, month, day, hour, min, sec, micros
    )
}

/// Convert Unix seconds to (year, month, day, hour, min, sec) in UTC.
fn secs_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec = secs % 60;
    let mins = secs / 60;
    let min = mins % 60;
    let hours = mins / 60;
    let hour = hours % 24;
    let days = hours / 24;

    // Compute year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days);

    (year, month, day, hour, min, sec)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Using the algorithm from civil_from_days (Howard Hinnant's date algorithms)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_basic() {
        assert_eq!(writer::escape_json("hello"), "hello");
        assert_eq!(writer::escape_json("say \"hi\""), r#"say \"hi\""#);
        assert_eq!(writer::escape_json("back\\slash"), r#"back\\slash"#);
        assert_eq!(writer::escape_json(""), "");
    }

    #[test]
    fn test_escape_json_control_chars() {
        assert_eq!(writer::escape_json("\n"), r"\n");
        assert_eq!(writer::escape_json("\r"), r"\r");
        assert_eq!(writer::escape_json("\t"), r"\t");
        assert_eq!(writer::escape_json("\x08"), r"\b");
        assert_eq!(writer::escape_json("\x0C"), r"\f");
        // U+0001 → \u0001
        assert_eq!(writer::escape_json("\x01"), r"\u0001");
        assert_eq!(writer::escape_json("\x1F"), r"\u001f");
    }

    #[test]
    fn test_escape_json_unicode_passthrough() {
        // Non-ASCII but above U+001F should pass through unescaped
        assert_eq!(writer::escape_json("café"), "café");
        assert_eq!(writer::escape_json("日本語"), "日本語");
    }

    #[test]
    fn test_f64_edge_cases() {
        let mut jw = JsonWriter::new();
        jw.val_f64(f64::NAN);
        assert_eq!(jw.finish(), "null");

        let mut jw = JsonWriter::new();
        jw.val_f64(f64::INFINITY);
        assert_eq!(jw.finish(), "null");

        let mut jw = JsonWriter::new();
        jw.val_f64(f64::NEG_INFINITY);
        assert_eq!(jw.finish(), "null");

        let mut jw = JsonWriter::new();
        jw.val_f64(-0.0_f64);
        let s = jw.finish();
        // -0.0 should be written as a number (not null)
        assert!(s == "-0" || s == "0" || s == "-0.0" || s == "0.0", "got: {s}");

        let mut jw = JsonWriter::new();
        jw.val_f64(3.14);
        let s = jw.finish();
        assert!(s.contains("3.14"), "got: {s}");
    }

    #[test]
    fn test_timestamp_format() {
        // Test known SystemTime value: Unix epoch
        let epoch = SystemTime::UNIX_EPOCH;
        let s = format_timestamp(epoch);
        assert_eq!(s, "1970-01-01T00:00:00.000000Z");

        // Test another known value: 2026-02-20T12:00:00Z = 1771588800 seconds
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1771588800);
        let s = format_timestamp(t);
        assert_eq!(s, "2026-02-20T12:00:00.000000Z");
    }

    #[test]
    fn test_timestamp_microsecond_precision() {
        // 2026-02-20T12:00:00Z + 123456 µs → .123456
        let t = SystemTime::UNIX_EPOCH
            + std::time::Duration::from_micros(1_771_588_800 * 1_000_000 + 123_456);
        let s = format_timestamp(t);
        assert_eq!(s, "2026-02-20T12:00:00.123456Z");

        // Exactly 1 µs past epoch
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_micros(1);
        let s = format_timestamp(t);
        assert_eq!(s, "1970-01-01T00:00:00.000001Z");

        // 999999 µs (all six digits occupied)
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_micros(999_999);
        let s = format_timestamp(t);
        assert_eq!(s, "1970-01-01T00:00:00.999999Z");
    }
}
