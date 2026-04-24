use std::sync::Arc;

use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::RequestRouteTelemetry;
use super::UnauthorizedRecoveryExecution;
use super::WebsocketSession;
use super::X_CODEX_INSTALLATION_ID_HEADER;
use super::X_CODEX_PARENT_THREAD_ID_HEADER;
use super::X_CODEX_TURN_METADATA_HEADER;
use super::X_CODEX_WINDOW_ID_HEADER;
use super::X_OPENAI_SUBAGENT_HEADER;
use super::format_retry_transport_error_details;
use super::should_emit_websocket_connect_or_request_log_trace;
use super::should_emit_websocket_event_log_trace;
use crate::Prompt;
use crate::ResponseEvent;
use crate::agent_identity::AgentIdentityManager;
use crate::agent_identity::RegisteredAgentTask;
use crate::agent_identity::StoredAgentIdentity;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use codex_app_server_protocol::AuthMode;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider::BearerAuthProvider;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_otel::SessionTelemetry;
use codex_protocol::ThreadId;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use core_test_support::responses;
use ed25519_dalek::Signature;
use ed25519_dalek::Verifier as _;
use futures::StreamExt;
use http::StatusCode;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::json;
use tempfile::TempDir;

fn test_model_client(session_source: SessionSource) -> ModelClient {
    let provider = create_oss_provider_with_base_url("https://example.com/v1", WireApi::Responses);
    ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider,
        session_source,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
    )
}

fn test_model_info() -> ModelInfo {
    serde_json::from_value(json!({
        "slug": "gpt-test",
        "display_name": "gpt-test",
        "description": "desc",
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            {"effort": "medium", "description": "medium"}
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 1,
        "upgrade": null,
        "base_instructions": "base instructions",
        "model_messages": null,
        "supports_reasoning_summaries": false,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10000},
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": 272000,
        "auto_compact_token_limit": null,
        "experimental_supported_tools": []
    }))
    .expect("deserialize test model info")
}

fn test_session_telemetry() -> SessionTelemetry {
    SessionTelemetry::new(
        ThreadId::new(),
        "gpt-test",
        "gpt-test",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "test-originator".to_string(),
        /*log_user_prompts*/ false,
        "test-terminal".to_string(),
        SessionSource::Cli,
    )
}

fn test_request_prompt() -> Prompt {
    Prompt {
        input: Vec::new(),
        tools: Vec::new(),
        parallel_tool_calls: false,
        base_instructions: BaseInstructions {
            text: "test instructions".to_string(),
        },
        personality: None,
        output_schema: None,
    }
}

fn build_test_responses_request(
    model_slug: &str,
    service_tier: Option<ServiceTier>,
) -> codex_api::ResponsesApiRequest {
    let client = test_model_client(SessionSource::Cli);
    build_test_responses_request_with_priority_fallback(
        client,
        model_slug,
        service_tier,
        /*force_gpt54_priority_fallback*/ true,
    )
}

fn build_test_responses_request_with_priority_fallback(
    client: ModelClient,
    model_slug: &str,
    service_tier: Option<ServiceTier>,
    force_gpt54_priority_fallback: bool,
) -> codex_api::ResponsesApiRequest {
    client.set_force_gpt54_priority_fallback(force_gpt54_priority_fallback);
    let session = client.new_session();
    let provider = client
        .provider_snapshot()
        .to_api_provider(/*auth_mode*/ None)
        .expect("test provider");
    let mut model_info = test_model_info();
    model_info.slug = model_slug.to_string();

    session
        .build_responses_request(
            &provider,
            &test_request_prompt(),
            &model_info,
            /*effort*/ None,
            ReasoningSummary::Auto,
            service_tier,
        )
        .expect("responses request")
}

fn test_prompt(text: &str) -> Prompt {
    Prompt {
        input: vec![ResponseItem::Message {
            id: None,
            role: "user".into(),
            content: vec![ContentItem::InputText {
                text: text.to_string(),
            }],
            end_turn: None,
            phase: None,
        }],
        ..Prompt::default()
    }
}

async fn drain_stream_to_completion(stream: &mut crate::ResponseStream) -> anyhow::Result<()> {
    while let Some(event) = stream.next().await {
        if matches!(event?, ResponseEvent::Completed { .. }) {
            break;
        }
    }
    Ok(())
}

