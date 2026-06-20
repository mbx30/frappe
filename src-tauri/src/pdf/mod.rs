use pdfium_render::prelude::*;
use std::fs;
use crate::models::PdfSummary;

pub struct PdfEngine {
    pdfium: Pdfium,
}

impl PdfEngine {
    pub fn init() -> Result<Self, String> {
        let pdfium = Pdfium::new(
            Pdfium::bind_to_system_library()
                .map_err(|e| format!("Failed to bind PDFium library: {}", e))?,
        );
        Ok(PdfEngine { pdfium })
    }

    pub fn load_pdf(&self, path: &str) -> Result<PdfDocument, String> {
        self.pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| format!("Failed to load PDF: {}", e))
    }
}

pub fn open_pdf(path: &str) -> Result<PdfSummary, String> {
    let pdfium = Pdfium::new(
        Pdfium::bind_to_system_library()
            .map_err(|e| format!("Failed to bind PDFium: {}", e))?,
    );

    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    let page_count = document.pages().len() as usize;
    let file_size_bytes = fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.pdf")
        .to_string();

    Ok(PdfSummary {
        id: 0,
        file_path: path.to_string(),
        file_name,
        page_count,
        pdf_version: "1.4".to_string(), // TODO: extract actual version from PDF header
        file_size_bytes,
        title: String::new(),
        creator: String::new(),
        producer: String::new(),
        creation_date: String::new(),
        is_encrypted: false, // TODO: check if PDF has security
    })
}
