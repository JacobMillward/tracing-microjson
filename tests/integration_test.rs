use std::sync::{Arc, Mutex};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

/// A thread-safe in-memory writer for capturing output in tests.
#[derive(Clone, Default)]
struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl TestWriter {
    fn new() -> Self {
        Self::default()
    }

    fn output(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
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

fn parse_line(s: &str) -> serde_json::Value {
    serde_json::from_str(s.trim()).expect("valid JSON")
}

// ──────────────────────────────────────────────────────────────────────────────
// Integration tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_event_no_fields() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("hello");
    });
    let out = w.output();
    let line = out.trim();
    let v = parse_line(line);
    assert_eq!(v["level"], "INFO");
    assert_eq!(v["fields"]["message"], "hello");
    assert!(v["timestamp"].is_string());
    assert!(v["target"].is_string());
}

#[test]
fn test_event_mixed_type_fields() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            count = 42u64,
            flag = true,
            ratio = 1.5f64,
            name = "Alice",
            "msg"
        );
    });
    let out = w.output();
    let v = parse_line(out.trim());
    assert_eq!(v["fields"]["count"], 42);
    assert_eq!(v["fields"]["flag"], true);
    assert_eq!(v["fields"]["ratio"], 1.5);
    assert_eq!(v["fields"]["name"], "Alice");
    assert_eq!(v["fields"]["message"], "msg");
}

#[test]
fn test_flatten_event() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).flatten_event(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(request_id = "abc-123", "invoke");
    });
    let out = w.output();
    let v = parse_line(out.trim());
    // Fields are at top level, not nested under "fields"
    assert_eq!(v["message"], "invoke");
    assert_eq!(v["request_id"], "abc-123");
    assert!(v.get("fields").is_none(), "fields key should not exist");
}

#[test]
fn test_event_inside_nested_spans() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let outer = tracing::info_span!("outer", req = "r1");
        let _og = outer.enter();
        let inner = tracing::info_span!("inner", step = 2u64);
        let _ig = inner.enter();
        tracing::info!("processing");
    });
    let out = w.output();
    let v = parse_line(out.trim());

    // "span" should be the innermost span
    assert_eq!(v["span"]["name"], "inner");
    assert_eq!(v["span"]["step"], 2);

    // "spans" should list from root to leaf
    let spans = v["spans"].as_array().expect("spans array");
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0]["name"], "outer");
    assert_eq!(spans[0]["req"], "r1");
    assert_eq!(spans[1]["name"], "inner");
    assert_eq!(spans[1]["step"], 2);
}

#[test]
fn test_on_record_span_fields_updated() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("my_span", initial = "yes", extra = tracing::field::Empty);
        let _g = span.enter();
        // Record additional fields after span creation
        span.record("extra", "value");
        tracing::info!("event");
    });
    let out = w.output();
    let v = parse_line(out.trim());
    assert_eq!(v["span"]["name"], "my_span");
    assert_eq!(v["span"]["initial"], "yes");
    assert_eq!(v["span"]["extra"], "value");
}

#[test]
fn test_optional_fields_filename_line() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone())
        .with_file(true)
        .with_line_number(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("with location");
    });
    let out = w.output();
    let v = parse_line(out.trim());
    assert!(
        v["filename"].is_string(),
        "filename field should be present"
    );
    assert!(
        v["line_number"].is_number(),
        "line_number field should be present"
    );
}

#[test]
fn test_target_hidden() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).with_target(false);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("no target");
    });
    let out = w.output();
    let v = parse_line(out.trim());
    assert!(v.get("target").is_none(), "target should be absent");
}

