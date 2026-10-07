//! Shared retry and protocol-preserving recovery for Responses requests.

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
    LocalCompaction,
    RemoteCompactionV2,
}

pub(crate) struct ResponsesStreamRetryState {
    pub(crate) retries: u64,
    connection_retries: u64,
    display_retries: u64,
    compaction_ws_failures: u64,
    compaction_size_failures: u64,
    compaction_1009_retried: bool,
    pub(crate) compaction_http: bool,
}

impl Default for ResponsesStreamRetryState {
    fn default() -> Self {
        Self {
            retries: 0,
            connection_retries: 0,
            display_retries: 0,
            compaction_ws_failures: 0,
            compaction_size_failures: 0,
            compaction_1009_retried: false,
            compaction_http: false,
        }
    }
}

impl ResponsesStreamRetryState {
    /// Decide recovery before the shared single ten-second wait. The returned tier must also
    /// be applied to the compaction caller's snapshot so stale images are never resent.
    pub(crate) async fn prepare_compaction_retry(
        &mut self,
        max_retries: u64,
        err: &CodexErr,
        client_session: &ModelClientSession,
        sess: &Session,
        turn_context: &TurnContext,
    ) -> Option<u8> {
        if self.retries >= max_retries || matches!(err.details(),
            CodexErrorDetails::Interrupted | CodexErrorDetails::TurnAborted
                | CodexErrorDetails::SessionBudgetExceeded) {
            return None;
        }
        let ws = !self.compaction_http && client_session.responses_websocket_enabled();
        let first_1009 = ws && !self.compaction_1009_retried
            && matches!(err.details(), CodexErrorDetails::Stream(message)
                if message.contains("(close code: 1009)"));
        if ws {
            self.compaction_ws_failures = self.compaction_ws_failures.saturating_add(1);
            if first_1009 {
                self.compaction_1009_retried = true;
            } else if self.compaction_1009_retried || self.compaction_ws_failures >= 3 {
                self.compaction_http = true;
            }
        }
        let size_error = matches!(err.details(), CodexErrorDetails::ContextWindowExceeded)
            || err.http_status_code_value() == Some(413);
        if size_error {
            self.compaction_size_failures = self.compaction_size_failures.saturating_add(1);
        }
        if first_1009 || (size_error && self.compaction_size_failures % 3 == 0) {
            if let Some(report) = sess.apply_context_overflow_image_ladder(&turn_context.sub_id).await {
                let tier = report.tier;
                sess.notify_stream_error(turn_context, report.message, CodexErr::ContextWindowExceeded).await;
                return Some(tier);
            }
        }
        None
    }
}

/// Server retry advice retained after stream retries are exhausted. The turn ID
/// prevents a reused Guardian session from applying advice from an earlier review.
pub(crate) struct ExhaustedResponseRetry {
    pub(crate) turn_id: String,
    pub(crate) retry_at: Option<tokio::time::Instant>,
}

/// Returns `Ok(())` when the caller should retry the request loop, or the original error when
/// it is terminal or the retry budget is exhausted.
pub(crate) async fn handle_response_stream_error(
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
        ResponsesStreamRequest::LocalCompaction => RetryOperation::LocalCompaction,
        ResponsesStreamRequest::RemoteCompactionV2 => RetryOperation::RemoteCompactionV2,
    };
    let delay = fixed_retry_delay();
    if matches!(
        err.details(),
        CodexErrorDetails::Interrupted
            | CodexErrorDetails::TurnAborted
            | CodexErrorDetails::SessionBudgetExceeded
    ) {
        return Err(err);
    }

    // Retire a failed socket without changing the provider's transport choice.
    client_session.reset_retry_websocket_connection();

    if max_retries > 0
        && turn_context
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
        retry_state.display_retries = retry_state.display_retries.saturating_add(1);
        log_retry(
            request,
            turn_context,
            &err,
            retry_state.display_retries,
            max_retries,
            delay,
        );
        sess.notify_stream_error(
            turn_context,
            retry_status_message(&err, retry_state.display_retries),
            err,
        )
        .await;
        codex_client::record_retry!(retry_state.connection_retries, delay, operation);
        tokio::time::sleep(delay).await;
        return Ok(());
    }

    if retry_state.retries < max_retries {
        retry_state.retries = retry_state.retries.saturating_add(1);
        let retry_count = retry_state.retries;
        retry_state.display_retries = retry_state.display_retries.saturating_add(1);
        maybe_activate_route_recovery(client_session, retry_state.display_retries);
        log_retry(
            request,
            turn_context,
            &err,
            retry_state.display_retries,
            max_retries,
            delay,
        );
        sess.notify_stream_error(
            turn_context,
            retry_status_message(&err, retry_state.display_retries),
            err,
        )
        .await;
        codex_client::record_retry!(retry_count, delay, operation);
        tokio::time::sleep(delay).await;
        return Ok(());
    }

    sess.services
        .thread_extension_data
        .insert(ExhaustedResponseRetry {
            turn_id: turn_context.sub_id.clone(),
            retry_at: Some(tokio::time::Instant::now() + delay),
        });
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
        ResponsesStreamRequest::LocalCompaction | ResponsesStreamRequest::RemoteCompactionV2 => {
            debug!(
                turn_id = %turn_context.sub_id,
                retries,
                max_retries,
                compact_error = %err,
                delay_ms = delay.as_millis() as u64,
                request = ?request,
                "compaction stream failed; retrying request after delay"
            );
        }
    }
}

#[cfg(test)]
#[path = "responses_retry_tests.rs"]
mod tests;
