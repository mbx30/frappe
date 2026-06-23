use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ColorSpaceKind {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
    CalGray,
    CalRGB,
    Lab,
    ICCBased(u8),
    Separation(Box<ColorSpaceKind>),
    DeviceN(Box<ColorSpaceKind>),
    Indexed(Box<ColorSpaceKind>),
    Pattern,
    Unknown(String),
}

impl ColorSpaceKind {
    pub fn display_name(&self) -> String {
        match self {
            ColorSpaceKind::DeviceGray => "DeviceGray".into(),
            ColorSpaceKind::DeviceRGB => "DeviceRGB".into(),
            ColorSpaceKind::DeviceCMYK => "DeviceCMYK".into(),
            ColorSpaceKind::CalGray => "CalGray".into(),
            ColorSpaceKind::CalRGB => "CalRGB".into(),
            ColorSpaceKind::Lab => "Lab".into(),
            ColorSpaceKind::ICCBased(n) => format!("ICCBased({}ch)", n),
            ColorSpaceKind::Separation(alt) => format!("Separation({})", alt.display_name()),
            ColorSpaceKind::DeviceN(alt) => format!("DeviceN({})", alt.display_name()),
            ColorSpaceKind::Indexed(base) => format!("Indexed({})", base.display_name()),
            ColorSpaceKind::Pattern => "Pattern".into(),
            ColorSpaceKind::Unknown(s) => s.clone(),
        }
    }

    pub fn is_rgb_family(&self) -> bool {
        matches!(
            self,
            ColorSpaceKind::DeviceRGB | ColorSpaceKind::CalRGB | ColorSpaceKind::Lab
        ) || matches!(self, ColorSpaceKind::ICCBased(3))
    }

    pub fn is_cmyk_family(&self) -> bool {
        matches!(self, ColorSpaceKind::DeviceCMYK) || matches!(self, ColorSpaceKind::ICCBased(4))
    }

