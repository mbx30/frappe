use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::Object;
use lopdf::Stream;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub enum ContentToken {
    Operator(String),
    Operand(String),
    BeginMarkedContent { tag: String },
    EndMarkedContent,
    BeginImageData,
    EndImageData,
    Comment(String),
    Unknown(String),
}

pub fn decode_stream(stream: &Stream) -> Result<Vec<u8>, String> {
    let filters = stream.dict.get(b"Filter");
    match filters {
        Ok(Object::Name(name)) if name == b"FlateDecode" => {
            let mut d = ZlibDecoder::new(stream.content.as_slice());
            let mut buf = Vec::new();
            d.read_to_end(&mut buf)
                .map_err(|e| format!("FlateDecode error: {e}"))?;
            Ok(buf)
        }
        Ok(Object::Array(arr)) => {
            let mut data = stream.content.clone();
            for filter_obj in arr.iter() {
                if let Object::Name(name) = filter_obj {
                    match name.as_slice() {
                        b"FlateDecode" | b"Fl" => {
                            let mut d = ZlibDecoder::new(data.as_slice());
                            let mut buf = Vec::new();
                            d.read_to_end(&mut buf)
                                .map_err(|e| format!("FlateDecode error: {e}"))?;
                            data = buf;
                        }
                        b"ASCII85Decode" | b"A85" => {
                            let decoded = ascii85_decode(&data)?;
                            data = decoded;
                        }
                        b"ASCIIHexDecode" | b"AHx" => {
                            let decoded = ascii_hex_decode(&data)?;
                            data = decoded;
                        }
                        other => {
                            let name_str = String::from_utf8_lossy(other);
                            return Err(format!("Unsupported filter: {name_str}"));
                        }
                    }
                }
            }
            Ok(data)
        }
        Ok(Object::Null) | Err(_) => Ok(stream.content.clone()),
        Ok(other) => {
            let s = format!("{:?}", other);
            return Err(format!("Unexpected Filter value: {s}"));
        }
    }
}

pub fn encode_stream(data: &[u8]) -> Stream {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    let compressed = encoder.finish().unwrap();
    Stream::new(
        lopdf::Dictionary::from_iter(vec![(
            b"Filter".to_vec(),
            Object::Name(b"FlateDecode".to_vec()),
        )]),
        compressed,
    )
}

