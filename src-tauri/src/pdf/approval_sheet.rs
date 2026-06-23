use printpdf::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSheetInfo {
    pub client_name: String,
    pub job_number: String,
    pub due_date: String,
    pub description: String,
    pub staff_name: String,
    pub include_preflight_summary: bool,
}

pub fn generate_approval_sheet(info: &ApprovalSheetInfo, output_path: &str) -> Result<(), String> {
    let (doc, page1, layer1) = PdfDocument::new("Approval Sheet", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;

    let title = "APPROVAL SHEET";
    current_layer.use_text(title, 24.0, Mm(20.0), Mm(270.0), &font_bold);

    let mut y = 250.0;
    let line_height = 8.0;
    let pairs = [
        ("Client", info.client_name.as_str()),
        ("Job Number", info.job_number.as_str()),
        ("Due Date", info.due_date.as_str()),
        ("Description", info.description.as_str()),
        ("Prepared By", info.staff_name.as_str()),
    ];
    for (label, value) in pairs.iter() {
        current_layer.use_text(format!("{}: {}", label, value), 12.0, Mm(20.0), Mm(y), &font);
        y -= line_height;
    }

    y -= 10.0;
    current_layer.use_text("Sign-off", 14.0, Mm(20.0), Mm(y), &font_bold);
    y -= line_height * 2.0;
    current_layer.use_text("Approved By: ________________________________", 12.0, Mm(20.0), Mm(y), &font);
    y -= line_height * 2.0;
    current_layer.use_text("Date: ________________________________", 12.0, Mm(20.0), Mm(y), &font);

    if info.include_preflight_summary {
        y -= line_height * 3.0;
        current_layer.use_text("Preflight Summary", 14.0, Mm(20.0), Mm(y), &font_bold);
        y -= line_height;
        current_layer.use_text("Preflight summary placeholder — integrate with preflight_findings table.", 10.0, Mm(20.0), Mm(y), &font);
    }

    let file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| e.to_string())?;
    Ok(())
}
