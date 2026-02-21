use super::common::{TestWriter, parse_line};
use tracing_microjson::JsonLayer;
use tracing_subscriber::prelude::*;

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