pub fn tokenize_content(data: &[u8]) -> Vec<ContentToken> {
    // Operate on bytes directly so escape sequences and octal/hex escapes can
    // be decoded without the lossy UTF-8 round-trip that previously corrupted
    // literal strings. Operands are still emitted as strings; the byte→string
    // conversion happens once per token via `from_utf8_lossy`.
    //
    // Fixes:
    //   #153 — parenthesis-balanced literal strings; `<<`/`>>` dict delimiters
    //          (previously `<<` was parsed as two separate `<` hex-string opens).
    //   #170 — escape sequences inside literal strings (\n \r \t \b \f \( \) \\
    //          \\ddd octal).
    //   #176 — `#XX` hex escapes inside name tokens.
    let mut tokens: Vec<ContentToken> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut in_comment = false;
    let mut comment_buf = String::new();

    let mut i = 0;
    let len = data.len();

    while i < len {
        let ch = data[i];

        if in_comment {
            if ch == b'\n' || ch == b'\r' {
                tokens.push(ContentToken::Comment(comment_buf.clone()));
                comment_buf.clear();
                in_comment = false;
            } else {
                comment_buf.push(ch as char);
            }
            i += 1;
            continue;
        }

        // ── Literal string: `( ... )` with parenthesis balancing + escapes ──
        if ch == b'(' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            // Walk the string tracking paren depth and honouring `\` escapes.
            // Per PDF spec §7.3.4.2, an unbalanced ')' inside an escape (\) is
            // allowed, and `\ddd` octal escapes (1–3 digits) are supported.
            let mut depth: i32 = 1;
            let mut buf = String::new();
            i += 1;
            while i < len && depth > 0 {
                let c = data[i];
                if c == b'\\' {
                    i += 1;
                    if i >= len {
                        break;
                    }
                    let esc = data[i];
                    match esc {
                        b'n' => {
                            buf.push('\n');
                            i += 1;
                        }
                        b'r' => {
                            buf.push('\r');
                            i += 1;
                        }
                        b't' => {
                            buf.push('\t');
                            i += 1;
                        }
                        b'b' => {
                            buf.push('\u{08}');
                            i += 1;
                        }
                        b'f' => {
                            buf.push('\u{0C}');
                            i += 1;
                        }
                        b'(' => {
                            buf.push('(');
                            i += 1;
                        }
                        b')' => {
                            buf.push(')');
                            i += 1;
                        }
                        b'\\' => {
                            buf.push('\\');
                            i += 1;
                        }
                        b'\n' => {
                            /* line continuation */
                            i += 1;
                        }
                        b'\r' => {
                            i += 1;
                            if i < len && data[i] == b'\n' {
                                i += 1;
                            }
                        }
                        d if d.is_ascii_digit() => {
                            // Up to 3 octal digits
                            let mut oct = String::new();
                            oct.push(d as char);
                            i += 1;
                            for _ in 0..2 {
                                if i < len && data[i].is_ascii_digit() {
                                    oct.push(data[i] as char);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                            if let Ok(val) = u8::from_str_radix(&oct, 8) {
                                buf.push(val as char);
                            }
                        }
                        _ => {
                            // Unknown escape: per spec, the backslash is dropped
                            // and the following character is used literally.
                            buf.push(esc as char);
                            i += 1;
                        }
                    }
                    continue;
                }
                if c == b'(' {
                    depth += 1;
                    buf.push('(');
                    i += 1;
                    continue;
                }
                if c == b')' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                    buf.push(')');
                    i += 1;
                    continue;
                }
                buf.push(c as char);
                i += 1;
            }
            tokens.push(ContentToken::Operand(format!("({buf})")));
            continue;
        }

        // ── `<<` dict-open vs `<` hex-string-open ──
        if ch == b'<' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            if i + 1 < len && data[i + 1] == b'<' {
                // Dict delimiter `<<` — emit as its own token (#153)
                tokens.push(ContentToken::Operand("<<".to_string()));
                i += 2;
                continue;
            }
            // Hex string `<...>`
            let mut hex_buf = String::new();
            i += 1;
            while i < len && data[i] != b'>' {
                if !data[i].is_ascii_whitespace() {
                    hex_buf.push(data[i] as char);
                }
                i += 1;
            }
            if i < len {
                i += 1;
            } // consume '>'
            tokens.push(ContentToken::Operand(format!("<{hex_buf}>")));
            continue;
        }

        // ── `>>` dict-close ──
        if ch == b'>' && i + 1 < len && data[i + 1] == b'>' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            tokens.push(ContentToken::Operand(">>".to_string()));
            i += 2;
            continue;
        }

        // ── Name token `/Name` with `#XX` hex escapes (#176) ──
        if ch == b'/' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            i += 1;
            // PDF §7.3.5: a name is terminated by whitespace or a delimiter
            // ( ( ) < > [ ] { } / % ). While scanning, decode `#XX` hex.
            let mut name = Vec::new();
            name.push(b'/');
            while i < len {
                let c = data[i];
                if c.is_ascii_whitespace() {
                    break;
                }
                if matches!(
                    c,
                    b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
                ) {
                    break;
                }
                if c == b'#' && i + 2 < len {
                    let h = &data[i + 1..i + 3];
                    let hex_ok = h[0].is_ascii_hexdigit() && h[1].is_ascii_hexdigit();
                    if hex_ok {
                        let val = u8::from_str_radix(&String::from_utf8_lossy(h), 16).unwrap_or(0);
                        name.push(val);
                        i += 3;
                        continue;
                    }
                }
                name.push(c);
                i += 1;
            }
            tokens.push(ContentToken::Operand(
                String::from_utf8_lossy(&name).to_string(),
            ));
            continue;
        }

        if ch == b'%' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            in_comment = true;
            i += 1;
            continue;
        }

        if ch == b'[' || ch == b']' || ch == b'{' || ch == b'}' {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            tokens.push(ContentToken::Operand((ch as char).to_string()));
            i += 1;
            continue;
        }

        if ch.is_ascii_whitespace() {
            if !current.is_empty() {
                tokens.push(ContentToken::Operand(
                    String::from_utf8_lossy(&current).to_string(),
                ));
                current.clear();
            }
            i += 1;
        } else {
            current.push(ch);
            i += 1;
        }
    }
    if !current.is_empty() {
        tokens.push(ContentToken::Operand(
            String::from_utf8_lossy(&current).to_string(),
        ));
    }

    // BMC, BDC, EMC, EI, and BI are handled by dedicated arms below — keep
    // them out of this set so those arms are reachable.
    let operators: std::collections::HashSet<&str> = [
        "b", "B", "b*", "B*", "BT", "BX", "c", "cm", "cs", "CS", "d", "d0", "d1", "Do", "DP", "ET",
        "EX", "f", "F", "f*", "g", "G", "gs", "h", "i", "ID", "j", "J", "k", "K", "l", "m", "M",
        "MP", "n", "q", "Q", "re", "rg", "RG", "ri", "s", "S", "sc", "SC", "sh", "T*", "Tc", "Td",
        "TD", "Tf", "Tj", "TJ", "TL", "Tm", "Tr", "Ts", "Tw", "Tz", "v", "w", "W", "W*", "x", "y",
        "'", "\"",
    ]
    .iter()
    .copied()
    .collect();

    let mut result = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].clone() {
            ContentToken::Operand(ref s) if operators.contains(s.as_str()) => {
                result.push(ContentToken::Operator(s.clone()));
            }
            ContentToken::Operand(ref s) if s == "BMC" || s == "BDC" => {
                if i > 0 {
                    if let ContentToken::Operand(tag) = &tokens[i - 1] {
                        let tag_clean = tag.trim_start_matches('/');
                        result.push(ContentToken::BeginMarkedContent {
                            tag: tag_clean.to_string(),
                        });
                    }
                }
            }
            ContentToken::Operand(ref s) if s == "EMC" => {
                result.push(ContentToken::EndMarkedContent);
            }
            ContentToken::Operand(ref s) if s == "BI" => {
                result.push(ContentToken::BeginImageData);
            }
            ContentToken::Operand(ref s) if s == "EI" => {
                result.push(ContentToken::EndImageData);
            }
            t => result.push(t),
        }
        i += 1;
    }
    result
}

