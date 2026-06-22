use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretValue {
    pub exists: bool,
    pub value: Option<String>,
}

/// Read a secret from the OS keychain.
/// Falls back to encrypted config file when keyring is unavailable.
pub fn read_secret(service: &str, key: &str) -> Result<SecretValue, String> {
    match keyring::Entry::new(service, key) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => Ok(SecretValue {
                exists: true,
                value: Some(password),
            }),
            Err(keyring::Error::NoEntry) => Ok(SecretValue {
                exists: false,
                value: None,
            }),
            Err(e) => {
                log::warn!("keyring read failed for {service}/{key}: {e}, falling back to config");
                read_fallback(service, key)
            }
        },
        Err(_) => {
            log::warn!("keyring not available for {service}/{key}, falling back to config");
            read_fallback(service, key)
        }
    }
}

/// Write a secret to the OS keychain.
pub fn write_secret(service: &str, key: &str, value: &str) -> Result<(), String> {
    match keyring::Entry::new(service, key) {
        Ok(entry) => {
            entry
                .set_password(value)
                .map_err(|e| format!("keyring write failed: {e}"))?;
            Ok(())
        }
        Err(_) => {
            log::warn!("keyring not available for {service}/{key}, writing config fallback");
            write_fallback(service, key, value)
        }
    }
}

/// Delete a secret from the OS keychain.
pub fn delete_secret(service: &str, key: &str) -> Result<(), String> {
    match keyring::Entry::new(service, key) {
        Ok(entry) => {
            entry
                .delete_credential()
                .map_err(|e| format!("keyring delete failed: {e}"))?;
            Ok(())
        }
        Err(_) => {
            log::warn!("keyring not available for {service}/{key}, deleting config fallback");
            delete_fallback(service, key)
        }
    }
}

// ── Fallback: PLAINTEXT JSON config file (NOT encrypted) ───────────────
// IMPORTANT: This fallback is PLAINTEXT on disk. It is used only when the
// OS keychain is unavailable (e.g. headless Linux, broken libsecret). The
// file is at $XDG_CONFIG_HOME/frappe/secrets.json (Unix) or
// %USERPROFILE%/AppData/Local/Frappe/secrets.json (Windows). The at-rest
// protection is the filesystem ACL. If a backup/sync tool syncs this
// file, the user's database encryption key is exfiltrated.
// TODO: implement chacha20poly1305 with a machine-derived key in v2.

fn secrets_path() -> Result<std::path::PathBuf, String> {
    let base = if let Some(home) = dirs::config_local_dir() {
        home.join("frappe")
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
            .join(".config")
            .join("frappe")
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        std::path::PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("Frappe")
    } else {
        return Err("cannot determine config directory".to_string());
    };
    std::fs::create_dir_all(&base).ok();
    Ok(base.join("secrets.json"))
}

fn read_fallback(service: &str, key: &str) -> Result<SecretValue, String> {
    let path = secrets_path()?;
    if !path.exists() {
        return Ok(SecretValue {
            exists: false,
            value: None,
        });
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("read fallback failed: {e}"))?;
    let store: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        serde_json::from_str(&data).map_err(|e| {
            log::error!(
                "secrets.json is corrupt ({}); refusing to wipe. Rename or delete manually.",
                e
            );
            format!("secrets.json is corrupt: {}", e)
        })?;
    let exists = store.get(service).and_then(|m| m.get(key)).is_some();
    Ok(SecretValue {
        exists,
        value: store.get(service).and_then(|m| m.get(key)).cloned(),
    })
}

fn write_fallback(service: &str, key: &str, value: &str) -> Result<(), String> {
    let path = secrets_path()?;
    let mut store: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        if path.exists() {
            let data = std::fs::read_to_string(&path).map_err(|e| format!("read fallback: {e}"))?;
            match serde_json::from_str(&data) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("secrets.json is corrupt ({}); refusing to overwrite. Aborting write of {}/{} to avoid wiping existing secrets.", e, service, key);
                    return Err(format!(
                        "secrets.json is corrupt, refusing to overwrite: {}",
                        e
                    ));
                }
            }
        } else {
            std::collections::HashMap::new()
        };
    store
        .entry(service.to_string())
        .or_default()
        .insert(key.to_string(), value.to_string());
    let data =
        serde_json::to_string_pretty(&store).map_err(|e| format!("serialize fallback: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("write fallback: {e}"))?;
    Ok(())
}

fn delete_fallback(service: &str, key: &str) -> Result<(), String> {
    let path = secrets_path()?;
    if !path.exists() {
        return Ok(());
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("read fallback: {e}"))?;
    let mut store: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        serde_json::from_str(&data).map_err(|e| {
            log::error!("secrets.json is corrupt ({}); refusing to wipe. Aborting delete of {}/{} to avoid wiping existing secrets.", e, service, key);
            format!("secrets.json is corrupt, refusing to wipe: {}", e)
        })?;
    if let Some(map) = store.get_mut(service) {
        map.remove(key);
    }
    let data =
        serde_json::to_string_pretty(&store).map_err(|e| format!("serialize fallback: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("write fallback: {e}"))?;
    Ok(())
}
