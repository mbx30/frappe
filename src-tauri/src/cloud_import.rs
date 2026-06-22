use serde_json::Value;

pub async fn import_google_sheet(
    spreadsheet_id: &str,
    api_key: &str,
    range: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        spreadsheet_id, range
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let resp = client
        .get(&url)
        .header("X-Goog-Api-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("Google Sheets request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Google Sheets API error ({}): {}", status, body));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let values = data["values"].as_array().ok_or("No data found in sheet")?;
    if values.is_empty() {
        return Err("Sheet is empty".to_string());
    }

    let headers: Vec<String> = values[0]
        .as_array()
        .ok_or("Invalid header row")?
        .iter()
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else if v.is_null() {
                String::new()
            } else {
                v.to_string()
            }
        })
        .collect();

    let mut rows = Vec::new();
    for row in values.iter().skip(1) {
        let cells: Vec<String> = row
            .as_array()
            .ok_or("Invalid row data")?
            .iter()
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else if v.is_null() {
                    String::new()
                } else {
                    v.to_string()
                }
            })
            .collect();
        rows.push(cells);
    }

    Ok((headers, rows))
}

pub async fn import_notion_database(
    database_id: &str,
    api_key: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut all_results: Vec<Value> = Vec::new();
    let mut columns: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    const MAX_ROWS: usize = 100_000; // safety cap to prevent runaway imports

    loop {
        let body = match &cursor {
            Some(c) => serde_json::json!({ "start_cursor": c, "page_size": 100 }),
            None => serde_json::json!({ "page_size": 100 }),
        };
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Notion request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Notion API error ({}): {}", status, body));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Build column names. The query response does NOT include a
        // top-level `properties` key (that's only in the GET database
        // response). The column names live inside each result's
        // `properties` object. We union the keys from the first page's
        // results. See #116/#129.
        if columns.is_empty() {
            if let Some(arr) = data["results"].as_array() {
                for result in arr {
                    if let Some(props_obj) = result["properties"].as_object() {
                        for name in props_obj.keys() {
                            if !columns.contains(name) {
                                columns.push(name.clone());
                            }
                        }
                    }
                }
            }
        }

        if let Some(arr) = data["results"].as_array() {
            all_results.extend(arr.iter().cloned());
        }

        if all_results.len() > MAX_ROWS {
            return Err(format!(
                "Notion import exceeds {} rows; refine your query",
                MAX_ROWS
            ));
        }

        if data["has_more"].as_bool() == Some(true) {
            match data["next_cursor"].as_str() {
                Some(c) if !c.is_empty() => cursor = Some(c.to_string()),
                _ => break,
            }
        } else {
            break;
        }
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    for result in &all_results {
        let props = result["properties"].as_object();
        let mut row = Vec::new();
        if let Some(props) = props {
            for col_key in &columns {
                if let Some(prop) = props.get(col_key) {
                    row.push(extract_notion_value(prop));
                } else {
                    row.push(String::new());
                }
            }
        }
        if !row.is_empty() {
            rows.push(row);
        }
    }

    Ok((columns, rows))
}

fn extract_notion_value(prop: &Value) -> String {
    let ptype = prop["type"].as_str().unwrap_or("");
    match ptype {
        "title" => prop["title"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v["plain_text"].as_str())
            .unwrap_or("")
            .to_string(),
        "rich_text" => prop["rich_text"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v["plain_text"].as_str())
            .unwrap_or("")
            .to_string(),
        "number" => prop["number"]
            .as_f64()
            .map(|n| n.to_string())
            .unwrap_or_default(),
        "select" => prop["select"]["name"].as_str().unwrap_or("").to_string(),
        "multi_select" => {
            let names: Vec<&str> = prop["multi_select"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v["name"].as_str()).collect())
                .unwrap_or_default();
            names.join(", ")
        }
        "date" => prop["date"]["start"].as_str().unwrap_or("").to_string(),
        "checkbox" => prop["checkbox"]
            .as_bool()
            .map(|b| b.to_string())
            .unwrap_or_default(),
        "email" => prop["email"].as_str().unwrap_or("").to_string(),
        "phone_number" => prop["phone_number"].as_str().unwrap_or("").to_string(),
        "url" => prop["url"].as_str().unwrap_or("").to_string(),
        "status" => prop["status"]["name"].as_str().unwrap_or("").to_string(),
        "created_time" => prop["created_time"].as_str().unwrap_or("").to_string(),
        "last_edited_time" => prop["last_edited_time"].as_str().unwrap_or("").to_string(),
        "formula" => extract_notion_value(&prop["formula"]),
        "rollup" => extract_notion_value(&prop["rollup"]),
        "people" => {
            let names: Vec<&str> = prop["people"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v["name"].as_str()).collect())
                .unwrap_or_default();
            names.join(", ")
        }
        "relation" => String::new(),
        "files" => {
            let names: Vec<&str> = prop["files"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v["name"].as_str()).collect())
                .unwrap_or_default();
            names.join(", ")
        }
        _ => serde_json::to_string(prop).unwrap_or_default(),
    }
}
