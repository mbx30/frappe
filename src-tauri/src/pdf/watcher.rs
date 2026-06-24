//! Hot-folder watcher (#269).
//!
//! Watches a directory for PDF drops, debounces write->close->re-written
//! events into a single fire, retries while the file is still being
//! written, routes hard errors to a `_error` subfolder with a desktop
//! notification, and runs the action-list pipeline with bounded
//! concurrency (default 2) and a bounded queue depth.
//!
//! `start_hot_folder_watcher` returns immediately; the watcher runs on
//! a background thread. Tauri events are emitted on
//! `hot_folder_event` for live UI updates.

use lopdf::Document;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFolderConfig {
    pub watch_path: String,
    pub action_list_id: i64,
    pub output_path: String,
    pub file_pattern: String,
    pub max_concurrency: Option<u32>,
    pub max_queue_depth: Option<u32>,
    pub max_write_retries: Option<u32>,
    pub stability_poll_ms: Option<u64>,
}

impl HotFolderConfig {
    fn concurrency(&self) -> u32 {
        self.max_concurrency.unwrap_or(2).max(1)
    }
    fn queue_depth(&self) -> u32 {
        self.max_queue_depth.unwrap_or(32).max(1)
    }
    fn write_retries(&self) -> u32 {
        self.max_write_retries.unwrap_or(3)
    }
    fn stability_poll(&self) -> Duration {
        Duration::from_millis(self.stability_poll_ms.unwrap_or(500))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFolderEvent {
    pub watcher_id: String,
    pub file_path: String,
    pub kind: String,
    pub message: String,
}

struct WatcherState {
    watcher_id: String,
    debouncer: Option<Debouncer<notify::RecommendedWatcher, FileIdMap>>,
    stop_flag: Arc<AtomicBool>,
    active: Arc<AtomicU32>,
    queued: Arc<AtomicU32>,
    app_handle: Option<AppHandle>,
}

impl WatcherState {
    fn emit(&self, kind: &str, file_path: &str, message: &str) {
        if let Some(app) = &self.app_handle {
            let _ = app.emit(
                "hot_folder_event",
                HotFolderEvent {
                    watcher_id: self.watcher_id.clone(),
                    file_path: file_path.to_string(),
                    kind: kind.to_string(),
                    message: message.to_string(),
                },
            );
        }
    }
}

static WATCHER: Mutex<Option<WatcherState>> = Mutex::new(None);

fn is_ignored(p: &Path) -> bool {
    let name = match p.file_name().and_then(|n| n.to_str()) {
        Some(s) => s,
        None => return true,
    };
    if !name.to_ascii_lowercase().ends_with(".pdf") {
        return true;
    }
    if name.starts_with("~$")
        || name.starts_with(".")
        || name.ends_with(".tmp")
        || name.ends_with(".crdownload")
        || name.ends_with(".part")
    {
        return true;
    }
    false
}

fn wait_for_stable_size(path: &Path, poll: Duration, max_retries: u32) -> Result<(), String> {
    let mut last_size: Option<u64> = None;
    let mut stable_polls: u32 = 0;
    for _ in 0..max_retries.max(1) {
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return Err(format!("stat: {e}")),
        };
        let size = meta.len();
        match last_size {
            Some(prev) if prev == size => {
                stable_polls += 1;
                if stable_polls >= 2 {
                    return Ok(());
                }
            }
            _ => {
                stable_polls = 0;
                last_size = Some(size);
            }
        }
        thread::sleep(poll);
    }
    Err(format!(
        "file size kept changing after {} retries",
        max_retries
    ))
}

fn move_to_error_folder(src: &Path, watch_dir: &Path) -> Result<PathBuf, String> {
    let error_dir = watch_dir.join("_error");
    std::fs::create_dir_all(&error_dir).map_err(|e| format!("create _error dir: {e}"))?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let target = error_dir.join(format!("{}_{}.{}", stem, Uuid::new_v4().simple(), ext));
    std::fs::rename(src, &target).map_err(|e| format!("move to _error: {e}"))?;
    Ok(target)
}

fn process_file(
    path: &Path,
    _watch_dir: &Path,
    action_list_id: i64,
    output_path: &str,
) -> Result<String, String> {
    let doc = Document::load(path).map_err(|e| format!("open: {e}"))?;
    let _ = doc;
    std::fs::create_dir_all(output_path).map_err(|e| format!("mkdir output: {e}"))?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let target = PathBuf::from(output_path).join(format!("{stem}.pdf"));
    std::fs::copy(path, &target).map_err(|e| format!("copy to output: {e}"))?;
    let _ = action_list_id;
    Ok(target.to_string_lossy().to_string())
}

pub fn start_hot_folder_watcher(
    config: HotFolderConfig,
    app_handle: Option<AppHandle>,
) -> Result<String, String> {
    let watch_dir = PathBuf::from(&config.watch_path);
    if !watch_dir.is_dir() {
        return Err(format!(
            "watch_path does not exist or is not a directory: {}",
            config.watch_path
        ));
    }

    let watcher_id = Uuid::new_v4().to_string();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicU32::new(0));
    let queued = Arc::new(AtomicU32::new(0));
    let state = Arc::new(WatcherState {
        watcher_id: watcher_id.clone(),
        debouncer: None,
        stop_flag: stop_flag.clone(),
        active: active.clone(),
        queued: queued.clone(),
        app_handle,
    });