async fn model_client_with_agent_task(
    provider: ModelProviderInfo,
) -> (
    TempDir,
    ModelClient,
    RegisteredAgentTask,
    StoredAgentIdentity,
) {
    let codex_home = tempfile::tempdir().expect("tempdir");
    let auth_manager =
        AuthManager::from_auth_for_testing(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let agent_identity_manager = Arc::new(AgentIdentityManager::new_for_tests(
        Arc::clone(&auth_manager),
        /*feature_enabled*/ true,
        "https://chatgpt.com/backend-api/".to_string(),
        SessionSource::Cli,
    ));
    let stored_identity = agent_identity_manager
        .seed_generated_identity_for_tests("agent-123")
        .await
        .expect("seed test identity");
    let agent_task = RegisteredAgentTask {
        agent_runtime_id: stored_identity.agent_runtime_id.clone(),
        task_id: "task-123".to_string(),
        registered_at: "2026-03-23T12:00:00Z".to_string(),
    };
    let client = ModelClient::new(
        Some(auth_manager),
        ThreadId::new(),
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider,
        SessionSource::Cli,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
    );
    (codex_home, client, agent_task, stored_identity)
}

#[derive(Debug, Deserialize)]
struct AgentAssertionEnvelope {
    agent_runtime_id: String,
    task_id: String,
    timestamp: String,
    signature: String,
}

fn assert_agent_assertion_header(
    authorization_header: &str,
    stored_identity: &StoredAgentIdentity,
    expected_agent_runtime_id: &str,
    expected_task_id: &str,
) {
    let token = authorization_header
        .strip_prefix("AgentAssertion ")
        .expect("agent assertion authorization scheme");
    let envelope: AgentAssertionEnvelope = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(token)
            .expect("base64url-encoded agent assertion"),
    )
    .expect("valid agent assertion envelope");

    assert_eq!(envelope.agent_runtime_id, expected_agent_runtime_id);
    assert_eq!(envelope.task_id, expected_task_id);

    let signature = Signature::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(&envelope.signature)
            .expect("base64 signature"),
    )
    .expect("signature bytes");
    stored_identity
        .signing_key()
        .expect("signing key")
        .verifying_key()
        .verify(
            format!(
                "{}:{}:{}",
                envelope.agent_runtime_id, envelope.task_id, envelope.timestamp
            )
            .as_bytes(),
            &signature,
        )
        .expect("signature should verify");
}

#[test]
fn build_subagent_headers_sets_other_subagent_label() {
    let client = test_model_client(SessionSource::SubAgent(SubAgentSource::Other(
        "memory_consolidation".to_string(),
    )));
    let headers = client.build_subagent_headers();
    let value = headers
        .get(X_OPENAI_SUBAGENT_HEADER)
        .and_then(|value| value.to_str().ok());
    assert_eq!(value, Some("memory_consolidation"));
}

