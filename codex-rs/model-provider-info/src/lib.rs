//! Registry of model providers supported by Codex.
//!
//! Providers can be defined in two places:
//!   1. Built-in defaults compiled into the binary so Codex works out-of-the-box.
//!   2. User-defined entries inside `~/.codex/config.toml` under the `model_providers`
//!      key. These override or extend the defaults at runtime.

use codex_api::Provider as ApiProvider;
use codex_api::RetryConfig as ApiRetryConfig;
use codex_protocol::auth::AuthMode;
use codex_protocol::config_types::ModelProviderAuthInfo;
use codex_protocol::error::CodexErr;
use codex_protocol::error::EnvVarError;
use codex_protocol::error::Result as CodexResult;
use codex_utils_redacted_string::RedactedString;
use http::HeaderMap;
use http::header::HeaderName;
use http::header::HeaderValue;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;
use std::time::Duration;

/// Wait for HTTP response headers on one streaming attempt.
pub const HEADER_WAIT_TIMEOUT_MS: u64 = 60_000;
/// Wait for the first model event after the stream is open (6.5 minutes).
pub const FIRST_MODEL_EVENT_TIMEOUT_MS: u64 = 390_000;
/// Idle after at least one model event has arrived.
pub const POST_OUTPUT_IDLE_TIMEOUT_MS: u64 = 60_000;
/// Cap for unary compact / realtime / WebRTC connect (same as first-event).
pub const MAX_MODEL_NETWORK_ATTEMPT_TIMEOUT_MS: u64 = FIRST_MODEL_EVENT_TIMEOUT_MS;
const DEFAULT_STREAM_IDLE_TIMEOUT_MS: u64 = POST_OUTPUT_IDLE_TIMEOUT_MS;
const DEFAULT_STREAM_MAX_RETRIES: u64 = 5;
const DEFAULT_REQUEST_MAX_RETRIES: u64 = 4;
const DEFAULT_AWS_AUTH_REFRESH_TIMEOUT_MS: u64 = 300_000;
pub const DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS: u64 = 15_000;
/// Hard cap for user-configured `stream_max_retries`.
const MAX_STREAM_MAX_RETRIES: u64 = 100;
/// Hard cap for user-configured `request_max_retries`.
const MAX_REQUEST_MAX_RETRIES: u64 = 100;
const UNBOUNDED_RETRY_ATTEMPTS: u64 = u64::MAX;
const INTERNAL_RETRY_MODE_ENV: &str = "CODEX_INTERNAL_RETRY_MODE";

const OPENAI_PROVIDER_NAME: &str = "OpenAI";
const OPENAI_ACTOR_AUTHORIZATION_HEADER: &str = "x-openai-actor-authorization";
pub const OPENAI_PROVIDER_ID: &str = "openai";
pub const CHATGPT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const AMAZON_BEDROCK_PROVIDER_NAME: &str = "Amazon Bedrock";
pub const AMAZON_BEDROCK_PROVIDER_ID: &str = "amazon-bedrock";
const AMAZON_BEDROCK_RUNTIME_PROVIDER_NAME: &str = "Amazon Bedrock Runtime";
pub const AMAZON_BEDROCK_RUNTIME_PROVIDER_ID: &str = "amazon-bedrock-runtime";
pub const AMAZON_BEDROCK_GPT_5_5_MODEL_ID: &str = "openai.gpt-5.5";
pub const AMAZON_BEDROCK_GPT_5_4_MODEL_ID: &str = "openai.gpt-5.4";
pub const AMAZON_BEDROCK_GPT_5_6_SOL_MODEL_ID: &str = "openai.gpt-5.6-sol";
pub const AMAZON_BEDROCK_GPT_6_ASTRA_MODEL_ID: &str = "openai.gpt-6-astra";
pub const AMAZON_BEDROCK_GPT_5_6_TERRA_MODEL_ID: &str = "openai.gpt-5.6-terra";
pub const AMAZON_BEDROCK_GPT_5_6_LUNA_MODEL_ID: &str = "openai.gpt-5.6-luna";
pub const AMAZON_BEDROCK_RUNTIME_GLOBAL_GPT_5_6_TERRA_MODEL_ID: &str =
    "global.openai.gpt-5.6-terra";
