pub use codex_client::Provider;
pub use codex_client::RetryConfig;
use codex_client::{RetryPolicy, TransportError};
use http::StatusCode;
use url::Url;

pub trait ProviderSource: Send + Sync {
    fn snapshot(&self) -> Provider;
}

pub fn is_chatgpt_codex_route(base_url: &str) -> bool {
    let Ok(url) = Url::parse(base_url.trim()) else {
        return false;
    };
    if url.scheme() != "https" && url.scheme() != "wss" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if !host.eq_ignore_ascii_case("chatgpt.com")
        && !host.to_ascii_lowercase().ends_with(".chatgpt.com")
    {
        return false;
    }
    url.path() == "/backend-api/codex" || url.path().starts_with("/backend-api/codex/")
}

pub fn responses_http_status_is_retryable(status: StatusCode) -> bool {
    let _ = status;
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRetryRoute {
    Responses,
    Other,
}

impl RequestRetryRoute {
    pub fn from_endpoint(endpoint: &str) -> Self {
        if endpoint.trim_start_matches('/') == "responses" {
            Self::Responses
        } else {
            Self::Other
        }
    }

    pub fn is_responses(self) -> bool {
        matches!(self, Self::Responses)
    }
}

pub fn should_retry_request_error(
    policy: &RetryPolicy,
    route: RequestRetryRoute,
    err: &TransportError,
    attempt: u64,
) -> bool {
    if attempt >= policy.max_attempts {
        return false;
    }

    match err {
        TransportError::Http { status, body, .. } => {
            let _ = (route, body);
            responses_http_status_is_retryable(*status)
        }
        TransportError::Timeout | TransportError::Network(_) | TransportError::Connection(_) => true,
        TransportError::RetryLimit
        | TransportError::RetryInterrupted(_)
        | TransportError::Build(_)
        | TransportError::Policy(_)
        | TransportError::ResponseTooLarge { .. } => false,
    }
}


pub fn is_azure_responses_provider(name: &str, base_url: Option<&str>) -> bool {
    if name.eq_ignore_ascii_case("azure") {
        true
    } else if let Some(base_url) = base_url {
        matches_azure_responses_base_url(base_url)
    } else {
        false
    }
}

fn matches_azure_responses_base_url(base_url: &str) -> bool {
    let base_url = base_url.to_ascii_lowercase();
    const AZURE_MARKERS: [&str; 6] = [
        "openai.azure.",
        "cognitiveservices.azure.",
        "aoai.azure.",
        "azure-api.",
        "azurefd.",
        "windows.net/openai",
    ];
    AZURE_MARKERS.iter().any(|marker| base_url.contains(marker))
}

impl ProviderSource for Provider {
    fn snapshot(&self) -> Provider {
        self.clone()
    }
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
                is_azure_responses_provider("test", Some(base_url)),
                "expected {base_url} to be detected as Azure"
            );
        }

        assert!(is_azure_responses_provider(
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
                !is_azure_responses_provider("test", Some(base_url)),
                "expected {base_url} not to be detected as Azure"
            );
        }
    }

    #[test]
    fn responses_http_status_retry_includes_401() {
        assert!(responses_http_status_is_retryable(StatusCode::BAD_REQUEST));
        assert!(responses_http_status_is_retryable(StatusCode::FORBIDDEN));
        assert!(responses_http_status_is_retryable(
            StatusCode::PAYMENT_REQUIRED
        ));
        assert!(responses_http_status_is_retryable(
            StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(responses_http_status_is_retryable(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn responses_requests_retry_non_401_http_statuses_without_whitelist() {
        let policy = RetryPolicy {
            max_attempts: 4,
            retry_on: RetryOn {
                retry_402: false,
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
        };
        let err = TransportError::Http {
            retry_after: None,
            status: StatusCode::FORBIDDEN,
            url: Some("https://proxy.example.com/no-route-hint".to_string()),
            headers: None,
            body: Some(r#"{"detail":"forbidden"}"#.to_string()),
        };

        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Responses,
            &err,
            0
        ));
    }

    #[test]
    fn responses_requests_retry_401() {
        let policy = RetryPolicy {
            max_attempts: 4,
            retry_on: RetryOn {
                retry_402: true,
                retry_429: true,
                retry_5xx: true,
                retry_transport: true,
            },
        };
        let err = TransportError::Http {
            retry_after: None,
            status: StatusCode::UNAUTHORIZED,
            url: Some("https://proxy.example.com/no-route-hint".to_string()),
            headers: None,
            body: Some(r#"{"detail":"Unauthorized"}"#.to_string()),
        };

        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Responses,
            &err,
            0
        ));
    }

    #[test]
    fn responses_requests_retry_usage_limit_429() {
        let policy = RetryPolicy {
            max_attempts: 4,
            retry_on: RetryOn {
                retry_402: true,
                retry_429: true,
                retry_5xx: true,
                retry_transport: true,
            },
        };
        let err = TransportError::Http {
            retry_after: None,
            status: StatusCode::TOO_MANY_REQUESTS,
            url: Some("https://proxy.example.com/no-route-hint".to_string()),
            headers: None,
            body: Some(
                r#"{"error":{"type":"usage_limit_reached","message":"The usage limit has been reached"}}"#
                    .to_string(),
            ),
        };

        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Responses,
            &err,
            0
        ));
    }

    #[test]
    fn non_responses_requests_retry_all_http_statuses_without_whitelist() {
        let policy = RetryPolicy {
            max_attempts: 4,
            retry_on: RetryOn {
                retry_402: false,
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
        };
        let payment_err = TransportError::Http {
            retry_after: None,
            status: StatusCode::PAYMENT_REQUIRED,
            url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
            headers: None,
            body: Some(r#"{"error":{"type":"usage_not_included"}}"#.to_string()),
        };
        let bad_request_err = TransportError::Http {
            retry_after: None,
            status: StatusCode::BAD_REQUEST,
            url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
            headers: None,
            body: Some(r#"{"error":{"type":"bad_request"}}"#.to_string()),
        };

        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Other,
            &payment_err,
            0
        ));
        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Other,
            &bad_request_err,
            0
        ));
    }

    #[test]
    fn remote_timeout_and_network_errors_retry_without_transport_flag() {
        let policy = RetryPolicy {
            max_attempts: 4,
            retry_on: RetryOn {
                retry_402: false,
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
        };

        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Other,
            &TransportError::Timeout,
            0
        ));
        assert!(should_retry_request_error(
            &policy,
            RequestRetryRoute::Other,
            &TransportError::Network("connection reset".to_string()),
            0
        ));
    }
}
