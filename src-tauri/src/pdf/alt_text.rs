//! PDF alt-text injection engine (#361-#362).
//!
//! Injects /Alt entries into Image XObject dictionaries and creates Structure Tree
//! Figure elements for PDF/UA compliance. Supports UTF-16BE encoding for non-ASCII
//! text and decorative image marking via /Artifact entries.
//!
//! Workflow:
//! 1. inject_alt_text() — adds or updates /Alt in XObject dictionary
//! 2. create_figure_element() — creates Structure Tree Figure element
//! 3. mark_as_decorative() — adds /Artifact marking for decorative images

use lopdf::{Dictionary, Document, Object};
use std::collections::BTreeMap;

/// Error type for alt-text operations.
#[derive(Debug)]
pub enum AltTextError {
    NotFound(String),
    InvalidObject(String),
    StructTreeMissing,
    EncodingError(String),
}

impl std::fmt::Display for AltTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AltTextError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AltTextError::InvalidObject(msg) => write!(f, "Invalid object: {}", msg),
            AltTextError::StructTreeMissing => write!(f, "Structure tree not found"),
            AltTextError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for AltTextError {}

type Result<T> = std::result::Result<T, AltTextError>;

/// Encode a string to UTF-16BE with BOM for PDF /Alt entries.
///
/// PDF /Alt entries support UTF-16BE encoding for non-ASCII text.
/// This function encodes the string and prepends the BOM (0xFEFF).
fn encode_alt_text_utf16be(text: &str) -> Result<Vec<u8>> {
    let mut result = vec![0xFE, 0xFF]; // UTF-16BE BOM
    for ch in text.chars() {
        let code = ch as u32;
        if code <= 0xFFFF {
            // BMP character
            result.push(((code >> 8) & 0xFF) as u8);
            result.push((code & 0xFF) as u8);
        } else {
            // Surrogate pair for characters outside BMP
            let code = code - 0x10000;
            let high = 0xD800 + ((code >> 10) & 0x3FF);
            let low = 0xDC00 + (code & 0x3FF);
            result.push(((high >> 8) & 0xFF) as u8);
            result.push((high & 0xFF) as u8);
            result.push(((low >> 8) & 0xFF) as u8);
            result.push((low & 0xFF) as u8);
        }
    }
    Ok(result)
}

/// Check if text contains only ASCII characters.
fn is_ascii_only(text: &str) -> bool {
    text.chars().all(|c| (c as u32) < 128)
}

/// Inject alt-text into an Image XObject dictionary.
///
/// This function:
/// 1. Locates the Image XObject by object ID
/// 2. Adds or updates the /Alt entry
/// 3. Uses UTF-16BE encoding for non-ASCII text
/// 4. Optionally marks the image as decorative via /Artifact
///
/// # Arguments
/// * `doc` — The PDF document
/// * `xobject_id` — The object ID of the Image XObject
/// * `alt_text` — The alt-text string (can be empty for decorative images)
/// * `is_decorative` — If true, adds /Artifact marking; /Alt can be empty
///
/// # Returns
/// Result containing the modified document or an error
pub fn inject_alt_text(
    doc: &mut Document,
    xobject_id: (u32, u16),
    alt_text: &str,
    is_decorative: bool,
) -> Result<()> {
    // Retrieve the Image XObject dictionary
    let obj = doc
        .get_object_mut(xobject_id)
        .map_err(|_| AltTextError::NotFound(format!("XObject ID {:?}", xobject_id)))?;

    let dict = obj
        .as_dict_mut()
        .ok_or_else(|| AltTextError::InvalidObject("XObject is not a dictionary".to_string()))?;

    // Verify it's an Image
    if dict.get(b"Subtype") != Ok(&Object::Name(b"Image".to_vec())) {
        return Err(AltTextError::InvalidObject(
            "XObject is not an Image subtype".to_string(),
        ));
    }

    // Add or update /Alt entry (as a text string, not a name)
    if !alt_text.is_empty() {
        let alt_bytes = if is_ascii_only(alt_text) {
            // ASCII text can be stored as a literal string
            alt_text.as_bytes().to_vec()
        } else {
            // Non-ASCII requires UTF-16BE encoding
            encode_alt_text_utf16be(alt_text)?
        };
        dict.set("Alt", Object::String(alt_bytes, lopdf::StringType::Literal));
    }

    // Mark as decorative if requested
    if is_decorative {
        // Create /Artifact dictionary with /Subtype /Background
        let mut artifact_dict = Dictionary::new();
        artifact_dict.set("Subtype", Object::Name(b"Background".to_vec()));
        dict.set("Artifact", Object::Dictionary(artifact_dict));
    }

    Ok(())
}

