use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::RequestRouteTelemetry;
use super::TransportError;
use super::UnauthorizedRecoveryExecution;
use super::WebsocketSession;
use super::X_CODEX_INSTALLATION_ID_HEADER;
use super::X_CODEX_PARENT_THREAD_ID_HEADER;
use super::X_CODEX_TURN_METADATA_HEADER;
use super::X_CODEX_WINDOW_ID_HEADER;
use super::X_OPENAI_SUBAGENT_HEADER;
use super::build_live_api_auth;
use super::format_retry_transport_error_details;
use super::should_emit_websocket_connect_or_request_log_trace;
use super::should_emit_websocket_event_log_trace;
use crate::Prompt;
use codex_api::ApiError;
use codex_api::ResponseEvent;
use codex_app_server_protocol::AuthMode;
use codex_model_provider::BearerAuthProvider;
use codex_model_provider::create_model_provider;
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
use codex_protocol::protocol::InternalSessionSource;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_rollout_trace::ExecutionStatus;
use codex_rollout_trace::InferenceTraceAttempt;
use codex_rollout_trace::InferenceTraceContext;
use codex_rollout_trace::RawTraceEventPayload;
use codex_rollout_trace::RolloutTrace;
use codex_rollout_trace::TraceWriter;
use codex_rollout_trace::replay_bundle;
use futures::StreamExt;
use http::StatusCode;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Notify;
use tracing::Event;
use tracing::Subscriber;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context as LayerContext;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

fn test_model_client(session_source: SessionSource) -> ModelClient {
    let provider = create_oss_provider_with_base_url("https://example.com/v1", WireApi::Responses);
    let thread_id = ThreadId::new();
    ModelClient::new(
        /*auth_manager*/ None,
        thread_id.into(),
        thread_id,
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
        output_schema_strict: true,
    }
}

fn build_test_responses_request(
    model_slug: &str,
    service_tier: Option<ServiceTier>,
) -> codex_api::ResponsesApiRequest {
    let client = test_model_client(SessionSource::Cli);
    build_test_responses_request_with_priority_hook(
        client,
        model_slug,
        service_tier,
        /*force_service_tier_priority*/ true,
    )
}

fn build_test_responses_request_with_priority_hook(
    client: ModelClient,
    model_slug: &str,
    service_tier: Option<ServiceTier>,
    force_service_tier_priority: bool,
) -> codex_api::ResponsesApiRequest {
    client.set_force_service_tier_priority(force_service_tier_priority);
    let provider = client
        .provider_snapshot()
        .to_api_provider(/*auth_mode*/ None)
        .expect("test provider");
    let mut model_info = test_model_info();
    model_info.slug = model_slug.to_string();

    client
        .build_responses_request(
            &provider,
            &test_request_prompt(),
            &model_info,
            /*effort*/ None,
            ReasoningSummary::Auto,
            service_tier.map(|tier| tier.request_value().to_string()),
        )
        .expect("responses request")
}

#[derive(Default)]
struct TagCollectorVisitor {
    tags: BTreeMap<String, String>,
}

impl Visit for TagCollectorVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.tags
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.tags
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

#[derive(Clone)]
struct TagCollectorLayer {
    tags: Arc<Mutex<BTreeMap<String, String>>>,
}

impl<S> Layer<S> for TagCollectorLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
        if event.metadata().target() != "feedback_tags" {
            return;
        }
        let mut visitor = TagCollectorVisitor::default();
        event.record(&mut visitor);
        self.tags.lock().unwrap().extend(visitor.tags);
    }
}

fn started_inference_attempt(temp: &TempDir) -> anyhow::Result<InferenceTraceAttempt> {
    let writer = Arc::new(TraceWriter::create(
        temp.path(),
        "trace-1".to_string(),
        "rollout-1".to_string(),
        "thread-root".to_string(),
    )?);
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    writer.append(RawTraceEventPayload::CodexTurnStarted {
        codex_turn_id: "turn-1".to_string(),
        thread_id: "thread-root".to_string(),
    })?;

    let inference_trace = InferenceTraceContext::enabled(
        writer,
        "thread-root".to_string(),
        "turn-1".to_string(),
        "gpt-test".to_string(),
        "test-provider".to_string(),
    );
    let attempt = inference_trace.start_attempt();
    attempt.record_started(&json!({
        "model": "gpt-test",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "hello"}]
        }],
    }));
    Ok(attempt)
}

