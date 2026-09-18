use super::Config;
use super::ConfigTomlLoadResult;
use super::ManagedFeatures;
use super::resolve_bootstrap_auth_route_config;
use codex_config::types::AuthKeyringBackendKind;
use codex_config::types::AuthCredentialsStoreMode;
use codex_features::Feature;
use codex_features::FeatureConfigSource;
use codex_features::FeatureOverrides;
use codex_features::Features;
use codex_login::AuthConfig;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

const AUTH_ACCOUNTS_DIR: &str = "accounts";
pub const CODEX_AUTH_ACCOUNT_ENV_VAR: &str = "CODEX_INTERNAL_AUTH_ACCOUNT";
static AUTH_ACCOUNT_OVERRIDE: OnceLock<String> = OnceLock::new();

pub fn validate_auth_account_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.trim() != name {
        return Err("account name cannot be empty or have leading/trailing whitespace".to_string());
    }
    if name.chars().count() > 64 {
        return Err("account name cannot exceed 64 characters".to_string());
    }
    if name == "." || name == ".." {
        return Err("account name cannot be `.` or `..`".to_string());
    }
    if name
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err("account name contains a character that is not valid in a file name".to_string());
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("account name cannot end with a dot or space".to_string());
    }
    let windows_stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let reserved = matches!(windows_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || windows_stem
            .strip_prefix("COM")
            .or_else(|| windows_stem.strip_prefix("LPT"))
            .is_some_and(|suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"));
    if reserved {
        return Err("account name is reserved by Windows".to_string());
    }
    Ok(())
}

pub(crate) fn resolve_auth_storage_home(
    codex_home: &Path,
    account: Option<&str>,
) -> std::io::Result<PathBuf> {
    let Some(account) = account else {
        return Ok(codex_home.to_path_buf());
    };
    validate_auth_account_name(account)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    Ok(codex_home.join(AUTH_ACCOUNTS_DIR).join(account))
}

pub fn set_auth_account_override(account: String) -> Result<(), String> {
    validate_auth_account_name(&account)?;
    AUTH_ACCOUNT_OVERRIDE
        .set(account)
        .map_err(|_| "authentication account was already selected for this process".to_string())
}

pub fn auth_account_override_is_set() -> bool {
    AUTH_ACCOUNT_OVERRIDE.get().is_some() || std::env::var_os(CODEX_AUTH_ACCOUNT_ENV_VAR).is_some()
}

pub(crate) fn auth_account_for_process() -> std::io::Result<Option<String>> {
    if let Some(account) = AUTH_ACCOUNT_OVERRIDE.get() {
        return Ok(Some(account.clone()));
    }
    let Some(value) = std::env::var_os(CODEX_AUTH_ACCOUNT_ENV_VAR) else {
        return Ok(None);
    };
    let account = value.into_string().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{CODEX_AUTH_ACCOUNT_ENV_VAR} must contain valid Unicode"),
        )
    })?;
    validate_auth_account_name(&account)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    Ok(Some(account))
}

pub(crate) fn resolve_account_auth_store_mode(
    account_selected: bool,
    managed: Option<AuthCredentialsStoreMode>,
    configured: AuthCredentialsStoreMode,
) -> std::io::Result<AuthCredentialsStoreMode> {
    if !account_selected {
        return Ok(managed.unwrap_or(configured));
    }
    if managed.is_some_and(|mode| mode != AuthCredentialsStoreMode::File) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "--account requires file credential storage, but managed requirements enforce a different storage mode",
        ));
    }
    Ok(AuthCredentialsStoreMode::File)
}

impl Config {
    pub fn auth_storage_home(&self) -> PathBuf {
        resolve_auth_storage_home(&self.codex_home, self.auth_account.as_deref())
            .expect("Config auth account names are validated while loading configuration")
    }

    pub fn auth_keyring_backend_kind(&self) -> AuthKeyringBackendKind {
        auth_keyring_backend_kind_from_secret_auth_storage(
            self.features.enabled(Feature::SecretAuthStorage),
        )
    }

