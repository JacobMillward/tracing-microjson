use super::common::{TestWriter, parse_line};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

#[test]
fn test_without_time_no_timestamp_field() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).without_time();
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("no timestamp");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v.get("timestamp").is_none(),
        "timestamp should be absent with without_time()"
    );
    assert_eq!(v["level"], "INFO");
    assert_eq!(v["fields"]["message"], "no timestamp");
}

#[test]
fn test_custom_timer() {
    use tracing_microjson::FormatTime;

    struct FixedTime;

    impl FormatTime for FixedTime {
        fn format_time(
            &self,
            w: &mut tracing_subscriber::fmt::format::Writer<'_>,
        ) -> std::fmt::Result {
            w.write_str("2020-01-01T00:00:00Z")
        }
    }

    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).with_timer(FixedTime);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("fixed");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["timestamp"], "2020-01-01T00:00:00Z");
    assert_eq!(v["fields"]["message"], "fixed");
}

#[test]
fn test_with_timer_unit_is_without_time() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).with_timer(());
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("unit timer");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v.get("timestamp").is_none(),
        "with_timer(()) should omit timestamp"
    );
    assert_eq!(v["level"], "INFO");
    assert_eq!(v["fields"]["message"], "unit timer");
}

#[test]
fn test_builder_chaining_with_timer() {
    use tracing_microjson::FormatTime;

    struct FixedTime;

    impl FormatTime for FixedTime {
        fn format_time(
            &self,
            w: &mut tracing_subscriber::fmt::format::Writer<'_>,
        ) -> std::fmt::Result {
            w.write_str("FIXED")
        }
    }

    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone())
        .with_target(false)
        .with_timer(FixedTime)
        .with_file(false)
        .flatten_event(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("chained");
    });
    let v = parse_line(w.output().trim());
    assert_eq!(v["timestamp"], "FIXED");
    assert_eq!(v["message"], "chained");
    assert!(v.get("target").is_none());
    assert!(v.get("fields").is_none());
}

#[test]
fn test_default_timer_produces_rfc3339() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("default timer");
    });
    let v = parse_line(w.output().trim());
    let ts = v["timestamp"]
        .as_str()
        .expect("timestamp should be a string");
    // RFC 3339 with microsecond precision: YYYY-MM-DDTHH:MM:SS.xxxxxxZ
    assert!(ts.ends_with('Z'), "timestamp should end with Z, got: {ts}");
    assert_eq!(ts.len(), 27, "timestamp should be 27 chars, got: {ts}");
    assert_eq!(&ts[10..11], "T", "timestamp should have T separator");
}

#[test]
fn test_without_time_valid_json_flat() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).without_time().flatten_event(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(key = "val", "flat no time");
    });
    let v = parse_line(w.output().trim());
    assert!(v.get("timestamp").is_none());
    assert_eq!(v["level"], "INFO");
    assert_eq!(v["message"], "flat no time");
    assert_eq!(v["key"], "val");
}