fn output_message(id: &str, text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: Some(id.to_string()),
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

async fn replay_until_cancelled(temp: &TempDir) -> anyhow::Result<RolloutTrace> {
    let mut rollout = replay_bundle(temp.path())?;
    for _ in 0..50 {
        let inference = rollout
            .inference_calls
            .values()
            .next()
            .expect("inference should be reduced");
        if inference.execution.status == ExecutionStatus::Cancelled {
            return Ok(rollout);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        rollout = replay_bundle(temp.path())?;
    }
    Ok(rollout)
}

struct NotifyAfterEventStream {
    events: VecDeque<ResponseEvent>,
    yielded: usize,
    notify_after: usize,
    notify: Arc<Notify>,
}

impl futures::Stream for NotifyAfterEventStream {
    type Item = std::result::Result<ResponseEvent, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(event) = self.events.pop_front() else {
            return Poll::Pending;
        };
        self.yielded += 1;
        if self.yielded == self.notify_after {
            self.notify.notify_one();
        }
        Poll::Ready(Some(Ok(event)))
    }
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
fn build_responses_request_forces_priority_without_service_tier_when_hook_enabled() {
    let request = build_test_responses_request("gpt-test", /*service_tier*/ None);
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_forces_priority_for_gpt_5_5_when_hook_enabled() {
    let request = build_test_responses_request("gpt-5.5", Some(ServiceTier::Flex));
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_forces_priority_for_flex_tier_when_hook_enabled() {
    let request = build_test_responses_request("gpt-test", Some(ServiceTier::Flex));
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_forces_priority_for_fast_tier_when_hook_enabled() {
    let request = build_test_responses_request("gpt-test", Some(ServiceTier::Fast));
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_omits_service_tier_when_hook_disabled_and_no_tier() {
    let request = build_test_responses_request_with_priority_hook(
        test_model_client(SessionSource::Cli),
        "gpt-test",
        /*service_tier*/ None,
        /*force_service_tier_priority*/ false,
    );
    assert_eq!(request.service_tier, None);
}

#[test]
fn build_responses_request_maps_fast_to_priority_when_hook_is_false() {
    let request = build_test_responses_request_with_priority_hook(
        test_model_client(SessionSource::Cli),
        "gpt-test",
        Some(ServiceTier::Fast),
        /*force_service_tier_priority*/ false,
    );
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_responses_request_preserves_flex_when_hook_is_false() {
    let request = build_test_responses_request_with_priority_hook(
        test_model_client(SessionSource::Cli),
        "gpt-test",
        Some(ServiceTier::Flex),
        /*force_service_tier_priority*/ false,
    );
    assert_eq!(request.service_tier.as_deref(), Some("flex"));
}

#[test]
fn build_responses_request_gpt_5_4_uses_normal_fast_mapping_when_hook_is_false() {
    let request = build_test_responses_request_with_priority_hook(
        test_model_client(SessionSource::Cli),
        "gpt-5.4",
        Some(ServiceTier::Fast),
        /*force_service_tier_priority*/ false,
    );
    assert_eq!(request.service_tier.as_deref(), Some("priority"));
}

#[test]
fn build_subagent_headers_sets_internal_memory_consolidation_label() {
    let client = test_model_client(SessionSource::Internal(
        InternalSessionSource::MemoryConsolidation,
    ));
    let headers = client.build_subagent_headers();
    let value = headers
        .get(X_OPENAI_SUBAGENT_HEADER)
        .and_then(|value| value.to_str().ok());
    assert_eq!(value, Some("memory_consolidation"));
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
    let thread_id = client.state.thread_id;
    assert_eq!(
        client_metadata,
        std::collections::HashMap::from([
            (
                X_CODEX_INSTALLATION_ID_HEADER.to_string(),
                "11111111-1111-4111-8111-111111111111".to_string(),
            ),
            (
                X_CODEX_WINDOW_ID_HEADER.to_string(),
                format!("{thread_id}:1"),
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

#[tokio::test]
async fn dropped_response_stream_traces_cancelled_partial_output() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let attempt = started_inference_attempt(&temp)?;

    // The provider has produced one complete output item, but no terminal
    // response.completed event. The harness has enough information to keep this
    // item in history, so the trace should preserve it when the stream is
    // abandoned.
    let item = output_message("msg-1", "partial answer");
    let api_stream = futures::stream::iter([Ok(ResponseEvent::OutputItemDone(item))])
        .chain(futures::stream::pending());
    let (mut stream, _) = super::map_response_events(
        /*upstream_request_id*/ None,
        api_stream,
        test_session_telemetry(),
        attempt,
    );

    let observed = stream
        .next()
        .await
        .expect("mapped stream should yield output item")?;
    assert!(matches!(observed, ResponseEvent::OutputItemDone(_)));

    // Dropping the consumer is how turn interruption/preemption stops polling
    // the provider stream. The mapper task observes that drop asynchronously
    // and records cancellation using the output items it has already seen.
    drop(stream);

    // Cancellation is recorded by the mapper task after Drop wakes it, so the
    // replay may need a short wait before the terminal event appears on disk.
    let rollout = replay_until_cancelled(&temp).await?;
    let inference = rollout
        .inference_calls
        .values()
        .next()
        .expect("inference should be reduced");

    assert_eq!(inference.execution.status, ExecutionStatus::Cancelled);
    assert_eq!(inference.response_item_ids.len(), 1);
    assert_eq!(rollout.raw_payloads.len(), 2);

    Ok(())
}

#[tokio::test]
async fn response_stream_records_last_model_feedback_ids() {
    let tags = Arc::new(Mutex::new(BTreeMap::new()));
    let _guard = tracing_subscriber::registry()
        .with(TagCollectorLayer { tags: tags.clone() })
        .set_default();

    let api_stream = futures::stream::iter([
        Ok(ResponseEvent::Created),
        Ok(ResponseEvent::Completed {
            response_id: "resp-123".to_string(),
            token_usage: None,
            end_turn: Some(true),
        }),
    ]);
    let (mut stream, _) = super::map_response_events(
        Some("req-123".to_string()),
        api_stream,
        test_session_telemetry(),
        InferenceTraceAttempt::disabled(),
    );

    while stream.next().await.is_some() {}

    let tags = tags.lock().unwrap().clone();
    assert_eq!(
        tags.get("last_model_request_id").map(String::as_str),
        Some("\"req-123\"")
    );
    assert_eq!(
        tags.get("last_model_response_id").map(String::as_str),
        Some("\"resp-123\"")
    );
}

#[tokio::test]
async fn dropped_backpressured_response_stream_traces_cancelled_partial_output()
-> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let attempt = started_inference_attempt(&temp)?;
    let backpressured_item_yielded = Arc::new(Notify::new());
    let mut events = VecDeque::new();
    for _ in 0..super::RESPONSE_STREAM_CHANNEL_CAPACITY {
        events.push_back(ResponseEvent::Created);
    }
    events.push_back(ResponseEvent::OutputItemDone(output_message(
        "msg-1",
        "partial answer",
    )));
    let api_stream = NotifyAfterEventStream {
        events,
        yielded: 0,
        notify_after: super::RESPONSE_STREAM_CHANNEL_CAPACITY + 1,
        notify: Arc::clone(&backpressured_item_yielded),
    };

    let (stream, _) = super::map_response_events(
        /*upstream_request_id*/ None,
        api_stream,
        test_session_telemetry(),
        attempt,
    );

    // Fill the mapper channel with non-terminal events, then yield one output
    // item. The mapper has observed that item and is blocked trying to send it
    // downstream, so dropping the consumer covers the send-failure path rather
    // than the `consumer_dropped` select branch.
    backpressured_item_yielded.notified().await;
    drop(stream);

    let rollout = replay_until_cancelled(&temp).await?;
    let inference = rollout
        .inference_calls
        .values()
        .next()
        .expect("inference should be reduced");

    assert_eq!(inference.execution.status, ExecutionStatus::Cancelled);
    assert_eq!(inference.response_item_ids.len(), 1);
    assert_eq!(rollout.raw_payloads.len(), 2);

    Ok(())
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
    let provider =
        ModelProviderInfo::create_openai_provider(Some("https://old.example.com/v1".to_string()));
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
fn live_api_auth_reads_refreshed_bearer_token() {
    let provider_info = ModelProviderInfo {
        experimental_bearer_token: Some("old-token".to_string()),
        ..ModelProviderInfo::create_openai_provider(Some("https://old.example.com/v1".to_string()))
    };
    let model_provider = create_model_provider(provider_info.clone(), /*auth_manager*/ None);
    let live_provider = Arc::new(StdRwLock::new(provider_info));
    let auth = build_live_api_auth(
        /*auth*/ None,
        model_provider,
        Arc::clone(&live_provider),
    )
    .expect("build live auth");

    let mut headers = http::HeaderMap::new();
    auth.add_auth_headers_result(&mut headers)
        .expect("add old auth header");
    assert_eq!(
        headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer old-token")
    );

    {
        let mut provider = live_provider
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        provider.experimental_bearer_token = Some("new-token".to_string());
    }

    let mut headers = http::HeaderMap::new();
    auth.add_auth_headers_result(&mut headers)
        .expect("add new auth header");
    assert_eq!(
        headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer new-token")
    );
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
}
