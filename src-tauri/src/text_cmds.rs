//! Text search and replacement commands (Phase 3.4).
//!
//! PDF text extraction, search, and replacement across content streams.

use tauri::State;

use crate::db::Database;
use crate::models::*;
use crate::pdf::engine::PdfEngine;
use crate::security;

/// Convert a 0-based page index to the 1-based lopdf page ID.
fn lopdf_page_id(page_index: usize) -> u32 {
    (page_index + 1) as u32
}

fn extract_text_from_page(doc: &lopdf::Document, page_index: usize) -> String {
    use lopdf::Object;
    let pages = doc.get_pages();
    let obj_id = match pages.get(&lopdf_page_id(page_index)) {
        Some(id) => *id,
        None => return String::new(),
    };
    let page_dict = match doc.get_dictionary(obj_id) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let contents = match page_dict.get(b"Contents") {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let resolve_stream = |obj: &lopdf::Object| -> Option<Vec<u8>> {
        match obj {
            Object::Stream(s) => Some(s.content.clone()),
            Object::Reference(r) => {
                if let Ok(o) = doc.get_object(*r) {
                    if let Object::Stream(s) = o {
                        return Some(s.content.clone());
                    }
                }
                None
            }
            _ => None,
        }
    };
    let mut combined: Vec<u8> = Vec::new();
    match contents {
        Object::Array(arr) => {
            for item in arr {
                if let Some(data) = resolve_stream(item) {
                    combined.extend_from_slice(&data);
                    combined.push(b'\n');
                }
            }
        }
        other => {
            if let Some(data) = resolve_stream(other) {
                combined = data;
            } else {
                return String::new();
            }
        }
    }
    let decoded = crate::pdf::content_stream::decode_stream(&lopdf::Stream::new(
        lopdf::Dictionary::new(),
        combined,
    ))
    .unwrap_or_default();

    let s = String::from_utf8_lossy(&decoded);
    let mut text = String::new();
    let mut in_paren = false;
    let mut paren_buf = String::new();
    for ch in s.chars() {
        if in_paren {
            if ch == ')' {
                in_paren = false;
                if !paren_buf.is_empty() {
                    text.push_str(&paren_buf);
                    text.push(' ');
                }
                paren_buf.clear();
            } else {
                paren_buf.push(ch);
            }
        } else if ch == '(' {
            in_paren = true;
            paren_buf.clear();
        } else if ch == '\\' {
        }
    }
    text
}

#[tauri::command]
pub fn search_text(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    path: String,
    query: String,
) -> Result<Vec<TextMatch>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let page_count = doc.get_pages().len();
    let mut results = Vec::new();

    let pdfium_doc = if engine.is_available() {
        engine.open_document(&path).ok()
    } else {
        None
    };

    for page_index in 0..page_count {
        let text = extract_text_from_page(&doc, page_index);
        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();
        let mut start = 0;
        while let Some(pos) = lower_text[start..].find(&lower_query) {
            let abs_pos = start + pos;
            let end = (abs_pos + query.len()).min(text.len());
            let snippet = if abs_pos <= text.len() {
                text[abs_pos..end].to_string()
            } else {
                String::new()
            };
            let bbox = if let Some(ref fd) = pdfium_doc {
                collect_bbox_for_match(fd, page_index as i32, &lower_text, &lower_query, abs_pos)
            } else {
                None
            };
            results.push(TextMatch {
                page_index,
                text: snippet,
                char_index: abs_pos,
                length: query.len(),
                bbox,
            });
            start = abs_pos + lower_query.len();
        }
    }
    Ok(results)
}

