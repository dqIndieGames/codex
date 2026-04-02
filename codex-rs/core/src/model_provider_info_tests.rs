use super::*;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::AbsolutePathBufGuard;
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;
use std::env;
use std::ffi::OsStr;
use serial_test::serial;
use tempfile::tempdir;

#[test]
fn test_deserialize_ollama_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Ollama"
base_url = "http://localhost:11434/v1"
        "#;
    let expected_provider = ModelProviderInfo {
        name: "Ollama".into(),
        base_url: Some("http://localhost:11434/v1".into()),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_azure_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Azure"
base_url = "https://xxxxx.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
        "#;
    let expected_provider = ModelProviderInfo {
        name: "Azure".into(),
        base_url: Some("https://xxxxx.openai.azure.com/openai".into()),
        env_key: Some("AZURE_OPENAI_API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: Some(maplit::hashmap! {
            "api-version".to_string() => "2025-04-01-preview".to_string(),
        }),
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_example_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Example"
base_url = "https://example.com"
env_key = "API_KEY"
http_headers = { "X-Example-Header" = "example-value" }
env_http_headers = { "X-Example-Env-Header" = "EXAMPLE_ENV_VAR" }
        "#;
    let expected_provider = ModelProviderInfo {
        name: "Example".into(),
        base_url: Some("https://example.com".into()),
        env_key: Some("API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: Some(maplit::hashmap! {
            "X-Example-Header".to_string() => "example-value".to_string(),
        }),
        env_http_headers: Some(maplit::hashmap! {
            "X-Example-Env-Header".to_string() => "EXAMPLE_ENV_VAR".to_string(),
        }),
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_chat_wire_api_shows_helpful_error() {
    let provider_toml = r#"
name = "OpenAI using Chat Completions"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"
        "#;

    let err = toml::from_str::<ModelProviderInfo>(provider_toml).unwrap_err();
    assert!(err.to_string().contains(CHAT_WIRE_API_REMOVED_ERROR));
}

#[test]
fn test_deserialize_websocket_connect_timeout() {
    let provider_toml = r#"
name = "OpenAI"
base_url = "https://api.openai.com/v1"
websocket_connect_timeout_ms = 15000
supports_websockets = true
        "#;

    let provider: ModelProviderInfo = toml::from_str(provider_toml).unwrap();
    assert_eq!(provider.websocket_connect_timeout_ms, Some(15_000));
}

#[test]
fn test_deserialize_provider_auth_config_defaults() {
    let base_dir = tempdir().unwrap();
    let provider_toml = r#"
name = "Corp"

[auth]
command = "./scripts/print-token"
args = ["--format=text"]
        "#;

    let provider: ModelProviderInfo = {
        let _guard = AbsolutePathBufGuard::new(base_dir.path());
        toml::from_str(provider_toml).unwrap()
    };

    assert_eq!(
        provider.auth,
        Some(ModelProviderAuthInfo {
            command: "./scripts/print-token".to_string(),
            args: vec!["--format=text".to_string()],
            timeout_ms: NonZeroU64::new(5_000).unwrap(),
            refresh_interval_ms: NonZeroU64::new(300_000).unwrap(),
            cwd: AbsolutePathBuf::resolve_path_against_base(".", base_dir.path()).unwrap(),
        })
    );
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let original = env::var_os(key);
        unsafe {
            env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.take() {
            Some(value) => unsafe {
                env::set_var(self.key, value);
            },
            None => unsafe {
                env::remove_var(self.key);
            },
        }
    }
}

#[test]
fn retry_mode_from_env_defaults_to_unbounded_without_test_threads() {
    assert_eq!(retry_mode_from_env(None, false), RetryMode::Unbounded);
}

#[test]
fn retry_mode_from_env_defaults_to_bounded_with_rust_test_threads() {
    assert_eq!(retry_mode_from_env(None, true), RetryMode::Bounded);
}

#[test]
#[serial]
fn bounded_retry_mode_preserves_configured_retry_limits() {
    let _guard = EnvVarGuard::set(INTERNAL_RETRY_MODE_ENV, OsStr::new("bounded"));
    let provider = ModelProviderInfo::create_openai_provider(None);

    assert_eq!(provider.request_retry_attempts(), provider.request_max_retries());
    assert_eq!(
        provider.stream_retry_budget(),
        Some(provider.stream_max_retries())
    );
    assert!(!provider.retries_are_unbounded());
}

#[test]
#[serial]
fn unbounded_retry_mode_exposes_local_retry_defaults() {
    let _guard = EnvVarGuard::set(INTERNAL_RETRY_MODE_ENV, OsStr::new("unbounded"));
    let provider = ModelProviderInfo::create_openai_provider(None);
    let api_provider = provider.to_api_provider(None).expect("api provider");

    assert_eq!(provider.request_retry_attempts(), UNBOUNDED_RETRY_ATTEMPTS);
    assert_eq!(provider.stream_retry_budget(), None);
    assert_eq!(
        provider.stream_fallback_retry_threshold(),
        provider.stream_max_retries()
    );
    assert!(provider.retries_are_unbounded());
    assert_eq!(api_provider.retry.max_attempts, UNBOUNDED_RETRY_ATTEMPTS);
    assert!(api_provider.retry.retry_402);
    assert!(api_provider.retry.retry_429);
}

#[test]
#[serial]
fn current_retry_mode_defaults_to_bounded_when_cargo_target_tmpdir_present() {
    let _tmpdir = EnvVarGuard::set("CARGO_TARGET_TMPDIR", OsStr::new("tmp"));
    let _override = EnvVarGuard::set(INTERNAL_RETRY_MODE_ENV, OsStr::new(""));

    assert_eq!(current_retry_mode(), RetryMode::Bounded);
}

#[test]
#[serial]
fn explicit_retry_mode_override_wins_inside_test_harness() {
    let _tmpdir = EnvVarGuard::set("CARGO_TARGET_TMPDIR", OsStr::new("tmp"));
    let _override = EnvVarGuard::set(INTERNAL_RETRY_MODE_ENV, OsStr::new("unbounded"));

    assert_eq!(current_retry_mode(), RetryMode::Unbounded);
}
