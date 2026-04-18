use crate::error::TransportError;
use crate::request::Request;
use rand::Rng;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRY_DELAY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub base_delay: Duration,
    pub retry_on: RetryOn,
}

#[derive(Debug, Clone)]
pub struct RetryOn {
    pub retry_402: bool,
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
                (self.retry_402
                    && status.as_u16() == 402
                    && payment_required_body_is_usage_limit(body.as_deref()))
                    || (self.retry_429 && status.as_u16() == 429)
                    || (self.retry_5xx && status.is_server_error())
            }
            TransportError::Timeout | TransportError::Network(_) => self.retry_transport,
            _ => false,
        }
    }
}

fn payment_required_body_is_usage_limit(body: Option<&str>) -> bool {
    let Some(body) = body.map(str::trim).filter(|body| !body.is_empty()) else {
        return false;
    };
    let normalized = body.to_ascii_lowercase();
    normalized.contains("usage_limit_reached")
        || normalized.contains("usage limit reached")
        || normalized.contains("daily spending limit reached")
}

pub fn backoff(base: Duration, attempt: u64) -> Duration {
    if attempt == 0 {
        return base.min(MAX_RETRY_DELAY);
    }
    let exp = 2u64.saturating_pow(attempt as u32 - 1);
    let millis = base.as_millis() as u64;
    let raw = millis.saturating_mul(exp);
    let jitter: f64 = rand::rng().random_range(0.9..1.1);
    Duration::from_millis((raw as f64 * jitter) as u64).min(MAX_RETRY_DELAY)
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
                sleep(backoff(policy.base_delay, attempt + 1)).await;
            }
            Err(err) => return Err(err),
        }
    }
    Err(TransportError::RetryLimit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn retry_402_requires_usage_limit_marker() {
        let retry_on = RetryOn {
            retry_402: true,
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
        };

        let usage_limit_err = TransportError::Http {
            status: StatusCode::PAYMENT_REQUIRED,
            url: None,
            headers: None,
            body: Some("Daily spending limit reached".to_string()),
        };
        assert!(retry_on.should_retry(&usage_limit_err, 0, 1));

        let non_usage_limit_err = TransportError::Http {
            status: StatusCode::PAYMENT_REQUIRED,
            url: None,
            headers: None,
            body: Some(r#"{"error":{"type":"usage_not_included"}}"#.to_string()),
        };
        assert!(!retry_on.should_retry(&non_usage_limit_err, 0, 1));
    }

    #[test]
    fn request_backoff_is_capped_to_ten_seconds() {
        assert_eq!(
            backoff(Duration::from_millis(200), 32),
            Duration::from_secs(10)
        );
    }
}
