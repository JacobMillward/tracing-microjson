use std::fmt::{self, Write as _};

/// Write JSON-escaped content for `s` directly into `buf` per [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259).
///
/// Uses byte-level scanning: safe ranges are flushed in bulk with a single
/// `extend_from_slice`, so the common case (no characters to escape) copies
/// the entire input in one shot.
fn escape_json_into(s: &str, buf: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    let mut start = 0;

    for (i, &b) in bytes.iter().enumerate() {
        let escape = match b {
            b'"' => &b"\\\""[..],
            b'\\' => &b"\\\\"[..],
            b'\x08' => &b"\\b"[..],
            b'\x0C' => &b"\\f"[..],
            b'\n' => &b"\\n"[..],
            b'\r' => &b"\\r"[..],
            b'\t' => &b"\\t"[..],
            b if b < 0x20 => {
                // Flush the safe range before this byte
                buf.extend_from_slice(&bytes[start..i]);
                // \u00XX — the two hex nibbles
                const HEX: &[u8; 16] = b"0123456789abcdef";
                buf.extend_from_slice(b"\\u00");
                buf.push(HEX[(b >> 4) as usize]);
                buf.push(HEX[(b & 0x0F) as usize]);
                start = i + 1;
                continue;
            }
            _ => {
                continue;
            }
        };
        // Flush safe range, then the fixed escape sequence
        buf.extend_from_slice(&bytes[start..i]);
        buf.extend_from_slice(escape);
        start = i + 1;
    }

    // Final flush of any remaining safe bytes
    buf.extend_from_slice(&bytes[start..]);
}

/// A minimal JSON string builder backed by a `Vec<u8>` buffer.
///
/// Implements [`fmt::Write`] so it can be used as a sink for `write!` macros
/// and with [`tracing_subscriber::fmt::format::Writer`].
pub struct JsonWriter {
    buf: Vec<u8>,
}

impl JsonWriter {
    /// Create a new, empty writer.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Create a writer that wraps an existing `Vec<u8>` (for buffer reuse).
    pub fn from_vec(buf: Vec<u8>) -> Self {
        Self { buf }
    }

    /// Create a writer that continues from existing content (e.g. span field fragments).
    /// The existing content is treated as already-written key-value pairs.
    pub fn continuing(existing: &[u8]) -> Self {
        Self {
            buf: existing.to_vec(),
        }
    }

    pub fn obj_start(&mut self) {
        self.buf.push(b'{');
    }

    pub fn obj_end(&mut self) {
        self.buf.push(b'}');
    }

    pub fn arr_start(&mut self) {
        self.buf.push(b'[');
    }

    pub fn arr_end(&mut self) {
        self.buf.push(b']');
    }

    /// Write a JSON object key (field names are Rust identifiers, safe without escaping).
    pub fn key(&mut self, name: &str) {
        self.buf.push(b'"');
        self.buf.extend_from_slice(name.as_bytes());
        self.buf.extend_from_slice(b"\":");
    }

    /// Write a JSON string value with proper escaping.
    pub fn val_str(&mut self, s: &str) {
        self.buf.push(b'"');
        escape_json_into(s, &mut self.buf);
        self.buf.push(b'"');
    }

    pub fn val_u64(&mut self, v: u64) {
        write!(self, "{v}").unwrap();
    }

    pub fn val_i64(&mut self, v: i64) {
        write!(self, "{v}").unwrap();
    }

    pub fn val_f64(&mut self, v: f64) {
        if v.is_nan() || v.is_infinite() {
            self.val_null();
        } else {
            // Format like serde_json: use Rust's default Display which gives
            // enough precision and handles -0.0 correctly.
            let start = self.buf.len();
            write!(self, "{v}").unwrap();
            // serde_json always includes a decimal point for floats
            let written = &self.buf[start..];
            if !written.contains(&b'.') && !written.contains(&b'e') && !written.contains(&b'E') {
                self.buf.extend_from_slice(b".0");
            }
        }
    }

    pub fn val_bool(&mut self, v: bool) {
        self.buf
            .extend_from_slice(if v { b"true" } else { b"false" });
    }

    pub fn val_null(&mut self) {
        self.buf.extend_from_slice(b"null");
    }

    /// Write a JSON string value from a `Debug` value, streaming the escape
    /// so no intermediate `String` is allocated.
    pub fn val_debug(&mut self, value: &dyn fmt::Debug) {
        self.buf.push(b'"');
        let _ = write!(JsonEscapingWriter { buf: &mut self.buf }, "{value:?}");
        self.buf.push(b'"');
    }

    /// Write a JSON string value from a `Display` value, streaming the escape
    /// so no intermediate `String` is allocated.
    pub fn val_display(&mut self, value: &dyn fmt::Display) {
        self.buf.push(b'"');
        let _ = write!(JsonEscapingWriter { buf: &mut self.buf }, "{value}");
        self.buf.push(b'"');
    }

    pub fn comma(&mut self) {
        self.buf.push(b',');
    }

    /// Write raw JSON content (pre-formatted byte fragment).
    pub fn raw(&mut self, s: &[u8]) {
        self.buf.extend_from_slice(s);
    }

    pub fn finish_line(&mut self) {
        self.buf.push(b'\n');
    }

    /// Push a single raw byte.
    pub(crate) fn push_byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    /// Current length of the buffer in bytes.
    pub(crate) fn len(&self) -> usize {
        self.buf.len()
    }

    /// Truncate the buffer to `len` bytes.
    pub(crate) fn truncate(&mut self, len: usize) {
        self.buf.truncate(len);
    }

    /// Return a byte slice of the buffer contents.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Consume and return the underlying `Vec<u8>`.
    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for JsonWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// An `fmt::Write` adapter that JSON-escapes all text written through it.
///
/// Used by [`JsonWriter::val_debug`] and [`JsonWriter::val_display`] to
/// stream-escape `Debug`/`Display` output without an intermediate `String`.
struct JsonEscapingWriter<'a> {
    buf: &'a mut Vec<u8>,
}

impl fmt::Write for JsonEscapingWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        escape_json_into(s, self.buf);
        Ok(())
    }
}