    pub fn base_kind(&self) -> String {
        match self {
            ColorSpaceKind::Separation(alt)
            | ColorSpaceKind::DeviceN(alt)
            | ColorSpaceKind::Indexed(alt) => alt.base_kind(),
            other => other.display_name(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorUsage {
    pub color_space: ColorSpaceKind,
    pub usage_type: String,
    pub page: usize,
    pub object_context: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IccProfileInfo {
    pub num_channels: u8,
    pub color_space_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorSpaceFinding {
    pub color_space: String,
    pub kind: String,
    pub pages: Vec<usize>,
    pub is_pdf_x_violation: bool,
    pub severity: String,
    pub message: String,
}

pub fn resolve_color_space(
    name: &[u8],
    doc: &Document,
    resources: &Dictionary,
    depth: usize,
) -> ColorSpaceKind {
    if depth > 10 {
        return ColorSpaceKind::Unknown(format!("/{} (max depth)", String::from_utf8_lossy(name)));
    }

    if name.eq_ignore_ascii_case(b"DeviceGray") {
        return ColorSpaceKind::DeviceGray;
    }
    if name.eq_ignore_ascii_case(b"DeviceRGB") {
        return ColorSpaceKind::DeviceRGB;
    }
    if name.eq_ignore_ascii_case(b"DeviceCMYK") {
        return ColorSpaceKind::DeviceCMYK;
    }
    if name.eq_ignore_ascii_case(b"CalGray") {
        return ColorSpaceKind::CalGray;
    }
    if name.eq_ignore_ascii_case(b"CalRGB") {
        return ColorSpaceKind::CalRGB;
    }
    if name.eq_ignore_ascii_case(b"Lab") {
        return ColorSpaceKind::Lab;
    }
    if name.eq_ignore_ascii_case(b"Pattern") {
        return ColorSpaceKind::Pattern;
    }

    let cs_dict = match resources.get(b"ColorSpace") {
        Ok(Object::Dictionary(d)) => d.clone(),
        Ok(Object::Reference(id)) => match doc.get_dictionary(*id) {
            Ok(d) => d.clone(),
            Err(_) => {
                return ColorSpaceKind::Unknown(format!("/{}", String::from_utf8_lossy(name)))
            }
        },
        _ => return ColorSpaceKind::Unknown(format!("/{}", String::from_utf8_lossy(name))),
    };

    let entry = match cs_dict.get(name) {
        Ok(o) => o,
        Err(_) => return ColorSpaceKind::Unknown(format!("/{}", String::from_utf8_lossy(name))),
    };

    resolve_color_space_object(entry, doc, resources, depth + 1)
}

fn resolve_color_space_object(
    obj: &Object,
    doc: &Document,
    resources: &Dictionary,
    depth: usize,
) -> ColorSpaceKind {
    match obj {
        Object::Name(n) => resolve_color_space(n, doc, resources, depth + 1),
        Object::Array(arr) => resolve_array_color_space(arr, doc, resources, depth),
        Object::Reference(id) => match doc.get_object(*id) {
            Ok(o) => resolve_color_space_object(o, doc, resources, depth + 1),
            Err(_) => ColorSpaceKind::Unknown("indirect_ref_error".into()),
        },
        _ => ColorSpaceKind::Unknown("unexpected_object".into()),
    }
}

fn resolve_array_color_space(
    arr: &[Object],
    doc: &Document,
    resources: &Dictionary,
    depth: usize,
) -> ColorSpaceKind {
    if arr.is_empty() {
        return ColorSpaceKind::Unknown("empty_array".into());
    }

    let family = match &arr[0] {
        Object::Name(n) => String::from_utf8_lossy(n).to_string(),
        _ => return ColorSpaceKind::Unknown("non_name_family".into()),
    };

    match family.as_str() {
        "ICCBased" if arr.len() >= 2 => {
            let stream_ref = &arr[1];
            let stream = match stream_ref {
                Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok()),
                _ => None,
            };
            let n = stream
                .and_then(|s| s.dict.get(b"N").ok().and_then(|n| n.as_i64().ok()))
                .unwrap_or(3) as u8;
            ColorSpaceKind::ICCBased(n)
        }
        "Separation" if arr.len() >= 3 => {
            let alt = resolve_color_space_object(&arr[2], doc, resources, depth + 1);
            ColorSpaceKind::Separation(Box::new(alt))
        }
        "DeviceN" if arr.len() >= 3 => {
            let alt = resolve_color_space_object(&arr[2], doc, resources, depth + 1);
            ColorSpaceKind::DeviceN(Box::new(alt))
        }
        "Indexed" if arr.len() >= 4 => {
            let base = resolve_color_space_object(&arr[1], doc, resources, depth + 1);
            ColorSpaceKind::Indexed(Box::new(base))
        }
        _ => {
            // Try resolving as a name from resources
            resolve_color_space(family.as_bytes(), doc, resources, depth + 1)
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

fn find_extgstate(doc: &Document, resources: &Dictionary, name: &[u8]) -> Option<Dictionary> {
    let gs_dict = resources.get(b"ExtGState").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })?;
    match gs_dict.get(name) {
        Ok(Object::Dictionary(d)) => Some(d.clone()),
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

fn collect_xobject_subtype(
    doc: &Document,
    xobject_dict: &Dictionary,
    name: &[u8],
) -> Option<Vec<u8>> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| n.to_vec())
    })
}

fn collect_form_xobject_stream(
    doc: &Document,
    xobject_dict: &Dictionary,
    name: &[u8],
) -> Option<Vec<u8>> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        Some(stream.content.clone())
    })
}

fn collect_form_xobject_resources(
    doc: &Document,
    xobject_dict: &Dictionary,
    name: &[u8],
) -> Option<Dictionary> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        stream.dict.get(b"Resources").ok().and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
            _ => None,
        })
    })
}

fn collect_color_usages_in_stream(
    doc: &Document,
    page_id: (u32, u16),
    content_bytes: &[u8],
    page: usize,
    xobject_dict: &Dictionary,
    depth: usize,
) -> Vec<ColorUsage> {
    let mut usages = Vec::new();
    let mut fill_cs: Option<ColorSpaceKind> = None;
    let mut stroke_cs: Option<ColorSpaceKind> = None;

    let resources = match get_resources(doc, page_id) {
        Some(r) => r,
        None => return usages,
    };

    process_color_stream(
        doc,
        content_bytes,
        &resources,
        &xobject_dict,
        page,
        &mut usages,
        &mut stroke_cs,
        &mut fill_cs,
        depth,
    );

    usages
}

