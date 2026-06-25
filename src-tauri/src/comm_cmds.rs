//! Communication commands (Phase 6.1).
//!
//! Email, FTP, and webhook integrations.

use tauri::State;

use crate::db::Database;
use crate::models::{EmailSettings, FtpSettings, WebhookEntry};
use crate::security;

/// Validate an SMTP host for SSRF protection.
fn validate_smtp_host(host: &str, port: u16) -> Result<(), String> {
    if host.is_empty() {
        return Err("SMTP host is empty".to_string());
    }
    if host.len() > 255 {
        return Err("SMTP host too long".to_string());
    }
    let allowed_ports = [25u16, 465, 587, 2525];
    if !allowed_ports.contains(&port) {
        return Err(format!(
            "SMTP port {} is not allowed (use 25, 465, 587, or 2525)",
            port
        ));
    }
    let resolved: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => {
            use std::net::ToSocketAddrs;
            (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("Cannot resolve SMTP host '{}': {}", host, e))?
                .map(|sa| sa.ip())
                .collect()
        }
    };
    for ip in &resolved {
        if is_blocked_ip(*ip) {
            return Err(format!("SMTP host resolves to blocked address: {}", ip));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_email_settings(db: State<'_, Database>, settings: EmailSettings) -> Result<(), String> {
    validate_smtp_host(&settings.smtp_host, settings.smtp_port)?;
    db.save_email_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_email_settings(db: State<'_, Database>) -> Result<Option<EmailSettings>, String> {
    db.get_email_settings().map_err(|e| e.to_string())
}

/// Validate an FTP host for SSRF protection.
fn validate_ftp_host(host: &str, port: u16) -> Result<(), String> {
    if host.is_empty() {
        return Err("FTP host is empty".to_string());
    }
    if host.len() > 255 {
        return Err("FTP host too long".to_string());
    }
    let allowed_ports = [21u16, 990];
    if !allowed_ports.contains(&port) {
        return Err(format!("FTP port {} is not allowed (use 21 or 990)", port));
    }
    let resolved: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => {
            use std::net::ToSocketAddrs;
            (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("Cannot resolve FTP host '{}': {}", host, e))?
                .map(|sa| sa.ip())
                .collect()
        }
    };
    for ip in &resolved {
        if is_blocked_ip(*ip) {
            return Err(format!("FTP host resolves to blocked address: {}", ip));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_ftp_settings(db: State<'_, Database>, settings: FtpSettings) -> Result<(), String> {
    validate_ftp_host(&settings.host, settings.port)?;
    db.save_ftp_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ftp_settings(db: State<'_, Database>) -> Result<Option<FtpSettings>, String> {
    db.get_ftp_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn send_email(
    db: State<'_, Database>,
    to: String,
    subject: String,
    body: String,
    attachment_path: Option<String>,
) -> Result<(), String> {
    let settings = db
        .get_email_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Email settings not configured. Save SMTP settings first.".to_string())?;
    let attachment_path = attachment_path
        .map(|p| security::validate_read_path(&p))
        .transpose()?;
    let attachment_path_str = attachment_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    crate::email::send_email_via_smtp(
        &settings,
        &to,
        &subject,
        &body,
        attachment_path_str.as_deref(),
    )
}

#[tauri::command]
pub fn ftp_upload(
    db: State<'_, Database>,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let settings = db
        .get_ftp_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "FTP settings not configured. Save FTP settings first.".to_string())?;
    let local_path = security::validate_read_path(&local_path)?;
    crate::ftp::upload_file_via_ftp(&settings, &local_path.to_string_lossy(), &remote_path)
}

#[tauri::command]
pub fn create_webhook(
    db: State<'_, Database>,
    url: String,
    event: String,
) -> Result<WebhookEntry, String> {
    if !url.starts_with("https://") {
        return Err("Webhook URL must use HTTPS".to_string());
    }
    if url.len() > 2048 {
        return Err("Webhook URL too long".to_string());
    }
    validate_command_url(&url)?;
    db.create_webhook(&url, &event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_webhooks(db: State<'_, Database>) -> Result<Vec<WebhookEntry>, String> {
    db.list_webhooks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_webhook(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_webhook(id).map_err(|e| e.to_string())
}

/// Validate that a user-supplied URL is safe to fetch from a Tauri command.
pub(crate) fn validate_command_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }
    if url.len() > 2048 {
        return Err("URL too long".to_string());
    }
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let scheme = parsed.scheme();
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL missing host".to_string())?;
    let is_local_dev =
        host == "localhost" || host == "127.0.0.1" || host == "::1" || host.ends_with(".localhost");
    if scheme != "https" && !(is_local_dev && scheme == "http") {
        return Err(format!(
            "URL must use HTTPS (got scheme '{}', host '{}')",
            scheme, host
        ));
    }
    let resolved: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => {
            use std::net::ToSocketAddrs;
            let port = if scheme == "https" { 443 } else { 80 };
            (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("Cannot resolve URL host '{}': {}", host, e))?
                .map(|sa| sa.ip())
                .collect()
        }
    };
    for ip in &resolved {
        if is_blocked_ip(*ip) {
            return Err(format!("URL resolves to blocked address: {}", ip));
        }
    }
    Ok(())
}

pub(crate) fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.octets()[0] == 100 && (v4.octets()[1] >= 64 && v4.octets()[1] <= 127)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.segments()[0] & 0xfe00 == 0xfc00
                || v6.segments()[0] & 0xffc0 == 0xfe80
        }
    }
}
