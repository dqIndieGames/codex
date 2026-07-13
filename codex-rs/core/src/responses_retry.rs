//! Shared retry and transport fallback decisions for Responses requests.

use std::time::Duration;

use crate::client::ModelClientSession;
use crate::client::RETRY_TIME_BUDGET_INTERRUPTED_MESSAGE;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::util::fixed_retry_delay;
use codex_protocol::error::CodexErr;

const STREAM_RETRY_INTERRUPT_POLL_INTERVAL: Duration = Duration::from_millis(250);
const ROUTE_RECOVERY_RETRY_THRESHOLD: u64 = 3;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResponsesStreamRequest {
    Sampling,
    LocalCompaction,
    RemoteCompactionV2,
}

/// Handles a retryable stream error and returns `Ok(())` when the caller should
/// retry the request loop.
pub(crate) async fn handle_retryable_response_stream_error(
    retries: &mut u64,
    display_retries: &mut u64,
    fallback_retry_threshold: u64,
    retry_budget: Option<u64>,
    err: CodexErr,
    client_session: &mut ModelClientSession,
    sess: &Session,
    turn_context: &TurnContext,
    _request: ResponsesStreamRequest,
    allow_route_recovery: bool,
) -> Result<(), CodexErr> {
    client_session.begin_retry_time_budget();

    if client_session.provider_runtime_changed() {
        *retries = 0;
        client_session.sync_latest_provider_runtime_generation();
        return Ok(());
    }

    let persistent_route_recovery = requires_persistent_route_recovery(&err);
    let effective_retry_budget =
        effective_stream_retry_budget(retry_budget, persistent_route_recovery);
    let effective_fallback_retry_threshold =
        effective_fallback_retry_threshold(fallback_retry_threshold, persistent_route_recovery);

    if *retries >= effective_fallback_retry_threshold
        && client_session.try_switch_fallback_transport(
            &turn_context.session_telemetry,
            &turn_context.model_info,
        )
    {
        let fallback_message = if err.is_retry_time_budget_interrupted() {
            format!(
                "{RETRY_TIME_BUDGET_INTERRUPTED_MESSAGE} Falling back from WebSockets to HTTPS transport. {err:#}"
            )
        } else {
            format!("Falling back from WebSockets to HTTPS transport. {err:#}")
        };
        sess.notify_transient_stream_error(
            turn_context,
            fallback_message,
            err,
        )
        .await;
        *retries = 0;
        return Ok(());
    }

    if effective_retry_budget.is_none_or(|max_retries| *retries < max_retries) {
        *retries += 1;
        let retry_count = *retries;
        let display_retry_count = next_display_retry_count(display_retries);
        if allow_route_recovery && retry_count % ROUTE_RECOVERY_RETRY_THRESHOLD == 0 {
            client_session.activate_retry_route_recovery();
        }
        let display_max_retries = effective_retry_budget.unwrap_or(u64::MAX);
        let delay = response_stream_retry_delay(&err);
        // Surface every visible retry so the user-facing count remains continuous from 1.
        let status_message = if err.is_retry_time_budget_interrupted() {
            format!(
                "{RETRY_TIME_BUDGET_INTERRUPTED_MESSAGE} {}",
                transport_retry_status_message(display_retry_count, display_max_retries)
            )
        } else {
            transport_retry_status_message(display_retry_count, display_max_retries)
        };
        sess.notify_transient_stream_error(
            turn_context,
            status_message,
            err,
        )
        .await;
        if sleep_stream_retry_delay(delay, retries, client_session).await {
            let interruption = CodexErr::RetryTimeBudgetInterrupted(
                RETRY_TIME_BUDGET_INTERRUPTED_MESSAGE.to_string(),
            );
            sess.notify_transient_stream_error(
                turn_context,
                RETRY_TIME_BUDGET_INTERRUPTED_MESSAGE.to_string(),
                interruption,
            )
            .await;
        }
        return Ok(());
    }

    Err(err)
}

