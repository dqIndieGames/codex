use codex_http_client::Request;
use codex_http_client::TransportError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

pub const FIXED_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u64,
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
            TransportError::Http { .. }
            | TransportError::Timeout
            | TransportError::Network(_) => true,
            TransportError::RetryLimit
            | TransportError::RetryInterrupted(_)
            | TransportError::Build(_) => false,
        }
    }
}

pub fn fixed_retry_delay() -> Duration {
    FIXED_RETRY_DELAY
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
                sleep(fixed_retry_delay()).await;
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
    fn all_remote_request_errors_share_the_retry_budget() {
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
        assert!(retry_on.should_retry(&non_usage_limit_err, 0, 1));
    }

    #[test]
    fn request_retry_delay_is_fixed_to_five_seconds() {
        assert_eq!(fixed_retry_delay(), Duration::from_secs(5));
    }
}