pub const AMAZON_BEDROCK_RUNTIME_GLOBAL_GPT_5_6_LUNA_MODEL_ID: &str = "global.openai.gpt-5.6-luna";
pub const AMAZON_BEDROCK_DEFAULT_BASE_URL: &str =
    "https://bedrock-mantle.us-east-1.api.aws/openai/v1";
const AMAZON_BEDROCK_MANTLE_CLIENT_AGENT_HEADER: &str = "x-amzn-mantle-client-agent";
const AMAZON_BEDROCK_MANTLE_CLIENT_AGENT_VALUE: &str = "codex";
const CHAT_WIRE_API_REMOVED_ERROR: &str = "`wire_api = \"chat\"` is no longer supported.\nHow to fix: set `wire_api = \"responses\"` in your provider config.\nMore info: https://github.com/openai/codex/discussions/7782";
pub const LEGACY_OLLAMA_CHAT_PROVIDER_ID: &str = "ollama-chat";
pub const OLLAMA_CHAT_PROVIDER_REMOVED_ERROR: &str = "`ollama-chat` is no longer supported.\nHow to fix: replace `ollama-chat` with `ollama` in `model_provider`, `oss_provider`, or `--local-provider`.\nMore info: https://github.com/openai/codex/discussions/7782";

pub fn is_chatgpt_codex_base_url(base_url: &str) -> bool {
    codex_api::is_chatgpt_codex_route(base_url)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetryMode {
    Bounded,
    Unbounded,
}

fn retry_mode_from_env(env_override: Option<&str>, rust_test_threads_present: bool) -> RetryMode {
    match env_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) if value.eq_ignore_ascii_case("bounded") => RetryMode::Bounded,
        Some(value) if value.eq_ignore_ascii_case("unbounded") => RetryMode::Unbounded,
        _ if rust_test_threads_present => RetryMode::Bounded,
        _ => RetryMode::Unbounded,
    }
}

fn running_under_test_harness() -> bool {
    cfg!(test)
        || std::env::var_os("RUST_TEST_THREADS").is_some()
        || std::env::var_os("NEXTEST").is_some()
        || std::env::var_os("CARGO_TARGET_TMPDIR").is_some()
}

fn current_retry_mode() -> RetryMode {
    let env_override = std::env::var(INTERNAL_RETRY_MODE_ENV).ok();
    retry_mode_from_env(env_override.as_deref(), running_under_test_harness())
}

/// Wire protocol that the provider speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    /// The Responses API exposed by OpenAI at `/v1/responses`.
    #[default]
    Responses,
}

impl fmt::Display for WireApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Responses => "responses",
        };
        f.write_str(value)
    }
}

impl<'de> Deserialize<'de> for WireApi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "responses" => Ok(Self::Responses),
            "chat" => Err(serde::de::Error::custom(CHAT_WIRE_API_REMOVED_ERROR)),
            _ => Err(serde::de::Error::unknown_variant(&value, &["responses"])),
        }
    }
}

