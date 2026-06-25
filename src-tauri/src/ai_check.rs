use crate::keychain;
use crate::models::AiCheckResult;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SERVICE_NAME: &str = "frappe-ai";
const API_KEY_NAME: &str = "openai_api_key";
const CONSENT_NAME: &str = "ai_consent";
const ENDPOINT_NAME: &str = "ai_endpoint";
const DAILY_LIMIT: u64 = 200;

static DAILY_USAGE: AtomicU64 = AtomicU64::new(0);
static DAILY_USAGE_DATE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize, Deserialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatChoice {
    message: ChatMessageOut,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageOut {
    content: String,
}

#[derive(Debug, Serialize)]
pub struct AiCheckError {
    pub kind: String,
    pub message: String,
}

impl std::fmt::Display for AiCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

/// One batch item for the batched visual check API.
#[derive(Debug, Clone)]
pub struct BatchedPage {
    pub page_index: i64,
    /// Local image path (PNG or JPEG) that will be base64-encoded and sent
    /// to the vision API.
    pub image_path: String,
}

/// Run an AI visual check on a single page image. Returns a structured
/// `AiCheckResult` and never panics on missing configuration.
pub async fn ai_visual_check(
    image_path: &str,
    prompt: &str,
) -> Result<AiCheckResult, String> {
    let items = vec![BatchedPage {
        page_index: 0,
        image_path: image_path.to_string(),
    }];
    let results = ai_visual_check_batched(&items, prompt).await?;
    results
        .into_iter()
        .next()
        .ok_or_else(|| "AI check returned no results".to_string())
}

/// Run an AI visual check on a batch of pages in a single API call. Used by
/// the preflight `MakePDFXWizard` flow to send every page of a multi-page
/// PDF in one request and keep the token cost predictable.
pub async fn ai_visual_check_batched(
    items: &[BatchedPage],
    prompt: &str,
) -> Result<Vec<AiCheckResult>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    ensure_consent()?;
    ensure_daily_quota()?;

    let api_key = read_api_key()?;
    let endpoint = read_endpoint()?;

    let mut message = ChatMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: prompt.to_string(),
        }],
    };
    for item in items {
        let url = encode_image_as_data_url(&item.image_path)?;
        message.content.push(ContentPart::ImageUrl {
            image_url: ImageUrl { url },
        });
    }

    let body = ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![message],
        max_tokens: 600,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let res = client
        .post(&endpoint)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI HTTP request failed: {e}"))?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read AI response: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "AI API returned {}: {}",
            status,
            text.chars().take(400).collect::<String>()
        ));
    }

    let parsed: ChatResponse = serde_json::from_str(&text)
        .map_err(|e| format!("AI response parse error: {e}; body={}", text.chars().take(200).collect::<String>()))?;

    let raw = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    let (summary, issues) = parse_response(&raw);

    increment_daily_usage();

    Ok(items
        .iter()
        .map(|i| AiCheckResult {
            page_index: i.page_index,
            summary: summary.clone(),
            issues: issues.clone(),
            confidence: 0.85,
            raw_response: raw.clone(),
            cached: false,
        })
        .collect())
}

fn ensure_consent() -> Result<(), String> {
    match keychain::read_secret(SERVICE_NAME, CONSENT_NAME) {
        Ok(secret) => {
            if secret.value.as_deref() == Some("1") {
                Ok(())
            } else {
                Err(
                    "AI visual checks require explicit user consent. Enable in preferences."
                        .to_string(),
                )
            }
        }
        Err(_) => Err(
            "AI visual checks require explicit user consent. Enable in preferences.".to_string(),
        ),
    }
}

fn read_api_key() -> Result<String, String> {
    let secret = keychain::read_secret(SERVICE_NAME, API_KEY_NAME)
        .map_err(|e| format!("Failed to read API key: {e}"))?;
    secret.value.ok_or_else(|| {
        "No OpenAI API key configured. Save one in Preferences → AI Visual Check.".to_string()
    })
}

fn read_endpoint() -> Result<String, String> {
    let endpoint = match keychain::read_secret(SERVICE_NAME, ENDPOINT_NAME) {
        Ok(secret) => secret
            .value
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string()),
        Err(_) => "https://api.openai.com/v1/chat/completions".to_string(),
    };
    crate::comm_cmds::validate_command_url(&endpoint)?;
    Ok(endpoint)
}

fn ensure_daily_quota() -> Result<(), String> {
    let today = current_day();
    let stored = DAILY_USAGE_DATE.load(Ordering::Relaxed);
    if stored != today {
        DAILY_USAGE_DATE.store(today, Ordering::Relaxed);
        DAILY_USAGE.store(0, Ordering::Relaxed);
    }
    let used = DAILY_USAGE.load(Ordering::Relaxed);
    if used >= DAILY_LIMIT {
        return Err(format!(
            "Daily AI check limit reached ({DAILY_LIMIT} per day). Try again tomorrow."
        ));
    }
    Ok(())
}

fn increment_daily_usage() {
    DAILY_USAGE.fetch_add(1, Ordering::Relaxed);
}

/// Load a previously persisted daily quota into the static counters.
/// Called at the start of a Tauri command that will call the AI API so the
/// quota is tracked across app restarts via the `preferences` table.
pub fn load_quota_from_prefs(day: u64, count: u64) {
    // Only restore if the persisted day is still today; otherwise the
    // existing day-rollover logic in `ensure_daily_quota` handles the reset.
    let today = current_day();
    if day == today {
        DAILY_USAGE_DATE.store(day, Ordering::Relaxed);
        DAILY_USAGE.store(count, Ordering::Relaxed);
    }
}

/// Snapshot the current in-memory daily usage counters so callers can persist
/// them to the DB after an AI API call.
pub fn quota_snapshot() -> (u64, u64) {
    let day = DAILY_USAGE_DATE.load(Ordering::Relaxed);
    let count = DAILY_USAGE.load(Ordering::Relaxed);
    (day, count)
}

fn current_day() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0)
}

fn encode_image_as_data_url(path: &str) -> Result<String, String> {
    let canonical = crate::security::validate_read_path(path)?;
    let bytes = std::fs::read(&canonical).map_err(|e| format!("Failed to read image: {e}"))?;
    let mime = match canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let b64 = STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

fn parse_response(raw: &str) -> (String, Vec<String>) {
    let mut issues = Vec::new();
    let mut summary_lines: Vec<&str> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('-') || trimmed.starts_with('*') {
            issues.push(trimmed.trim_start_matches(|c: char| c == '-' || c == '*' || c == ' ').to_string());
        } else {
            summary_lines.push(trimmed);
        }
    }
    if summary_lines.is_empty() {
        (raw.to_string(), issues)
    } else {
        (summary_lines.join(" "), issues)
    }
}
