//! Shared retry and transport fallback decisions for Responses requests.

use std::time::Duration;

use crate::client::ModelClientSession;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::util::fixed_retry_delay;
use codex_client::RetryOperation;
use codex_features::Feature;
use codex_protocol::error::CodexErr;
use codex_protocol::error::CodexErrorDetails;
use tracing::debug;

const ROUTE_RECOVERY_RETRY_THRESHOLD: u64 = 3;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResponsesStreamRequest {
    Sampling,
    RemoteCompactionV2,
}

pub(crate) struct ResponsesStreamRetryState {
    pub(crate) retries: u64,
    connection_retries: u64,
}

impl Default for ResponsesStreamRetryState {
    fn default() -> Self {
        Self {
            retries: 0,
            connection_retries: 0,
        }
    }
}

/// Handles a retryable stream error and returns `Ok(())` when the caller should
/// retry the request loop.
pub(crate) async fn handle_retryable_response_stream_error(
    retry_state: &mut ResponsesStreamRetryState,
    max_retries: u64,
    err: CodexErr,
    client_session: &mut ModelClientSession,
    sess: &Session,
    turn_context: &TurnContext,
    request: ResponsesStreamRequest,
) -> Result<(), CodexErr> {
    let operation = match request {
        ResponsesStreamRequest::Sampling => RetryOperation::Sampling,
        ResponsesStreamRequest::RemoteCompactionV2 => RetryOperation::RemoteCompactionV2,
    };
    let delay = fixed_retry_delay();

    if turn_context
        .config
        .features
        .enabled(Feature::UnboundedConnectionRetries)
        && matches!(request, ResponsesStreamRequest::Sampling)
        && matches!(err.details(), CodexErrorDetails::ConnectionFailed(_))
        && !turn_context.session_source.is_internal()
        && !turn_context.provider.info().is_amazon_bedrock()
    {
        retry_state.connection_retries = retry_state.connection_retries.saturating_add(1);
        maybe_activate_route_recovery(client_session, retry_state.connection_retries);
        log_retry(request, turn_context, &err, retry_state.connection_retries, max_retries, delay);
        sess.notify_stream_error(
            turn_context,
            retry_status_message(&err, retry_state.connection_retries),
            err,
        )
        .await;
        codex_client::record_retry!(retry_state.connection_retries, delay, operation);
        tokio::time::sleep(delay).await;
        return Ok(());
    }

    if retry_state.retries >= max_retries
        && client_session.try_switch_fallback_transport(
            &turn_context.session_telemetry,
            turn_context.model_info(),
        )
    {
        retry_state.retries = 0;
        return Ok(());
    }

    if retry_state.retries < max_retries {
        retry_state.retries += 1;
        let retry_count = retry_state.retries;
        maybe_activate_route_recovery(client_session, retry_count);
        log_retry(request, turn_context, &err, retry_count, max_retries, delay);
        sess.notify_stream_error(
            turn_context,
            retry_status_message(&err, retry_count),
            err,
        )
        .await;
        codex_client::record_retry!(retry_count, delay, operation);
        tokio::time::sleep(delay).await;
        return Ok(());
    }

    Err(err)
}

fn maybe_activate_route_recovery(client_session: &mut ModelClientSession, retry_count: u64) {
    if retry_count > 0 && retry_count % ROUTE_RECOVERY_RETRY_THRESHOLD == 0 {
        client_session.activate_retry_route_recovery();
    }
}

fn retry_status_message(err: &CodexErr, retry_count: u64) -> String {
    if err.is_retry_time_budget_interrupted() {
        err.to_string()
    } else {
        format!("Reconnecting... {retry_count} (auto retry)")
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
            debug!(
                turn_id = %turn_context.sub_id,
                retries,
                max_retries,
                sampling_error = %err,
                delay_ms = delay.as_millis() as u64,
                "stream disconnected - retrying sampling request",
            );
        }
        ResponsesStreamRequest::RemoteCompactionV2 => {
            debug!(
                turn_id = %turn_context.sub_id,
                retries,
                max_retries,
                compact_error = %err,
                delay_ms = delay.as_millis() as u64,
                "remote compaction v2 stream failed; retrying request after delay"
            );
        }
    }
}

#[cfg(test)]
#[path = "responses_retry_tests.rs"]
mod tests;
