use super::common::{parse_line, TestWriter};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

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