/// Create a Figure element in the PDF Structure Tree.
///
/// Creates a new StructElem with /S /Figure and optionally a nested Text element
/// for the full description. Links to the Image XObject via page and object references.
///
/// # Arguments
/// * `doc` — The PDF document
/// * `page_id` — The page object ID (for /Pg reference)
/// * `xobject_id` — The Image XObject ID (for /K reference)
/// * `alt_text` — The alt-text (stored in /Alt entry of StructElem)
/// * `description` — Optional long description (stored as nested Text element)
///
/// # Returns
/// Result containing the object ID of the newly created Figure element
pub fn create_figure_element(
    doc: &mut Document,
    page_id: (u32, u16),
    xobject_id: (u32, u16),
    alt_text: &str,
    description: Option<&str>,
) -> Result<(u32, u16)> {
    // Create the Figure StructElem
    let mut figure_dict = Dictionary::new();
    figure_dict.set("Type", Object::Name(b"StructElem".to_vec()));
    figure_dict.set("S", Object::Name(b"Figure".to_vec()));
    figure_dict.set("Pg", Object::Reference(page_id));

    // Set /Alt entry with alt-text
    if !alt_text.is_empty() {
        let alt_bytes = if is_ascii_only(alt_text) {
            alt_text.as_bytes().to_vec()
        } else {
            encode_alt_text_utf16be(alt_text)?
        };
        figure_dict.set("Alt", Object::String(alt_bytes, lopdf::StringType::Literal));
    }

    // Create /K array: reference to the Image XObject
    let mut kids = vec![Object::Reference(xobject_id)];

    // If description provided, create a nested Text element
    if let Some(desc) = description {
        if !desc.is_empty() {
            let mut text_dict = Dictionary::new();
            text_dict.set("Type", Object::Name(b"StructElem".to_vec()));
            text_dict.set("S", Object::Name(b"Text".to_vec()));
            text_dict.set("Pg", Object::Reference(page_id));

            let desc_bytes = if is_ascii_only(desc) {
                desc.as_bytes().to_vec()
            } else {
                encode_alt_text_utf16be(desc)?
            };
            text_dict.set("Txt", Object::String(desc_bytes, lopdf::StringType::Literal));

            // Add text element to document and append reference to kids
            let text_id = doc.add_object(text_dict);
            kids.push(Object::Reference(text_id));
        }
    }

    figure_dict.set("K", Object::Array(kids));

    // Add Figure element to document and return its ID
    let figure_id = doc.add_object(figure_dict);
    Ok(figure_id)
}

/// Mark an Image XObject as decorative (artifact).
///
/// Sets the /Artifact flag on an Image XObject with /Subtype /Background,
/// indicating that the image should be skipped by screen readers.
///
/// # Arguments
/// * `doc` — The PDF document
/// * `xobject_id` — The object ID of the Image XObject
///
/// # Returns
/// Result indicating success or error
pub fn mark_as_decorative(doc: &mut Document, xobject_id: (u32, u16)) -> Result<()> {
    inject_alt_text(doc, xobject_id, "", true)
}

/// Retrieve an Image XObject from the document.
///
/// Helper function to locate and verify an Image XObject exists and is valid.
pub fn get_image_xobject(
    doc: &Document,
    xobject_id: (u32, u16),
) -> Result<&Dictionary> {
    let obj = doc
        .get_object(xobject_id)
        .map_err(|_| AltTextError::NotFound(format!("XObject ID {:?}", xobject_id)))?;

    let dict = obj
        .as_dict()
        .ok_or_else(|| AltTextError::InvalidObject("XObject is not a dictionary".to_string()))?;

    // Verify it's an Image
    if dict.get(b"Subtype") != Ok(&Object::Name(b"Image".to_vec())) {
        return Err(AltTextError::InvalidObject(
            "XObject is not an Image subtype".to_string(),
        ));
    }

    Ok(dict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf16be_encoding_ascii() {
        let text = "Hello";
        let encoded = encode_alt_text_utf16be(text).unwrap();
        // Should have BOM (2 bytes) + 5 chars * 2 bytes each = 12 bytes
        assert_eq!(encoded.len(), 12);
        assert_eq!(&encoded[0..2], &[0xFE, 0xFF]); // BOM
    }

    #[test]
    fn test_utf16be_encoding_unicode() {
        let text = "café";
        let encoded = encode_alt_text_utf16be(text).unwrap();
        // BOM (2) + 'c' (2) + 'a' (2) + 'f' (2) + 'é' (2) = 10 bytes
        assert_eq!(encoded.len(), 10);
        assert_eq!(&encoded[0..2], &[0xFE, 0xFF]); // BOM
    }

    #[test]
    fn test_ascii_only_detection() {
        assert!(is_ascii_only("Hello"));
        assert!(!is_ascii_only("café"));
        assert!(!is_ascii_only("你好"));
    }
}
