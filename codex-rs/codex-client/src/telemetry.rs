use codex_http_client::TransportError;
use http::StatusCode;
use std::time::Duration;

/// API specific telemetry.
pub trait RequestTelemetry: Send + Sync {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<StatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
        emit_log_trace: bool,
    );

    fn on_request_retry(
        &self,
        _retry_number: u64,
        _max_attempts: u64,
        _status: Option<StatusCode>,
        _error: &TransportError,
    ) {
    }

    fn retry_after_unauthorized(&self) -> bool {
        false
    }

    fn can_retry_after_unauthorized(&self) -> bool {
        false
    }

    /// Returns whether the current request-level retry loop can keep using its
    /// existing request setup.
    ///
    /// Implementations can return `false` when request-scoped state becomes
    /// stale, such as after a provider runtime refresh changes the endpoint or
    /// auth for the current session.
    fn can_continue_request_retry(&self) -> bool {
        true
    }

    /// Returns the full timeout for the next independent request attempt.
    /// The request layer queries this before every attempt, including the first,
    /// and converts an overdue future into a normal retryable timeout.
    fn request_retry_timeout(&self) -> Option<Duration> {
        None
    }

    fn request_retry_interruption_reason(&self) -> Option<String> {
        None
    }
}
