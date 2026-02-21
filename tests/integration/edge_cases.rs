use super::common::{parse_line, TestWriter};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

#[test]
fn test_all_log_levels() {
    #[allow(clippy::type_complexity)]
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
fn test_record_u128_field() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            big = 340_282_366_920_938_463_463_374_607_431_768_211_455u128,
            "u128 max"
        );
    });
    let v = parse_line(w.output().trim());
    // u128 values are emitted as JSON strings since JSON numbers can't represent the full range
    assert_eq!(
        v["fields"]["big"],
        "340282366920938463463374607431768211455"
    );
    assert_eq!(v["fields"]["message"], "u128 max");
}

#[test]
fn test_record_i128_field() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            neg = -170_141_183_460_469_231_731_687_303_715_884_105_728i128,
            "i128 min"
        );
    });
    let v = parse_line(w.output().trim());
    assert_eq!(
        v["fields"]["neg"],
        "-170141183460469231731687303715884105728"
    );
    assert_eq!(v["fields"]["message"], "i128 min");
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
