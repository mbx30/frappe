use crate::keychain;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

const SERVICE_NAME: &str = "frappe-observability";
const DSN_NAME: &str = "sentry_dsn";
const OPT_IN_NAME: &str = "telemetry_opt_in";
const HOURLY_LIMIT: u64 = 50;

static HOURLY_USAGE: AtomicU64 = AtomicU64::new(0);
static HOURLY_USAGE_TIME: AtomicU64 = AtomicU64::new(0);
static TOTAL_REPORTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrashReport {
    pub error_message: String,
    pub stack_trace: String,
    pub context: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrashResponse {
    pub accepted: bool,
    pub reason: String,
}

pub fn record_metric(name: &str, value: f64) {
    if !opt_in_enabled() {
        return;
    }
    tracing::info!(metric = name, value = value, "observability metric");
}

pub fn record_error(error_message: &str, stack_trace: Option<&str>) {
    if !opt_in_enabled() {
        return;
    }
    if read_dsn().is_none() {
        return;
    }
    let hour = current_hour();
    let stored = HOURLY_USAGE_TIME.load(Ordering::Relaxed);
    if stored != hour {
        HOURLY_USAGE_TIME.store(hour, Ordering::Relaxed);
        HOURLY_USAGE.store(0, Ordering::Relaxed);
    }
    if HOURLY_USAGE.load(Ordering::Relaxed) >= HOURLY_LIMIT {
        return;
    }
    HOURLY_USAGE.fetch_add(1, Ordering::Relaxed);
    TOTAL_REPORTS.fetch_add(1, Ordering::Relaxed);

    let report = CrashReport {
        error_message: redact_secrets(error_message),
        stack_trace: stack_trace
            .map(redact_secrets)
            .unwrap_or_default(),
        context: "tauri_command".to_string(),
        timestamp: current_unix_ts(),
    };
    send_to_sentry_async(&report);
}

pub async fn crash_report(
    error_message: String,
    stack_trace: String,
) -> Result<CrashResponse, String> {
    if !opt_in_enabled() {
        return Ok(CrashResponse {
            accepted: false,
            reason: "Telemetry opt-in disabled. Enable in Preferences → Privacy.".to_string(),
        });
    }
    let dsn = match read_dsn() {
        Some(d) => d,
        None => {
            return Ok(CrashResponse {
                accepted: false,
                reason: "No Sentry DSN configured.".to_string(),
            });
        }
    };

    let report = CrashReport {
        error_message: redact_secrets(&error_message),
        stack_trace: redact_secrets(&stack_trace),
        context: "manual".to_string(),
        timestamp: current_unix_ts(),
    };

    send_to_sentry(&dsn, &report).await?;
    Ok(CrashResponse {
        accepted: true,
        reason: "ok".to_string(),
    })
}

fn opt_in_enabled() -> bool {
    match keychain::read_secret(SERVICE_NAME, OPT_IN_NAME) {
        Ok(secret) => secret.value.as_deref() == Some("1"),
        Err(_) => false,
    }
}

fn read_dsn() -> Option<String> {
    match keychain::read_secret(SERVICE_NAME, DSN_NAME) {
        Ok(secret) => secret.value.filter(|v| !v.is_empty()),
        Err(_) => None,
    }
}

fn redact_secrets(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let lower = input.to_ascii_lowercase();
    let patterns = [
        "password",
        "passwd",
        "secret",
        "api_key",
        "apikey",
        "token",
        "smtp_password",
        "bearer ",
        "authorization:",
    ];
    let mut i = 0;
    while i < input.len() {
        let mut redacted = false;
        for pat in &patterns {
            if lower[i..].starts_with(pat) {
                out.push_str(pat);
                out.push_str("=[REDACTED]");
                while i < input.len() && input.as_bytes()[i] != b'\n' {
                    i += 1;
                }
                redacted = true;
                break;
            }
        }
        if !redacted {
            let ch = input[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn send_to_sentry_async(report: &CrashReport) {
    let report = report.clone();
    let dsn = match read_dsn() {
        Some(d) => d,
        None => return,
    };
    std::thread::spawn(move || {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = client.post(dsn).json(&report).send();
    });
}

async fn send_to_sentry(dsn: &str, report: &CrashReport) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
    let res = client
        .post(dsn)
        .json(report)
        .send()
        .await
        .map_err(|e| format!("Sentry send failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!(
            "Sentry returned non-success status: {}",
            res.status()
        ));
    }
    Ok(())
}

fn current_unix_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn current_hour() -> u64 {
    current_unix_ts() / 3600
}
