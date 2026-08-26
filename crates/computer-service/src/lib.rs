//! Personal Agent Computer Service. It connects a computer to Personal Agent and exposes
//! explicitly enabled tools and capabilities. It never stores a user/chat credential.

mod client;
mod config;
mod credential;
mod i18n;
mod jail;
mod lsp;
mod oidc;
mod proc;
mod pty;
mod sandbox;

use anyhow::Result;

use config::Config;

fn problem_detail(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("detail")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(500).collect())
}

async fn enrollment_error(response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = problem_detail(&body);
    i18n::enrollment_failed(status, detail.as_deref())
}

#[derive(serde::Deserialize)]
struct Enrollment {
    token: String,
}

#[derive(serde::Deserialize)]
struct SelfEnrollment {
    device_id: String,
    token: String,
}

/// A name for THIS machine, for the device list in the web UI.
///
/// Read without a dependency, because the value is cosmetic: the backend falls back to a
/// generic name when this comes back empty, and the user renames the device in Settings if the
/// hostname is not what they want to see. Never a reason to fail an enrollment.
fn machine_name() -> String {
    let from_env = std::env::var("COMPUTERNAME") // Windows
        .or_else(|_| std::env::var("HOSTNAME")) // most Unix shells export it
        .ok()
        .filter(|v| !v.trim().is_empty());
    from_env
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_default()
}

/// Register this machine as a NEW device and get its credential in one call.
///
/// The alternative was making the user create the device in Settings and paste its UUID into
/// the install command -- the id of a row they had to make by hand for the machine they were
/// sitting at (personal-agent-org/personal-agent#122).
async fn register_this_machine(server: &str, user_access_token: &str) -> Result<(String, String)> {
    let url = format!(
        "{}/api/v1/devices/computer-service/enroll",
        server.trim_end_matches('/')
    );
    let response = pa_oidc::tls::http_client()
        .post(url)
        .bearer_auth(user_access_token)
        .json(&serde_json::json!({ "name": machine_name() }))
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!(enrollment_error(response).await);
    }
    let enrollment: SelfEnrollment = response.json().await?;
    Ok((enrollment.device_id, enrollment.token))
}

/// The device this machine is ALREADY enrolled as with this server, if any.
///
/// Re-running the installer must not litter the device list with a new row every time, and it
/// must not take the old one offline either -- so a machine that already knows its id renews
/// that one. Matched on the server too: pointing an existing install at a different backend is
/// a new device there, not a rotation of an id that backend has never seen.
fn already_enrolled_with(server: &str) -> Option<String> {
    let cfg = config::load().ok()?;
    (cfg.server.trim_end_matches('/') == server.trim_end_matches('/')).then_some(cfg.device_id)
}

async fn exchange_for_service_token(
    server: &str,
    device: &str,
    user_access_token: &str,
) -> Result<String> {
    let url = format!(
        "{}/api/v1/devices/{}/computer-service/enroll",
        server.trim_end_matches('/'),
        device
    );
    let response = pa_oidc::tls::http_client()
        .post(url)
        .bearer_auth(user_access_token)
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!(enrollment_error(response).await);
    }
    let enrollment: Enrollment = response.json().await?;
    Ok(enrollment.token)
}

/// Log in via the discovered device flow (external OIDC or backend-local auth), exchange the
/// short-lived user credential for a dedicated service token, and store only that token.
///
/// Takes no device id: this machine registers itself, or renews the device it is already
/// enrolled as. There is deliberately no flag for it -- an optional one still teaches people
/// that a UUID is part of installing (personal-agent-org/personal-agent#122).
pub async fn enroll(server: String, workspace: String) -> Result<()> {
    let abs = std::fs::canonicalize(&workspace)
        .map(|p| p.display().to_string())
        .unwrap_or(workspace);
    let disco = pa_oidc::discover(&server).await?;
    let client = disco.device_client_id;
    // The user token exists only long enough to prove ownership and exchange it for a dedicated,
    // device-bound Computer Service token. Neither access nor refresh token is persisted.
    let user_tokens = oidc::device_login(&disco.endpoints, &client).await?;
    let (device, service_token) = match already_enrolled_with(&server) {
        Some(id) => {
            let token = exchange_for_service_token(&server, &id, &user_tokens.access_token).await?;
            (id, token)
        }
        None => register_this_machine(&server, &user_tokens.access_token).await?,
    };
    let cfg = Config {
        server,
        device_id: device,
        workspace: abs,
        service_token,
        sandbox_token: None,
        home_root: None, // default $HOME at runtime via home_root_resolved()
        sandbox: false,  // opt-in OS command sandbox; user enables in config.toml
        disabled_tools: Vec::new(), // everything exposed until the desktop app narrows it
        expose_home_index: true,
        jail: true, // jailed by default; user sets `jail = false` in config.toml
    };
    config::save(&cfg)?;
    println!(
        "{}",
        i18n::connected(
            &cfg.device_id,
            &config::config_path().display().to_string(),
            &cfg.workspace,
        )
    );
    println!("{}", i18n::start_hint());
    Ok(())
}

/// Connect and serve tool calls.
pub async fn run() -> Result<()> {
    // A backend-spawned cloud sandbox injects its config via env (PA_SANDBOX_TOKEN);
    // a normal device reads the enrolled config file.
    let cfg = match Config::from_env() {
        Some(c) => c,
        None => config::load()?,
    };
    client::run(cfg).await
}

/// Print the available coding tools (name + description) as JSON. The desktop app uses this
/// to render the per-device tool-exposure toggles.
pub async fn tools() -> Result<()> {
    println!("{}", jail::tool_specs());
    Ok(())
}

/// Git credential helper (invoked by git for Personal-Agent-cloned repos). Internal.
pub async fn credential_helper(operation: &str) -> Result<()> {
    credential::run(operation).await
}

#[cfg(test)]
mod enroll_tests {
    use super::{machine_name, problem_detail};

    #[test]
    fn enrollment_errors_expose_only_the_problem_detail() {
        assert_eq!(
            problem_detail(r#"{"title":"Unauthorized","detail":"invalid audience"}"#),
            Some("invalid audience".to_string())
        );
        assert_eq!(problem_detail("<html>upstream failed</html>"), None);
        assert_eq!(problem_detail(r#"{"detail":["not a safe string"]}"#), None);
    }

    /// The name is cosmetic and must never be the reason an enrollment fails: the backend
    /// falls back to a generic one, and the user renames the device in Settings.
    #[test]
    fn a_machine_with_no_discoverable_name_still_yields_a_string() {
        // Nothing is asserted about the CONTENT -- on a container /etc/hostname is a random
        // hex id, in CI it may be unset entirely. Only that it never panics and never returns
        // something that would break the JSON body.
        let name = machine_name();
        assert!(!name.contains('\0'));
    }

    #[test]
    fn the_name_carries_no_surrounding_whitespace() {
        // /etc/hostname ends with a newline, and an untrimmed one would show up in the device
        // list as a name with a blank line in it.
        let name = machine_name();
        assert_eq!(name.trim(), name);
    }
}