#[test]
fn build_responses_request_forces_priority_for_gpt_5_4_without_service_tier() {
    let request = build_test_responses_request("gpt-5.4", /*service_tier*/ None);
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_forces_priority_for_gpt_5_4_flex_tier() {
    let request = build_test_responses_request("gpt-5.4", Some(ServiceTier::Flex));
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_disables_gpt_5_4_priority_when_fallback_is_false() {
    let request = build_test_responses_request_with_priority_fallback(
        test_model_client(SessionSource::Cli),
        "gpt-5.4",
        /*service_tier*/ None,
        /*force_gpt54_priority_fallback*/ false,
    );
    assert_eq!(request.service_tier, None);
}

#[test]
fn build_responses_request_disables_fast_passthrough_for_gpt_5_4_when_fallback_is_false() {
    let request = build_test_responses_request_with_priority_fallback(
        test_model_client(SessionSource::Cli),
        "gpt-5.4",
        Some(ServiceTier::Fast),
        /*force_gpt54_priority_fallback*/ false,
    );
    assert_eq!(request.service_tier, None);
}

#[test]
fn build_responses_request_preserves_flex_for_gpt_5_4_when_fallback_is_false() {
    let request = build_test_responses_request_with_priority_fallback(
        test_model_client(SessionSource::Cli),
        "gpt-5.4",
        Some(ServiceTier::Flex),
        /*force_gpt54_priority_fallback*/ false,
    );
    assert_eq!(request.service_tier.as_deref(), Some("flex"));
}

#[test]
fn build_responses_request_preserves_flex_for_non_gpt_5_4() {
    let request = build_test_responses_request("gpt-5.1", Some(ServiceTier::Flex));
    assert_eq!(request.service_tier.as_deref(), Some("flex"));
}

#[test]
fn build_responses_request_keeps_non_gpt_5_4_fast_mapping_when_fallback_is_false() {
    let request = build_test_responses_request_with_priority_fallback(
        test_model_client(SessionSource::Cli),
        "gpt-5.1",
        Some(ServiceTier::Fast),
        /*force_gpt54_priority_fallback*/ false,
    );
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_ws_client_metadata_includes_window_lineage_and_turn_metadata() {
    let parent_thread_id = ThreadId::new();
    let client = test_model_client(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth: 2,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
    }));

    client.advance_window_generation();

    let client_metadata = client.build_ws_client_metadata(Some(r#"{"turn_id":"turn-123"}"#));
    let conversation_id = client.state.conversation_id;
    assert_eq!(
        client_metadata,
        std::collections::HashMap::from([
            (
                X_CODEX_INSTALLATION_ID_HEADER.to_string(),
                "11111111-1111-4111-8111-111111111111".to_string(),
            ),
            (
                X_CODEX_WINDOW_ID_HEADER.to_string(),
                format!("{conversation_id}:1"),
            ),
            (
                X_OPENAI_SUBAGENT_HEADER.to_string(),
                "collab_spawn".to_string(),
            ),
            (
                X_CODEX_PARENT_THREAD_ID_HEADER.to_string(),
                parent_thread_id.to_string(),
            ),
            (
                X_CODEX_TURN_METADATA_HEADER.to_string(),
                r#"{"turn_id":"turn-123"}"#.to_string(),
            ),
        ])
    );
}

#[tokio::test]
async fn summarize_memories_returns_empty_for_empty_input() {
    let client = test_model_client(SessionSource::Cli);
    let model_info = test_model_info();
    let session_telemetry = test_session_telemetry();

    let output = client
        .summarize_memories(
            Vec::new(),
            &model_info,
            /*effort*/ None,
            &session_telemetry,
        )
        .await
        .expect("empty summarize request should succeed");
    assert_eq!(output.len(), 0);
}

#[test]
fn auth_request_telemetry_context_tracks_attached_auth_and_retry_phase() {
    let auth_context = AuthRequestTelemetryContext::new(
        Some(AuthMode::Chatgpt),
        &BearerAuthProvider::for_test(Some("access-token"), Some("workspace-123")),
        PendingUnauthorizedRetry::from_recovery(UnauthorizedRecoveryExecution {
            mode: "managed",
            phase: "refresh_token",
        }),
    );

    assert_eq!(auth_context.auth_mode, Some("Chatgpt"));
    assert!(auth_context.auth_header_attached);
    assert_eq!(auth_context.auth_header_name, Some("authorization"));
    assert!(auth_context.retry_after_unauthorized);
    assert_eq!(auth_context.recovery_mode, Some("managed"));
    assert_eq!(auth_context.recovery_phase, Some("refresh_token"));
}

#[test]
fn refresh_provider_runtime_updates_only_runtime_fields_and_clears_cached_websocket_session() {
    let provider = ModelProviderInfo::create_openai_provider(Some(
        "https://old.example.com/v1".to_string(),
    ));
    let client = ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider.clone(),
        SessionSource::Cli,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
    );

    client
        .state
        .disable_websockets
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let cached_websocket_session = WebsocketSession::default();
    cached_websocket_session.set_connection_reused(true);
    client.store_cached_websocket_session(cached_websocket_session);

    client.refresh_provider_runtime(
        Some("https://new.example.com/v1".to_string()),
        Some("new-token".to_string()),
    );

    let refreshed_provider = client.provider_snapshot();
    assert_eq!(
        refreshed_provider.base_url.as_deref(),
        Some("https://new.example.com/v1")
    );
    assert_eq!(
        refreshed_provider.experimental_bearer_token.as_deref(),
        Some("new-token")
    );
    assert_eq!(refreshed_provider.wire_api, provider.wire_api);
    assert_eq!(
        refreshed_provider.supports_websockets,
        provider.supports_websockets
    );
    assert_eq!(
        refreshed_provider.request_max_retries,
        provider.request_max_retries
    );
    assert_eq!(
        refreshed_provider.stream_max_retries,
        provider.stream_max_retries
    );
    assert!(
        client
            .state
            .disable_websockets
            .load(std::sync::atomic::Ordering::Relaxed)
    );

    let cached_websocket_session = client.take_cached_websocket_session();
    assert!(!cached_websocket_session.connection_reused());
    assert!(cached_websocket_session.last_request.is_none());
}

#[test]
fn retry_transport_error_details_reuse_semantic_usage_limit_formatter_when_available() {
    let error = TransportError::Http {
        status: StatusCode::TOO_MANY_REQUESTS,
        url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
        headers: None,
        body: Some(
            r#"{"error":{"type":"usage_limit_reached","message":"The usage limit has been reached"}}"#
                .to_string(),
        ),
    };

    let details = format_retry_transport_error_details(&error);
    assert!(
        !details.starts_with("unexpected status 429"),
        "expected semantic usage-limit details, got {details}"
    );
}

#[test]
fn retry_transport_error_details_keep_plain_429_as_raw_http_details() {
    let error = TransportError::Http {
        status: StatusCode::TOO_MANY_REQUESTS,
        url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
        headers: None,
        body: Some(r#"{"detail":"rate limited"}"#.to_string()),
    };

    let details = format_retry_transport_error_details(&error);
    assert!(details.starts_with("unexpected status 429"));
    assert!(details.contains("rate limited"));
}

#[test]
fn websocket_connect_log_trace_is_suppressed_for_responses_retry_chain() {
    assert!(!should_emit_websocket_connect_or_request_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        /*retry_chain_active*/ true,
        /*error*/ None,
    ));
}

#[test]
fn websocket_request_log_trace_is_retained_for_non_responses_endpoint() {
    let error = codex_api::ApiError::Transport(TransportError::Http {
        status: StatusCode::SERVICE_UNAVAILABLE,
        url: Some("https://example.com/v1/responses/compact".to_string()),
        headers: None,
        body: Some(r#"{"detail":"retry"}"#.to_string()),
    });

    assert!(should_emit_websocket_connect_or_request_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses/compact"),
        /*retry_chain_active*/ false,
        Some(&error),
    ));
}

#[test]
fn websocket_request_log_trace_is_suppressed_for_retryable_responses_failure() {
    let error = codex_api::ApiError::Transport(TransportError::Http {
        status: StatusCode::SERVICE_UNAVAILABLE,
        url: Some("https://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(r#"{"detail":"retry"}"#.to_string()),
    });

    assert!(!should_emit_websocket_connect_or_request_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        /*retry_chain_active*/ false,
        Some(&error),
    ));
}

#[test]
fn websocket_request_log_trace_is_suppressed_for_retryable_responses_unauthorized() {
    let error = codex_api::ApiError::Transport(TransportError::Http {
        status: StatusCode::UNAUTHORIZED,
        url: Some("https://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(r#"{"detail":"unauthorized"}"#.to_string()),
    });

    assert!(!should_emit_websocket_connect_or_request_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        /*retry_chain_active*/ false,
        Some(&error),
    ));
}

#[test]
fn websocket_event_log_trace_is_suppressed_for_retryable_response_failed() {
    let result: std::result::Result<
        Option<
            std::result::Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        >,
        codex_api::ApiError,
    > = Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(
        r#"{"type":"response.failed","response":{"error":{"code":"server_is_overloaded","message":"try again"}}}"#
            .into(),
    ))));

    assert!(!should_emit_websocket_event_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        &result,
    ));
}

