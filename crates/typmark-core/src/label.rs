pub(crate) fn normalize_link_label(bytes: &[u8]) -> String {
    let mut out = Vec::new();
    let mut escaped = false;
    let mut last_space = false;
    for (idx, &b) in bytes.iter().enumerate() {
        if escaped {
            let lowered = if b.is_ascii_uppercase() {
                b.to_ascii_lowercase()
            } else {
                b
            };
            out.push(lowered);
            escaped = false;
            last_space = false;
            continue;
        }
        if b == b'\\' {
            if idx + 1 < bytes.len() && is_label_escape(bytes[idx + 1]) {
                escaped = true;
                continue;
            }
            out.push(b'\\');
            last_space = false;
            continue;
        }
        if b.is_ascii_whitespace() {
            if !out.is_empty() && !last_space {
                out.push(b' ');
                last_space = true;
            }
            continue;
        }
        last_space = false;
        let lowered = if b.is_ascii_uppercase() {
            b.to_ascii_lowercase()
        } else {
            b
        };
        out.push(lowered);
    }
    if escaped {
        out.push(b'\\');
    }
    if out.last() == Some(&b' ') {
        out.pop();
    }
    let normalized = match String::from_utf8(out) {
        Ok(value) => value,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
    };
    let lowered = normalized.to_lowercase();
    lowered.replace('ß', "ss").replace('ẞ', "ss")
}

pub(crate) fn is_label_escape(byte: u8) -> bool {
    byte == b'[' || byte == b']' || byte == b'\\'
}

pub(crate) fn unescape_backslash_punct(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_punctuation() {
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
