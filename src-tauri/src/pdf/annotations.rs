//! PDF annotation helpers (#230).
//!
//! Annotations are stored as metadata in SQLite and rendered as an overlay on
//! top of the page image in the frontend. Nothing is written into the PDF bytes.

use std::collections::HashMap;

use rusqlite::{params, Connection, Result};

/// Returns a map of `page_index → annotation_count` for `file_path`.
/// Used to show per-page annotation count badges in the viewer.
pub fn count_by_page(conn: &Connection, file_path: &str) -> Result<HashMap<i64, i64>> {
    let mut stmt = conn
        .prepare("SELECT page, COUNT(*) FROM pdf_annotations WHERE file_path = ?1 GROUP BY page")?;
    let rows = stmt.query_map(params![file_path], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (page, count) = row?;
        map.insert(page, count);
    }
    Ok(map)
}

/// Returns `true` if the annotation dimensions have positive area.
pub fn is_valid_rect(width: f64, height: f64) -> bool {
    width > 0.0 && height > 0.0
}
