use codex_client::Request;
use codex_client::RequestCompression;
use codex_client::RetryOn;
use codex_client::RetryPolicy;
use codex_client::TransportError;
use http::Method;
use http::StatusCode;
use http::header::HeaderMap;
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

/// High-level retry configuration for a provider.
///
/// This is converted into a `RetryPolicy` used by `codex-client` to drive
/// transport-level retries for both unary and streaming calls.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u64,
    pub base_delay: Duration,
    pub retry_402: bool,
    pub retry_429: bool,
    pub retry_5xx: bool,
    pub retry_transport: bool,
}

impl RetryConfig {
    pub fn to_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_attempts: self.max_attempts,
            base_delay: self.base_delay,
            retry_on: RetryOn {
                retry_402: self.retry_402,
                retry_429: self.retry_429,
                retry_5xx: self.retry_5xx,
                retry_transport: self.retry_transport,
            },
        }
    }
}

pub fn responses_http_status_is_retryable(status: StatusCode) -> bool {
    status != StatusCode::UNAUTHORIZED
}

pub fn should_retry_request_error(
    policy: &RetryPolicy,
    request: &Request,
    err: &TransportError,
    attempt: u64,
) -> bool {
    if attempt >= policy.max_attempts {
        return false;
    }

    match err {
        TransportError::Http {
            status, url, body, ..
        } => {
            let effective_url = url.as_deref().unwrap_or(&request.url);
            if request_targets_responses_endpoint(effective_url) {
                return responses_http_status_is_retryable(*status);
            }

            (policy.retry_on.retry_402
                && status.as_u16() == 402
                && payment_required_body_is_usage_limit(body.as_deref()))
                || (policy.retry_on.retry_429 && status.as_u16() == 429)
                || (policy.retry_on.retry_5xx && status.is_server_error())
        }
        TransportError::Timeout | TransportError::Network(_) => policy.retry_on.retry_transport,
        TransportError::RetryLimit | TransportError::Build(_) => false,
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

fn request_targets_responses_endpoint(url: &str) -> bool {
    Url::parse(url)
        .map(|parsed| parsed.path().ends_with("/responses"))
        .unwrap_or_else(|_| url.contains("/responses"))
}

/// HTTP endpoint configuration used to talk to a concrete API deployment.
///
/// Encapsulates base URL, default headers, query params, retry policy, and
/// stream idle timeout, plus helper methods for building requests.
#[derive(Debug, Clone)]
pub struct Provider {
    pub name: String,
    pub base_url: String,
    pub query_params: Option<HashMap<String, String>>,
    pub headers: HeaderMap,
    pub retry: RetryConfig,
    pub stream_idle_timeout: Duration,
}

impl Provider {
    pub fn with_retry_max_attempts(mut self, max_attempts: u64) -> Self {
        self.retry.max_attempts = max_attempts;
        self
    }

    pub fn url_for_path(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let mut url = if path.is_empty() {
            base.to_string()
        } else {
            format!("{base}/{path}")
        };

        if let Some(params) = &self.query_params
            && !params.is_empty()
        {
            let qs = params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&qs);
        }

        url
    }

    pub fn build_request(&self, method: Method, path: &str) -> Request {
        Request {
            method,
            url: self.url_for_path(path),
            headers: self.headers.clone(),
            body: None,
            compression: RequestCompression::None,
            timeout: None,
        }
    }

    pub fn is_azure_responses_endpoint(&self) -> bool {
        is_azure_responses_wire_base_url(&self.name, Some(&self.base_url))
    }

    pub fn websocket_url_for_path(&self, path: &str) -> Result<Url, url::ParseError> {
        let mut url = Url::parse(&self.url_for_path(path))?;

        let scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            "ws" | "wss" => return Ok(url),
            _ => return Ok(url),
        };
        let _ = url.set_scheme(scheme);
        Ok(url)
    }
}

pub fn is_azure_responses_wire_base_url(name: &str, base_url: Option<&str>) -> bool {
    if name.eq_ignore_ascii_case("azure") {
        return true;
    }

    let Some(base_url) = base_url else {
        return false;
    };

    let base = base_url.to_ascii_lowercase();
    base.contains("openai.azure.") || matches_azure_responses_base_url(&base)
}

