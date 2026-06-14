use crate::error::ApiError;
use crate::provider::RequestRetryRoute;
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

const REQUEST_RETRY_INTERRUPTED: &str = "provider runtime changed during request retry";
const REQUEST_RETRY_INTERRUPT_POLL_INTERVAL: Duration = Duration::from_millis(250);

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

fn responses_request_will_continue_main_chain(endpoint: &str, should_retry: bool) -> bool {
    RequestRetryRoute::from_endpoint(endpoint).is_responses() && should_retry
}

fn request_retry_interrupted_error(
    telemetry: Option<&Arc<dyn RequestTelemetry>>,
) -> TransportError {
    let reason = telemetry
        .and_then(|telemetry| telemetry.request_retry_interruption_reason())
        .unwrap_or_else(|| REQUEST_RETRY_INTERRUPTED.to_string());
    TransportError::RetryInterrupted(reason)
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
        if telemetry
            .as_ref()
            .is_some_and(|telemetry| !telemetry.can_continue_request_retry())
        {
            return Err(TransportError::RetryInterrupted(
                REQUEST_RETRY_INTERRUPTED.to_string(),
            ));
        }
        let req = make_request();
        let retry_route = RequestRetryRoute::from_endpoint(endpoint);
        let start = Instant::now();
        let result = send.clone()(req).await;
        let should_retry = match &result {
            Ok(_) => false,
            Err(err) => should_retry_request_error(&policy, retry_route, err, attempt),
        };
        let can_notify_request_retry = !should_retry
            || telemetry
                .as_ref()
                .is_none_or(|telemetry| telemetry.can_continue_request_retry());

        if let Some(t) = telemetry.as_ref() {
            let (status, err) = match &result {
                Ok(resp) => (Some(resp.status()), None),
                Err(err) => (http_status(err), Some(err)),
            };
            let emit_log_trace = !retry_route.is_responses()
                || (!retry_after_unauthorized
                    && attempt == 0
                    && !responses_request_will_continue_main_chain(endpoint, should_retry));
            t.on_request(attempt, status, err, start.elapsed(), emit_log_trace);
            if let Some(err) = err
                && should_retry
                && can_notify_request_retry
            {
                t.on_request_retry(attempt + 1, policy.max_attempts, status, err);
            }
        }
        let can_continue_request_retry = !should_retry
            || telemetry
                .as_ref()
                .is_none_or(|telemetry| telemetry.can_continue_request_retry());

        match result {
            Ok(resp) => return Ok(resp),
            Err(_err) if should_retry => {
                if !can_continue_request_retry {
                    return Err(request_retry_interrupted_error(telemetry.as_ref()));
                }
                sleep_request_retry_delay(
                    backoff(policy.base_delay, attempt + 1),
                    telemetry.as_ref(),
                )
                .await?;
            }
            Err(err) => return Err(err),
        }
    }

    Err(TransportError::RetryLimit)
}

async fn sleep_request_retry_delay(
    delay: Duration,
    telemetry: Option<&Arc<dyn RequestTelemetry>>,
) -> Result<(), TransportError> {
    if delay.is_zero() {
        return Ok(());
    }

    let start = Instant::now();
    loop {
        if telemetry.is_some_and(|telemetry| !telemetry.can_continue_request_retry()) {
            return Err(request_retry_interrupted_error(telemetry));
        }

        let elapsed = start.elapsed();
        if elapsed >= delay {
            return Ok(());
        }

        sleep((delay - elapsed).min(REQUEST_RETRY_INTERRUPT_POLL_INTERVAL)).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::provider::RequestRetryRoute;
    use codex_client::Request;
    use codex_client::RequestTelemetry;
    use codex_client::Response;
    use codex_client::RetryOn;
    use codex_client::RetryPolicy;
    use http::Method;
    use http::StatusCode;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    struct InterruptingTelemetry {
        allow_retry: AtomicBool,
        retry_notifications: AtomicU64,
    }

    impl RequestTelemetry for InterruptingTelemetry {
        fn on_request(
            &self,
            _attempt: u64,
            _status: Option<StatusCode>,
            _error: Option<&codex_client::TransportError>,
            _duration: Duration,
            _emit_log_trace: bool,
        ) {
        }

        fn on_request_retry(
            &self,
            _retry_number: u64,
            _max_attempts: u64,
            _status: Option<StatusCode>,
            _error: &codex_client::TransportError,
        ) {
            self.retry_notifications.fetch_add(1, Ordering::AcqRel);
            self.allow_retry.store(false, Ordering::Release);
        }

        fn can_continue_request_retry(&self) -> bool {
            self.allow_retry.load(Ordering::Acquire)
        }
    }

    #[test]
    fn primary_responses_endpoint_accepts_relative_path() {
        assert!(RequestRetryRoute::from_endpoint("responses").is_responses());
    }

    #[test]
    fn primary_responses_endpoint_accepts_absolute_path() {
        assert!(RequestRetryRoute::from_endpoint("/responses").is_responses());
    }

    #[test]
    fn primary_responses_endpoint_rejects_other_routes() {
        assert!(!RequestRetryRoute::from_endpoint("responses/compact").is_responses());
        assert!(!RequestRetryRoute::from_endpoint("/responses/compact").is_responses());
        assert!(!RequestRetryRoute::from_endpoint("models").is_responses());
    }

    #[tokio::test]
    async fn request_retry_guard_interrupts_before_next_attempt() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_secs(30),
            retry_on: RetryOn {
                retry_402: true,
                retry_429: true,
                retry_5xx: true,
                retry_transport: true,
            },
        };
        let telemetry = Arc::new(InterruptingTelemetry {
            allow_retry: AtomicBool::new(true),
            retry_notifications: AtomicU64::new(0),
        });
        let telemetry_handle = Arc::clone(&telemetry);
        let request_telemetry: Arc<dyn RequestTelemetry> = telemetry;
        let send_count = Arc::new(AtomicU64::new(0));
        let send_count_handle = Arc::clone(&send_count);

        let result: Result<Response, codex_client::TransportError> = run_with_request_telemetry(
            policy,
            "responses",
            Some(request_telemetry),
            || Request::new(Method::POST, "https://old.example/v1/responses".to_string()),
            move |_req| {
                let send_count = Arc::clone(&send_count);
                async move {
                    send_count.fetch_add(1, Ordering::AcqRel);
                    Err(codex_client::TransportError::Http {
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        url: Some("https://old.example/v1/responses".to_string()),
                        headers: None,
                        body: Some("old provider unavailable".to_string()),
                    })
                }
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(codex_client::TransportError::RetryInterrupted(_))
        ));
        assert_eq!(send_count_handle.load(Ordering::Acquire), 1);
        assert_eq!(
            telemetry_handle.retry_notifications.load(Ordering::Acquire),
            1
        );
    }
}
