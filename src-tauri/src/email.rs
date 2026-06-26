use crate::keychain;
use crate::models::EmailSettings;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::path::Path;

/// Send an email via SMTP using the persisted `EmailSettings`. The SMTP
/// password is read from the OS keychain at call time, never logged.
pub fn send_email_via_smtp(
    settings: &EmailSettings,
    to: &str,
    subject: &str,
    body: &str,
    attachment_path: Option<&str>,
) -> Result<(), String> {
    if settings.smtp_host.trim().is_empty() {
        return Err("SMTP host not configured".to_string());
    }
    if settings.from_address.trim().is_empty() {
        return Err("From address not configured".to_string());
    }
    if to.trim().is_empty() {
        return Err("Recipient address is required".to_string());
    }

    let password = if settings.smtp_password.is_empty() {
        match keychain::read_secret("frappe-email", "smtp_password") {
            Ok(secret) => secret.value.unwrap_or_default(),
            Err(e) => {
                return Err(format!("Failed to read SMTP password from keychain: {e}"));
            }
        }
    } else {
        settings.smtp_password.clone()
    };

    if password.is_empty() && !settings.smtp_username.is_empty() {
        return Err(
            "SMTP password is required when username is set. Save it in the keychain.".to_string(),
        );
    }

    let from = settings
        .from_address
        .parse()
        .map_err(|e| format!("Invalid from address: {e}"))?;
    let to_addr = to
        .parse()
        .map_err(|e| format!("Invalid recipient address: {e}"))?;

    let builder = Message::builder().from(from).to(to_addr);
    let builder = builder.subject(subject.to_string());

    let message = if let Some(path) = attachment_path {
        let p = Path::new(path);
        if !p.exists() {
            return Err(format!("Attachment file not found: {path}"));
        }
        let bytes = std::fs::read(p).map_err(|e| format!("Failed to read attachment: {e}"))?;
        let filename = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment.pdf")
            .to_string();
        let content_type = guess_content_type(p);
        let attachment = Attachment::new(filename).body(bytes, content_type);
        builder
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(body.to_string()))
                    .singlepart(attachment),
            )
            .map_err(|e| format!("Failed to build message: {e}"))?
    } else {
        builder
            .body(body.to_string())
            .map_err(|e| format!("Failed to build message: {e}"))?
    };

    let mut transport_builder =
        SmtpTransport::relay(&settings.smtp_host).map_err(|e| format!("SMTP relay: {e}"))?;
    transport_builder = transport_builder.port(settings.smtp_port);
    if settings.use_tls {
        let tls = lettre::transport::smtp::client::TlsParameters::new(settings.smtp_host.clone())
            .map_err(|e| format!("TLS parameters: {e}"))?;
        transport_builder =
            transport_builder.tls(lettre::transport::smtp::client::Tls::Required(tls));
    } else {
        transport_builder = transport_builder.tls(lettre::transport::smtp::client::Tls::None);
    }
    if !settings.smtp_username.is_empty() {
        transport_builder = transport_builder
            .credentials(Credentials::new(settings.smtp_username.clone(), password));
    }
    let transport = transport_builder.build();

    transport
        .send(&message)
        .map_err(|e| format!("SMTP send failed: {e}"))?;

    Ok(())
}

fn guess_content_type(p: &Path) -> ContentType {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let mime_str = match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "txt" => "text/plain; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    };
    ContentType::parse(mime_str).unwrap_or_else(|_| {
        ContentType::parse("application/octet-stream").expect("static mime is valid")
    })
}
