//! Shared retry and transport fallback decisions for Responses requests.

use std::time::Duration;

use crate::client::ModelClientSession;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::util::backoff;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::WarningEvent;
use tracing::warn;

const STREAM_RETRY_INTERRUPT_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResponsesStreamRequest {
    Sampling,
    RemoteCompactionV2,
}

/// Handles a retryable stream error and returns `Ok(())` when the caller should
/// retry the request loop.
pub(crate) async fn handle_retryable_response_stream_error(
    retries: &mut u64,
    fallback_retry_threshold: u64,
    retry_budget: Option<u64>,
    err: CodexErr,
    client_session: &mut ModelClientSession,
    sess: &Session,
    turn_context: &TurnContext,
    request: ResponsesStreamRequest,
) -> Result<(), CodexErr> {
    if client_session.provider_runtime_changed() {
        *retries = 0;
        client_session.sync_latest_provider_runtime_generation();
        return Ok(());
    }

    if *retries >= fallback_retry_threshold
        && client_session.try_switch_fallback_transport(
            &turn_context.session_telemetry,
            &turn_context.model_info,
        )
    {
        sess.send_event(
            turn_context,
            EventMsg::Warning(WarningEvent {
                message: format!("Falling back from WebSockets to HTTPS transport. {err:#}"),
            }),
        )
        .await;
        *retries = 0;
        return Ok(());
    }

    if retry_budget.is_none_or(|max_retries| *retries < max_retries) {
        *retries += 1;
        let retry_count = *retries;
        let display_max_retries = retry_budget.unwrap_or(u64::MAX);
        let delay = match &err {
            CodexErr::Stream(_, requested_delay) => {
                requested_delay.unwrap_or_else(|| backoff(retry_count))
            }
            _ => backoff(retry_count),
        };
        log_retry(
            request,
            turn_context,
            &err,
            retry_count,
            display_max_retries,
            delay,
        );

        // In release builds, hide the first websocket retry notification to reduce noisy
        // transient reconnect messages. In debug builds, keep full visibility for diagnosis.
        let report_error = retry_count > 1
            || cfg!(debug_assertions)
            || !sess.services.model_client.responses_websocket_enabled();
        if report_error {
            // Surface retry information to any UI/front-end so the user understands what is
            // happening instead of staring at a seemingly frozen screen.
            sess.notify_stream_error(
                turn_context,
                transport_retry_status_message(retry_count, display_max_retries),
                err,
            )
            .await;
        }
        sleep_stream_retry_delay(delay, retries, client_session).await;
        return Ok(());
    }

    Err(err)
}

fn retry_status_suffix(retries: u64, max_retries: u64) -> String {
    if max_retries == u64::MAX {
        format!("{retries} (unbounded)")
    } else {
        format!("{retries}/{max_retries}")
    }
}

fn transport_retry_status_message(retries: u64, max_retries: u64) -> String {
    format!("Reconnecting... {}", retry_status_suffix(retries, max_retries))
}

async fn sleep_stream_retry_delay(
    delay: Duration,
    retries: &mut u64,
    client_session: &mut ModelClientSession,
) {
    if delay.is_zero() {
        return;
    }

    let start = tokio::time::Instant::now();
    loop {
        if client_session.provider_runtime_changed() {
            *retries = 0;
            client_session.sync_latest_provider_runtime_generation();
            return;
        }

        let elapsed = start.elapsed();
        if elapsed >= delay {
            return;
        }

        tokio::time::sleep((delay - elapsed).min(STREAM_RETRY_INTERRUPT_POLL_INTERVAL)).await;
    }
}

fn log_retry(
    request: ResponsesStreamRequest,
    turn_context: &TurnContext,
    err: &CodexErr,
    retries: u64,
    max_retries: u64,
    delay: Duration,
) {
    match request {
        ResponsesStreamRequest::Sampling => {
            warn!(
                retry = %retry_status_suffix(retries, max_retries),
                "stream disconnected - retrying sampling request in {delay:?}...",
            );
        }
        ResponsesStreamRequest::RemoteCompactionV2 => {
            warn!(
                turn_id = %turn_context.sub_id,
                retries,
                max_retries,
                compact_error = %err,
                "remote compaction v2 stream failed; retrying request after delay"
            );
        }
    }
}