fn process_color_stream(
    doc: &Document,
    content_bytes: &[u8],
    resources: &Dictionary,
    xobject_dict: &Dictionary,
    page: usize,
    usages: &mut Vec<ColorUsage>,
    stroke_cs: &mut Option<ColorSpaceKind>,
    fill_cs: &mut Option<ColorSpaceKind>,
    depth: usize,
) {
    if depth > 10 {
        return;
    }

    let ops = parse_content_operations(content_bytes);

    for (op, _nums, names) in &ops {
        match op.as_str() {
            "CS" => {
                if let Some(name_bytes) = names.first() {
                    *stroke_cs = Some(resolve_color_space(name_bytes, doc, resources, 0));
                    if let Some(ref cs) = *stroke_cs {
                        usages.push(ColorUsage {
                            color_space: cs.clone(),
                            usage_type: "stroke".into(),
                            page,
                            object_context: format!("CS /{}", String::from_utf8_lossy(name_bytes)),
                        });
                    }
                }
            }
            "cs" => {
                if let Some(name_bytes) = names.first() {
                    *fill_cs = Some(resolve_color_space(name_bytes, doc, resources, 0));
                    if let Some(ref cs) = *fill_cs {
                        usages.push(ColorUsage {
                            color_space: cs.clone(),
                            usage_type: "fill".into(),
                            page,
                            object_context: format!("cs /{}", String::from_utf8_lossy(name_bytes)),
                        });
                    }
                }
            }
            "G" => {
                let changed = stroke_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceGray)
                    .unwrap_or(true);
                if changed {
                    *stroke_cs = Some(ColorSpaceKind::DeviceGray);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceGray,
                        usage_type: "stroke".into(),
                        page,
                        object_context: "G (implied DeviceGray)".into(),
                    });
                }
            }
            "g" => {
                let changed = fill_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceGray)
                    .unwrap_or(true);
                if changed {
                    *fill_cs = Some(ColorSpaceKind::DeviceGray);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceGray,
                        usage_type: "fill".into(),
                        page,
                        object_context: "g (implied DeviceGray)".into(),
                    });
                }
            }
            "RG" => {
                let changed = stroke_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceRGB)
                    .unwrap_or(true);
                if changed {
                    *stroke_cs = Some(ColorSpaceKind::DeviceRGB);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceRGB,
                        usage_type: "stroke".into(),
                        page,
                        object_context: "RG (implied DeviceRGB)".into(),
                    });
                }
            }
            "rg" => {
                let changed = fill_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceRGB)
                    .unwrap_or(true);
                if changed {
                    *fill_cs = Some(ColorSpaceKind::DeviceRGB);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceRGB,
                        usage_type: "fill".into(),
                        page,
                        object_context: "rg (implied DeviceRGB)".into(),
                    });
                }
            }
            "K" => {
                let changed = stroke_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceCMYK)
                    .unwrap_or(true);
                if changed {
                    *stroke_cs = Some(ColorSpaceKind::DeviceCMYK);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceCMYK,
                        usage_type: "stroke".into(),
                        page,
                        object_context: "K (implied DeviceCMYK)".into(),
                    });
                }
            }
            "k" => {
                let changed = fill_cs
                    .as_ref()
                    .map(|c| c != &ColorSpaceKind::DeviceCMYK)
                    .unwrap_or(true);
                if changed {
                    *fill_cs = Some(ColorSpaceKind::DeviceCMYK);
                    usages.push(ColorUsage {
                        color_space: ColorSpaceKind::DeviceCMYK,
                        usage_type: "fill".into(),
                        page,
                        object_context: "k (implied DeviceCMYK)".into(),
                    });
                }
            }
            "SC" | "SCN" => {
                if let Some(ref cs) = *stroke_cs {
                    usages.push(ColorUsage {
                        color_space: cs.clone(),
                        usage_type: "stroke".into(),
                        page,
                        object_context: format!("{} (stroke value)", op),
                    });
                }
            }
            "sc" | "scn" => {
                if let Some(ref cs) = *fill_cs {
                    usages.push(ColorUsage {
                        color_space: cs.clone(),
                        usage_type: "fill".into(),
                        page,
                        object_context: format!("{} (fill value)", op),
                    });
                }
            }
            "gs" => {
                if let Some(name_bytes) = names.first() {
                    if let Some(gs_dict) = find_extgstate(doc, resources, name_bytes) {
                        if let Ok(_sa) = gs_dict.get(b"SA").and_then(|o| o.as_bool()) {}
                        if let Ok(op) = gs_dict.get(b"OP").and_then(|o| o.as_bool()) {
                            if op {
                                usages.push(ColorUsage {
                                    color_space: ColorSpaceKind::Unknown("overprint".into()),
                                    usage_type: "stroke".into(),
                                    page,
                                    object_context: format!(
                                        "gs /{} OP=true",
                                        String::from_utf8_lossy(name_bytes)
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            "Do" => {
                if let Some(name_bytes) = names.first() {
                    let subtype = collect_xobject_subtype(doc, xobject_dict, name_bytes);
                    if subtype.as_deref() == Some(b"Form") {
                        if let Some(form_content) =
                            collect_form_xobject_stream(doc, xobject_dict, name_bytes)
                        {
                            let form_resources =
                                collect_form_xobject_resources(doc, xobject_dict, name_bytes)
                                    .unwrap_or_else(|| resources.clone());
                            let form_xobject_dict = find_xobject_dict(doc, &form_resources)
                                .unwrap_or_else(|| Dictionary::new());

                            process_color_stream(
                                doc,
                                &form_content,
                                &form_resources,
                                &form_xobject_dict,
                                page,
                                usages,
                                stroke_cs,
                                fill_cs,
                                depth + 1,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn compute_findings(usages: &[ColorUsage], target_profile: &str) -> Vec<ColorSpaceFinding> {
    let mut seen: std::collections::HashMap<String, Vec<&ColorUsage>> =
        std::collections::HashMap::new();
    for usage in usages {
        let key = usage.color_space.display_name();
        seen.entry(key).or_default().push(usage);
    }

    let mut findings = Vec::new();
    for (cs_name, usages) in seen {
        let cs = &usages[0].color_space;
        let pages: Vec<usize> = {
            let mut p: Vec<usize> = usages.iter().map(|u| u.page).collect();
            p.sort();
            p.dedup();
            p
        };
        let kind = usages[0].usage_type.clone();

        let (violation, severity, message) = classify_color_space(&cs, target_profile);

        findings.push(ColorSpaceFinding {
            color_space: cs_name,
            kind,
            pages,
            is_pdf_x_violation: violation,
            severity,
            message,
        });
    }

    findings
}

fn classify_color_space(cs: &ColorSpaceKind, target_profile: &str) -> (bool, String, String) {
    match target_profile {
        "pdfx_1a" => match cs {
            ColorSpaceKind::DeviceRGB | ColorSpaceKind::CalRGB | ColorSpaceKind::Lab => {
                (true, "error".into(), format!("{} is not permitted in PDF/X-1a. Only DeviceGray, DeviceCMYK, or Spot colors are allowed.", cs.display_name()))
            }
            ColorSpaceKind::ICCBased(n) if *n < 4 => {
                (true, "error".into(), format!("ICCBased({}ch) is not permitted in PDF/X-1a. Use DeviceCMYK instead.", n))
            }
            ColorSpaceKind::DeviceCMYK => {
                (false, "ok".into(), format!("{} is permitted in PDF/X-1a.", cs.display_name()))
            }
            _ => (false, "info".into(), format!("{} — acceptable in PDF/X-1a.", cs.display_name())),
        },
        "pdfx_3" | "pdfx_4" => match cs {
            ColorSpaceKind::DeviceRGB | ColorSpaceKind::CalRGB | ColorSpaceKind::Lab => {
                (true, "error".into(), format!("{} without an embedded ICC profile is not permitted in {}. Use ICCBased color spaces instead.", cs.display_name(), target_profile))
            }
            ColorSpaceKind::ICCBased(n) if *n >= 3 => {
                (false, "ok".into(), format!("ICCBased({}ch) is permitted in {}.", n, target_profile))
            }
            ColorSpaceKind::DeviceGray => {
                (false, "ok".into(), "DeviceGray is permitted.".into())
            }
            ColorSpaceKind::DeviceCMYK => {
                (false, "ok".into(), "DeviceCMYK is permitted.".into())
            }
            _ => (false, "info".into(), format!("{} — acceptable in {}.", cs.display_name(), target_profile)),
        },
        "cmyk_only" => match cs {
            ColorSpaceKind::DeviceCMYK => (false, "ok".into(), "DeviceCMYK — target space.".into()),
            ColorSpaceKind::ICCBased(n) if *n == 4 => {
                (false, "ok".into(), "ICCBased(4ch) — CMYK profile.".into())
            }
            ColorSpaceKind::Separation(alt) if alt.is_cmyk_family() => {
                (false, "info".into(), "Spot color using CMYK alternative — OK.".into())
            }
            _ => {
                (true, "warning".into(), format!("{} — not CMYK. Will need conversion for CMYK-only workflow.", cs.display_name()))
            }
        },
        _ => {
            // "any" or unknown profile — just report
            (false, "info".into(), format!("{} found.", cs.display_name()))
        }
    }
}

pub fn parse_icc_header(stream_data: &[u8]) -> IccProfileInfo {
    let color_space_type = if stream_data.len() > 19 {
        let sig_bytes = &stream_data[16..20];
        match sig_bytes {
            b"RGB " => "RGB".to_string(),
            b"CMYK" => "CMYK".to_string(),
            b"GRAY" => "GRAY".to_string(),
            b"LAB " => "Lab".to_string(),
            _ => format!("0x{}", hex_encode(sig_bytes)),
        }
    } else {
        "Unknown".into()
    };

    let description = extract_icc_description(stream_data);

    IccProfileInfo {
        num_channels: match color_space_type.as_str() {
            "GRAY" => 1,
            "RGB" => 3,
            "CMYK" => 4,
            _ => 0,
        },
        color_space_type,
        description,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

fn extract_icc_description(stream_data: &[u8]) -> String {
    // Try to find 'desc' tag in ICC profile
    // MultiLocalizedUnicodeTag (type 'mftt') or simple text
    if let Some(desc) = find_mlu_description(stream_data) {
        return desc;
    }
    if let Some(desc) = find_text_description(stream_data) {
        return desc;
    }
    "Unknown".into()
}

fn find_mlu_description(data: &[u8]) -> Option<String> {
    // Scan for 'desc' tag followed by a MultiLocalizedUnicodeTag
    // Tag table is at offset 128 (tag_count at 128)
    if data.len() < 132 {
        return None;
    }
    let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
    if data.len() < 132 + tag_count * 12 {
        return None;
    }

    for i in 0..tag_count {
        let entry_offset = 132 + i * 12;
        if entry_offset + 12 > data.len() {
            break;
        }
        let tag = &data[entry_offset..entry_offset + 4];
        if tag != b"desc" {
            continue;
        }
        // Offset and size of the tag data
        let offset = u32::from_be_bytes([
            data[entry_offset + 4],
            data[entry_offset + 5],
            data[entry_offset + 6],
            data[entry_offset + 7],
        ]) as usize;
        let _size = u32::from_be_bytes([
            data[entry_offset + 8],
            data[entry_offset + 9],
            data[entry_offset + 10],
            data[entry_offset + 11],
        ]) as usize;
        if offset + 12 > data.len() {
            return None;
        }

        let desc_type = &data[offset..offset + 4];
        if desc_type == b"desc" {
            // Simple text description
            let text_len = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]) as usize;
            let text_start = offset + 12;
            if text_start + text_len <= data.len() {
                return Some(
                    String::from_utf8_lossy(&data[text_start..text_start + text_len]).to_string(),
                );
            }
        } else if desc_type == b"mftt" {
            // MultiLocalizedUnicodeTag
            if offset + 16 > data.len() {
                return None;
            }
            let _record_count = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]) as usize;
            let record_size = u32::from_be_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]) as usize;
            let rec_offset = offset + 16;
            if rec_offset + record_size <= data.len() {
                // Try to get the first record's text
                let lang_len = u32::from_be_bytes([
                    data[rec_offset + 8],
                    data[rec_offset + 9],
                    data[rec_offset + 10],
                    data[rec_offset + 11],
                ]) as usize;
                let lang_offset = u32::from_be_bytes([
                    data[rec_offset + 12],
                    data[rec_offset + 13],
                    data[rec_offset + 14],
                    data[rec_offset + 15],
                ]) as usize;
                let text_start = offset + lang_offset;
                if text_start + lang_len * 2 <= data.len() {
                    let utf16: Vec<u16> = data[text_start..text_start + lang_len * 2]
                        .chunks(2)
                        .map(|c| u16::from_be_bytes([c[0], c[1]]))
                        .collect();
                    return String::from_utf16(&utf16).ok();
                }
            }
        }
    }
    None
}

fn find_text_description(data: &[u8]) -> Option<String> {
    // Fallback: scan for a simple null-terminated ASCII string near the header
    if data.len() > 60 {
        let candidate = &data[36..60]; // frequently has a description in older profiles
        if let Ok(s) = String::from_utf8(candidate.to_vec()) {
            let trimmed = s.trim_matches(char::from(0)).trim();
            if !trimmed.is_empty() && trimmed.len() > 3 {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub fn check_color_spaces(doc: &Document, target_profile: &str) -> Vec<ColorSpaceFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut all_usages = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let content_bytes = match doc.get_page_content(obj_id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let resources = match get_resources(doc, obj_id) {
            Some(r) => r,
            None => continue,
        };
        let xobject_dict = find_xobject_dict(doc, &resources).unwrap_or_else(|| Dictionary::new());

        let usages =
            collect_color_usages_in_stream(doc, obj_id, &content_bytes, page, &xobject_dict, 0);
        all_usages.extend(usages);

        // Also scan image color spaces
        let image_defs = collect_image_color_spaces(doc, obj_id);
        for (cs, ctx) in image_defs {
            all_usages.push(ColorUsage {
                color_space: cs,
                usage_type: "image".into(),
                page,
                object_context: ctx,
            });
        }
    }

    compute_findings(&all_usages, target_profile)
}

fn collect_image_color_spaces(
    doc: &Document,
    page_id: (u32, u16),
) -> Vec<(ColorSpaceKind, String)> {
    let mut results = Vec::new();
    let resources = match get_resources(doc, page_id) {
        Some(r) => r,
        None => return results,
    };
    let xobject_dict = match find_xobject_dict(doc, &resources) {
        Some(x) => x,
        None => return results,
    };

    for (name, value) in xobject_dict.iter() {
        let stream = match value {
            Object::Reference(id) => {
                match doc.get_object(*id).ok().and_then(|o| o.as_stream().ok()) {
                    Some(s) => s,
                    None => continue,
                }
            }
            _ => continue,
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok());
        if subtype != Some(b"Image") {
            continue;
        }

        let cs = match stream.dict.get(b"ColorSpace") {
            Ok(Object::Name(n)) => resolve_color_space(n, doc, &resources, 0),
            Ok(Object::Array(arr)) => resolve_array_color_space(arr, doc, &resources, 0),
            Ok(Object::Reference(id)) => match doc.get_object(*id) {
                Ok(o) => match o {
                    Object::Name(n) => resolve_color_space(n, doc, &resources, 0),
                    Object::Array(arr) => resolve_array_color_space(arr, doc, &resources, 0),
                    _ => ColorSpaceKind::Unknown("indirect_noname".into()),
                },
                Err(_) => ColorSpaceKind::Unknown("indirect_error".into()),
            },
            _ => continue,
        };

        let ctx = format!("Image /{}", String::from_utf8_lossy(name));
        results.push((cs, ctx));
    }

    results
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotColorFinding {
    pub name: String,
    pub pages: Vec<usize>,
    pub has_alternate_colorspace: bool,
    pub alternate_colorspace_type: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InkCoverageFinding {
    pub page: usize,
    pub max_tac: f64,
    pub average_tac: f64,
    pub exceeds_threshold: bool,
    pub severity: String,
    pub message: String,
}

pub fn check_spot_colors(doc: &Document) -> Vec<SpotColorFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut spot_names: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    let mut alt_spaces: std::collections::HashMap<String, ColorSpaceKind> =
        std::collections::HashMap::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let resources = match get_resources(doc, obj_id) {
            Some(r) => r,
            None => continue,
        };

        let cs_dict = match resources.get(b"ColorSpace") {
            Ok(Object::Dictionary(d)) => d.clone(),
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok().cloned().unwrap_or_default(),
            _ => continue,
        };

        for (_name, value) in cs_dict.iter() {
            let arr: Vec<Object> = match value {
                Object::Array(a) => a.clone(),
                Object::Reference(id) => {
                    match doc.get_object(*id).ok().and_then(|o| o.as_array().ok()) {
                        Some(a) => a.clone(),
                        None => continue,
                    }
                }
                _ => continue,
            };

            if arr.is_empty() {
                continue;
            }
            let family = match &arr[0] {
                Object::Name(n) => String::from_utf8_lossy(&n).to_string(),
                _ => continue,
            };

            if family == "Separation" && arr.len() >= 3 {
                let spot_name = match &arr[1] {
                    Object::Name(n) => String::from_utf8_lossy(&n).to_string(),
                    _ => continue,
                };
                let alt = resolve_color_space_object(&arr[2], doc, &resources, 0);
                alt_spaces.insert(spot_name.clone(), alt);
                spot_names.entry(spot_name).or_default().push(page);
            } else if family == "DeviceN" && arr.len() >= 4 {
                let names_arr = match &arr[1] {
                    Object::Array(a) => a,
                    _ => continue,
                };
                for name_obj in names_arr {
                    if let Object::Name(n) = name_obj {
                        let name = String::from_utf8_lossy(&n).to_string();
                        let alt = resolve_color_space_object(&arr[2], doc, &resources, 0);
                        alt_spaces.insert(name.clone(), alt);
                        spot_names.entry(name).or_default().push(page);
                    }
                }
            }
        }

        // Also check image color spaces for Separation/DeviceN
        let xobject_dict = find_xobject_dict(doc, &resources).unwrap_or_else(|| Dictionary::new());
        for (_name, value) in xobject_dict.iter() {
            let stream = match value {
                Object::Reference(id) => {
                    match doc.get_object(*id).ok().and_then(|o| o.as_stream().ok()) {
                        Some(s) => s,
                        None => continue,
                    }
                }
                _ => continue,
            };
            let subtype = stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|o| o.as_name().ok());
            if subtype != Some(b"Image") {
                continue;
            }
            if let Ok(cs) = stream.dict.get(b"ColorSpace") {
                let resolved = resolve_color_space_object(cs, doc, &resources, 0);
                if let ColorSpaceKind::Separation(_) = &resolved {
                    // Get the name from the color space array
                    if let Object::Array(arr) = cs {
                        if arr.len() >= 2 {
                            if let Object::Name(n) = &arr[1] {
                                let name = String::from_utf8_lossy(n).to_string();
                                spot_names.entry(name).or_default().push(page);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut findings: Vec<SpotColorFinding> = spot_names
        .into_iter()
        .map(|(name, pages)| {
            let alt = alt_spaces.get(&name);
            let is_process = name.eq_ignore_ascii_case("All")
                || name.eq_ignore_ascii_case("None")
                || name.contains("Cut")
                || name.contains("Die")
                || name.contains("Cyan")
                || name.contains("Magenta")
                || name.contains("Yellow")
                || name.contains("Black")
                || name.contains("Varnish")
                || name.contains("Coat");
            SpotColorFinding {
                name: name.clone(),
                pages: pages.clone(),
                has_alternate_colorspace: alt.is_some(),
                alternate_colorspace_type: alt
                    .map(|a| a.display_name())
                    .unwrap_or_else(|| "none".into()),
                severity: if is_process {
                    "info".into()
                } else {
                    "warning".into()
                },
                message: if is_process {
                    format!(
                        "'{}' — appears to be a process/finishing name. Verify intent.",
                        name
                    )
                } else {
                    let alt_str = alt
                        .map(|a| format!(" using {}", a.display_name()))
                        .unwrap_or_default();
                    format!(
                        "Spot color '{}' found{} on pages {}.",
                        name,
                        alt_str,
                        pages
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
            }
        })
        .collect();

    if findings.is_empty() {
        findings.push(SpotColorFinding {
            name: "No spot colors".into(),
            pages: vec![],
            has_alternate_colorspace: false,
            alternate_colorspace_type: "none".into(),
            severity: "ok".into(),
            message: "No spot colors (Separation or DeviceN) found in the document.".into(),
        });
    }

    findings
}

pub fn check_ink_coverage(doc: &lopdf::Document) -> Vec<InkCoverageFinding> {
    let mut findings = Vec::new();
    let threshold = 300.0;

    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();

    for (idx, &obj_id) in page_ids.iter().enumerate() {
        let page_num = idx + 1;
        let bytes = match doc.get_page_content(obj_id) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let text = String::from_utf8_lossy(&bytes);
        let mut max_tac = 0.0f64;

        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            let tok = tokens[i];
            match tok {
                "k" | "K" | "sc" | "SC" | "SCN" | "scn" => {
                    // DeviceCMYK color: preceding 4 values are C M Y K
                    if i >= 4 {
                        let c = tokens[i - 4].parse::<f64>().unwrap_or(0.0);
                        let m = tokens[i - 3].parse::<f64>().unwrap_or(0.0);
                        let y = tokens[i - 2].parse::<f64>().unwrap_or(0.0);
                        let k_val = tokens[i - 1].parse::<f64>().unwrap_or(0.0);
                        let tac = c + m + y + k_val;
                        if tac > max_tac {
                            max_tac = tac;
                        }
                    }
                }
                "g" => {
                    if i >= 1 {
                        let g = tokens[i - 1].parse::<f64>().unwrap_or(0.0);
                        let tac = g; // device gray maps to K only in prepress
                        if tac > max_tac {
                            max_tac = tac;
                        }
                    }
                }
                "rg" | "RG" => {
                    if i >= 3 {
                        let r = tokens[i - 3].parse::<f64>().unwrap_or(0.0);
                        let g_val = tokens[i - 2].parse::<f64>().unwrap_or(0.0);
                        let b = tokens[i - 1].parse::<f64>().unwrap_or(0.0);
                        // RGB → CMYK approximation: K = 1 - max(R,G,B); C = (1-R-K)/(1-K)
                        let k_c = (1.0 - r).max(1.0 - g_val).max(1.0 - b);
                        let tac = if k_c < 1.0 {
                            (1.0 - r - k_c) / (1.0 - k_c)
                                + (1.0 - g_val - k_c) / (1.0 - k_c)
                                + (1.0 - b - k_c) / (1.0 - k_c)
                                + k_c
                        } else {
                            k_c
                        };
                        if tac > max_tac {
                            max_tac = tac;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        let exceeds = max_tac > threshold;
        findings.push(InkCoverageFinding {
            page: page_num,
            max_tac: (max_tac * 100.0).min(400.0),
            average_tac: max_tac * 50.0, // rough estimate: average ~ half of max
            exceeds_threshold: exceeds,
            severity: if exceeds { "warning" } else { "ok" }.into(),
            message: if exceeds {
                format!(
                    "Page {} max TAC ~{:.0}% exceeds {:.0}% threshold. Reduce ink density.",
                    page_num,
                    max_tac * 100.0,
                    threshold
                )
            } else {
                format!("Page {} ink coverage within threshold.", page_num)
            },
        });
    }

    if findings.is_empty() {
        findings.push(InkCoverageFinding {
            page: 0,
            max_tac: 0.0,
            average_tac: 0.0,
            exceeds_threshold: false,
            severity: "info".into(),
            message: "No page content found for ink coverage analysis.".into(),
        });
    }

    findings
}

pub fn icc_profile_from_stream(doc: &Document, stream_ref: &Object) -> Option<IccProfileInfo> {
    let stream = match stream_ref {
        Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
        _ => return None,
    };
    let n = stream
        .dict
        .get(b"N")
        .ok()
        .and_then(|n| n.as_i64().ok())
        .unwrap_or(3) as u8;

    let info = parse_icc_header(&stream.content);
    Some(IccProfileInfo {
        num_channels: n,
        ..info
    })
}
