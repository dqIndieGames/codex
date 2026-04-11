use crate::error::ApiError;
use crate::provider::should_retry_request_error;
use codex_client::Request;
use codex_client::RequestTelemetry;
use codex_client::Response;
use codex_client::RetryPolicy;
use codex_client::StreamResponse;
use codex_client::TransportError;
use codex_client::backoff;
use http::StatusCode;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::Message;

/// Generic telemetry.
pub trait SseTelemetry: Send + Sync {
    fn on_sse_poll(
        &self,
        result: &Result<
            Option<
                Result<
                    eventsource_stream::Event,
                    eventsource_stream::EventStreamError<TransportError>,
                >,
            >,
            tokio::time::error::Elapsed,
        >,
        duration: Duration,
    );
}

/// Telemetry for Responses WebSocket transport.
pub trait WebsocketTelemetry: Send + Sync {
    fn on_ws_request(&self, duration: Duration, error: Option<&ApiError>, connection_reused: bool);

    fn on_ws_event(
        &self,
        result: &Result<Option<Result<Message, Error>>, ApiError>,
        duration: Duration,
    );
}

pub(crate) trait WithStatus {
    fn status(&self) -> StatusCode;
}

fn http_status(err: &TransportError) -> Option<StatusCode> {
    match err {
        TransportError::Http { status, .. } => Some(*status),
        _ => None,
    }
}

impl WithStatus for Response {
    fn status(&self) -> StatusCode {
        self.status
    }
}

impl WithStatus for StreamResponse {
    fn status(&self) -> StatusCode {
        self.status
    }
}

fn is_primary_responses_endpoint(endpoint: &str) -> bool {
    endpoint == "/responses"
}

fn responses_request_will_continue_main_chain(
    endpoint: &str,
    error: Option<&TransportError>,
    should_retry: bool,
    can_retry_after_unauthorized: bool,
) -> bool {
    is_primary_responses_endpoint(endpoint)
        && (should_retry
            || matches!(
                error,
                Some(TransportError::Http { status, .. })
                    if *status == StatusCode::UNAUTHORIZED && can_retry_after_unauthorized
            ))
}

pub(crate) async fn run_with_request_telemetry<T, F, Fut>(
    policy: RetryPolicy,
    endpoint: &str,
    telemetry: Option<Arc<dyn RequestTelemetry>>,
    mut make_request: impl FnMut() -> Request,
    send: F,
) -> Result<T, TransportError>
where
    T: WithStatus,
    F: Clone + Fn(Request) -> Fut,
    Fut: Future<Output = Result<T, TransportError>>,
{
    // Attach per-attempt request telemetry while keeping `/responses` request retries
    // aligned with the shared main-chain retry classifier.
    for attempt in 0..=policy.max_attempts {
        let retry_after_unauthorized = telemetry
            .as_ref()
            .is_some_and(|telemetry| telemetry.retry_after_unauthorized());
        let can_retry_after_unauthorized = telemetry
            .as_ref()
            .is_some_and(|telemetry| telemetry.can_retry_after_unauthorized());
        let req = make_request();
        let req_for_retry = req.clone();
        let start = Instant::now();
        let result = send.clone()(req).await;
        let should_retry = match &result {
            Ok(_) => false,
            Err(err) => should_retry_request_error(&policy, &req_for_retry, err, attempt),
        };

        if let Some(t) = telemetry.as_ref() {
            let (status, err) = match &result {
                Ok(resp) => (Some(resp.status()), None),
                Err(err) => (http_status(err), Some(err)),
            };
            let emit_log_trace = !is_primary_responses_endpoint(endpoint)
                || (!retry_after_unauthorized
                    && attempt == 0
                    && !responses_request_will_continue_main_chain(
                        endpoint,
                        err,
                        should_retry,
                        can_retry_after_unauthorized,
                    ));
            t.on_request(attempt, status, err, start.elapsed(), emit_log_trace);
            if let Some(err) = err
                && should_retry
            {
                t.on_request_retry(attempt + 1, policy.max_attempts, status, err);
            }
        }

        match result {
            Ok(resp) => return Ok(resp),
            Err(err) if should_retry => {
                sleep(backoff(policy.base_delay, attempt + 1)).await;
            }
            Err(err) => return Err(err),
        }
    }

    Err(TransportError::RetryLimit)
}