    pub fn auth_config(&self) -> AuthConfig {
        AuthConfig {
            codex_home: self.auth_storage_home(),
            auth_credentials_store_mode: self.cli_auth_credentials_store_mode,
            keyring_backend_kind: self.auth_keyring_backend_kind(),
            forced_login_method: self.forced_login_method,
            chatgpt_base_url: Some(self.chatgpt_base_url.clone()),
            forced_chatgpt_workspace_id: self.forced_chatgpt_workspace_id.clone(),
            managed_auth_policy: self.config_layer_stack.requirements().managed_auth_policy(),
            auth_route_config: self.auth_route_config(),
        }
    }
}

/// Builds authentication settings from the locally resolved bootstrap config.
///
/// Use this before fetching cloud requirements, when a full [`Config`] is not
/// yet available. Preserves the configured credential store, keyring backend,
/// ChatGPT base URL, auth routing, and managed login/workspace restrictions.
pub fn bootstrap_auth_config(
    codex_home: &Path,
    bootstrap_config: &ConfigTomlLoadResult,
) -> std::io::Result<AuthConfig> {
    let config = &bootstrap_config.config_toml;
    let requirements = bootstrap_config.config_layer_stack.requirements();
    // Empty legacy workspace settings mean unrestricted, not an empty allowlist.
    let forced_chatgpt_workspace_id = config
        .forced_chatgpt_workspace_id
        .clone()
        .map(|workspaces| {
            workspaces
                .into_vec()
                .into_iter()
                .map(|workspace| workspace.trim().to_string())
                .filter(|workspace| !workspace.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|workspaces| !workspaces.is_empty());
    let auth_account = auth_account_for_process()?;
    let account_selected = auth_account.is_some();
    let mut auth_config = AuthConfig {
        codex_home: resolve_auth_storage_home(codex_home, auth_account.as_deref())?,
        auth_credentials_store_mode: resolve_account_auth_store_mode(
            account_selected,
            requirements
                .cli_auth_credentials_store
                .as_ref()
                .map(|required| required.value),
            config.cli_auth_credentials_store.unwrap_or_default(),
        )?,
        keyring_backend_kind: resolve_bootstrap_auth_keyring_backend_kind(bootstrap_config)?,
        forced_login_method: config.forced_login_method,
        chatgpt_base_url: config.chatgpt_base_url.clone(),
        forced_chatgpt_workspace_id,
        managed_auth_policy: requirements.managed_auth_policy(),
        auth_route_config: resolve_bootstrap_auth_route_config(
            config,
            requirements.feature_requirements.as_ref(),
        )?,
    };
    if let Some(required) = requirements.chatgpt_base_url.as_ref() {
        auth_config.chatgpt_base_url = Some(required.value.clone());
    }
    auth_config.validate()?;
    Ok(auth_config)
}

/// Resolve the auth keyring backend from a partially loaded bootstrap config.
///
/// This is intended for startup paths that must read auth before managed cloud
/// requirements can be loaded and before a full [`Config`] exists.
pub fn resolve_bootstrap_auth_keyring_backend_kind(
    bootstrap_config: &ConfigTomlLoadResult,
) -> std::io::Result<AuthKeyringBackendKind> {
    let config_toml = &bootstrap_config.config_toml;
    let features = Features::from_sources(
        FeatureConfigSource {
            features: config_toml.features.as_ref(),
            experimental_use_unified_exec_tool: config_toml.experimental_use_unified_exec_tool,
        },
        FeatureConfigSource::default(),
        FeatureOverrides::default(),
    );
    let managed_features = ManagedFeatures::from_configured(
        features,
        bootstrap_config
            .config_layer_stack
            .requirements()
            .feature_requirements
            .clone(),
    )?;
    Ok(auth_keyring_backend_kind_from_secret_auth_storage(
        managed_features.enabled(Feature::SecretAuthStorage),
    ))
}

fn auth_keyring_backend_kind_from_secret_auth_storage(
    secret_auth_storage_enabled: bool,
) -> AuthKeyringBackendKind {
    if secret_auth_storage_enabled {
        AuthKeyringBackendKind::Secrets
    } else {
        AuthKeyringBackendKind::Direct
    }
}

#[cfg(test)]
#[path = "auth_keyring_tests.rs"]
mod tests;
