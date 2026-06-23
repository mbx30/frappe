use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub severity: &'static str,
    pub params_schema: &'static str,
}

pub const CHECK_REGISTRY: &[CheckDefinition] = &[
    CheckDefinition {
        id: "page_boxes",
        name: "Page boxes",
        category: "page",
        severity: "error",
        params_schema: "{}",
    },
    CheckDefinition {
        id: "fonts",
        name: "Fonts embedded",
        category: "font",
        severity: "error",
        params_schema: "{}",
    },
    CheckDefinition {
        id: "image_resolution",
        name: "Image resolution",
        category: "image",
        severity: "warning",
        params_schema: "{\"min_dpi\": number}",
    },
    CheckDefinition {
        id: "bleed",
        name: "Bleed",
        category: "page",
        severity: "error",
        params_schema: "{\"amount_mm\": number}",
    },
    CheckDefinition {
        id: "color_spaces",
        name: "Color spaces",
        category: "color",
        severity: "error",
        params_schema: "{\"target_profile\": string}",
    },
    CheckDefinition {
        id: "overprint",
        name: "Overprint",
        category: "color",
        severity: "warning",
        params_schema: "{}",
    },
    CheckDefinition {
        id: "transparency",
        name: "Transparency",
        category: "color",
        severity: "warning",
        params_schema: "{}",
    },
    CheckDefinition {
        id: "spot_colors",
        name: "Spot colors",
        category: "color",
        severity: "info",
        params_schema: "{}",
    },
    CheckDefinition {
        id: "ink_coverage",
        name: "Ink coverage (TAC)",
        category: "color",
        severity: "warning",
        params_schema: "{\"threshold\": number}",
    },
    CheckDefinition {
        id: "pdfx",
        name: "PDF/X compliance",
        category: "compliance",
        severity: "error",
        params_schema: "{\"standard\": string}",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunProfileResult {
    pub profile_name: String,
    pub findings_count: usize,
}
