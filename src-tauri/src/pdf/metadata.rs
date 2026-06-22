use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct OutputIntent {
    pub s_key: String,
    pub output_condition: String,
    pub output_condition_id: String,
    pub registry_name: String,
    pub has_embedded_icc: bool,
    pub icc_num_channels: i32,
}

fn obj_to_string(o: &Object) -> String {
    match o {
        Object::Name(n) => String::from_utf8_lossy(n).to_string(),
        Object::String(s, _) => String::from_utf8_lossy(s).to_string(),
        _ => String::new(),
    }
}

fn deref_string(doc: &Document, obj: &Object) -> String {
    doc.dereference(obj)
        .ok()
        .and_then(|(_, o)| Some(obj_to_string(&o)))
        .unwrap_or_default()
}

pub fn get_output_intents(doc: &Document) -> Vec<OutputIntent> {
    // Get catalog via trailer's Root reference (not hardcoded (1,0)).
    let catalog_id = match doc.trailer.get(b"Root") {
        Ok(r) => match r.as_reference() {
            Ok(id) => id,
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };
    let catalog = match doc.get_object(catalog_id) {
        Ok(o) => match o.as_dict() {
            Ok(d) => d.clone(),
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };

    let intents = match catalog.get(b"OutputIntents") {
        Ok(o) => match doc.dereference(o).ok() {
            Some((_, Object::Array(a))) => a.clone(),
            Some((_, Object::Dictionary(_))) => return vec![],
            _ => return vec![],
        },
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    for item in &intents {
        let (_, dict_obj) = match doc.dereference(item).ok() {
            Some(d) => d,
            None => continue,
        };
        let dict = match dict_obj.as_dict() {
            Ok(d) => d,
            Err(_) => continue,
        };

        let s_key = dict
            .get(b"S")
            .ok()
            .and_then(|o| doc.dereference(o).ok())
            .map(|(_, o)| obj_to_string(&o))
            .unwrap_or_default();

        let output_condition = dict
            .get(b"OutputCondition")
            .ok()
            .and_then(|o| Some(deref_string(doc, o)))
            .unwrap_or_default();

        let output_condition_id = dict
            .get(b"OutputConditionIdentifier")
            .ok()
            .and_then(|o| Some(deref_string(doc, o)))
            .unwrap_or_default();

        let registry_name = dict
            .get(b"RegistryName")
            .ok()
            .and_then(|o| Some(deref_string(doc, o)))
            .unwrap_or_default();

        let (has_embedded_icc, icc_num_channels) = match dict.get(b"DestOutputProfile") {
            Ok(o) => match doc.dereference(o).ok() {
                Some((_, Object::Stream(stream))) => {
                    let channels = stream
                        .dict
                        .get(b"N")
                        .ok()
                        .and_then(|n| n.as_i64().ok())
                        .unwrap_or(0) as i32;
                    (true, channels)
                }
                _ => (false, 0),
            },
            Err(_) => (false, 0),
        };

        results.push(OutputIntent {
            s_key,
            output_condition,
            output_condition_id,
            registry_name,
            has_embedded_icc,
            icc_num_channels,
        });
    }

    results
}
