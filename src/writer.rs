/// Write JSON-escaped content for `s` directly into `buf` per [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259).
fn escape_json_into(s: &str, buf: &mut String) {
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\x08' => buf.push_str("\\b"),
            '\x0C' => buf.push_str("\\f"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => buf.push(c),
        }
    }
}

use std::fmt::Write;

/// A minimal JSON string builder that writes into a `String` buffer.
pub struct JsonWriter {
    buf: String,
}

impl JsonWriter {
    /// Create a new, empty writer.
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    /// Create a writer that continues from existing content (e.g. span field fragments).
    /// The existing content is treated as already-written key-value pairs.
    pub fn continuing(existing: &str) -> Self {
        Self {
            buf: existing.to_owned(),
        }
    }

    pub fn obj_start(&mut self) {
        self.buf.push('{');
    }

    pub fn obj_end(&mut self) {
        self.buf.push('}');
    }

    pub fn arr_start(&mut self) {
        self.buf.push('[');
    }

    pub fn arr_end(&mut self) {
        self.buf.push(']');
    }

    /// Write a JSON object key (field names are Rust identifiers, safe without escaping).
    pub fn key(&mut self, name: &str) {
        self.buf.push('"');
        self.buf.push_str(name);
        self.buf.push_str("\":");
    }

    /// Write a JSON string value with proper escaping.
    pub fn val_str(&mut self, s: &str) {
        self.buf.push('"');
        escape_json_into(s, &mut self.buf);
        self.buf.push('"');
    }

    pub fn val_u64(&mut self, v: u64) {
        write!(self.buf, "{v}").unwrap();
    }

    pub fn val_i64(&mut self, v: i64) {
        write!(self.buf, "{v}").unwrap();
    }

    pub fn val_f64(&mut self, v: f64) {
        if v.is_nan() || v.is_infinite() {
            self.val_null();
        } else {
            // Format like serde_json: use Rust's default Display which gives
            // enough precision and handles -0.0 correctly.
            let start = self.buf.len();
            write!(self.buf, "{v}").unwrap();
            // serde_json always includes a decimal point for floats
            let written = &self.buf[start..];
            if !written.contains('.') && !written.contains('e') && !written.contains('E') {
                self.buf.push_str(".0");
            }
        }
    }

    pub fn val_bool(&mut self, v: bool) {
        self.buf.push_str(if v { "true" } else { "false" });
    }

    pub fn val_null(&mut self) {
        self.buf.push_str("null");
    }

    pub fn comma(&mut self) {
        self.buf.push(',');
    }

    /// Write raw JSON content (pre-formatted fragment).
    pub fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    pub fn finish_line(&mut self) {
        self.buf.push('\n');
    }

    /// Consume and return the buffer.
    pub fn into_string(self) -> String {
        self.buf
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}