fn matches_azure_responses_base_url(base_url: &str) -> bool {
    const AZURE_MARKERS: [&str; 5] = [
        "cognitiveservices.azure.",
        "aoai.azure.",
        "azure-api.",
        "azurefd.",
        "windows.net/openai",
    ];
    AZURE_MARKERS.iter().any(|marker| base_url.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn detects_azure_responses_base_urls() {
        let positive_cases = [
            "https://foo.openai.azure.com/openai",
            "https://foo.openai.azure.us/openai/deployments/bar",
            "https://foo.cognitiveservices.azure.cn/openai",
            "https://foo.aoai.azure.com/openai",
            "https://foo.openai.azure-api.net/openai",
            "https://foo.z01.azurefd.net/",
        ];

        for base_url in positive_cases {
            assert!(
                is_azure_responses_wire_base_url("test", Some(base_url)),
                "expected {base_url} to be detected as Azure"
            );
        }

        assert!(is_azure_responses_wire_base_url(
            "Azure",
            Some("https://example.com")
        ));

        let negative_cases = [
            "https://api.openai.com/v1",
            "https://example.com/openai",
            "https://myproxy.azurewebsites.net/openai",
        ];

        for base_url in negative_cases {
            assert!(
                !is_azure_responses_wire_base_url("test", Some(base_url)),
                "expected {base_url} not to be detected as Azure"
            );
        }
    }

    #[test]
    fn responses_http_status_retry_only_excludes_401() {
        assert!(responses_http_status_is_retryable(StatusCode::BAD_REQUEST));
        assert!(responses_http_status_is_retryable(StatusCode::FORBIDDEN));
        assert!(responses_http_status_is_retryable(StatusCode::PAYMENT_REQUIRED));
        assert!(responses_http_status_is_retryable(StatusCode::TOO_MANY_REQUESTS));
        assert!(!responses_http_status_is_retryable(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn responses_requests_retry_non_401_http_statuses_without_whitelist() {
        let policy = RetryPolicy {
            max_attempts: 4,
            base_delay: Duration::from_millis(200),
            retry_on: RetryOn {
                retry_402: false,
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
        };
        let request = Request::new(
            Method::POST,
            "https://chatgpt.com/backend-api/codex/responses".to_string(),
        );
        let err = TransportError::Http {
            status: StatusCode::FORBIDDEN,
            url: Some(request.url.clone()),
            headers: None,
            body: Some(r#"{"detail":"forbidden"}"#.to_string()),
        };

        assert!(should_retry_request_error(&policy, &request, &err, 0));
    }

    #[test]
    fn responses_requests_do_not_retry_401() {
        let policy = RetryPolicy {
            max_attempts: 4,
            base_delay: Duration::from_millis(200),
            retry_on: RetryOn {
                retry_402: true,
                retry_429: true,
                retry_5xx: true,
                retry_transport: true,
            },
        };
        let request = Request::new(
            Method::POST,
            "https://chatgpt.com/backend-api/codex/responses".to_string(),
        );
        let err = TransportError::Http {
            status: StatusCode::UNAUTHORIZED,
            url: Some(request.url.clone()),
            headers: None,
            body: Some(r#"{"detail":"Unauthorized"}"#.to_string()),
        };

        assert!(!should_retry_request_error(&policy, &request, &err, 0));
    }

    #[test]
    fn non_responses_requests_keep_existing_402_whitelist_behavior() {
        let policy = RetryPolicy {
            max_attempts: 4,
            base_delay: Duration::from_millis(200),
            retry_on: RetryOn {
                retry_402: true,
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
        };
        let request = Request::new(Method::GET, "https://chatgpt.com/backend-api/codex/models".to_string());
        let usage_limit_err = TransportError::Http {
            status: StatusCode::PAYMENT_REQUIRED,
            url: Some(request.url.clone()),
            headers: None,
            body: Some("Daily spending limit reached".to_string()),
        };
        let non_usage_limit_err = TransportError::Http {
            status: StatusCode::PAYMENT_REQUIRED,
            url: Some(request.url.clone()),
            headers: None,
            body: Some(r#"{"error":{"type":"usage_not_included"}}"#.to_string()),
        };

        assert!(should_retry_request_error(
            &policy,
            &request,
            &usage_limit_err,
            0
        ));
        assert!(!should_retry_request_error(
            &policy,
            &request,
            &non_usage_limit_err,
            0
        ));
    }
}
