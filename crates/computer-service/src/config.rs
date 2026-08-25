//! Computer Service config. Per-user enrollment is stored at
//! `~/.config/personal-agent/computer-service/config.toml` (mode 0600 on Unix). If that file is
//! absent, Unix services may load `/etc/personal-agent/computer-service/config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: String,
    pub device_id: String,
    pub workspace: String,
    // Opaque, device-bound Computer Service credential. It is not a user JWT and the backend
    // accepts it only for the service WebSocket and explicitly scoped service endpoints.
    pub service_token: String,
    // Backend-managed cloud sandbox: when set, the agent authenticates with this
    // short-lived internal token over the `sandbox:` subprotocol (no OIDC/refresh).
    #[serde(default)]
    pub sandbox_token: Option<String>,
    // Read-only home-file index root (a SEPARATE surface from `workspace`). Default $HOME.
    // Optional + serde(default) so already-enrolled config.toml files load unchanged.
    #[serde(default)]
    pub home_root: Option<String>,
    // OS-level command sandbox (Linux landlock): confine `run_command` WRITES to the
    // workspace + build caches. Opt-in, fail-safe; default off so existing configs load
    // unchanged and the toolchain is never surprised. See sandbox.rs.
    #[serde(default)]
    pub sandbox: bool,
    // Tool exposure: coding-tool names the user turned OFF for this device (configured from the
    // desktop app). The hello announcement drops them so the backend never offers them. Empty =
    // everything exposed. serde(default) so already-enrolled configs load unchanged.
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    // Whether to expose the read-only $HOME search/read surface (home_index capability). Default
    // true so existing configs keep it; the desktop app can turn it off.
    #[serde(default = "default_true")]
    pub expose_home_index: bool,
    // Path jail (DEVICE-AUTHORITATIVE): when true, every tool path is confined under `workspace`
    // (the default). When false, the agent operates UNJAILED on the full filesystem — the user's
    // explicit local choice. Reported to the backend on connect; the backend mirrors it read-only
    // and can never override it. Default true so an existing config.toml loads as jailed (safe).
    #[serde(default = "default_true")]
    pub jail: bool,
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Sandbox mode — config injected via env by the backend that spawned this container.
    /// No file, no OIDC; the credential is PA_SANDBOX_TOKEN.
    pub fn from_env() -> Option<Config> {
        let token = std::env::var("PA_SANDBOX_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Config {
            server: std::env::var("PA_SERVER").ok()?,
            device_id: std::env::var("PA_DEVICE_ID").ok()?,
            workspace: std::env::var("PA_WORKSPACE").unwrap_or_else(|_| "/workspace".into()),
            service_token: String::new(),
            sandbox_token: Some(token),
            home_root: std::env::var("PA_HOME_ROOT").ok().filter(|s| !s.is_empty()),
            sandbox: matches!(std::env::var("PA_SANDBOX").as_deref(), Ok("1") | Ok("true")),
            disabled_tools: Vec::new(),
            expose_home_index: true,
            // A managed cloud sandbox is already an isolated container → run UNJAILED by default
            // (full FS inside the sandbox). PA_JAIL=1 forces jailing if ever wanted.
            jail: matches!(std::env::var("PA_JAIL").as_deref(), Ok("1") | Ok("true")),
        })
    }

    /// The home-index root: the configured value (expanding a leading `~/`), else $HOME.
    /// `None` only on exotic platforms where the home dir can't be determined.
    pub fn home_root_resolved(&self) -> Option<String> {
        match self.home_root.as_deref().filter(|s| !s.is_empty()) {
            Some("~") => dirs::home_dir().map(|h| h.to_string_lossy().into_owned()),
            Some(raw) => {
                if let Some(rest) = raw.strip_prefix("~/") {
                    dirs::home_dir().map(|h| h.join(rest).to_string_lossy().into_owned())
                } else {
                    Some(raw.to_string())
                }
            }
            None => dirs::home_dir().map(|h| h.to_string_lossy().into_owned()),
        }
    }
}

impl Config {
    /// The device WebSocket URL (http(s) → ws(s)).
    pub fn ws_url(&self) -> String {
        let base = self.server.trim_end_matches('/');
        let base = base
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        format!("{base}/api/v1/ws/device")
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("personal-agent")
        .join("computer-service")
        .join("config.toml")
}

#[cfg(unix)]
pub fn system_config_path() -> Option<PathBuf> {
    Some(PathBuf::from(
        "/etc/personal-agent/computer-service/config.toml",
    ))
}

#[cfg(not(unix))]
pub fn system_config_path() -> Option<PathBuf> {
    None
}

fn select_config_path(user: &Path, system: Option<&Path>) -> Option<PathBuf> {
    if user.is_file() {
        Some(user.to_path_buf())
    } else {
        system.filter(|path| path.is_file()).map(Path::to_path_buf)
    }
}

pub fn active_config_path() -> Option<PathBuf> {
    let user = config_path();
    let system = system_config_path();
    select_config_path(&user, system.as_deref())
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, toml::to_string(cfg)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load() -> Result<Config> {
    let user = config_path();
    let system = system_config_path();
    let Some(path) = active_config_path() else {
        let checked = system.map_or_else(
            || user.display().to_string(),
            |system| format!("{} or {}", user.display(), system.display()),
        );
        bail!("not enrolled — run `pacs enroll` first (checked {checked})");
    };
    Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
}

#[cfg(test)]
mod tests {
    use super::select_config_path;

    #[test]
    fn user_config_has_priority_over_system_config() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user.toml");
        let system = tmp.path().join("system.toml");
        std::fs::write(&user, "user").unwrap();
        std::fs::write(&system, "system").unwrap();
        assert_eq!(select_config_path(&user, Some(&system)), Some(user));
    }

    #[test]
    fn system_config_is_used_only_when_user_config_is_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user.toml");
        let system = tmp.path().join("system.toml");
        std::fs::write(&system, "system").unwrap();
        assert_eq!(select_config_path(&user, Some(&system)), Some(system));
    }

    #[test]
    fn missing_configs_do_not_resolve() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            select_config_path(
                &tmp.path().join("user.toml"),
                Some(&tmp.path().join("system.toml"))
            ),
            None
        );
    }
}