/// Serializable representation of a provider definition.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ModelProviderInfo {
    /// Friendly display name.
    #[serde(default)]
    pub name: String,
    /// Base URL for the provider's OpenAI-compatible API.
    pub base_url: Option<String>,
    /// Environment variable that stores the user's API key for this provider.
    pub env_key: Option<String>,

    /// Optional instructions to help the user get a valid value for the
    /// variable and set it.
    pub env_key_instructions: Option<String>,
    /// Value to use with `Authorization: Bearer <token>` header. Use of this
    /// config is discouraged in favor of `env_key` for security reasons, but
    /// this may be necessary when using this programmatically.
    pub experimental_bearer_token: Option<RedactedString>,
    /// Command-backed bearer-token configuration for this provider.
    pub auth: Option<ModelProviderAuthInfo>,
    /// AWS SigV4 auth configuration for this provider.
    pub aws: Option<ModelProviderAwsAuthInfo>,
    /// Which wire protocol this provider expects.
    #[serde(default)]
    pub wire_api: WireApi,
    /// Optional query parameters to append to the base URL.
    pub query_params: Option<HashMap<String, RedactedString>>,
    /// Additional HTTP headers to include in requests to this provider where
    /// the (key, value) pairs are the header name and value.
    pub http_headers: Option<HashMap<String, RedactedString>>,
    /// Optional HTTP headers to include in requests to this provider where the
    /// (key, value) pairs are the header name and _environment variable_ whose
    /// value should be used. If the environment variable is not set, or the
    /// value is empty, the header will not be included in the request.
    pub env_http_headers: Option<HashMap<String, String>>,
    /// Maximum number of times to retry a failed HTTP request to this provider.
    pub request_max_retries: Option<u64>,
    /// Number of times to retry reconnecting a dropped streaming response before failing.
    pub stream_max_retries: Option<u64>,
    /// Idle timeout in milliseconds after the first model event (phase 3).
    /// Default 60s; an explicit shorter value can fire earlier. This does not
    /// cap the 390s first-event thinking window.
    pub stream_idle_timeout_ms: Option<u64>,
    /// Maximum time (in milliseconds) to wait for a websocket connection attempt before treating
    /// it as failed.
    pub websocket_connect_timeout_ms: Option<u64>,
    /// Does this provider require an OpenAI API Key or ChatGPT login token? If true,
    /// user is presented with login screen on first run, and login preference and token/key
    /// are stored in auth.json. If false (which is the default), login screen is skipped,
    /// and API key (if needed) comes from the "env_key" environment variable.
    #[serde(default)]
    pub requires_openai_auth: bool,
    /// Whether this provider supports the Responses API WebSocket transport.
    #[serde(default)]
    pub supports_websockets: bool,
    /// Whether this provider supports the standalone web-search endpoint.
    #[serde(default)]
    pub supports_standalone_web_search: bool,
}

/// AWS SigV4 auth configuration for a model provider.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ModelProviderAwsAuthInfo {
    /// AWS profile name to use. When unset, the AWS SDK default chain decides.
    pub profile: Option<String>,
    /// AWS region to use for provider-specific endpoints.
    pub region: Option<String>,
    /// Optional command used to reauthenticate after a refreshable AWS auth failure.
    pub auth_refresh: Option<AwsAuthRefreshConfig>,
}

/// Command used to refresh AWS credentials for a model provider.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AwsAuthRefreshConfig {
    /// Executable to invoke directly, without a shell.
    pub command: String,
    /// Arguments passed to the refresh command.
    #[serde(default)]
    pub args: Vec<RedactedString>,
    /// Maximum time to wait for the refresh command to complete.
    #[serde(default = "default_aws_auth_refresh_timeout_ms")]
    pub timeout_ms: NonZeroU64,
}

impl AwsAuthRefreshConfig {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.get())
    }
}

fn default_aws_auth_refresh_timeout_ms() -> NonZeroU64 {
    match NonZeroU64::new(DEFAULT_AWS_AUTH_REFRESH_TIMEOUT_MS) {
        Some(timeout_ms) => timeout_ms,
        None => panic!("AWS auth refresh timeout must be non-zero"),
    }
}

impl ModelProviderInfo {
    /// Returns the configured provider-scoped bearer token when it is non-empty.
    ///
    /// Whitespace-only values are treated as absent so a config typo does not
    /// produce a broken `Authorization: Bearer ` request or accidentally shadow
    /// the normal AuthManager path.
    pub fn experimental_bearer_token_non_empty(&self) -> Option<String> {
        self.experimental_bearer_token
            .as_ref()
            .map(|token| token.trim())
            .filter(|token| !token.is_empty())
            .map(str::to_string)
    }