#[test]
fn websocket_event_log_trace_is_retained_for_terminal_response_failed() {
    let result: std::result::Result<
        Option<
            std::result::Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        >,
        codex_api::ApiError,
    > = Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(
        r#"{"type":"response.failed","response":{"error":{"code":"invalid_prompt","message":"terminal"}}}"#
            .into(),
    ))));

    assert!(should_emit_websocket_event_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        &result,
    ));
}

#[test]
fn websocket_event_log_trace_is_suppressed_for_retryable_responses_unauthorized() {
    let result: std::result::Result<
        Option<
            std::result::Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        >,
        codex_api::ApiError,
    > = Err(codex_api::ApiError::Transport(TransportError::Http {
        status: StatusCode::UNAUTHORIZED,
        url: Some("https://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(r#"{"detail":"unauthorized"}"#.to_string()),
    }));

    assert!(!should_emit_websocket_event_log_trace(
        RequestRouteTelemetry::for_endpoint("/responses"),
        &result,
    ));
#[tokio::test]
async fn responses_http_uses_agent_assertion_when_agent_task_is_present() {
    core_test_support::skip_if_no_network!();

    let server = responses::start_mock_server().await;
    let request_recorder = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let provider =
        create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses);
    let (_codex_home, client, agent_task, stored_identity) =
        model_client_with_agent_task(provider).await;
    let model_info = test_model_info();
    let session_telemetry = test_session_telemetry();
    let mut client_session = client.new_session_with_agent_task(Some(agent_task.clone()));

    let mut stream = client_session
        .stream(
            &test_prompt("hello"),
            &model_info,
            &session_telemetry,
            /*effort*/ None,
            ReasoningSummary::Auto,
            /*service_tier*/ None,
            /*turn_metadata_header*/ None,
        )
        .await
        .expect("stream request should succeed");
    drain_stream_to_completion(&mut stream)
        .await
        .expect("stream should complete");

    let request = request_recorder.single_request();
    let authorization = request
        .header("authorization")
        .expect("authorization header should be present");
    assert_agent_assertion_header(
        &authorization,
        &stored_identity,
        &agent_task.agent_runtime_id,
        &agent_task.task_id,
    );
    assert_eq!(request.header("chatgpt-account-id"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_agent_task_bypasses_cached_bearer_prewarm() {
    core_test_support::skip_if_no_network!();

    let server = responses::start_websocket_server(vec![
        vec![vec![
            responses::ev_response_created("resp-prewarm"),
            responses::ev_completed("resp-prewarm"),
        ]],
        vec![vec![
            responses::ev_response_created("resp-1"),
            responses::ev_completed("resp-1"),
        ]],
    ])
    .await;
    let mut provider =
        create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses);
    provider.supports_websockets = true;
    provider.websocket_connect_timeout_ms = Some(5_000);
    let (_codex_home, client, agent_task, stored_identity) =
        model_client_with_agent_task(provider).await;
    let model_info = test_model_info();
    let session_telemetry = test_session_telemetry();
    let prompt = test_prompt("hello");

    let mut prewarm_session = client.new_session();
    prewarm_session
        .prewarm_websocket(
            &prompt,
            &model_info,
            &session_telemetry,
            /*effort*/ None,
            ReasoningSummary::Auto,
            /*service_tier*/ None,
            /*turn_metadata_header*/ None,
        )
        .await
        .expect("bearer prewarm should succeed");
    drop(prewarm_session);

    let mut agent_task_session = client.new_session_with_agent_task(Some(agent_task.clone()));
    let mut stream = agent_task_session
        .stream(
            &prompt,
            &model_info,
            &session_telemetry,
            /*effort*/ None,
            ReasoningSummary::Auto,
            /*service_tier*/ None,
            /*turn_metadata_header*/ None,
        )
        .await
        .expect("agent task stream should succeed");
    drain_stream_to_completion(&mut stream)
        .await
        .expect("agent task websocket stream should complete");

    let handshakes = server.handshakes();
    assert_eq!(handshakes.len(), 2);
    assert_eq!(
        handshakes[0].header("authorization"),
        Some("Bearer Access Token".to_string())
    );
    let agent_authorization = handshakes[1]
        .header("authorization")
        .expect("agent handshake should include authorization");
    assert_agent_assertion_header(
        &agent_authorization,
        &stored_identity,
        &agent_task.agent_runtime_id,
        &agent_task.task_id,
    );
    assert_eq!(handshakes[1].header("chatgpt-account-id"), None);

    server.shutdown().await;
}
