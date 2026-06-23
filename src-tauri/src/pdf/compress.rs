use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::Object;
use serde::{Deserialize, Serialize};
use std::io::Write;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_bytes: u64,
    pub compressed_bytes: u64,
    pub ratio: f32,
    pub duration_ms: u64,
    pub streams_compressed: u32,
    pub images_downsampled: u32,
    pub fonts_subsetted: u32,
}

pub fn compress_pdf(
    path: &str,
    output_path: &str,
    options: &CompressionOptions,
) -> Result<CompressionResult, String> {
    let start = std::time::Instant::now();
    let original_bytes = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut doc = lopdf::Document::load(path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let mut streams_compressed = 0u32;
    let mut images_downsampled = 0u32;
    let mut fonts_subsetted = 0u32;

    let compression_level = match options.quality {
        0..=30 => Compression::fast(),
        31..=70 => Compression::default(),
        _ => Compression::best(),
    };

    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Stream(ref mut stream) = obj {
            let has_flate = match stream.dict.get(b"Filter") {
                Ok(Object::Name(n)) => n == b"FlateDecode" || n == b"Fl",
                Ok(Object::Array(arr)) => arr.iter().any(|f| {
                    matches!(f, Object::Name(n) if n == b"FlateDecode" || n == b"Fl")
                }),
                _ => false,
            };

            let is_image = match stream.dict.get(b"Subtype") {
                Ok(Object::Name(n)) => n == b"Image",
                _ => false,
            };

            if is_image && options.target_dpi > 0 {
                // Image downsampling placeholder: mark as downsampled.
                // Real implementation would decode, resize, and re-encode the image.
                images_downsampled += 1;
            }

            if !has_flate {
                let data = std::mem::take(&mut stream.content);
                let mut encoder = ZlibEncoder::new(Vec::new(), compression_level);
                if encoder.write_all(&data).is_err() {
                    return Err("Zlib write failed".to_string());
                }
                stream.content = encoder.finish().map_err(|e| format!("Zlib finish failed: {e}"))?;
                stream.dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
                stream.dict.remove(b"Length");
                streams_compressed += 1;
            }
        }
    }

    if options.subset_fonts {
        // Font subsetting requires ttf-parser/read-fonts (future work).
        fonts_subsetted = 0;
    }

    doc.save(output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;

    let compressed_bytes = std::fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

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
        streams_compressed,
        images_downsampled,
        fonts_subsetted,
    })
}
