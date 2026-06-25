//! New Tauri commands added in the #289-#293 batch.

use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Debug, Clone, Serialize)]
pub struct AppEventMetrics {
    pub snapshot: crate::metrics::MetricsSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppEventHeartbeat {
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AppEvent {
    HotFolder {
        watcher_id: String,
        file_path: String,
        event_kind: String,
        message: String,
    },
    Metrics(AppEventMetrics),
    Heartbeat(AppEventHeartbeat),
}

#[tauri::command]
pub async fn subscribe_events(on_event: Channel<AppEvent>) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let stop = Arc::new(AtomicBool::new(false));
    let on_event_clone = on_event.clone();
    let stop_clone = stop.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = 0u64;
        while !stop_clone.load(Ordering::SeqCst) {
            tick = tick.wrapping_add(1);
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let event = if tick % 10 == 0 {
                AppEvent::Metrics(AppEventMetrics {
                    snapshot: crate::metrics::snapshot(),
                })
            } else {
                AppEvent::Heartbeat(AppEventHeartbeat { ts })
            };
            if on_event_clone.send(event).is_err() {
                break;
            }
            tauri::async_runtime::spawn_blocking(|| {
                std::thread::sleep(Duration::from_millis(500));
            })
            .await
            .ok();
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn render_page_b64(
    engine: tauri::State<'_, crate::pdf::engine::PdfEngine>,
    path: String,
    page_index: usize,
    dpi: Option<f32>,
) -> Result<String, String> {
    let path = crate::security::validate_read_path(&path)?;
    let doc = engine.open_document(&path.to_string_lossy())?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi_val = dpi.unwrap_or(144.0) as f64;
    let page_width = page.width().value as f64;
    let px_width = (page_width * dpi_val / 72.0) as i32;
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(px_width);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {}", e))?;
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes: Vec<u8> = bitmap.as_raw_bytes().to_vec();
    drop(bitmap);
    let png_bytes: Vec<u8> = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, String> {
        use image::ImageEncoder;
        if bytes.len() < (pw as usize) * (ph as usize) * 4 {
            return Err("Rendered bitmap is shorter than expected".to_string());
        }
        let mut img = image::RgbaImage::new(pw, ph);
        for y in 0..ph {
            for x in 0..pw {
                let i = ((y * pw + x) * 4) as usize;
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]),
                );
            }
        }
        let mut png: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ColorType::Rgba8.into(),
            )
            .map_err(|e| format!("PNG encode error: {e}"))?;
        Ok(png)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))??;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}