// ──────────────────────────────────────────────────────────────────────────────
// Compatibility test: compare output with tracing-subscriber's json layer
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_compatibility_with_tracing_subscriber_json() {
    use tracing_subscriber::fmt;

    // Capture output from tracing-subscriber's JSON formatter
    let ts_writer = TestWriter::new();
    {
        let subscriber = tracing_subscriber::registry().with(
            fmt::Layer::new()
                .json()
                .with_writer(ts_writer.clone())
                .with_current_span(true)
                .with_span_list(true),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(request_id = "abc-123", "invoke");
        });
    }

    // Capture output from our JsonLayer
    let our_writer = TestWriter::new();
    {
        let subscriber = tracing_subscriber::registry().with(JsonLayer::new(our_writer.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(request_id = "abc-123", "invoke");
        });
    }

    let ts_val = parse_line(ts_writer.output().trim());
    let our_val = parse_line(our_writer.output().trim());

    // Compare all fields except "timestamp" (which will differ)
    for key in ["level", "target"] {
        assert_eq!(
            ts_val[key], our_val[key],
            "field '{}' should match: ts={} ours={}",
            key, ts_val[key], our_val[key]
        );
    }
    // Fields object structure should match
    assert_eq!(
        ts_val["fields"]["message"], our_val["fields"]["message"],
        "message field should match"
    );
    assert_eq!(
        ts_val["fields"]["request_id"], our_val["fields"]["request_id"],
        "request_id field should match"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Coverage gap tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_all_log_levels() {
    let cases: &[(&str, Box<dyn Fn()>)] = &[
        ("TRACE", Box::new(|| tracing::trace!("msg"))),
        ("DEBUG", Box::new(|| tracing::debug!("msg"))),
        ("INFO", Box::new(|| tracing::info!("msg"))),
        ("WARN", Box::new(|| tracing::warn!("msg"))),
        ("ERROR", Box::new(|| tracing::error!("msg"))),
    ];
    for (expected_level, emit) in cases {
        let w = TestWriter::new();
        let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
        tracing::subscriber::with_default(subscriber, emit);
        let out = w.output();
        let v = parse_line(out.trim());
        assert_eq!(
            v["level"], *expected_level,
            "level mismatch for {expected_level}"
        );
    }
}

#[test]
fn test_i64_negative_field() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(offset = -42i64, "negative int");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["fields"]["offset"], -42);
    assert_eq!(v["fields"]["message"], "negative int");
}

#[test]
fn test_record_debug_field() {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Point {
        x: i32,
        y: i32,
    }

    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let p = Point { x: 1, y: 2 };
        tracing::info!(point = ?p, "debug field");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["fields"]["point"], "Point { x: 1, y: 2 }");
    assert_eq!(v["fields"]["message"], "debug field");
}

#[test]
fn test_record_error_field() {
    #[derive(Debug)]
    struct MyError(String);
    impl std::fmt::Display for MyError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::error::Error for MyError {}

    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let err = MyError("something failed".to_string());
        tracing::error!(err = &err as &dyn std::error::Error, "failure");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["fields"]["err"], "something failed");
    assert_eq!(v["fields"]["message"], "failure");
    assert_eq!(v["level"], "ERROR");
}

#[test]
fn test_event_outside_span_has_no_span_fields() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("no span context");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v.get("span").is_none(),
        "span key must be absent outside any span"
    );
    assert!(
        v.get("spans").is_none(),
        "spans key must be absent outside any span"
    );
}

#[test]
fn test_flatten_event_with_span() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).flatten_event(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("my_span", req_id = "xyz");
        let _g = span.enter();
        tracing::info!(extra = "val", "flat with span");
    });
    let v = parse_line(w.output().trim());
    // Event fields must be at top level
    assert_eq!(v["message"], "flat with span");
    assert_eq!(v["extra"], "val");
    assert!(
        v.get("fields").is_none(),
        "fields key must not exist when flattened"
    );
    // Span context must still be present
    assert_eq!(v["span"]["name"], "my_span");
    assert_eq!(v["span"]["req_id"], "xyz");
    let spans = v["spans"].as_array().expect("spans array");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0]["name"], "my_span");
    assert_eq!(spans[0]["req_id"], "xyz");
}

#[test]
fn test_single_span() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("solo", key = "v");
        let _g = span.enter();
        tracing::info!("inside single span");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["span"]["name"], "solo");
    assert_eq!(v["span"]["key"], "v");
    let spans = v["spans"].as_array().expect("spans array");
    assert_eq!(spans.len(), 1, "spans must have exactly one entry");
    assert_eq!(spans[0]["name"], "solo");
    assert_eq!(spans[0]["key"], "v");
}

#[test]
fn test_span_with_no_fields() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("empty_span");
        let _g = span.enter();
        tracing::info!("inside no-field span");
    });
    let v = parse_line(w.output().trim());
    // span object should have only "name" — no extra keys from an empty field fragment
    let span_obj = v["span"].as_object().expect("span object");
    assert_eq!(span_obj.len(), 1, "span object must have only 'name'");
    assert_eq!(v["span"]["name"], "empty_span");
    let spans = v["spans"].as_array().expect("spans array");
    let span0_obj = spans[0].as_object().expect("spans[0] object");
    assert_eq!(span0_obj.len(), 1, "spans[0] must have only 'name'");
    assert_eq!(spans[0]["name"], "empty_span");
}
