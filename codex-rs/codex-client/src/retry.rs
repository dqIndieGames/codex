use crate::error::TransportError;
use crate::request::Request;
use http::HeaderMap;
use http::HeaderValue;
use http::StatusCode;
use rand::Rng;
use serde::Deserialize;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub base_delay: Duration,
    pub retry_on: RetryOn,
}

#[derive(Debug, Clone)]
pub struct RetryOn {
    pub retry_429: bool,
    pub retry_5xx: bool,
    pub retry_transport: bool,
}

impl RetryOn {
    pub fn should_retry(&self, err: &TransportError, attempt: u64, max_attempts: u64) -> bool {
        if attempt >= max_attempts {
            return false;
        }
        match err {
            TransportError::Http { status, body, .. } => {
                (self.retry_429 && status.as_u16() == 429 && !is_non_retryable_429(body.as_deref()))
                    || (self.retry_5xx && status.is_server_error())
            }
            TransportError::Timeout | TransportError::Network(_) => self.retry_transport,
            _ => false,
        }
    }
}

pub fn backoff(base: Duration, attempt: u64) -> Duration {
    if attempt == 0 {
        return base;
    }
    let exp = 2u64.saturating_pow(attempt as u32 - 1);
    let millis = base.as_millis() as u64;
    let raw = millis.saturating_mul(exp);
    let jitter: f64 = rand::rng().random_range(0.9..1.1);
    Duration::from_millis((raw as f64 * jitter) as u64)
}

pub fn retry_delay_for_error(err: &TransportError) -> Option<Duration> {
    let TransportError::Http {
        status,
        headers,
        body,
        ..
    } = err
    else {
        return None;
    };

    if *status != StatusCode::TOO_MANY_REQUESTS || is_non_retryable_429(body.as_deref()) {
        return None;
    }

    parse_retry_after_headers(headers.as_ref()).or_else(|| parse_retry_after_body(body.as_deref()))
}

pub async fn run_with_retry<T, F, Fut>(
    policy: RetryPolicy,
    mut make_req: impl FnMut() -> Request,
    op: F,
) -> Result<T, TransportError>
where
    F: Fn(Request, u64) -> Fut,
    Fut: Future<Output = Result<T, TransportError>>,
{
    for attempt in 0..=policy.max_attempts {
        let req = make_req();
        match op(req, attempt).await {
            Ok(resp) => return Ok(resp),
            Err(err)
                if policy
                    .retry_on
                    .should_retry(&err, attempt, policy.max_attempts) =>
            {
                let delay = retry_delay_for_error(&err)
                    .unwrap_or_else(|| backoff(policy.base_delay, attempt + 1));
                sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
    Err(TransportError::RetryLimit)
}

#[derive(Debug, Deserialize)]
struct RetryErrorEnvelope {
    error: RetryErrorPayload,
}

#[derive(Debug, Deserialize)]
struct RetryErrorPayload {
    #[serde(default, rename = "type")]
    error_type: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

fn is_non_retryable_429(body: Option<&str>) -> bool {
    let Some(body) = body else {
        return false;
    };

    let Ok(parsed) = serde_json::from_str::<RetryErrorEnvelope>(body) else {
        return false;
    };

    matches!(
        parsed
            .error
            .error_type
            .as_deref()
            .or(parsed.error.code.as_deref()),
        Some("usage_limit_reached" | "usage_not_included")
    )
}

fn parse_retry_after_headers(headers: Option<&HeaderMap>) -> Option<Duration> {
    let headers = headers?;

    if let Some(ms) = headers.get("retry-after-ms").and_then(parse_header_f64) {
        return Some(Duration::from_secs_f64((ms / 1000.0).max(0.0)));
    }

    headers
        .get("retry-after")
        .and_then(parse_header_f64)
        .map(|seconds| Duration::from_secs_f64(seconds.max(0.0)))
}

fn parse_header_f64(value: &HeaderValue) -> Option<f64> {
    value.to_str().ok()?.trim().parse::<f64>().ok()
}

fn parse_retry_after_body(body: Option<&str>) -> Option<Duration> {
    let message = parse_retry_message(body?)?;
    let captures = retry_after_regex().captures(&message)?;
    let value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    let unit = captures.get(2)?.as_str().to_ascii_lowercase();

    if unit == "ms" || unit.starts_with("milli") {
        Some(Duration::from_secs_f64((value / 1000.0).max(0.0)))
    } else {
        Some(Duration::from_secs_f64(value.max(0.0)))
    }
}

fn parse_retry_message(body: &str) -> Option<String> {
    serde_json::from_str::<RetryErrorEnvelope>(body)
        .ok()
        .and_then(|parsed| parsed.error.message)
        .or_else(|| Some(body.to_string()))
}

fn retry_after_regex() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    #[expect(clippy::unwrap_used)]
    RE.get_or_init(|| {
        regex_lite::Regex::new(
            r"(?i)(?:try again|retry)(?:\s+after)?\s+in?\s*(\d+(?:\.\d+)?)\s*(ms|milliseconds?|s|sec|seconds?)",
        )
        .unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn http_error(
        status: StatusCode,
        headers: Option<HeaderMap>,
        body: Option<&str>,
    ) -> TransportError {
        TransportError::Http {
            status,
            url: Some("https://example.test/v1/responses".to_string()),
            headers,
            body: body.map(str::to_string),
        }
    }

    #[test]
    fn retryable_429_does_not_retry_usage_limit_reached() {
        let retry_on = RetryOn {
            retry_429: true,
            retry_5xx: true,
            retry_transport: true,
        };
        let err = http_error(
            StatusCode::TOO_MANY_REQUESTS,
            None,
            Some(r#"{"error":{"type":"usage_limit_reached","message":"hard limit"}}"#),
        );

        assert!(!retry_on.should_retry(&err, 0, 4));
    }

    #[test]
    fn retry_delay_prefers_retry_after_header_for_retryable_429() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("3"));
        let err = http_error(
            StatusCode::TOO_MANY_REQUESTS,
            Some(headers),
            Some(
                r#"{"error":{"code":"rate_limit_exceeded","message":"Rate limit exceeded. Try again in 35 seconds."}}"#,
            ),
        );

        assert_eq!(retry_delay_for_error(&err), Some(Duration::from_secs(3)));
    }
}
