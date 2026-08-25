//! Small CLI localization layer. `PA_LANG` wins, followed by the process locale.

fn english() -> bool {
    let locale = std::env::var("PA_LANG")
        .ok()
        .or_else(|| std::env::var("LC_ALL").ok())
        .or_else(|| std::env::var("LC_MESSAGES").ok())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    locale.starts_with("en")
}

pub fn authorization(url: &str, code: &str) -> String {
    if english() {
        format!("Open in your browser:\n  {url}\n  and confirm this code: {code}\n\n  Waiting for confirmation …")
    } else {
        format!("Im Browser öffnen:\n  {url}\n  und diesen Code bestätigen: {code}\n\n  Warte auf Bestätigung …")
    }
}

pub fn authorization_failed(error: &str) -> String {
    if english() {
        format!("Sign-in failed: {error}")
    } else {
        format!("Anmeldung fehlgeschlagen: {error}")
    }
}

pub fn enrollment_failed(status: reqwest::StatusCode) -> String {
    if english() {
        format!("Computer Service enrollment failed (HTTP {status})")
    } else {
        format!("Computer-Service-Registrierung fehlgeschlagen (HTTP {status})")
    }
}

pub fn connected(device: &str, config: &str, workspace: &str) -> String {
    if english() {
        format!("\nConnected ✓ device {device} → {config} (workspace {workspace})")
    } else {
        format!("\nVerbunden ✓ Gerät {device} → {config} (Workspace {workspace})")
    }
}

pub fn start_hint() -> &'static str {
    if english() {
        "Start Computer Service with: pacs run"
    } else {
        "Computer Service starten mit: pacs run"
    }
}