fn collect_bbox_for_match(
    doc: &pdfium_render::prelude::PdfDocument<'_>,
    page_index: i32,
    lower_text: &str,
    lower_query: &str,
    abs_pos: usize,
) -> Option<[f64; 4]> {
    let page = doc.pages().get(page_index).ok()?;
    let text_page = page.text().ok()?;
    let mut chars: Vec<(String, f32, f32, f32, f32)> = Vec::new();
    for c in text_page.chars().iter() {
        let s = c.unicode_string().unwrap_or_default().to_lowercase();
        let b = match c.loose_bounds() {
            Ok(r) => (r.left().value, r.bottom().value, r.right().value, r.top().value),
            Err(_) => (0.0, 0.0, 0.0, 0.0),
        };
        chars.push((s, b.0, b.1, b.2, b.3));
    }
    if chars.is_empty() {
        return None;
    }
    let collected: String = chars.iter().map(|(s, ..)| s.as_str()).collect();
    let lower_collected = collected.to_lowercase();
    if lower_collected.replace(' ', "") != lower_text.replace(' ', "") {
        return None;
    }
    let query_len = lower_query.chars().count();
    let start_char = lower_text[..abs_pos.min(lower_text.len())]
        .chars()
        .count();
    let end_char = start_char + query_len;
    if end_char > chars.len() {
        return None;
    }
    let mut min_left = f32::INFINITY;
    let mut min_bottom = f32::INFINITY;
    let mut max_right = f32::NEG_INFINITY;
    let mut max_top = f32::NEG_INFINITY;
    let mut any = false;
    for c in &chars[start_char..end_char] {
        if c.0.trim().is_empty() {
            continue;
        }
        any = true;
        min_left = min_left.min(c.1);
        min_bottom = min_bottom.min(c.2);
        max_right = max_right.max(c.3);
        max_top = max_top.max(c.4);
    }
    if !any {
        return None;
    }
    Some([min_left as f64, min_bottom as f64, max_right as f64, max_top as f64])
}

