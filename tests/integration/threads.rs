use super::common::{parse_line, TestWriter};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

#[test]
fn test_thread_id_present() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).with_thread_ids(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("with thread id");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v["threadId"].is_string(),
        "threadId should be a string, got: {}",
        v["threadId"]
    );
    let tid = v["threadId"].as_str().unwrap();
    assert!(
        tid.starts_with("ThreadId("),
        "threadId should start with 'ThreadId(', got: {tid}"
    );
}

#[test]
fn test_thread_name_present() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone()).with_thread_names(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("with thread name");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v["threadName"].is_string(),
        "threadName should be a string, got: {}",
        v["threadName"]
    );
}

#[test]
fn test_thread_fields_absent_by_default() {
    let w = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(JsonLayer::new(w.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("default config");
    });
    let v = parse_line(w.output().trim());
    assert!(
        v.get("threadId").is_none(),
        "threadId should be absent by default"
    );
    assert!(
        v.get("threadName").is_none(),
        "threadName should be absent by default"
    );
}

#[test]
fn test_thread_id_and_name_together() {
    let w = TestWriter::new();
    let layer = JsonLayer::new(w.clone())
        .with_thread_ids(true)
        .with_thread_names(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("both");
    });
    let v = parse_line(w.output().trim());
    assert!(v["threadId"].is_string());
    assert!(v["threadName"].is_string());
}
