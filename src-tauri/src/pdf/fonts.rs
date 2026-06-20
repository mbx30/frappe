use lopdf::Document;

pub struct FontFinding {
    pub font_name: String,
    pub font_type: String,
    pub is_embedded: bool,
    pub is_subsetted: bool,
    pub pages: Vec<usize>,
    pub severity: String,
    pub message: String,
}

pub fn collect_fonts(_doc: &Document) -> Vec<FontFinding> {
    Vec::new()
}
