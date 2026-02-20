use crate::writer::JsonWriter;
use tracing_core::field::{Field, Visit};

/// A [`Visit`] implementation that writes fields as JSON key-value pairs.
pub(crate) struct JsonVisitor<'a> {
    writer: &'a mut JsonWriter,
    first: bool,
}

impl<'a> JsonVisitor<'a> {
    /// Create a new visitor that writes the first field without a leading comma.
    pub(crate) fn new(writer: &'a mut JsonWriter) -> Self {
        Self {
            writer,
            first: true,
        }
    }

    /// Create a visitor that treats the writer as already having content,
    /// so all fields are preceded by a comma.
    pub(crate) fn continuing(writer: &'a mut JsonWriter) -> Self {
        Self {
            writer,
            first: false,
        }
    }
}

impl<'a> Visit for JsonVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_str(value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_u64(value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_i64(value);
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_f64(value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_bool(value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_str(&format!("{:?}", value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        if !self.first {
            self.writer.comma();
        }
        self.first = false;
        self.writer.key(field.name());
        self.writer.val_str(&value.to_string());
    }
}
