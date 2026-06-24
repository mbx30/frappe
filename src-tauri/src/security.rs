// Security utilities and input validation for Tauri IPC boundary.
// This module enforces defense-in-depth for all user-supplied inputs,
// particularly filesystem paths used in PDF operations.

use std::path::{Component, Path, PathBuf};

/// Result type for security validation operations.
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Security validation errors.
#[derive(Debug, Clone)]
pub enum SecurityError {
    PathContainsNullBytes,
    PathNotFound,
    PathCanonicalizeFailure(String),
    PathTraversalAttempt,
    PathInsideSystemLocation,
    PathEmpty,
    PathNoParent,
    PathNoFilename,
    InvalidFileExtension { allowed: Vec<String>, got: String },
    InvalidIntRange { param: String, min: i64, max: i64, got: i64 },
    InvalidStringFormat { param: String, reason: String },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathContainsNullBytes => write!(f, "Path contains null bytes"),
            Self::PathNotFound => write!(f, "Path does not exist"),
            Self::PathCanonicalizeFailure(e) => write!(f, "Cannot canonicalize path: {}", e),
            Self::PathTraversalAttempt => write!(f, "Path contains parent directory traversal (..)"),
            Self::PathInsideSystemLocation => write!(f, "Path is inside a system location"),
            Self::PathEmpty => write!(f, "Path is empty"),
            Self::PathNoParent => write!(f, "Path has no parent directory"),
            Self::PathNoFilename => write!(f, "Path has no filename component"),
            Self::InvalidFileExtension { allowed, got } => {
                write!(f, "Invalid file extension '{}'. Allowed: {:?}", got, allowed)
            }
            Self::InvalidIntRange { param, min, max, got } => {
                write!(f, "Parameter '{}' out of range [{}, {}]: {}", param, min, max, got)
            }
            Self::InvalidStringFormat { param, reason } => {
                write!(f, "Parameter '{}' has invalid format: {}", param, reason)
            }
        }
    }
}

impl From<SecurityError> for String {
    fn from(err: SecurityError) -> Self {
        err.to_string()
    }
}

/// Validate a path for read operations.
/// The path must:
///   1. Contain no NUL bytes
///   2. Exist on the filesystem
///   3. Canonicalize to an absolute path
pub fn validate_read_path(path: &str) -> SecurityResult<PathBuf> {
    if path.contains('\0') {
        return Err(SecurityError::PathContainsNullBytes);
    }
    if path.is_empty() {
        return Err(SecurityError::PathEmpty);
    }

    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(SecurityError::PathNotFound);
    }

    // Reject parent-directory components in the original path.
    for component in p.components() {
        if matches!(component, Component::ParentDir) {
            return Err(SecurityError::PathTraversalAttempt);
        }
    }

    p.canonicalize()
        .map_err(|e| SecurityError::PathCanonicalizeFailure(e.to_string()))
}

/// Validate a path for read operations with optional file extension allowlist.
pub fn validate_read_path_with_extension(
    path: &str,
    allowed_extensions: &[&str],
) -> SecurityResult<PathBuf> {
    let canonical = validate_read_path(path)?;

    // Check file extension.
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !allowed_extensions.is_empty() && !allowed_extensions.contains(&ext) {
        return Err(SecurityError::InvalidFileExtension {
            allowed: allowed_extensions.iter().map(|s| s.to_string()).collect(),
            got: ext.to_string(),
        });
    }

    Ok(canonical)
}

/// Validate a path for write operations.
/// The path must:
///   1. Contain no NUL bytes
///   2. Not be empty
///   3. Have a parent directory that exists
///   4. Canonicalize to an absolute path (via parent)
///   5. Not be inside a system/read-only location
///   6. Not contain parent-directory (..) traversal
pub fn validate_write_path(path: &str) -> SecurityResult<PathBuf> {
    if path.contains('\0') {
        return Err(SecurityError::PathContainsNullBytes);
    }
    if path.is_empty() {
        return Err(SecurityError::PathEmpty);
    }

    let p = PathBuf::from(path);

    // Reject parent-directory components in the original path.
    for component in p.components() {
        if matches!(component, Component::ParentDir) {
            return Err(SecurityError::PathTraversalAttempt);
        }
    }

    let parent = p.parent().ok_or(SecurityError::PathNoParent)?;
    if !parent.exists() {
        return Err(SecurityError::PathCanonicalizeFailure(
            "Parent directory does not exist".to_string(),
        ));
    }

    // Reject system locations.
    reject_system_location(parent)?;

    // Canonicalize the parent (which must exist) and re-join the filename.
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| SecurityError::PathCanonicalizeFailure(e.to_string()))?;

    let file_name = p.file_name().ok_or(SecurityError::PathNoFilename)?;
    Ok(canonical_parent.join(file_name))
}

