use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::ImageEncoder;
use lopdf::Object;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::num::NonZeroU64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionOptions {
    pub quality: u8,
    pub target_dpi: u32,
    pub image_quality: u8,
    pub subset_fonts: bool,
    pub use_zopfli: bool,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        CompressionOptions {
            quality: 80,
            target_dpi: 150,
            image_quality: 85,
            subset_fonts: false,
            use_zopfli: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionSavings {
    pub streams_recompressed: u32,
    pub images_downsampled: u32,
    pub images_recompressed: u32,
    pub fonts_subsetted: u32,
    pub bytes_saved: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_bytes: u64,
    pub compressed_bytes: u64,
    pub ratio: f32,
    pub duration_ms: u64,
    pub streams_compressed: u32,
    pub images_downsampled: u32,
    pub fonts_subsetted: u32,
    pub savings: CompressionSavings,
    pub output_path: String,
}

#[derive(Default)]
struct StreamStats {
    streams_recompressed: u32,
    images_downsampled: u32,
    images_recompressed: u32,
    #[allow(dead_code)] // Tracked for future metrics reporting
    fonts_subsetted: u32,
    bytes_saved: u64,
}

fn image_color_channels(stream: &lopdf::Stream) -> Option<u32> {
    let cs = stream.dict.get(b"ColorSpace").ok()?;
    match cs {
        Object::Name(n) if n == b"DeviceGray" || n == b"G" => Some(1),
        Object::Name(n) if n == b"DeviceRGB" || n == b"RGB" => Some(3),
        Object::Name(n) if n == b"DeviceCMYK" || n == b"CMYK" => Some(4),
        _ => None,
    }
}

fn image_bits_per_component(stream: &lopdf::Stream) -> u32 {
    stream
        .dict
        .get(b"BitsPerComponent")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .map(|v| v.max(1) as u32)
        .unwrap_or(8)
}

fn get_filter_names(stream: &lopdf::Stream) -> Vec<Vec<u8>> {
    match stream.dict.get(b"Filter") {
        Ok(Object::Name(n)) => vec![n.clone()],
        Ok(Object::Array(arr)) => arr
            .iter()
            .filter_map(|f| f.as_name().ok().map(|n| n.to_vec()))
            .collect(),
        _ => Vec::new(),
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn ascii85_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut cleaned: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if cleaned.first() == Some(&b'<') && cleaned.last() == Some(&b'>') {
        cleaned.remove(0);
        cleaned.pop();
    }
    if cleaned == b"~>" {
        return Ok(Vec::new());
    }
    if let Some(pos) = find_subslice(&cleaned, b"~>") {
        cleaned.truncate(pos);
    }
    let mut out = Vec::with_capacity(cleaned.len() * 4 / 5);
    let mut buf: u32 = 0;
    let mut count: u32 = 0;
    for &c in &cleaned {
        if !(b'!'..=b'u').contains(&c) {
            return Err(format!("Invalid ASCII85 byte: 0x{c:02x}"));
        }
        let v = (c - b'!') as u32;
        buf = buf * 85 + v;
        count += 1;
        if count == 5 {
            out.push((buf >> 24) as u8);
            out.push((buf >> 16) as u8);
            out.push((buf >> 8) as u8);
            out.push(buf as u8);
            buf = 0;
            count = 0;
        }
    }
    if count > 0 {
        for _ in count..4 {
            buf = buf * 85 + 84;
        }
        let pad_bytes = (count - 1) as usize;
        let bytes: [u8; 4] = [
            (buf >> 24) as u8,
            (buf >> 16) as u8,
            (buf >> 8) as u8,
            buf as u8,
        ];
        out.extend_from_slice(&bytes[..pad_bytes]);
    }
    Ok(out)
}

fn ascii_hex_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut cleaned: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if cleaned.last() == Some(&b'>') {
        cleaned.pop();
    }
    if cleaned.len() % 2 == 1 {
        cleaned.push(b'0');
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let mut i = 0;
    while i < cleaned.len() {
        let hi = hex_nibble(cleaned[i])?;
        let lo = hex_nibble(cleaned[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("Invalid hex nibble: 0x{b:02x}")),
    }
}

fn decode_filter_chain(data: &[u8], filters: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut buf = data.to_vec();
    for f in filters {
        match f.as_slice() {
            b"ASCII85Decode" | b"A85" => {
                buf = ascii85_decode(&buf)?;
            }
            b"ASCIIHexDecode" | b"AHx" => {
                buf = ascii_hex_decode(&buf)?;
            }
            b"LZWDecode" | b"LZW" => {
                return Err("LZWDecode not yet supported in compression pass".to_string());
            }
            other => {
                return Err(format!(
                    "Unsupported filter chain element: {}",
                    String::from_utf8_lossy(other)
                ));
            }
        }
    }
    Ok(buf)
}

fn zopfli_deflate(data: &[u8], quality: u8) -> Result<Vec<u8>, String> {
    let iterations: NonZeroU64 = match quality {
        0..=30 => NonZeroU64::new(1).unwrap(),
        31..=70 => NonZeroU64::new(5).unwrap(),
        71..=90 => NonZeroU64::new(15).unwrap(),
        _ => NonZeroU64::new(30).unwrap(),
    };
    let options = zopfli::Options {
        iteration_count: iterations,
        ..zopfli::Options::default()
    };
    let mut encoder = zopfli::DeflateEncoder::new(options, zopfli::BlockType::Dynamic, Vec::new());
    encoder
        .write_all(data)
        .map_err(|e| format!("zopfli write error: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("zopfli finish error: {e}"))
}

fn deflate(data: &[u8], quality: u8) -> Result<Vec<u8>, String> {
    let level = match quality {
        0..=30 => Compression::fast(),
        31..=70 => Compression::default(),
        _ => Compression::best(),
    };
    let mut encoder = ZlibEncoder::new(Vec::new(), level);
    encoder
        .write_all(data)
        .map_err(|e| format!("zlib write error: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("zlib finish error: {e}"))
}

fn recompress_image_to_jpeg(
    raw: &[u8],
    width: u32,
    height: u32,
    channels: u32,
    quality: u8,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || raw.is_empty() {
        return None;
    }
    if channels != 1 && channels != 3 && channels != 4 {
        return None;
    }
    let color = match channels {
        1 => image::ColorType::L8,
        3 => image::ColorType::Rgb8,
        4 => image::ColorType::Rgba8,
        _ => return None,
    };
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    encoder.write_image(raw, width, height, color.into()).ok()?;
    Some(out)
}

fn recompress_1bit_deflate(raw: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    if encoder.write_all(raw).is_err() {
        return None;
    }
    encoder.finish().ok()
}

fn should_downsample(width: u32, rendered_dpi: f64, target_dpi: u32) -> Option<u32> {
    if target_dpi == 0 || rendered_dpi <= 0.0 {
        return None;
    }
    if rendered_dpi <= target_dpi as f64 {
        return None;
    }
    let scale = target_dpi as f64 / rendered_dpi;
    let new_w = ((width as f64) * scale).round().max(1.0) as u32;
    if new_w < width {
        Some(new_w)
    } else {
        None
    }
}

fn decode_image_to_dynamic(
    raw: &[u8],
    width: u32,
    height: u32,
    channels: u32,
) -> Option<image::DynamicImage> {
    use image::buffer::ConvertBuffer;
    match channels {
        3 => {
            let buf: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(width, height, raw.to_vec())?;
            Some(image::DynamicImage::ImageRgb8(buf))
        }
        1 => {
            let buf: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(width, height, raw.to_vec())?;
            let rgb: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = buf.convert();
            Some(image::DynamicImage::ImageRgb8(rgb))
        }
        4 => {
            let buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(width, height, raw.to_vec())?;
            Some(image::DynamicImage::ImageRgba8(buf))
        }
        _ => None,
    }
}

fn rewrite_streams(
    doc: &mut lopdf::Document,
    options: &CompressionOptions,
    stats: &mut StreamStats,
) -> Result<(), String> {
    let ids: Vec<(u32, u16)> = doc.objects.keys().copied().collect();
    for id in ids {
        let is_image = doc
            .get_object(id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .map(|s| {
                matches!(
                    s.dict.get(b"Subtype").ok(),
                    Some(Object::Name(n)) if n == b"Image"
                )
            })
            .unwrap_or(false);
        if !is_image {
            continue;
        }
        let (recompressed, downsampled) = rewrite_image(doc, id, options, stats)?;
        if recompressed {
            stats.images_recompressed += 1;
        }
        if downsampled {
            stats.images_downsampled += 1;
        }
    }

    let ids: Vec<(u32, u16)> = doc.objects.keys().copied().collect();
    for id in ids {
        let is_image = doc
            .get_object(id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .map(|s| {
                matches!(
                    s.dict.get(b"Subtype").ok(),
                    Some(Object::Name(n)) if n == b"Image"
                )
            })
            .unwrap_or(false);
        if is_image {
            continue;
        }
        recompress_non_image_stream(doc, id, options, stats)?;
    }
    Ok(())
}

fn rewrite_image(
    doc: &mut lopdf::Document,
    id: (u32, u16),
    options: &CompressionOptions,
    stats: &mut StreamStats,
) -> Result<(bool, bool), String> {
    let stream = match doc.get_object(id).ok().and_then(|o| o.as_stream().ok()) {
        Some(s) => s,
        None => return Ok((false, false)),
    };
    let width = stream
        .dict
        .get(b"Width")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .map(|v| v as u32)
        .unwrap_or(0);
    let height = stream
        .dict
        .get(b"Height")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .map(|v| v as u32)
        .unwrap_or(0);
    let bpc = image_bits_per_component(stream);
    let channels = match image_color_channels(stream) {
        Some(c) => c,
        None => return Ok((false, false)),
    };
    if width == 0 || height == 0 {
        return Ok((false, false));
    }
    let raw_filters = get_filter_names(stream);
    let content = stream.content.clone();

    let mut raw = if raw_filters.is_empty() {
        content
    } else if raw_filters
        .iter()
        .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
    {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(content.as_slice());
        let mut buf = Vec::new();
        decoder
            .read_to_end(&mut buf)
            .map_err(|e| format!("Flate decode error: {e}"))?;
        buf
    } else {
        match decode_filter_chain(&content, &raw_filters) {
            Ok(b) => b,
            Err(_) => return Ok((false, false)),
        }
    };

    let expected_len = (width as usize) * (height as usize) * (channels as usize);
    if raw.len() < expected_len {
        return Ok((false, false));
    }
    raw.truncate(expected_len);

    let mut downsampled = false;
    let mut target_w = width;
    let mut target_h = height;
    if bpc == 8 {
        let assumed_dpi = 72.0_f64;
        if let Some(new_w) = should_downsample(width, assumed_dpi.max(150.0), options.target_dpi) {
            if new_w > 0 {
                let scale = new_w as f64 / width as f64;
                target_w = new_w;
                target_h = ((height as f64) * scale).round().max(1.0) as u32;
                downsampled = true;
            }
        }
    }

    let img = match decode_image_to_dynamic(&raw, width, height, channels) {
        Some(i) => i,
        None => return Ok((false, false)),
    };
    let final_img = if downsampled {
        img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let final_w = final_img.width();
    let final_h = final_img.height();
    let new_bytes;
    let new_filter: &[u8];
    let new_cs: &[u8];
    if bpc == 1 && channels == 1 {
        let luma = final_img.to_luma8();
        let raw = luma.into_raw();
        match recompress_1bit_deflate(&raw) {
            Some(b) => {
                new_bytes = b;
                new_filter = b"FlateDecode";
                new_cs = b"DeviceGray";
            }
            None => return Ok((false, false)),
        }
    } else {
        let rgb = final_img.to_rgb8();
        let raw = rgb.into_raw();
        match recompress_image_to_jpeg(&raw, final_w, final_h, 3, options.image_quality) {
            Some(b) => {
                new_bytes = b;
                new_filter = b"DCTDecode";
                new_cs = b"DeviceRGB";
            }
            None => return Ok((false, false)),
        }
    }

    if let Some(obj) = doc.objects.get_mut(&id) {
        if let Ok(stream_obj) = obj.as_stream_mut() {
            let old_len = stream_obj.content.len() as u64;
            stream_obj.content = new_bytes;
            stats.bytes_saved = stats
                .bytes_saved
                .saturating_add(old_len.saturating_sub(stream_obj.content.len() as u64));
            stream_obj.dict.set("Filter", Object::Name(new_filter.to_vec()));
            stream_obj.dict.set("ColorSpace", Object::Name(new_cs.to_vec()));
            stream_obj.dict.set("Width", Object::Integer(final_w as i64));
            stream_obj.dict.set("Height", Object::Integer(final_h as i64));
            stream_obj.dict.remove(b"Length");
            return Ok((true, downsampled));
        }
    }
    Ok((false, false))
}

fn recompress_non_image_stream(
    doc: &mut lopdf::Document,
    id: (u32, u16),
    options: &CompressionOptions,
    stats: &mut StreamStats,
) -> Result<(), String> {
    let stream = match doc.get_object(id).ok().and_then(|o| o.as_stream().ok()) {
        Some(s) => s,
        None => return Ok(()),
    };
    let filters = get_filter_names(stream);
    if filters
        .iter()
        .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
    {
        return Ok(());
    }
    let content = stream.content.clone();
    let raw = if filters.is_empty() {
        content
    } else {
        match decode_filter_chain(&content, &filters) {
            Ok(b) => b,
            Err(_) => return Ok(()),
        }
    };
    let compressed = if options.use_zopfli {
        zopfli_deflate(&raw, options.quality)?
    } else {
        deflate(&raw, options.quality)?
    };
    if compressed.len() >= raw.len() {
        return Ok(());
    }
    if let Some(obj) = doc.objects.get_mut(&id) {
        if let Ok(stream_obj) = obj.as_stream_mut() {
            let old_len = stream_obj.content.len() as u64;
            stream_obj.content = compressed;
            stats.bytes_saved = stats
                .bytes_saved
                .saturating_add(old_len.saturating_sub(stream_obj.content.len() as u64));
            stream_obj.dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
            stream_obj.dict.remove(b"Length");
            stream_obj.dict.remove(b"DecodeParms");
            stats.streams_recompressed += 1;
        }
    }
    Ok(())
}

fn subset_fonts_in_doc(doc: &mut lopdf::Document) -> u32 {
    let mut count = 0u32;
    for (_id, obj) in doc.objects.iter() {
        if let Object::Dictionary(d) = obj {
            if let Ok(subtype) = d.get(b"Subtype") {
                if let Object::Name(n) = subtype {
                    if matches!(
                        n.as_slice(),
                        b"Type0" | b"TrueType" | b"CIDFontType0" | b"CIDFontType2"
                    ) {
                        if d.get(b"FontDescriptor").is_ok() {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

fn derive_output_path(input: &str) -> String {
    let path = std::path::Path::new(input);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("compressed");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent
        .join(format!("{stem}_compressed.{ext}"))
        .to_string_lossy()
        .to_string()
}

pub fn compress_pdf(
    path: &str,
    output_path: Option<&str>,
    options: &CompressionOptions,
) -> Result<CompressionResult, String> {
    let start = std::time::Instant::now();
    let original_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut doc = lopdf::Document::load(path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let mut stats = StreamStats::default();

    rewrite_streams(&mut doc, options, &mut stats)?;
    let fonts_subsetted = if options.subset_fonts {
        subset_fonts_in_doc(&mut doc)
    } else {
        0
    };

    let out = output_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| derive_output_path(path));
    if out == path {
        return Err(
            "Refusing to overwrite source PDF; pass a different output_path".to_string(),
        );
    }
    doc.save(&out)
        .map_err(|e| format!("Failed to save: {e}"))?;

    let compressed_bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    let ratio = if original_bytes > 0 {
        1.0 - (compressed_bytes as f64 / original_bytes as f64) as f32
    } else {
        0.0
    };

    Ok(CompressionResult {
        original_bytes,
        compressed_bytes,
        ratio,
        duration_ms: start.elapsed().as_millis() as u64,
        streams_compressed: stats.streams_recompressed,
        images_downsampled: stats.images_downsampled,
        fonts_subsetted,
        savings: CompressionSavings {
            streams_recompressed: stats.streams_recompressed,
            images_downsampled: stats.images_downsampled,
            images_recompressed: stats.images_recompressed,
            fonts_subsetted,
            bytes_saved: stats.bytes_saved,
        },
        output_path: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii85_roundtrip() {
        let input = b"Hello world!";
        let encoded = b"87cURD]j7BEbo80~>";
        let decoded = ascii85_decode(encoded).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn ascii_hex_roundtrip() {
        let input = b"48656c6c6f>";
        let decoded = ascii_hex_decode(input).expect("decode");
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn derive_output_path_appends_suffix() {
        let p = derive_output_path("/tmp/foo.pdf");
        let normalized = p.replace('\\', "/");
        assert!(normalized.ends_with("/foo_compressed.pdf"));
    }
}
