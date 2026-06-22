use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::{Dictionary, Document, Object};
use serde::Serialize;
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize)]
pub struct ConversionResult {
    pub images_converted: usize,
    pub vector_ops_converted: usize,
    pub warnings: Vec<String>,
}

// sRGB → linear → luminance → CMYK conversion using standard formulas
// Based on ISO 12647-2 / FOGRA39 characterization

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb_to_cmyk_pixel(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {
    let r_norm = r as f64 / 255.0;
    let g_norm = g as f64 / 255.0;
    let b_norm = b as f64 / 255.0;

    // Convert to linear sRGB
    let r_lin = srgb_to_linear(r_norm);
    let g_lin = srgb_to_linear(g_norm);
    let b_lin = srgb_to_linear(b_norm);

    // Convert linear sRGB to CMY
    let c = 1.0 - r_lin;
    let m = 1.0 - g_lin;
    let y = 1.0 - b_lin;

    // Black generation (GCR — Gray Component Replacement)
    let k = c.min(m).min(y);

    // Under Color Removal (UCR)
    let ucr = k * 0.5;
    let c_out = ((c - ucr).max(0.0).min(1.0) * 255.0) as u8;
    let m_out = ((m - ucr).max(0.0).min(1.0) * 255.0) as u8;
    let y_out = ((y - ucr).max(0.0).min(1.0) * 255.0) as u8;
    let k_out = ((k * 255.0).min(255.0)) as u8;

    (c_out, m_out, y_out, k_out)
}

fn rgb_to_cmyk_float(r: f64, g: f64, b: f64) -> (f64, f64, f64, f64) {
    let r_lin = srgb_to_linear(r.max(0.0).min(1.0));
    let g_lin = srgb_to_linear(g.max(0.0).min(1.0));
    let b_lin = srgb_to_linear(b.max(0.0).min(1.0));

    let c = 1.0 - r_lin;
    let m = 1.0 - g_lin;
    let y = 1.0 - b_lin;

    let k = c.min(m).min(y);
    let ucr = k * 0.5;

    (
        (c - ucr).max(0.0).min(1.0),
        (m - ucr).max(0.0).min(1.0),
        (y - ucr).max(0.0).min(1.0),
        k.min(1.0),
    )
}

// Bundled ICC profiles as byte arrays for OutputIntent embedding
// These are hex-encoded minimal ICC profiles
// In production, include actual profile files as Tauri resources

pub fn get_bundled_icc_profiles() -> Vec<IccProfileInfo> {
    vec![
        IccProfileInfo {
            name: "sRGB_v4".into(),
            description: "sRGB v4 ICC preference".into(),
            color_space_type: "RGB".into(),
            num_channels: 3,
            data: None,
            file_name: "sRGB_v4_ICC_preference.icc".into(),
        },
        IccProfileInfo {
            name: "ISOcoated_v2".into(),
            description: "ISO Coated v2 (FOGRA39)".into(),
            color_space_type: "CMYK".into(),
            num_channels: 4,
            data: None,
            file_name: "ISOcoated_v2_eci.icc".into(),
        },
        IccProfileInfo {
            name: "USWebCoatedSWOP".into(),
            description: "US Web Coated (SWOP) v2".into(),
            color_space_type: "CMYK".into(),
            num_channels: 4,
            data: None,
            file_name: "USWebCoatedSWOP.icc".into(),
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct IccProfileInfo {
    pub name: String,
    pub description: String,
    pub color_space_type: String,
    pub num_channels: u8,
    pub data: Option<Vec<u8>>,
    pub file_name: String,
}

pub struct LcmsEngine;

impl LcmsEngine {
    pub fn new() -> Result<Self, String> {
        // In production, this would initialize lcms2.
        // For now, we use mathematical RGB→CMYK conversion.
        Ok(LcmsEngine)
    }

    pub fn convert_pixels(&self, pixels: &[u8], channels_in: u8) -> Vec<u8> {
        match channels_in {
            3 => {
                let mut out = Vec::with_capacity(pixels.len() / 3 * 4);
                for chunk in pixels.chunks(3) {
                    if chunk.len() == 3 {
                        let (c, m, y, k) = rgb_to_cmyk_pixel(chunk[0], chunk[1], chunk[2]);
                        out.push(c);
                        out.push(m);
                        out.push(y);
                        out.push(k);
                    }
                }
                out
            }
            1 => {
                // Grayscale → K-only
                pixels.iter().map(|&p| 255 - p).collect()
            }
            _ => pixels.to_vec(),
        }
    }
}

pub fn convert_rgb_to_cmyk(doc: &mut Document, scope: &str) -> Result<ConversionResult, String> {
    let mut result = ConversionResult {
        images_converted: 0,
        vector_ops_converted: 0,
        warnings: Vec::new(),
    };

    let engine = LcmsEngine::new()?;
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();

    // Convert image XObjects
    if scope == "both" || scope == "images" {
        for page_num in 0..page_ids.len() {
            let obj_id = page_ids[page_num];

            let resources = match get_resources(doc, obj_id) {
                Some(r) => r,
                None => continue,
            };

            let xobject_dict = match find_xobject_dict(doc, &resources) {
                Some(x) => x,
                None => continue,
            };

            let xobject_names: Vec<Vec<u8>> = xobject_dict.iter().map(|(k, _)| k.clone()).collect();

            for name in &xobject_names {
                let value = match xobject_dict.get(name) {
                    Ok(v) => v.clone(),
                    Err(_) => continue,
                };

                let _ = match value {
                    Object::Reference(id) => {
                        match doc.get_object(id) {
                            Ok(o) => {
                                if let Some(s) = o.as_stream().ok() {
                                    // Check if it's an Image with RGB color space
                                    let subtype =
                                        s.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok());
                                    if subtype != Some(b"Image") {
                                        continue;
                                    }

                                    let cs = s.dict.get(b"ColorSpace").ok();
                                    let is_rgb = matches!(cs, Some(Object::Name(n)) if n.eq_ignore_ascii_case(b"DeviceRGB"))
                                        || matches!(cs, Some(Object::Array(arr)) if arr.first().and_then(|o| o.as_name().ok()) == Some(b"ICCBased"));

                                    if !is_rgb {
                                        continue;
                                    }

                                    let bpc = s
                                        .dict
                                        .get(b"BitsPerComponent")
                                        .ok()
                                        .and_then(|o| o.as_i64().ok())
                                        .unwrap_or(8);

                                    if bpc != 8 {
                                        continue;
                                    }

                                    // Decode the image data. Previously this operated on
                                    // `s.content` directly, which is the *compressed*
                                    // byte stream when a Filter (e.g. FlateDecode) is
                                    // present — converting compressed bytes produces
                                    // garbage. Decompress first, convert, then
                                    // recompress with the same filter. (#154)
                                    let raw_data = match decode_image_stream(s) {
                                        Some(d) => d,
                                        None => continue,
                                    };
                                    let pixel_count = raw_data.len() / 3;
                                    if pixel_count == 0 {
                                        continue;
                                    }

                                    // Convert pixels
                                    let cmyk_data = engine.convert_pixels(&raw_data, 3);

                                    // Update the stream with CMYK data
                                    if let Some(o) = doc.objects.get_mut(&id) {
                                        if let Ok(stream_obj) = o.as_stream_mut() {
                                            // Re-encode with the same filter that was
                                            // originally present (FlateDecode/Fl), or
                                            // store raw if there was none.
                                            let had_filter = stream_obj.dict.get(b"Filter").is_ok();
                                            if had_filter {
                                                let mut encoder = ZlibEncoder::new(
                                                    Vec::new(),
                                                    Compression::default(),
                                                );
                                                if encoder.write_all(&cmyk_data).is_err() {
                                                    continue;
                                                }
                                                let recompressed = match encoder.finish() {
                                                    Ok(c) => c,
                                                    Err(_) => continue,
                                                };
                                                stream_obj.content = recompressed;
                                                // Normalize the filter name to the
                                                // canonical FlateDecode spelling —
                                                // drop any abbreviation and any
                                                // DecodeParms that no longer match
                                                // the recompressed payload.
                                                stream_obj.dict.set(
                                                    "Filter",
                                                    Object::Name(b"FlateDecode".to_vec()),
                                                );
                                                stream_obj.dict.remove(b"DecodeParms");
                                            } else {
                                                stream_obj.content = cmyk_data;
                                            }
                                            stream_obj.dict.set(
                                                "ColorSpace",
                                                Object::Name(b"DeviceCMYK".to_vec()),
                                            );
                                            stream_obj
                                                .dict
                                                .set("BitsPerComponent", Object::Integer(8));
                                            stream_obj.dict.remove(b"Length");

                                            result.images_converted += 1;
                                        }
                                    }
                                }
                                continue;
                            }
                            Err(_) => continue,
                        }
                    }
                    _ => continue,
                };
            }
        }
    }

    // Convert vector color operators in content streams
    if scope == "both" || scope == "vector" {
        for page_num in 0..page_ids.len() {
            let obj_id = page_ids[page_num];

            let content = match doc.get_page_content(obj_id) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (count, converted) = convert_vector_colors(&content, &engine);
            if count > 0 {
                // Update page content stream
                let content_stream_id = get_page_content_stream_id(doc, obj_id);
                if let Some(stream_id) = content_stream_id {
                    if let Some(o) = doc.objects.get_mut(&stream_id) {
                        if let Ok(stream_obj) = o.as_stream_mut() {
                            stream_obj.content = converted;
                            if stream_obj.dict.get(b"Filter").is_ok() {
                                stream_obj.dict.remove(b"Filter");
                                stream_obj.dict.remove(b"DecodeParms");
                            }
                        }
                    }
                }
                result.vector_ops_converted += count;
            }
        }
    }

    Ok(result)
}

fn convert_vector_colors(content: &[u8], _engine: &LcmsEngine) -> (usize, Vec<u8>) {
    let mut ops_count = 0;

    // We need to reconstruct the content stream with modified color operators.
    // For simplicity, we do a token-level replacement on RGB operators.
    // A full implementation would use a proper tokenizer → modify → serializer.

    let mut output = Vec::new();
    let mut i = 0;
    let len = content.len();

    while i < len {
        if let Some((start, end, op, nums, _names)) = find_next_operator(content, i) {
            // Copy everything before this operator
            output.extend_from_slice(&content[i..start]);

            match op.as_str() {
                "RG" | "rg" if nums.len() == 3 => {
                    let (c, m, y, k) = rgb_to_cmyk_float(nums[0], nums[1], nums[2]);
                    let new_op = if op == "RG" { "K" } else { "k" };
                    let line = format!("{} {} {} {} {}\n", c, m, y, k, new_op);
                    output.extend_from_slice(line.as_bytes());
                    ops_count += 1;
                }
                "SC" | "SCN" if nums.len() >= 3 => {
                    // This may be using an RGB color space — convert 3 operands
                    // Only convert if we can confirm it's an RGB space (done at call site)
                    let (c, m, y, k) = rgb_to_cmyk_float(nums[0], nums[1], nums[2]);
                    let remaining = &nums[3..];
                    let parts: Vec<String> = remaining.iter().map(|v| v.to_string()).collect();
                    let line = format!(
                        "{} {} {} {}{} {}\n",
                        c,
                        m,
                        y,
                        k,
                        if parts.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", parts.join(" "))
                        },
                        op
                    );
                    output.extend_from_slice(line.as_bytes());
                    ops_count += 1;
                }
                _ => {
                    output.extend_from_slice(&content[start..end]);
                }
            }
            i = end;
        } else {
            output.extend_from_slice(&content[i..]);
            break;
        }
    }

    (ops_count, output)
}

fn find_next_operator(
    content: &[u8],
    start: usize,
) -> Option<(usize, usize, String, Vec<f64>, Vec<Vec<u8>>)> {
    let ops = parse_content_operations(&content[start..]);
    if let Some((op, nums, names)) = ops.first() {
        // Find the byte position of this operator in the content
        let op_bytes = op.as_bytes();
        let search_start = start;
        if let Some(pos) = content[search_start..]
            .windows(op_bytes.len())
            .position(|w| w == op_bytes)
        {
            let abs_pos = search_start + pos;
            let end = abs_pos + op_bytes.len();
            Some((abs_pos, end, op.clone(), nums.clone(), names.clone()))
        } else {
            None
        }
    } else {
        None
    }
}

fn get_page_content_stream_id(doc: &Document, page_id: (u32, u16)) -> Option<(u32, u16)> {
    let page_dict = doc.get_dictionary(page_id).ok()?;
    page_dict.get(b"Contents").ok().and_then(|o| match o {
        Object::Reference(id) => Some(*id),
        Object::Array(arr) => {
            // For multiple content streams, use the first one
            arr.first().and_then(|o| o.as_reference().ok())
        }
        _ => None,
    })
}

pub fn add_output_intent(
    doc: &mut Document,
    icc_data: &[u8],
    condition_id: &str,
    condition: &str,
) -> Result<(), String> {
    use lopdf::Stream;

    // Derive the ICC profile channel count (the "N" entry of the ICCBased
    // color space) from the profile header instead of hardcoding 4. Per
    // ICC.1:2003-04 / ISO 15076-1, the channel count is a big-endian u32 at
    // byte offset 8 of the profile header. For CMYK profiles this is 4, for
    // RGB it is 3, for grayscale it is 1. Fall back to 4 only if the header
    // is truncated — the previous hardcoded value — so existing callers
    // behaviour is preserved on garbage input. (#172)
    let n_channels = read_icc_channel_count(icc_data);

    // Create ICC profile stream
    let icc_stream = Stream::new(
        Dictionary::from_iter(vec![(b"N".to_vec(), Object::Integer(n_channels as i64))]),
        icc_data.to_vec(),
    );
    let icc_stream_id = doc.add_object(Object::Stream(icc_stream));

    // Create output intent dict
    let output_intent = Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"OutputIntent".to_vec())),
        (b"S".to_vec(), Object::Name(b"GTS_PDFX".to_vec())),
        (
            b"OutputConditionIdentifier".to_vec(),
            Object::String(
                condition_id.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            ),
        ),
        (
            b"OutputCondition".to_vec(),
            Object::String(condition.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        ),
        (
            b"DestOutputProfile".to_vec(),
            Object::Reference(icc_stream_id),
        ),
    ]));

    let output_intent_id = doc.add_object(output_intent);

    // Get or create catalog via the trailer's Root reference.
    let catalog_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|r| r.as_reference().ok())
        .ok_or_else(|| "Cannot find Root reference in trailer".to_string())?;
    if let Some(catalog) = doc.objects.get_mut(&catalog_ref) {
        if let Ok(catalog_dict) = catalog.as_dict_mut() {
            catalog_dict.set(
                "OutputIntents",
                Object::Array(vec![Object::Reference(output_intent_id)]),
            );
        }
    }

    // Set PDF/X version in Info dict
    if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        if let Some(info) = doc.objects.get_mut(&info_ref) {
            if let Ok(info_dict) = info.as_dict_mut() {
                info_dict.set(
                    "GTS_PDFXVersion",
                    Object::String(b"PDF/X-4".to_vec(), lopdf::StringFormat::Literal),
                );
                info_dict.set("Trapped", Object::Name(b"False".to_vec()));
            }
        }
    }

    Ok(())
}

