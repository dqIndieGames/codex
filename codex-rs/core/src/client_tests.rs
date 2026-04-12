use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::RequestRouteTelemetry;
use super::UnauthorizedRecoveryExecution;
use super::format_retry_transport_error_details;
use super::should_emit_websocket_connect_or_request_log_trace;
use super::should_emit_websocket_event_log_trace;
use super::unauthorized_transport_from_codex_error;
use crate::client_common::Prompt;
use codex_otel::SessionTelemetry;
use codex_protocol::ThreadId;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::models::BaseInstructions;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_client::TransportError;
use http::StatusCode;
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

fn test_prompt() -> Prompt {
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
            &test_prompt(),
            &model_info,
            /*effort*/ None,
            ReasoningSummary::Auto,
            service_tier,
        )
        .expect("responses request")
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

#[test]
fn unauthorized_transport_from_codex_error_rebuilds_http_401() {
    let err = crate::error::CodexErr::UnexpectedStatus(crate::error::UnexpectedResponseError {
        status: StatusCode::UNAUTHORIZED,
        body: r#"{"detail":"Unauthorized"}"#.to_string(),
        url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
        cf_ray: None,
        request_id: Some("req-401".to_string()),
        identity_authorization_error: Some("missing_authorization_header".to_string()),
        identity_error_code: Some("token_expired".to_string()),
        retry_source: crate::error::UnexpectedResponseRetrySource::Turn,
    });

    let Some(TransportError::Http { status, url, body, .. }) =
        unauthorized_transport_from_codex_error(&err)
    else {
        panic!("expected unauthorized transport error");
    };

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        url.as_deref(),
        Some("https://chatgpt.com/backend-api/codex/responses")
    );
    assert_eq!(body.as_deref(), Some(r#"{"detail":"Unauthorized"}"#));
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
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
        /*can_retry_after_unauthorized*/ false,
    ));
}