fn ascii85_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    let s = String::from_utf8_lossy(data);
    let mut result = Vec::new();
    let mut buf: [u8; 5] = [0u8; 5];
    let mut buf_len = 0;
    for ch in s.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '~' {
            break;
        }
        if ch == 'z' {
            result.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let val = (ch as u8).wrapping_sub(33);
        if val > 84 {
            return Err(format!("Invalid ASCII85 character: {ch}"));
        }
        buf[buf_len] = val;
        buf_len += 1;
        if buf_len == 5 {
            let mut code: u32 = 0;
            for j in 0..5 {
                code = code * 85 + buf[j] as u32;
            }
            result.extend_from_slice(&[
                (code >> 24) as u8,
                (code >> 16) as u8,
                (code >> 8) as u8,
                code as u8,
            ]);
            buf_len = 0;
        }
    }
    if buf_len > 1 {
        for j in buf_len..5 {
            buf[j] = 84;
        }
        let mut code: u32 = 0;
        for j in 0..5 {
            code = code * 85 + buf[j] as u32;
        }
        result.extend_from_slice(&[
            (code >> 24) as u8,
            (code >> 16) as u8,
            (code >> 8) as u8,
            code as u8,
        ]);
        result.truncate(result.len() - (5 - buf_len));
    }
    Ok(result)
}

fn ascii_hex_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    let s = String::from_utf8_lossy(data);
    let hex_part: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .take_while(|c| *c != '>')
        .collect();
    let bytes: Result<Vec<u8>, _> = (0..hex_part.len())
        .step_by(2)
        .map(|i| {
            let end = (i + 2).min(hex_part.len());
            let p = if end - i == 1 {
                format!("{}0", &hex_part[i..end])
            } else {
                hex_part[i..end].to_string()
            };
            u8::from_str_radix(&p, 16).map_err(|e| format!("Hex decode error: {e}"))
        })
        .collect();
    bytes
}