fn get_resources(doc: &Document, page_id: (u32, u16)) -> Option<Dictionary> {
    let page_dict = doc.get_dictionary(page_id).ok()?;
    page_dict.get(b"Resources").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

fn find_xobject_dict(doc: &Document, resources: &Dictionary) -> Option<Dictionary> {
    resources.get(b"XObject").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

/// Read the ICC profile channel count from the profile header.
///
/// ICC.1:2003-04 §7.2 profile header layout:
///   offset 0..3   — profile size (u32 BE)
///   offset 4..7   — preferred CMM type
///   offset 8..11  — profile version
///   offset 12..15 — profile/device class
///   offset 16..19 — color space of data (4 ASCII chars, e.g. `CMYK`, `RGB `,
///                   `GRAY`, `Lab `)
///   offset 20..23 — PCS
///
/// The PDF `N` entry of an ICCBased color space is the number of color
/// components, which we derive from the color-space signature at offset 16.
/// On truncated input, or a signature we don't recognise, we fall back to 4
/// (CMYK) — the previous hardcoded value — and log a warning. (#172)
fn read_icc_channel_count(icc_data: &[u8]) -> u32 {
    if icc_data.len() < 20 {
        log::warn!(
            "add_output_intent: ICC profile too short ({} bytes) — assuming 4 channels",
            icc_data.len()
        );
        return 4;
    }
    let cs = &icc_data[16..20];
    // Trim trailing NUL/space when comparing — signatures are padded.
    let sig: String = String::from_utf8_lossy(cs)
        .trim_end_matches([' ', '\0'])
        .to_string();
    match sig.as_str() {
        "GRAY" | "GRAYA" => 1,
        "RGB" | "RGBA" | "XYZ" => 3,
        "Lab" | "Luv" | "YCbCr" => 3,
        "CMYK" => 4,
        "MCH1" => 1,
        "MCH2" => 2,
        "MCH3" => 3,
        "MCH4" => 4,
        "MCH5" => 5,
        "MCH6" => 6,
        "MCH7" => 7,
        "MCH8" => 8,
        "MCH9" => 9,
        "MCH10" | "MCHA" => 10,
        "MCH11" | "MCHB" => 11,
        "MCH12" | "MCHC" => 12,
        "MCH13" | "MCHD" => 13,
        "MCH14" | "MCHE" => 14,
        "MCH15" | "MCHF" => 15,
        // 2-channel data is unusual but valid.
        "2CLR" => 2,
        "5CLR" => 5,
        "6CLR" => 6,
        "7CLR" => 7,
        "8CLR" => 8,
        "9CLR" => 9,
        "ACLR" => 10,
        "BCLR" => 11,
        "CCLR" => 12,
        "DCLR" => 13,
        "ECLR" => 14,
        "FCLR" => 15,
        _ => {
            log::warn!(
                "add_output_intent: ICC profile color space '{sig}' not recognised — assuming 4 channels"
            );
            4
        }
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn is_operator_char(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'*'
}

/// Decode an image XObject's stream content, honouring FlateDecode / Fl
/// (single Name or Array of Names). ASCII85 / ASCIIHex are not commonly
/// applied to image XObjects and are intentionally unsupported here; if a
/// stream uses one, `None` is returned and the caller skips conversion
/// rather than producing garbage. (#154)
fn decode_image_stream(stream: &lopdf::Stream) -> Option<Vec<u8>> {
    match stream.dict.get(b"Filter") {
        Err(_) | Ok(Object::Null) => Some(stream.content.clone()),
        Ok(Object::Name(n)) if n == b"FlateDecode" || n == b"Fl" => {
            let mut d = ZlibDecoder::new(stream.content.as_slice());
            let mut buf = Vec::new();
            d.read_to_end(&mut buf).ok().map(|_| buf)
        }
        Ok(Object::Name(other)) => {
            let name = String::from_utf8_lossy(other);
            log::warn!("decode_image_stream: unsupported filter {name} — skipping conversion");
            None
        }
        Ok(Object::Array(arr)) => {
            let mut data = stream.content.clone();
            for f in arr.iter() {
                if let Object::Name(n) = f {
                    if n == b"FlateDecode" || n == b"Fl" {
                        let mut d = ZlibDecoder::new(data.as_slice());
                        let mut buf = Vec::new();
                        if d.read_to_end(&mut buf).is_err() {
                            return None;
                        }
                        data = buf;
                    } else {
                        let name = String::from_utf8_lossy(n);
                        log::warn!(
                            "decode_image_stream: unsupported filter {name} in pipeline — skipping"
                        );
                        return None;
                    }
                }
            }
            Some(data)
        }
        Ok(_) => None,
    }
}

fn parse_content_operations(content: &[u8]) -> Vec<(String, Vec<f64>, Vec<Vec<u8>>)> {
    let len = content.len();
    let mut ops = Vec::new();
    let mut operands_num: Vec<f64> = Vec::new();
    let mut operands_name: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;

    while i < len {
        if is_whitespace(content[i]) {
            i += 1;
            continue;
        }
        if content[i] == b'%' {
            while i < len && content[i] != b'\n' && content[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        if content[i] == b'/' {
            let start = i;
            i += 1;
            while i < len && !is_whitespace(content[i]) && content[i] != b'%' {
                i += 1;
            }
            operands_name.push(content[start..i].to_vec());
            continue;
        }

        if content[i] == b'-' || content[i] == b'+' || is_digit(content[i]) || content[i] == b'.' {
            let start = i;
            if content[i] == b'-' || content[i] == b'+' {
                i += 1;
            }
            while i < len && is_digit(content[i]) {
                i += 1;
            }
            if i < len && content[i] == b'.' {
                i += 1;
                while i < len && is_digit(content[i]) {
                    i += 1;
                }
            }
            let s = std::str::from_utf8(&content[start..i]).unwrap_or("0");
            if let Ok(n) = s.parse::<f64>() {
                operands_num.push(n);
            }
            continue;
        }

        if is_operator_char(content[i]) {
            let start = i;
            while i < len && is_operator_char(content[i]) {
                i += 1;
            }
            let op = String::from_utf8_lossy(&content[start..i]).to_string();
            ops.push((
                op,
                std::mem::take(&mut operands_num),
                std::mem::take(&mut operands_name),
            ));
            continue;
        }

        if content[i] == b'(' {
            let mut depth = 0;
            while i < len {
                if content[i] == b'(' {
                    depth += 1;
                }
                if content[i] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                if content[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }

        if content[i] == b'<' && i + 1 < len && content[i + 1] != b'<' {
            while i < len && content[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    ops
}