fn effective_stream_retry_budget(
    retry_budget: Option<u64>,
    requires_persistent_route_recovery: bool,
) -> Option<u64> {
    if requires_persistent_route_recovery {
        None
    } else {
        retry_budget
    }
}

fn effective_fallback_retry_threshold(
    fallback_retry_threshold: u64,
    requires_persistent_route_recovery: bool,
) -> u64 {
    if requires_persistent_route_recovery {
        fallback_retry_threshold.max(ROUTE_RECOVERY_RETRY_THRESHOLD)
    } else {
        fallback_retry_threshold
    }
}

fn requires_persistent_route_recovery(err: &CodexErr) -> bool {
    matches!(err, CodexErr::ServerOverloaded)
}

fn retry_status_suffix(retries: u64, max_retries: u64) -> String {
    if max_retries == u64::MAX {
        format!("{retries} (auto retry)")
    } else {
        format!("{retries}/{max_retries}")
    }
}

fn transport_retry_status_message(retries: u64, max_retries: u64) -> String {
    format!(
        "Reconnecting... {}",
        retry_status_suffix(retries, max_retries)
    )
}

fn response_stream_retry_delay(_err: &CodexErr) -> Duration {
    fixed_retry_delay()
}

fn next_display_retry_count(display_retries: &mut u64) -> u64 {
    *display_retries = (*display_retries).saturating_add(1);
    *display_retries
}

async fn sleep_stream_retry_delay(
    delay: Duration,
    retries: &mut u64,
    client_session: &mut ModelClientSession,
) -> bool {
    if delay.is_zero() {
        return false;
    }

    let start = tokio::time::Instant::now();
    loop {
        if client_session
            .retry_time_budget()
            .interruption_error()
            .is_some()
        {
            return true;
        }
        if client_session.provider_runtime_changed() {
            *retries = 0;
            client_session.sync_latest_provider_runtime_generation();
            return false;
        }

        let elapsed = start.elapsed();
        if elapsed >= delay {
            return false;
        }

        tokio::time::sleep((delay - elapsed).min(STREAM_RETRY_INTERRUPT_POLL_INTERVAL)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::next_display_retry_count;
    use super::response_stream_retry_delay;
    use super::transport_retry_status_message;
    use codex_protocol::error::CodexErr;
    use std::time::Duration;

    #[test]
    fn visible_stream_retry_count_survives_internal_retry_reset() {
        let mut internal_retries = 0;
        let mut display_retries = 0;

        internal_retries += 1;
        assert_eq!(internal_retries, 1);
        assert_eq!(next_display_retry_count(&mut display_retries), 1);
        internal_retries += 1;
        assert_eq!(internal_retries, 2);
        assert_eq!(next_display_retry_count(&mut display_retries), 2);
        internal_retries += 1;
        assert_eq!(internal_retries, 3);
        assert_eq!(next_display_retry_count(&mut display_retries), 3);

        internal_retries = 0;
        internal_retries += 1;

        assert_eq!(internal_retries, 1);
        assert_eq!(next_display_retry_count(&mut display_retries), 4);
    }

    #[test]
    fn visible_stream_retry_message_marks_automatic_retry() {
        let message = transport_retry_status_message(6, u64::MAX);

        assert!(message.starts_with("Reconnecting... 6"));
        assert!(message.contains("auto retry"));
        assert!(!message.contains("unbounded"));
        assert!(!message.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn stream_retry_delay_is_fixed_and_ignores_retry_after() {
        let short_retry_after = CodexErr::Stream(
            "retry after short".to_string(),
            Some(Duration::from_millis(28)),
        );
        let long_retry_after = CodexErr::Stream(
            "retry after long".to_string(),
            Some(Duration::from_secs(35)),
        );
        let no_retry_after = CodexErr::Stream("no retry after".to_string(), None);

        assert_eq!(
            response_stream_retry_delay(&short_retry_after),
            Duration::from_secs(5)
        );
        assert_eq!(
            response_stream_retry_delay(&long_retry_after),
            Duration::from_secs(5)
        );
        assert_eq!(
            response_stream_retry_delay(&no_retry_after),
            Duration::from_secs(5)
        );
    }
}