#[tauri::command]
pub async fn replace_text(
    path: String,
    page_index: usize,
    find: String,
    replace: String,
    output_path: String,
) -> Result<ReplaceResult, String> {
    if find.is_empty() {
        return Err("`find` string must not be empty".to_string());
    }
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ReplaceResult, String> {
        let mut doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
        let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
        let obj_id = page_ids
            .get(page_index)
            .copied()
            .ok_or_else(|| format!("Page {page_index} not found"))?;
        let mut total_replacements = 0usize;
        process_page_text_replacement(&mut doc, obj_id, &find, &replace, &mut total_replacements)?;
        doc.save(&output_path)
            .map_err(|e| format!("Failed to save: {e}"))?;
        Ok(ReplaceResult {
            replacements_made: total_replacements,
            output_path: output_path.to_string_lossy().to_string(),
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

fn process_page_text_replacement(
    doc: &mut lopdf::Document,
    page_id: (u32, u16),
    find: &str,
    replace: &str,
    counter: &mut usize,
) -> Result<(), String> {
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| format!("page dict: {e}"))?;
    let contents = match page_dict.get(b"Contents").ok() {
        Some(c) => c.clone(),
        None => return Ok(()),
    };
    let mut stream_ids: Vec<(u32, u16)> = Vec::new();
    let mut form_refs: Vec<(u32, u16)> = Vec::new();
    match &contents {
        lopdf::Object::Reference(r) => stream_ids.push(*r),
        lopdf::Object::Array(arr) => {
            for e in arr {
                if let lopdf::Object::Reference(r) = e {
                    stream_ids.push(*r);
                }
            }
        }
        _ => {}
    }

    if let Ok(resources) = page_dict.get(b"Resources") {
        if let Ok(resources_dict) = match resources {
            lopdf::Object::Dictionary(d) => Ok(d.clone()),
            lopdf::Object::Reference(r) => doc
                .get_dictionary(*r)
                .ok()
                .cloned()
                .ok_or_else(|| "resources not a dict".to_string()),
            _ => Err("unexpected resources type".to_string()),
        } {
            if let Ok(xo) = resources_dict.get(b"XObject") {
                let xo_dict = match xo {
                    lopdf::Object::Dictionary(d) => Some(d.clone()),
                    lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                    _ => None,
                };
                if let Some(xo_dict) = xo_dict {
                    for (_name, v) in xo_dict.iter() {
                        if let lopdf::Object::Reference(r) = v {
                            if let Ok(stream) = doc.get_object(*r).and_then(|o| o.as_stream()) {
                                let is_form = stream
                                    .dict
                                    .get(b"Subtype")
                                    .ok()
                                    .and_then(|o| o.as_name().ok())
                                    .map(|n| n == b"Form")
                                    .unwrap_or(false);
                                if is_form {
                                    form_refs.push(*r);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for sid in stream_ids {
        let (decoded, filters) = {
            let s = doc
                .get_object(sid)
                .ok()
                .and_then(|o| o.as_stream().ok())
                .ok_or_else(|| format!("content stream {}.{} not found", sid.0, sid.1))?;
            let filters: Vec<Vec<u8>> = match s.dict.get(b"Filter").ok() {
                Some(lopdf::Object::Name(n)) => vec![n.clone()],
                Some(lopdf::Object::Array(arr)) => arr
                    .iter()
                    .filter_map(|f| f.as_name().ok().map(|n| n.to_vec()))
                    .collect(),
                _ => Vec::new(),
            };
            let data = s.content.clone();
            let decoded = crate::pdf::content_stream::decode_stream(&s).unwrap_or(data);
            (decoded, filters)
        };
        let (new_bytes, replacements) =
            replace_text_in_decoded(&decoded, find, replace);
        *counter += replacements;
        if replacements == 0 {
            continue;
        }
        let encoded = if filters
            .iter()
            .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
        {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(&new_bytes)
                .map_err(|e| format!("zlib write: {e}"))?;
            encoder
                .finish()
                .map_err(|e| format!("zlib finish: {e}"))?
        } else {
            new_bytes
        };
        if let Some(obj) = doc.objects.get_mut(&sid) {
            if let Ok(stream_obj) = obj.as_stream_mut() {
                stream_obj.content = encoded;
                stream_obj.dict.remove(b"Length");
            }
        }
    }

    for fid in form_refs {
        process_form_xobject_text_replacement(doc, fid, find, replace, counter)?;
    }
    Ok(())
}

fn process_form_xobject_text_replacement(
    doc: &mut lopdf::Document,
    form_id: (u32, u16),
    find: &str,
    replace: &str,
    counter: &mut usize,
) -> Result<(), String> {
    let (decoded, filters) = {
        let s = doc
            .get_object(form_id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .ok_or_else(|| format!("form stream {}.{} not found", form_id.0, form_id.1))?;
        let filters: Vec<Vec<u8>> = match s.dict.get(b"Filter").ok() {
            Some(lopdf::Object::Name(n)) => vec![n.clone()],
            Some(lopdf::Object::Array(arr)) => arr
                .iter()
                .filter_map(|f| f.as_name().ok().map(|n| n.to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        let data = s.content.clone();
        let decoded = crate::pdf::content_stream::decode_stream(&s).unwrap_or(data);
        (decoded, filters)
    };
    let (new_bytes, replacements) = replace_text_in_decoded(&decoded, find, replace);
    *counter += replacements;
    if replacements == 0 {
        return Ok(());
    }
    let encoded = if filters
        .iter()
        .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
    {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&new_bytes)
            .map_err(|e| format!("zlib write: {e}"))?;
        encoder
            .finish()
            .map_err(|e| format!("zlib finish: {e}"))?
    } else {
        new_bytes
    };
    if let Some(obj) = doc.objects.get_mut(&form_id) {
        if let Ok(stream_obj) = obj.as_stream_mut() {
            stream_obj.content = encoded;
            stream_obj.dict.remove(b"Length");
        }
    }
    let form_dict = doc
        .get_dictionary(form_id)
        .ok()
        .cloned();
    if let Some(form_dict) = form_dict {
        if let Ok(resources) = form_dict.get(b"Resources") {
            let resources_dict = match resources {
                lopdf::Object::Dictionary(d) => Some(d.clone()),
                lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                _ => None,
            };
            if let Some(rd) = resources_dict {
                if let Ok(xo) = rd.get(b"XObject") {
                    let xo_dict = match xo {
                        lopdf::Object::Dictionary(d) => Some(d.clone()),
                        lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                        _ => None,
                    };
                    if let Some(xd) = xo_dict {
                        for (_name, v) in xd.iter() {
                            if let lopdf::Object::Reference(r) = v {
                            if let Ok(stream) = doc.get_object(*r).and_then(|o| o.as_stream()) {
                                let is_form = stream
                                    .dict
                                    .get(b"Subtype")
                                    .ok()
                                    .and_then(|o| o.as_name().ok())
                                    .map(|n| n == b"Form")
                                    .unwrap_or(false);
                                if is_form {
                                    process_form_xobject_text_replacement(
                                        doc, *r, find, replace, counter,
                                    )?;
                                }
                            }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn replace_text_in_decoded(input: &[u8], find: &str, replace: &str) -> (Vec<u8>, usize) {
    use crate::pdf::content_stream::ContentToken;
    let tokens = crate::pdf::content_stream::tokenize_content(input);
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut replacements = 0usize;
    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i], ContentToken::Operator(ref s) if s == "Tj")
            && i > 0
        {
            if let ContentToken::Operand(s) = &tokens[i - 1] {
                let decoded = decode_pdfdoc_string(s);
                let mut new = decoded.clone();
                if new.contains(find) {
                    new = new.replace(find, replace);
                    replacements += new.matches(replace).count()
                        .saturating_sub(decoded.matches(replace).count());
                }
                out.extend_from_slice(&encode_pdfdoc_string(&new));
                out.extend_from_slice(b" ");
                out.extend_from_slice(tokens[i].render().as_bytes());
                i += 1;
                continue;
            }
        }
        if matches!(tokens[i], ContentToken::Operator(ref s) if s == "TJ")
            && i >= 1
        {
            if let ContentToken::Operand(s) = &tokens[i - 1] {
                if s.starts_with('[') && s.ends_with(']') {
                    let inner = &s[1..s.len() - 1];
                    let mut new_inner = String::new();
                    new_inner.push('[');
                    for piece in split_tj_array(inner) {
                        if piece.starts_with('(') {
                            let literal = piece[1..piece.len().saturating_sub(1)].to_string();
                            let mut new_literal = literal.clone();
                            if new_literal.contains(find) {
                                new_literal = new_literal.replace(find, replace);
                                replacements += new_literal
                                    .matches(replace)
                                    .count()
                                    .saturating_sub(literal.matches(replace).count());
                            }
                            new_inner.push('(');
                            new_inner.push_str(std::str::from_utf8(&encode_pdfdoc_string(&new_literal)).unwrap_or(&new_literal));
                            new_inner.push(')');
                        } else {
                            new_inner.push_str(&piece);
                        }
                    }
                    new_inner.push(']');
                    out.extend_from_slice(new_inner.as_bytes());
                    out.extend_from_slice(b" TJ");
                    i += 1;
                    continue;
                }
            }
        }
        out.extend_from_slice(tokens[i].render().as_bytes());
        out.push(b' ');
        i += 1;
    }
    (out, replacements)
}

fn decode_pdfdoc_string(literal: &str) -> String {
    let mut s = literal.to_string();
    if s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        s = s[1..s.len() - 1].to_string();
    }
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('(') => out.push('('),
                Some(')') => out.push(')'),
                Some('\\') => out.push('\\'),
                Some('0'..='7') => {
                    let mut oct = String::new();
                    oct.push(c);
                    if let Some(&next) = chars.peek() {
                        if ('0'..='7').contains(&next) {
                            oct.push(chars.next().unwrap());
                        }
                    }
                    if let Some(&next) = chars.peek() {
                        if ('0'..='7').contains(&next) {
                            oct.push(chars.next().unwrap());
                        }
                    }
                    if let Ok(n) = u8::from_str_radix(&oct, 8) {
                        out.push(n as char);
                    }
                }
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn encode_pdfdoc_string(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len() + 2);
    out.push(b'(');
    for c in s.chars() {
        match c {
            '(' => out.extend_from_slice(b"\\("),
            ')' => out.extend_from_slice(b"\\)"),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            other => {
                let mut buf = [0u8; 4];
                let s = other.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    out.push(b')');
    out
}

fn split_tj_array(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_str = 0i32;
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' if depth_str == 0 => {
                depth_paren += 1;
                current.push(c);
            }
            ')' if depth_str == 0 => {
                depth_paren -= 1;
                current.push(c);
            }
            '<' if depth_str == 0 && depth_paren == 0 => {
                depth_str += 1;
                current.push(c);
            }
            '>' if depth_str == 1 && depth_paren == 0 => {
                depth_str -= 1;
                current.push(c);
            }
            _ if depth_paren + depth_str == 0
                && (c == ' ' || c == '\n' || c == '\r' || c == '\t') =>
            {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
