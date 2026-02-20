/// Escape a string for JSON output per RFC 8259.
pub fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// A minimal JSON string builder that writes into a `String` buffer.
pub struct JsonWriter {
    buf: String,
}

impl JsonWriter {
    /// Create a new, empty writer.
    pub fn new() -> Self {
        Self {
            buf: String::new(),
        }
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
        self.buf.push_str(&escape_json(s));
        self.buf.push('"');
    }

    pub fn val_u64(&mut self, v: u64) {
        self.buf.push_str(&v.to_string());
    }

    pub fn val_i64(&mut self, v: i64) {
        self.buf.push_str(&v.to_string());
    }

    pub fn val_f64(&mut self, v: f64) {
        if v.is_nan() || v.is_infinite() {
            self.buf.push_str("null");
        } else {
            // Format like serde_json: use Rust's default Display which gives
            // enough precision and handles -0.0 correctly.
            let s = format!("{}", v);
            // serde_json always includes a decimal point for floats
            if s.contains('.') || s.contains('e') || s.contains('E') {
                self.buf.push_str(&s);
            } else {
                self.buf.push_str(&s);
                self.buf.push_str(".0");
            }
        }
    }

    pub fn val_bool(&mut self, v: bool) {
        self.buf.push_str(if v { "true" } else { "false" });
    }

    #[allow(dead_code)]
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

    /// Return the buffer without the trailing newline (for span field storage).
    pub fn finish(self) -> String {
        self.buf
    }

    /// Consume and return the buffer (including any trailing newline).
    pub fn into_string(self) -> String {
        self.buf
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}
