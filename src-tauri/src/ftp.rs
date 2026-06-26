use crate::keychain;
use crate::models::FtpSettings;
use std::io::Cursor;
use std::path::Path;
use suppaftp::types::FileType;
use suppaftp::FtpStream;

/// Upload a single local file to a remote FTP server using the persisted
/// `FtpSettings`. The FTP password is read from the OS keychain at call time
/// and is never logged.
pub fn upload_file_via_ftp(
    settings: &FtpSettings,
    local_path: &str,
    remote_path: &str,
) -> Result<(), String> {
    if settings.host.trim().is_empty() {
        return Err("FTP host not configured".to_string());
    }
    let p = Path::new(local_path);
    if !p.exists() {
        return Err(format!("Local file not found: {local_path}"));
    }
    let bytes = std::fs::read(p).map_err(|e| format!("Failed to read local file: {e}"))?;

    let password = if settings.password.is_empty() {
        match keychain::read_secret("frappe-ftp", "password") {
            Ok(secret) => secret.value.unwrap_or_default(),
            Err(e) => return Err(format!("Failed to read FTP password from keychain: {e}")),
        }
    } else {
        settings.password.clone()
    };

    let target = if remote_path.is_empty() {
        p.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "Cannot derive remote filename".to_string())?
            .to_string()
    } else {
        remote_path.to_string()
    };

    let full_target = if !settings.remote_dir.is_empty() && !target.contains('/') {
        join_remote_path(&settings.remote_dir, &target)
    } else {
        target
    };

    let mut conn = FtpStream::connect(format!("{}:{}", settings.host, settings.port))
        .map_err(|e| format!("FTP connect failed: {e}"))?;

    conn.login(&settings.username, &password)
        .map_err(|e| format!("FTP login failed: {e}"))?;

    conn.transfer_type(FileType::Binary)
        .map_err(|e| format!("FTP set binary: {e}"))?;

    if let Some(dir) = parent_dir(&full_target) {
        ensure_remote_dir(&mut conn, dir)?;
    }

    let mut reader = Cursor::new(bytes);
    conn.put_file(&full_target, &mut reader)
        .map_err(|e| format!("FTP upload failed: {e}"))?;

    conn.quit().ok();
    Ok(())
}

fn join_remote_path(dir: &str, name: &str) -> String {
    let trimmed = dir.trim_end_matches('/');
    format!("{}/{}", trimmed, name)
}

fn parent_dir(path: &str) -> Option<&str> {
    let idx = path.rfind('/')?;
    if idx == 0 {
        None
    } else {
        Some(&path[..idx])
    }
}

fn ensure_remote_dir(conn: &mut FtpStream, dir: &str) -> Result<(), String> {
    let parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    let mut current = String::new();
    for part in parts {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        if let Err(e) = conn.mkdir(&current) {
            let msg = e.to_string();
            if !msg.to_lowercase().contains("exist") && !msg.to_lowercase().contains("already") {
                tracing::debug!("FTP mkdir {current} returned: {e}");
            }
        }
    }
    Ok(())
}
