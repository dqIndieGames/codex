use super::ResponsesStreamRequest;
use super::ResponsesStreamRetryState;
use super::handle_retryable_response_stream_error;
use super::log_retry;
use crate::session::tests::make_session_and_context;
use crate::session::tests::make_session_and_context_with_auth_and_config_and_rx;
use codex_login::CodexAuth;
use codex_protocol::error::CodexErr;
use std::time::Duration;
use tracing_test::internal::MockWriter;

// Product contract: local3 checklist transport recovery, three consecutive
// WS failures even with an unlimited task budget; official sticky state stays intact.
#[tokio::test(start_paused = true)]
async fn websocket_transport_fallback_covers_sampling_and_both_compaction_paths() {
    for official in [true, false] {
        for request in [
            ResponsesStreamRequest::Sampling,
            ResponsesStreamRequest::LocalCompaction,
            ResponsesStreamRequest::RemoteCompactionV2,
        ] {
            let (session, context, _events) = make_session_and_context_with_auth_and_config_and_rx(
                CodexAuth::from_api_key("fixture-key"),
                Vec::new(),
                |config| {
                    config.model_provider.base_url = Some(if official {
                        "https://chatgpt.com/backend-api/codex".to_string()
                    } else {
                        "http://127.0.0.1:1/v1".to_string()
                    });
                    config.model_provider.supports_websockets = true;
                },
            )
            .await;
            let mut client = session.services.model_client.new_session();
            client
                .turn_state()
                .set("sticky-fixture".to_string())
                .unwrap();
            let mut state = ResponsesStreamRetryState::default();
            for attempt in 1..=6 {
                let start = tokio::time::Instant::now();
                handle_retryable_response_stream_error(
                    &mut state,
                    u64::MAX,
                    CodexErr::Stream("websocket closed before completion".to_string()),
                    &mut client,
                    &session,
                    &context,
                    request,
                )
                .await
                .unwrap();
                assert_eq!(
                    state.display_retries, attempt,
                    "fallback must not reset visible retries"
                );
                assert_eq!(
                    session.services.model_client.responses_websocket_enabled(),
                    attempt < 3,
                    "the third failed WS attempt must select HTTP, independent of retry budget",
                );
                assert!(start.elapsed() >= Duration::from_secs(5));
                if official {
                    assert_eq!(
                        client.turn_state().get().map(String::as_str),
                        Some("sticky-fixture")
                    );
                } else if attempt >= 3 {
                    assert!(client.turn_state().get().is_none());
                }
            }
        }
    }
}

#[tokio::test(start_paused = true)]
async fn websocket_recovery_keeps_cancellation_terminal() {
    let (session, context) = make_session_and_context().await;
    let mut client = session.services.model_client.new_session();
    let mut state = ResponsesStreamRetryState::default();
    for error in [
        CodexErr::Interrupted,
        CodexErr::TurnAborted,
        CodexErr::SessionBudgetExceeded,
    ] {
        let result = handle_retryable_response_stream_error(
            &mut state,
            u64::MAX,
            error,
            &mut client,
            &session,
            &context,
            ResponsesStreamRequest::RemoteCompactionV2,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            state.display_retries, 0,
            "cancellation must not schedule another retry"
        );
    }
}

#[tokio::test]
async fn sampling_retry_does_not_warn_on_intermediate_stream_error() {
    let (_session, turn_context) = make_session_and_context().await;
    let buffer: &'static std::sync::Mutex<Vec<u8>> =
        Box::leak(Box::new(std::sync::Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(MockWriter::new(buffer))
        .finish();
    let _subscriber_guard = tracing::subscriber::set_default(subscriber);

    log_retry(
        ResponsesStreamRequest::Sampling,
        &turn_context,
        &CodexErr::Stream("websocket closed by server before response.completed".to_string()),
        /*retries*/ 2,
        /*max_retries*/ 5,
        Duration::from_secs(5),
    );

    let logs = String::from_utf8(
        buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
    .expect("retry log should be valid utf-8");
    assert!(
        logs.is_empty(),
        "retry intermediate state must not write warn/error logs: {logs}"
    );
}
