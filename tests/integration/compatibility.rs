use super::common::{TestWriter, parse_line};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

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

#[test]
fn test_thread_id_compat_with_tracing_subscriber() {
    use tracing_subscriber::fmt;

    // Capture output from tracing-subscriber's JSON formatter with thread info
    let ts_writer = TestWriter::new();
    {
        let subscriber = tracing_subscriber::registry().with(
            fmt::Layer::new()
                .json()
                .with_writer(ts_writer.clone())
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_current_span(true)
                .with_span_list(true),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("compat");
        });
    }

    // Capture output from our JsonLayer with thread info
    let our_writer = TestWriter::new();
    {
        let subscriber = tracing_subscriber::registry().with(
            JsonLayer::new(our_writer.clone())
                .with_thread_ids(true)
                .with_thread_names(true),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("compat");
        });
    }

    let ts_val = parse_line(ts_writer.output().trim());
    let our_val = parse_line(our_writer.output().trim());

    // Both should have threadId and threadName fields with matching types
    assert!(
        ts_val["threadId"].is_string(),
        "tracing-subscriber threadId: {ts_val}",
    );
    assert!(our_val["threadId"].is_string(), "our threadId: {our_val}");
    assert!(
        ts_val["threadName"].is_string(),
        "tracing-subscriber threadName: {ts_val}",
    );
    assert!(
        our_val["threadName"].is_string(),
        "our threadName: {our_val}",
    );

    // Values should match since both run on the same thread
    assert_eq!(
        ts_val["threadId"], our_val["threadId"],
        "threadId should match: ts={} ours={}",
        ts_val["threadId"], our_val["threadId"]
    );
    assert_eq!(
        ts_val["threadName"], our_val["threadName"],
        "threadName should match: ts={} ours={}",
        ts_val["threadName"], our_val["threadName"]
    );
}
