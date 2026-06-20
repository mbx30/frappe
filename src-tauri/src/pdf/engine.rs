use pdfium_render::prelude::*;
use std::path::PathBuf;

pub struct PdfEngine {
    pdfium: Pdfium,
}

impl PdfEngine {
    pub fn init() -> Result<Self, String> {
        let bindings = Self::load_bindings()?;
        let pdfium = Pdfium::new(bindings);
        Ok(PdfEngine { pdfium })
    }

    fn load_bindings() -> Result<Box<dyn PdfiumLibraryBindings>, String> {
        let resource_path = PdfEngine::bundled_path();
        if let Some(path) = &resource_path {
            if path.exists() {
                if let Ok(bindings) = Pdfium::bind_to_library(path) {
                    return Ok(bindings);
                }
            }
        }
        Pdfium::bind_to_system_library()
            .map_err(|e| format!("Failed to load PDFium: {}", e))
    }

    fn bundled_path() -> Option<PathBuf> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        #[cfg(target_os = "windows")]
        let path = dir.join("pdfium.dll");
        #[cfg(target_os = "macos")]
        let path = dir.join("libpdfium.dylib");
        #[cfg(target_os = "linux")]
        let path = dir.join("libpdfium.so");
        Some(path)
    }

    pub fn open_document(&self, path: &str) -> Result<PdfDocument<'_>, String> {
        self.pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| format!("Failed to open PDF: {}", e))
    }
}
