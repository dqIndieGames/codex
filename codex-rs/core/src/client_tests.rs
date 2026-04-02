use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::UnauthorizedRecoveryExecution;
use codex_otel::SessionTelemetry;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use pretty_assertions::assert_eq;
use serde_json::json;

fn test_model_client(session_source: SessionSource) -> ModelClient {
    let provider = crate::model_provider_info::create_oss_provider_with_base_url(
        "https://example.com/v1",
        crate::model_provider_info::WireApi::Responses,
    );
    ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
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

#[test]
fn build_subagent_headers_sets_other_subagent_label() {
    let client = test_model_client(SessionSource::SubAgent(SubAgentSource::Other(
        "memory_consolidation".to_string(),
    )));
    let headers = client.build_subagent_headers();
    let value = headers
        .get("x-openai-subagent")
        .and_then(|value| value.to_str().ok());
    assert_eq!(value, Some("memory_consolidation"));
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
        Some(crate::auth::AuthMode::Chatgpt),
        &crate::api_bridge::CoreAuthProvider::for_test(Some("access-token"), Some("workspace-123")),
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
    let provider = crate::model_provider_info::ModelProviderInfo::create_openai_provider(Some(
        "https://old.example.com/v1".to_string(),
    ));
    let client = ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
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
    let mut cached_websocket_session = WebsocketSession::default();
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
