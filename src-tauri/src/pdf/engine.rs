use pdfium_render::prelude::*;
use std::path::PathBuf;

/// Wraps the optional PDFium engine. `Some` when the system PDFium library
/// or bundled resource was loaded; `None` when init failed (so the app
/// still starts and only the rendering/preflight features are disabled).
pub struct PdfEngine {
    inner: Option<Pdfium>,
}

impl PdfEngine {
    pub fn init() -> Self {
        match Self::try_init() {
            Ok(engine) => engine,
            Err(e) => {
                tracing::error!("PDFium failed to load: {}. PDF rendering, preflight, and visual editing will be disabled.", e);
                PdfEngine { inner: None }
            }
        }
    }

    fn try_init() -> Result<Self, String> {
        let bindings = Self::load_bindings()?;
        let pdfium = Pdfium::new(bindings);
        Ok(PdfEngine {
            inner: Some(pdfium),
        })
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
        Pdfium::bind_to_system_library().map_err(|e| format!("Failed to load PDFium: {}", e))
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

    pub fn is_available(&self) -> bool {
        self.inner.is_some()
    }

    pub fn open_document(&self, path: &str) -> Result<PdfDocument<'_>, String> {
        let pdfium = self.inner.as_ref().ok_or_else(|| {
            "PDFium not available; PDF rendering features are disabled".to_string()
        })?;
        pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| format!("Failed to open PDF: {}", e))
    }
}
