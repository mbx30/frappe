use std::path::Path;

use crate::models::ImportResult;

pub fn import_csv(path: &Path) -> Result<ImportResult, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to read CSV: {}", e))?;

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    if headers.is_empty() {
        return Err("CSV file has no columns".to_string());
    }

    let mut rows_imported = 0;
    for result in reader.records() {
        let _record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        rows_imported += 1;
    }

    // Re-read to get actual data (calamine will handle this in the full flow)
    Ok(ImportResult {
        rows_imported,
        columns: headers,
        sheet_name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    })
}

fn preview_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to read CSV: {}", e))?;

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        let row: Vec<String> = record.iter().map(|v| v.to_string()).collect();
        rows.push(row);
    }

    Ok((headers, rows))
}

pub fn import_excel(path: &Path) -> Result<(String, Vec<String>, Vec<Vec<String>>), String> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<std::io::BufReader<std::fs::File>> =
        open_workbook(path).map_err(|e| format!("Failed to open Excel file: {}", e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let first_sheet = sheet_names
        .first()
        .cloned()
        .unwrap_or_else(|| "Sheet1".to_string());

    let range = workbook
        .worksheet_range(&first_sheet)
        .map_err(|e| format!("Failed to read Excel sheet '{}': {}", first_sheet, e))?;

    let mut rows_iter = range.rows();
    let headers: Vec<String> = if let Some(first_row) = rows_iter.next() {
        first_row.iter().map(|cell| cell.to_string()).collect()
    } else {
        return Err("Excel sheet is empty".to_string());
    };

    let mut rows = Vec::new();
    for row in rows_iter {
        let row_data: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
        rows.push(row_data);
    }

    Ok((first_sheet, headers, rows))
}

pub fn import_csv_data(path: &Path) -> Result<(String, Vec<String>, Vec<Vec<String>>), String> {
    let (headers, rows) = preview_csv(path)?;
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    Ok((name, headers, rows))
}