/// Validate a path for write operations with optional file extension.
pub fn validate_write_path_with_extension(
    path: &str,
    allowed_extensions: &[&str],
) -> SecurityResult<PathBuf> {
    let canonical = validate_write_path(path)?;

    // Check file extension.
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !allowed_extensions.is_empty() && !allowed_extensions.contains(&ext) {
        return Err(SecurityError::InvalidFileExtension {
            allowed: allowed_extensions.iter().map(|s| s.to_string()).collect(),
            got: ext.to_string(),
        });
    }

    Ok(canonical)
}

/// Reject paths inside system/read-only locations.
fn reject_system_location(path: &Path) -> SecurityResult<()> {
    let s = path.to_string_lossy().to_lowercase();
    let blocked_prefixes = blocked_system_locations();

    for blocked in blocked_prefixes {
        if s == blocked || s.starts_with(&format!("{}/", blocked)) {
            return Err(SecurityError::PathInsideSystemLocation);
        }
    }

    Ok(())
}

/// Get platform-specific blocked system locations.
#[cfg(windows)]
fn blocked_system_locations() -> Vec<&'static str> {
    vec![
        "c:\\windows",
        "c:\\program files",
        "c:\\program files (x86)",
        "c:\\programdata",
    ]
}

#[cfg(unix)]
fn blocked_system_locations() -> Vec<&'static str> {
    vec![
        "/etc", "/usr", "/bin", "/sbin", "/var", "/boot", "/sys", "/proc", "/root",
    ]
}

/// Validate a user-supplied i64 is within bounds.
pub fn validate_int_range(param: &str, value: i64, min: i64, max: i64) -> SecurityResult<i64> {
    if value < min || value > max {
        return Err(SecurityError::InvalidIntRange {
            param: param.to_string(),
            min,
            max,
            got: value,
        });
    }
    Ok(value)
}

/// Validate a user-supplied string is not empty and has reasonable length.
pub fn validate_string(param: &str, value: &str, max_len: usize) -> SecurityResult<()> {
    if value.is_empty() {
        return Err(SecurityError::InvalidStringFormat {
            param: param.to_string(),
            reason: "Value is empty".to_string(),
        });
    }
    if value.len() > max_len {
        return Err(SecurityError::InvalidStringFormat {
            param: param.to_string(),
            reason: format!("Value exceeds {} characters", max_len),
        });
    }
    Ok(())
}

/// Validate a user-supplied string contains only alphanumeric characters and hyphens.
pub fn validate_alphanumeric_with_hyphens(param: &str, value: &str) -> SecurityResult<()> {
    if !value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(SecurityError::InvalidStringFormat {
            param: param.to_string(),
            reason: "Value must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_read_path_with_null_bytes() {
        let result = validate_read_path("path\0with\0nulls");
        assert!(matches!(result, Err(SecurityError::PathContainsNullBytes)));
    }

    #[test]
    fn test_validate_read_path_nonexistent() {
        let result = validate_read_path("/tmp/definitely_does_not_exist_12345");
        assert!(matches!(result, Err(SecurityError::PathNotFound)));
    }

    #[test]
    fn test_validate_read_path_with_traversal() {
        let result = validate_read_path("../../../etc/passwd");
        assert!(matches!(result, Err(SecurityError::PathTraversalAttempt)));
    }

    #[test]
    fn test_validate_read_path_valid() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path().join("test.txt");
        fs::write(&temp_path, "test").unwrap();

        let result = validate_read_path(temp_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_write_path_system_location() {
        #[cfg(unix)]
        {
            let result = validate_write_path("/usr/local/bin/output.txt");
            assert!(matches!(result, Err(SecurityError::PathInsideSystemLocation)));
        }
    }

    #[test]
    fn test_validate_write_path_valid() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path().join("output.txt");

        let result = validate_write_path(temp_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_int_range() {
        assert!(validate_int_range("page", 5, 0, 100).is_ok());
        assert!(matches!(
            validate_int_range("page", -1, 0, 100),
            Err(SecurityError::InvalidIntRange { .. })
        ));
        assert!(matches!(
            validate_int_range("page", 101, 0, 100),
            Err(SecurityError::InvalidIntRange { .. })
        ));
    }

    #[test]
    fn test_validate_string() {
        assert!(validate_string("name", "test", 100).is_ok());
        assert!(matches!(
            validate_string("name", "", 100),
            Err(SecurityError::InvalidStringFormat { .. })
        ));
        assert!(matches!(
            validate_string("name", "x".repeat(101).as_str(), 100),
            Err(SecurityError::InvalidStringFormat { .. })
        ));
    }
}