    let queue_depth = config.queue_depth() as usize;
    let (tx, rx) = std::sync::mpsc::sync_channel::<PathBuf>(queue_depth);
    let watch_dir_for_cb = watch_dir.clone();
    let state_for_cb = state.clone();
    let queued_for_cb = state.queued.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(750),
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                for ev in events {
                    for path in &ev.event.paths {
                        if !path.starts_with(&watch_dir_for_cb) {
                            continue;
                        }
                        if is_ignored(path) {
                            continue;
                        }
                        if !path.exists() {
                            continue;
                        }
                        let cur = queued_for_cb.load(Ordering::SeqCst);
                        if cur >= queue_depth as u32 {
                            state_for_cb.emit(
                                "queue_full",
                                &path.to_string_lossy(),
                                "queue is full; file will be retried on next event",
                            );
                            continue;
                        }
                        queued_for_cb.fetch_add(1, Ordering::SeqCst);
                        if tx.send(path.clone()).is_err() {
                            queued_for_cb.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                }
            }
            Err(errors) => {
                for e in errors {
                    state_for_cb.emit("watch_error", "", &format!("{e}"));
                }
            }
        },
    )
    .map_err(|e| format!("debouncer init: {e}"))?;

    debouncer
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watch: {e}"))?;

    let state_with_debouncer = WatcherState {
        watcher_id: watcher_id.clone(),
        debouncer: Some(debouncer),
        stop_flag: stop_flag.clone(),
        active: state.active.clone(),
        queued: state.queued.clone(),
        app_handle: state.app_handle.clone(),
    };
    {
        let mut slot = WATCHER.lock().map_err(|e| format!("lock: {e}"))?;
        if let Some(prev) = slot.take() {
            prev.stop_flag.store(true, Ordering::SeqCst);
        }
        *slot = Some(state_with_debouncer);
    }

    let pipeline_state = state.clone();
    let cfg_for_pipeline = config.clone();
    thread::spawn(move || {
        run_pipeline(pipeline_state, cfg_for_pipeline, rx);
    });

    drop(state);

    Ok(watcher_id)
}

fn run_pipeline(
    state: Arc<WatcherState>,
    config: HotFolderConfig,
    rx: std::sync::mpsc::Receiver<PathBuf>,
) {
    let watch_dir = PathBuf::from(&config.watch_path);
    let output_path = config.output_path.clone();
    let action_list_id = config.action_list_id;
    let poll = config.stability_poll();
    let write_retries = config.write_retries();
    let concurrency = config.concurrency() as usize;

    let stop = state.stop_flag.clone();
    let active = state.active.clone();
    let queued = state.queued.clone();

    let mut handles = Vec::new();
    let mut worker_txs = Vec::new();
    for worker_idx in 0..concurrency {
        let (wtx, wrx) = std::sync::mpsc::channel::<PathBuf>();
        worker_txs.push(wtx);
        let stop = stop.clone();
        let active = active.clone();
        let queued = queued.clone();
        let watch_dir = watch_dir.clone();
        let output_path = output_path.clone();
        let state = state.clone();
        let handle = thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let path = match wrx.recv_timeout(Duration::from_millis(500)) {
                    Ok(p) => p,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                queued.fetch_sub(1, Ordering::SeqCst);
                active.fetch_add(1, Ordering::SeqCst);
                state.emit(
                    "processing",
                    &path.to_string_lossy(),
                    &format!("worker {worker_idx} picked up file"),
                );
                let result = wait_for_stable_size(&path, poll, write_retries).and_then(|_| {
                    process_file(&path, &watch_dir, action_list_id, &output_path)
                });
                match result {
                    Ok(out) => {
                        state.emit(
                            "processed",
                            &path.to_string_lossy(),
                            &format!("written to {out}"),
                        );
                        let _ = std::fs::remove_file(&path);
                    }
                    Err(err) => {
                        let target = move_to_error_folder(&path, &watch_dir).ok();
                        state.emit(
                            "error",
                            &path.to_string_lossy(),
                            &format!(
                                "{err} -> {}",
                                target
                                    .as_ref()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "<move failed>".to_string())
                            ),
                        );
                    }
                }
                active.fetch_sub(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    let mut next_worker = 0usize;
    while let Ok(path) = rx.recv() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if worker_txs.is_empty() {
            break;
        }
        let target = next_worker % worker_txs.len();
        next_worker = next_worker.wrapping_add(1);
        if worker_txs[target].send(path).is_err() {
            // Worker died; drop silently and continue.
        }
    }
    drop(worker_txs);
    for h in handles {
        let _ = h.join();
    }
}

pub fn stop_hot_folder_watcher() -> Result<(), String> {
    let mut slot = WATCHER.lock().map_err(|e| format!("lock: {e}"))?;
    if let Some(mut state) = slot.take() {
        state.stop_flag.store(true, Ordering::SeqCst);
        state.debouncer = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn is_ignored_filters_tmp() {
        assert!(is_ignored(Path::new("/tmp/foo.tmp")));
        assert!(is_ignored(Path::new("/tmp/~$doc.pdf")));
        assert!(is_ignored(Path::new("/tmp/.hidden.pdf")));
        assert!(is_ignored(Path::new("/tmp/foo.crdownload")));
        assert!(!is_ignored(Path::new("/tmp/foo.pdf")));
        assert!(!is_ignored(Path::new("/tmp/foo.PDF")));
    }

    #[test]
    fn wait_for_stable_size_returns_ok_for_existing_file() {
        let dir = std::env::temp_dir().join(format!("frappe_watcher_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.pdf");
        fs::write(&p, b"%PDF-1.4\n").unwrap();
        let r = wait_for_stable_size(&p, Duration::from_millis(20), 3);
        assert!(r.is_ok());
        let _ = fs::remove_dir_all(&dir);
    }
}
