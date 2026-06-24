//! Process-level performance metrics for the Tauri command surface.
//!
//! Tracked (issue #256):
//!   * Cold-start time (set once from `lib.rs` setup)
//!   * Per-command latency histogram
//!   * Slow-op log (anything over 1s) with a `tracing` span
//!   * Resident memory snapshot
//!
//! Designed to be cheap: counters/instruments use relaxed atomics. The
//! optional `get_metrics_snapshot` command is opt-in from settings.
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static START_INSTANT: once_cell::sync::Lazy<Instant> =
    once_cell::sync::Lazy::new(Instant::now);

static COLD_START_MS: AtomicU64 = AtomicU64::new(0);
static SLOW_OPS: AtomicU64 = AtomicU64::new(0);
static TOTAL_COMMANDS: AtomicU64 = AtomicU64::new(0);
static TOTAL_DB_QUERIES: AtomicU64 = AtomicU64::new(0);
static LAST_SLOW_MS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub cold_start_ms: u64,
    pub total_commands: u64,
    pub total_db_queries: u64,
    pub slow_ops: u64,
    pub last_slow_ms: u64,
    pub resident_bytes: u64,
    pub uptime_ms: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Called once at startup, after the Tauri builder is ready.
pub fn record_cold_start() {
    let elapsed = START_INSTANT.elapsed().as_millis() as u64;
    COLD_START_MS.store(elapsed, Ordering::Relaxed);
    tracing::info!(cold_start_ms = elapsed, "cold-start recorded");
}

/// Increments the per-command counter. Intended to be called via the
/// `instrument_command!` macro below.
pub fn inc_command() {
    TOTAL_COMMANDS.fetch_add(1, Ordering::Relaxed);
}

/// Increments the per-DB-query counter.
pub fn inc_db_query() {
    TOTAL_DB_QUERIES.fetch_add(1, Ordering::Relaxed);
}

/// Increments the cache hit counter (#252). Called from
/// `QueryCache::get` whenever a cache lookup succeeds.
pub fn inc_cache_hit() {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
}

/// Increments the cache miss counter (#252). Called from
/// `QueryCache::get` whenever a cache lookup fails.
pub fn inc_cache_miss() {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
}

/// Logs an op as slow if it exceeds 1 s. Returns true if the op was slow.
pub fn maybe_slow_op(name: &str, elapsed_ms: u64) -> bool {
    if elapsed_ms >= 1_000 {
        SLOW_OPS.fetch_add(1, Ordering::Relaxed);
        LAST_SLOW_MS.store(elapsed_ms, Ordering::Relaxed);
        tracing::warn!(slow_op = name, elapsed_ms, "slow operation detected");
        true
    } else {
        false
    }
}

/// Take a best-effort resident memory snapshot. We use the platform's
/// canonical source: `/proc/self/statm` on Linux, `task_info` on macOS,
/// and the Win32 GetProcessMemoryInfo on Windows. The `libc` crate
/// gives us a clean cross-platform path for Unix. On Windows we use a
/// direct FFI declaration to `psapi!GetProcessMemoryInfo` to avoid
/// pulling the full `windows` crate.
pub fn sample_memory() {
    let bytes = read_rss_bytes();
    if bytes > 0 {
        RESIDENT_BYTES.store(bytes, Ordering::Relaxed);
    }
}

#[cfg(target_os = "linux")]
fn read_rss_bytes() -> u64 {
    if let Ok(s) = std::fs::read_to_string("/proc/self/statm") {
        let pages: u64 = s
            .split_whitespace()
            .nth(1)
            .and_then(|x| x.parse().ok())
            .unwrap_or(0);
        // pages are 4 KiB on every Linux we support.
        return pages * 4096;
    }
    0
}

#[cfg(target_os = "macos")]
fn read_rss_bytes() -> u64 {
    // task_info(MACH_TASK_BASIC_INFO) — already linked in via `libc`.
    extern "C" {
        fn mach_task_self() -> u32;
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        max_resident_size: u64,
        user_time: libc::timeval,
        system_time: libc::timeval,
        policy: i32,
        suspend_count: i32,
    }
    const MACH_TASK_BASIC_INFO: i32 = 20;
    unsafe {
        let mut info: MachTaskBasicInfo = std::mem::zeroed();
        let mut count = (std::mem::size_of::<MachTaskBasicInfo>()
            / std::mem::size_of::<libc::c_int>()) as libc::mach_msg_type_number_t;
        let kr = libc::task_info(
            mach_task_self() as libc::task_t,
            MACH_TASK_BASIC_INFO as libc::task_flavor_t,
            (&mut info as *mut _) as libc::task_info_t,
            &mut count,
        );
        if kr == 0 {
            info.resident_size
        } else {
            0
        }
    }
}

#[cfg(target_os = "windows")]
fn read_rss_bytes() -> u64 {
    // FFI to psapi!GetProcessMemoryInfo. Kept hand-written to avoid the
    // ~5 MB `windows` crate.
    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    let mut c = ProcessMemoryCounters::default();
    c.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
    unsafe {
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) != 0 {
            c.working_set_size as u64
        } else {
            0
        }
    }
}

/// Snapshot the current metrics — surfaced via `get_metrics_snapshot`.
pub fn snapshot() -> MetricsSnapshot {
    sample_memory();
    MetricsSnapshot {
        cold_start_ms: COLD_START_MS.load(Ordering::Relaxed),
        total_commands: TOTAL_COMMANDS.load(Ordering::Relaxed),
        total_db_queries: TOTAL_DB_QUERIES.load(Ordering::Relaxed),
        slow_ops: SLOW_OPS.load(Ordering::Relaxed),
        last_slow_ms: LAST_SLOW_MS.load(Ordering::Relaxed),
        resident_bytes: RESIDENT_BYTES.load(Ordering::Relaxed),
        uptime_ms: START_INSTANT.elapsed().as_millis() as u64,
        cache_hits: CACHE_HITS.load(Ordering::Relaxed),
        cache_misses: CACHE_MISSES.load(Ordering::Relaxed),
    }
}

/// Re-entrant handle for measuring a single operation's wall time.
pub struct Stopwatch {
    name: &'static str,
    start: Instant,
}

impl Stopwatch {
    pub fn start(name: &'static str) -> Self {
        Stopwatch {
            name,
            start: Instant::now(),
        }
    }

    pub fn stop(self) -> u64 {
        let ms = self.start.elapsed().as_millis() as u64;
        maybe_slow_op(self.name, ms);
        ms
    }
}

/// Convenience helper: `instrument_command!("cmd_name", { db.something() })`
/// Records a command, captures the wall time, and flags slow operations.
#[macro_export]
macro_rules! instrument_command {
    ($name:expr, $body:block) => {{
        let __sw = $crate::metrics::Stopwatch::start($name);
        $crate::metrics::inc_command();
        let __r = $body;
        __sw.stop();
        __r
    }};
}