    pub fn experimental_bearer_token_is_non_empty(&self) -> bool {
        self.experimental_bearer_token_non_empty().is_some()
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.aws.is_some() {
            if self.supports_websockets {
                // TODO(celia-oai): Support AWS SigV4 signing for WebSocket
                // upgrade requests before allowing AWS-authenticated providers
                // to enable Responses-over-WebSocket.
                return Err("provider aws cannot be combined with supports_websockets".to_string());
            }

            let mut conflicts = Vec::new();
            if self.env_key.is_some() {
                conflicts.push("env_key");
            }
            if self.experimental_bearer_token.is_some() {
                conflicts.push("experimental_bearer_token");
            }
            if self.auth.is_some() {
                conflicts.push("auth");
            }
            if self.requires_openai_auth {
                conflicts.push("requires_openai_auth");
            }

            if !conflicts.is_empty() {
                return Err(format!(
                    "provider aws cannot be combined with {}",
                    conflicts.join(", ")
                ));
            }

            if let Some(auth_refresh) = self.aws.as_ref().and_then(|aws| aws.auth_refresh.as_ref())
            {
                if auth_refresh.command.trim().is_empty() {
                    return Err("provider aws.auth_refresh.command must not be empty".to_string());
                }
                if auth_refresh.command != "aws" {
                    return Err("provider aws.auth_refresh.command must be `aws`".to_string());
                }
            }
        }

        let Some(auth) = self.auth.as_ref() else {
            return Ok(());
        };

        if auth.command.trim().is_empty() {
            return Err("provider auth.command must not be empty".to_string());
        }

        let mut conflicts = Vec::new();
        if self.env_key.is_some() {
            conflicts.push("env_key");
        }
        if self.experimental_bearer_token.is_some() {
            conflicts.push("experimental_bearer_token");
        }
        if self.requires_openai_auth {
            conflicts.push("requires_openai_auth");
        }

        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "provider auth cannot be combined with {}",
                conflicts.join(", ")
            ))
        }
    }

    fn build_header_map(&self) -> CodexResult<HeaderMap> {
        let capacity = self.http_headers.as_ref().map_or(0, HashMap::len)
            + self.env_http_headers.as_ref().map_or(0, HashMap::len);
        let mut headers = HeaderMap::with_capacity(capacity);
        if let Some(extra) = &self.http_headers {
            for (k, v) in extra {
                if let (Ok(name), Ok(value)) =
                    (HeaderName::try_from(k), HeaderValue::try_from(v.as_str()))
                {
                    headers.insert(name, value);
                }
            }
        }

        if let Some(env_headers) = &self.env_http_headers {
            for (header, env_var) in env_headers {
                if let Ok(val) = std::env::var(env_var)
                    && !val.trim().is_empty()
                    && let (Ok(name), Ok(value)) =
                        (HeaderName::try_from(header), HeaderValue::try_from(val))
                {
                    headers.insert(name, value);
                }
            }
        }

        Ok(headers)
    }

    pub fn to_api_provider(&self, auth_mode: Option<AuthMode>) -> CodexResult<ApiProvider> {
        let auth_mode = if self.experimental_bearer_token_is_non_empty() {
            None
        } else {
            auth_mode
        };
        let default_base_url = if matches!(
            auth_mode,
            Some(
                AuthMode::Chatgpt
                    | AuthMode::ChatgptAuthTokens
                    | AuthMode::Headers
                    | AuthMode::AgentIdentity
                    | AuthMode::PersonalAccessToken
            )
        ) {
            CHATGPT_CODEX_BASE_URL
        } else {
            "https://api.openai.com/v1"
        };
        let base_url = self
            .base_url
            .clone()
            .unwrap_or_else(|| default_base_url.to_string());

        let headers = self.build_header_map()?;
        let retry = ApiRetryConfig {
            max_attempts: self.request_retry_attempts(),
            retry_402: true,
            retry_429: true,
            retry_5xx: true,
            retry_transport: true,
        };

        Ok(ApiProvider {
            name: self.name.clone(),
            base_url,
            query_params: self.query_params.clone().map(|params| {
                params
                    .into_iter()
                    .map(|(name, value)| (name, value.into_inner()))
                    .collect()
            }),
            headers,
            retry,
            stream_idle_timeout: self.stream_idle_timeout(),
            first_model_event_timeout: self.first_model_event_timeout(),
        })
    }

    /// If `env_key` is Some, returns the API key for this provider if present
    /// (and non-empty) in the environment. If `env_key` is required but
    /// cannot be found, returns an error.
    pub fn api_key(&self) -> CodexResult<Option<String>> {
        match &self.env_key {
            Some(env_key) => {
                let api_key = std::env::var(env_key)
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .ok_or_else(|| {
                        CodexErr::EnvVar(EnvVarError {
                            var: env_key.clone(),
                            instructions: self.env_key_instructions.clone(),
                        })
                    })?;
                Ok(Some(api_key))
            }
            None => Ok(None),
        }
    }

    /// Effective maximum number of request retries for this provider.
    pub fn request_max_retries(&self) -> u64 {
        self.request_max_retries
            .unwrap_or(DEFAULT_REQUEST_MAX_RETRIES)
            .min(MAX_REQUEST_MAX_RETRIES)
    }

    /// Effective request retry attempts for the current runtime mode.
    pub fn request_retry_attempts(&self) -> u64 {
        if self.request_max_retries == Some(0) {
            return 0;
        }
        match current_retry_mode() {
            RetryMode::Bounded => self.request_max_retries(),
            RetryMode::Unbounded => UNBOUNDED_RETRY_ATTEMPTS,
        }
    }

    /// Effective maximum number of stream reconnection attempts for this provider.
    pub fn stream_max_retries(&self) -> u64 {
        self.stream_max_retries
            .unwrap_or(DEFAULT_STREAM_MAX_RETRIES)
            .min(MAX_STREAM_MAX_RETRIES)
    }

    /// Retry budget that can terminate a turn/compact retry loop in the current runtime mode.
    pub fn stream_retry_budget(&self) -> Option<u64> {
        if self.stream_max_retries == Some(0) {
            return Some(0);
        }
        match current_retry_mode() {
            RetryMode::Bounded => Some(self.stream_max_retries()),
            RetryMode::Unbounded => None,
        }
    }

    /// WebSocket fallback still uses the configured threshold even when retries are unbounded.
    pub fn stream_fallback_retry_threshold(&self) -> u64 {
        self.stream_max_retries()
    }

    pub fn retries_are_unbounded(&self) -> bool {
        matches!(current_retry_mode(), RetryMode::Unbounded)
    }

    /// Effective idle timeout after the first model event (phase 3).
    ///
    /// Explicit shorter `stream_idle_timeout_ms` can fire earlier; the default
    /// must not extend past 60s or cut the 390s first-event window.
    pub fn stream_idle_timeout(&self) -> Duration {
        Duration::from_millis(
            self.stream_idle_timeout_ms
                .unwrap_or(DEFAULT_STREAM_IDLE_TIMEOUT_MS)
                .min(POST_OUTPUT_IDLE_TIMEOUT_MS),
        )
    }

    /// Effective wait for the first model event after the stream is open.
    ///
    /// Phase 2 is the 390s thinking window. `stream_idle_timeout_ms` is an idle
    /// timeout for other phases and must not cut this window (including legacy
    /// `300000` / 5 minute configs).
    pub fn first_model_event_timeout(&self) -> Duration {
        Duration::from_millis(FIRST_MODEL_EVENT_TIMEOUT_MS)
    }

    /// Effective wait for HTTP response headers on one streaming attempt.
    pub fn header_wait_timeout(&self) -> Duration {
        Duration::from_millis(
            self.stream_idle_timeout_ms
                .unwrap_or(HEADER_WAIT_TIMEOUT_MS)
                .min(HEADER_WAIT_TIMEOUT_MS),
        )
    }

    /// Effective timeout for websocket connect attempts.
    pub fn websocket_connect_timeout(&self) -> Duration {
        Duration::from_millis(
            self.websocket_connect_timeout_ms
                .unwrap_or(DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS)
                .min(MAX_MODEL_NETWORK_ATTEMPT_TIMEOUT_MS),
        )
    }

    pub fn create_openai_provider(base_url: Option<String>) -> ModelProviderInfo {
        ModelProviderInfo {
            name: OPENAI_PROVIDER_NAME.into(),
            base_url,
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: Some(
                [("version".to_string(), env!("CARGO_PKG_VERSION").into())]
                    .into_iter()
                    .collect(),
            ),
            env_http_headers: Some(
                [
                    (
                        "OpenAI-Organization".to_string(),
                        "OPENAI_ORGANIZATION".to_string(),
                    ),
                    ("OpenAI-Project".to_string(), "OPENAI_PROJECT".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
            // Use global defaults for retry/timeout unless overridden in config.toml.
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: true,
            supports_websockets: true,
            supports_standalone_web_search: true,
        }
    }

    pub fn create_amazon_bedrock_provider(
        aws: Option<ModelProviderAwsAuthInfo>,
    ) -> ModelProviderInfo {
        ModelProviderInfo {
            name: AMAZON_BEDROCK_PROVIDER_NAME.into(),
            // The runtime provider derives the regional Mantle endpoint when
            // this is unset. A configured value is therefore unambiguously an
            // endpoint override.
            base_url: None,
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: Some(aws.unwrap_or(ModelProviderAwsAuthInfo {
                profile: None,
                region: None,
                auth_refresh: None,
            })),
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: Some(HashMap::from([(
                AMAZON_BEDROCK_MANTLE_CLIENT_AGENT_HEADER.to_string(),
                AMAZON_BEDROCK_MANTLE_CLIENT_AGENT_VALUE.into(),
            )])),
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            supports_standalone_web_search: false,
        }
    }

    pub fn create_amazon_bedrock_runtime_provider(
        aws: Option<ModelProviderAwsAuthInfo>,
    ) -> ModelProviderInfo {
        let mut provider = Self::create_amazon_bedrock_provider(aws);
        provider.name = AMAZON_BEDROCK_RUNTIME_PROVIDER_NAME.into();
        provider.http_headers = None;
        provider
    }

    pub fn is_openai(&self) -> bool {
        self.name == OPENAI_PROVIDER_NAME
    }

    pub fn supports_codex_backend_routes(&self) -> bool {
        self.is_openai()
            && self.base_url.as_deref().is_none_or(|base_url| {
                base_url
                    .trim_end_matches('/')
                    .ends_with("/backend-api/codex")
            })
    }

    pub fn uses_openai_actor_authorization(&self) -> bool {
        !self.requires_openai_auth
            && self.http_headers.as_ref().is_some_and(|headers| {
                headers.iter().any(|(name, value)| {
                    name.eq_ignore_ascii_case(OPENAI_ACTOR_AUTHORIZATION_HEADER)
                        && !value.trim().is_empty()
                })
            })
    }

    pub fn is_amazon_bedrock(&self) -> bool {
        self.name == AMAZON_BEDROCK_PROVIDER_NAME
            || self.name == AMAZON_BEDROCK_RUNTIME_PROVIDER_NAME
    }

    pub fn is_amazon_bedrock_runtime(&self) -> bool {
        self.name == AMAZON_BEDROCK_RUNTIME_PROVIDER_NAME
    }

    pub fn has_command_auth(&self) -> bool {
        self.auth.is_some()
    }
}

pub const DEFAULT_LMSTUDIO_PORT: u16 = 1234;
pub const DEFAULT_OLLAMA_PORT: u16 = 11434;

pub const LMSTUDIO_OSS_PROVIDER_ID: &str = "lmstudio";
pub const OLLAMA_OSS_PROVIDER_ID: &str = "ollama";

/// Built-in default provider list.
pub fn built_in_model_providers(
    openai_base_url: Option<String>,
) -> HashMap<String, ModelProviderInfo> {
    use ModelProviderInfo as P;
    let openai_provider = P::create_openai_provider(openai_base_url);
    let amazon_bedrock_provider = P::create_amazon_bedrock_provider(/*aws*/ None);
    let amazon_bedrock_runtime_provider =
        P::create_amazon_bedrock_runtime_provider(/*aws*/ None);

    // We do not want to be in the business of adjucating which third-party
    // providers are bundled with Codex CLI, so we only include the OpenAI and
    // open source ("oss") providers by default. Users are encouraged to add to
    // `model_providers` in config.toml to add their own providers.
    [
        (OPENAI_PROVIDER_ID, openai_provider),
        (AMAZON_BEDROCK_PROVIDER_ID, amazon_bedrock_provider),
        (
            AMAZON_BEDROCK_RUNTIME_PROVIDER_ID,
            amazon_bedrock_runtime_provider,
        ),
        (
            OLLAMA_OSS_PROVIDER_ID,
            create_oss_provider(DEFAULT_OLLAMA_PORT, WireApi::Responses),
        ),
        (
            LMSTUDIO_OSS_PROVIDER_ID,
            create_oss_provider(DEFAULT_LMSTUDIO_PORT, WireApi::Responses),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// Merge configured providers into the built-in provider catalog.
///
/// Configured providers extend the built-in set. Built-in providers are not
/// generally overridable, but built-in Amazon Bedrock providers allow the user
/// to customize their endpoint, authentication, headers, and AWS settings.
pub fn merge_configured_model_providers(
    mut model_providers: HashMap<String, ModelProviderInfo>,
    configured_model_providers: HashMap<String, ModelProviderInfo>,
) -> Result<HashMap<String, ModelProviderInfo>, String> {
    for (key, mut provider) in configured_model_providers {
        if matches!(
            key.as_str(),
            AMAZON_BEDROCK_PROVIDER_ID | AMAZON_BEDROCK_RUNTIME_PROVIDER_ID
        ) {
            let base_url_override = provider.base_url.take();
            let auth_override = provider.auth.take();
            let aws_override = provider.aws.take();
            let http_headers_override = provider.http_headers.take();
            if provider != ModelProviderInfo::default() {
                return Err(format!(
                    "model_providers.{key} only supports changing \
`base_url`, `auth`, `http_headers`, `aws.profile`, `aws.region`, and `aws.auth_refresh`; \
other non-default provider fields are not supported"
                ));
            }

            if let Some(built_in_provider) = model_providers.get_mut(&key) {
                built_in_provider.base_url = base_url_override;
                built_in_provider.auth = auth_override;
                if let Some(aws_override) = aws_override {
                    built_in_provider.aws = Some(aws_override);
                }
                if let Some(http_headers_override) = http_headers_override {
                    built_in_provider
                        .http_headers
                        .get_or_insert_default()
                        .extend(http_headers_override);
                }
            }
        } else {
            model_providers.entry(key).or_insert(provider);
        }
    }

    Ok(model_providers)
}

pub fn create_oss_provider(default_provider_port: u16, wire_api: WireApi) -> ModelProviderInfo {
    // These CODEX_OSS_ environment variables are experimental: we may
    // switch to reading values from config.toml instead.
    let default_codex_oss_base_url = format!(
        "http://localhost:{codex_oss_port}/v1",
        codex_oss_port = std::env::var("CODEX_OSS_PORT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(default_provider_port)
    );

    let codex_oss_base_url = std::env::var("CODEX_OSS_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(default_codex_oss_base_url);
    create_oss_provider_with_base_url(&codex_oss_base_url, wire_api)
}

pub fn create_oss_provider_with_base_url(base_url: &str, wire_api: WireApi) -> ModelProviderInfo {
    ModelProviderInfo {
        name: "gpt-oss".into(),
        base_url: Some(base_url.into()),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
        supports_standalone_web_search: false,
    }
}

#[cfg(test)]
#[path = "model_provider_info_tests.rs"]
mod tests;
