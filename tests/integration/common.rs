use std::sync::{Arc, Mutex};

/// A thread-safe in-memory writer for capturing output in tests.
#[derive(Clone, Default)]
pub(super) struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl TestWriter {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn output(&self) -> String {
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

pub(super) fn parse_line(s: &str) -> serde_json::Value {
    serde_json::from_str(s.trim()).expect("valid JSON")
}
