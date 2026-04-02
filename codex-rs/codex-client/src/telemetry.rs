use crate::error::TransportError;
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
    );

    fn on_request_retry(
        &self,
        _retry_number: u64,
        _max_attempts: u64,
        _status: Option<StatusCode>,
        _error: &TransportError,
    ) {
    }
}
