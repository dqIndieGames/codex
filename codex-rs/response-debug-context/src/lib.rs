use base64::Engine;
use codex_api::ApiError;
use codex_api::TransportError;

const REQUEST_ID_HEADER: &str = "x-request-id";
const OAI_REQUEST_ID_HEADER: &str = "x-oai-request-id";
const CF_RAY_HEADER: &str = "cf-ray";
const AUTH_ERROR_HEADER: &str = "x-openai-authorization-error";
const X_ERROR_JSON_HEADER: &str = "x-error-json";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResponseDebugContext {
    pub request_id: Option<String>,
    pub cf_ray: Option<String>,
    pub auth_error: Option<String>,
    pub auth_error_code: Option<String>,
}

pub fn extract_response_debug_context(transport: &TransportError) -> ResponseDebugContext {
    let mut context = ResponseDebugContext::default();

    let TransportError::Http {
        headers, body: _, ..
    } = transport
    else {
        return context;
    };

    let extract_header = |name: &str| {
        headers
            .as_ref()
            .and_then(|headers| headers.get(name))
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    };

    context.request_id =
        extract_header(REQUEST_ID_HEADER).or_else(|| extract_header(OAI_REQUEST_ID_HEADER));
    context.cf_ray = extract_header(CF_RAY_HEADER);
    context.auth_error = extract_header(AUTH_ERROR_HEADER);
    context.auth_error_code = extract_header(X_ERROR_JSON_HEADER).and_then(|encoded| {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        let parsed = serde_json::from_slice::<serde_json::Value>(&decoded).ok()?;
        parsed
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    });

    context
}

pub fn extract_response_debug_context_from_api_error(error: &ApiError) -> ResponseDebugContext {
    match error {
        ApiError::Transport(transport) => extract_response_debug_context(transport),
        _ => ResponseDebugContext::default(),
    }
}

pub fn telemetry_transport_error_message(error: &TransportError) -> String {
    match error {
        TransportError::Http { status, .. } => format!("http {}", status.as_u16()),
        TransportError::RetryLimit => "retry limit reached".to_string(),
        TransportError::RetryInterrupted(err) => err.to_string(),
        TransportError::Timeout => "timeout".to_string(),
        TransportError::Network(err) => err.to_string(),
        TransportError::Build(err) => err.to_string(),
    }
}

pub fn user_visible_transport_retry_details(error: &TransportError) -> String {
    match error {
        TransportError::Http { status, url, .. } => {
            let mut details = format!(
                "HTTP {} {}, retrying",
                status.as_u16(),
                http_status_reason(*status)
            );
            if let Some(endpoint) = safe_endpoint_from_url(url.as_deref()) {
                details.push_str(&format!(", endpoint: {endpoint}"));
            }
            let context = extract_response_debug_context(error);
            if let Some(request_id) = context.request_id {
                details.push_str(&format!(", request id: {request_id}"));
            }
            if let Some(cf_ray) = context.cf_ray {
                details.push_str(&format!(", cf-ray: {cf_ray}"));
            }
            if let Some(auth_error) = context.auth_error {
                details.push_str(&format!(", auth error: {auth_error}"));
            }
            if let Some(auth_error_code) = context.auth_error_code {
                details.push_str(&format!(", auth error code: {auth_error_code}"));
            }
            details
        }
        TransportError::RetryLimit => "Retry limit reached".to_string(),
        TransportError::RetryInterrupted(_) => "Request retry interrupted".to_string(),
        TransportError::Timeout => "Request timed out, retrying".to_string(),
        TransportError::Network(_) => "Network error, retrying".to_string(),
        TransportError::Build(_) => "Request build error".to_string(),
    }
}

fn http_status_reason(status: http::StatusCode) -> &'static str {
    match status {
        http::StatusCode::BAD_REQUEST => "Bad Request",
        http::StatusCode::UNAUTHORIZED => "Unauthorized",
        http::StatusCode::PAYMENT_REQUIRED => "Payment Required",
        http::StatusCode::FORBIDDEN => "Forbidden",
        http::StatusCode::NOT_FOUND => "Not Found",
        http::StatusCode::REQUEST_TIMEOUT => "Request Timeout",
        http::StatusCode::CONFLICT => "Conflict",
        http::StatusCode::TOO_MANY_REQUESTS => "Too Many Requests",
        http::StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error",
        http::StatusCode::BAD_GATEWAY => "Bad Gateway",
        http::StatusCode::SERVICE_UNAVAILABLE => "Service Unavailable",
        http::StatusCode::GATEWAY_TIMEOUT => "Gateway Timeout",
        _ if status.is_client_error() => "Client Error",
        _ if status.is_server_error() => "Server Error",
        _ => "HTTP Error",
    }
}

fn safe_endpoint_from_url(url: Option<&str>) -> Option<String> {
    let raw = url?.trim();
    if raw.is_empty() {
        return None;
    }

    let without_scheme = raw.split_once("://").map(|(_, tail)| tail).unwrap_or(raw);
    let without_fragment = without_scheme.split('#').next().unwrap_or(without_scheme);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let without_userinfo = without_query
        .rsplit_once('@')
        .map(|(_, tail)| tail)
        .unwrap_or(without_query);
    let mut host_and_path = without_userinfo.splitn(2, '/');
    let host = host_and_path.next()?.trim();
    if host.is_empty() {
        return None;
    }
    let path = host_and_path.next().unwrap_or_default();
    let route = if path.contains("chat/completions") {
        "/chat/completions"
    } else if path.contains("responses") {
        "/responses"
    } else if path.contains("models") {
        "/models"
    } else {
        ""
    };
    Some(format!("{host}{route}"))
}

pub fn telemetry_api_error_message(error: &ApiError) -> String {
    match error {
        ApiError::Transport(transport) => telemetry_transport_error_message(transport),
        ApiError::Api { status, .. } => format!("api error {}", status.as_u16()),
        ApiError::Stream(err) => err.to_string(),
        ApiError::ContextWindowExceeded => "context window exceeded".to_string(),
        ApiError::QuotaExceeded => "quota exceeded".to_string(),
        ApiError::UsageNotIncluded => "usage not included".to_string(),
        ApiError::Retryable { .. } => "retryable error".to_string(),
        ApiError::RateLimit(_) => "rate limit".to_string(),
        ApiError::InvalidRequest { .. } => "invalid request".to_string(),
        ApiError::CyberPolicy { .. } => "cyber policy".to_string(),
        ApiError::ServerOverloaded => "server overloaded".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseDebugContext;
    use super::extract_response_debug_context;
    use super::telemetry_api_error_message;
    use super::telemetry_transport_error_message;
    use super::user_visible_transport_retry_details;
    use codex_api::ApiError;
    use codex_api::TransportError;
    use http::HeaderMap;
    use http::HeaderValue;
    use http::StatusCode;
    use pretty_assertions::assert_eq;

    #[test]
    fn extract_response_debug_context_decodes_identity_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-oai-request-id", HeaderValue::from_static("req-auth"));
        headers.insert("cf-ray", HeaderValue::from_static("ray-auth"));
        headers.insert(
            "x-openai-authorization-error",
            HeaderValue::from_static("missing_authorization_header"),
        );
        headers.insert(
            "x-error-json",
            HeaderValue::from_static("eyJlcnJvciI6eyJjb2RlIjoidG9rZW5fZXhwaXJlZCJ9fQ=="),
        );

        let context = extract_response_debug_context(&TransportError::Http {
            status: StatusCode::UNAUTHORIZED,
            url: Some("https://chatgpt.com/backend-api/codex/models".to_string()),
            headers: Some(headers),
            body: Some(r#"{"error":{"message":"plain text error"},"status":401}"#.to_string()),
        });

        assert_eq!(
            context,
            ResponseDebugContext {
                request_id: Some("req-auth".to_string()),
                cf_ray: Some("ray-auth".to_string()),
                auth_error: Some("missing_authorization_header".to_string()),
                auth_error_code: Some("token_expired".to_string()),
            }
        );
    }

    #[test]
    fn telemetry_error_messages_omit_http_bodies() {
        let transport = TransportError::Http {
            status: StatusCode::UNAUTHORIZED,
            url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
            headers: None,
            body: Some(r#"{"error":{"message":"secret token leaked"}}"#.to_string()),
        };

        assert_eq!(telemetry_transport_error_message(&transport), "http 401");
        assert_eq!(
            telemetry_api_error_message(&ApiError::Transport(transport)),
            "http 401"
        );
    }

    #[test]
    fn user_visible_retry_details_describe_http_without_body_or_query_secrets() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req-429"));
        headers.insert("cf-ray", HeaderValue::from_static("ray-429"));
        let transport = TransportError::Http {
            status: StatusCode::TOO_MANY_REQUESTS,
            url: Some("https://user:secret@example.com/v1/responses?api_key=secret".to_string()),
            headers: Some(headers),
            body: Some(r#"{"error":{"message":"secret token leaked"}}"#.to_string()),
        };

        let details = user_visible_transport_retry_details(&transport);
        assert!(details.contains("HTTP 429 Too Many Requests, retrying"));
        assert!(details.contains("endpoint: example.com/responses"));
        assert!(details.contains("request id: req-429"));
        assert!(details.contains("cf-ray: ray-429"));
        assert!(!details.contains("secret token leaked"));
        assert!(!details.contains("api_key"));
        assert!(!details.contains("user:secret"));
    }

    #[test]
    fn user_visible_retry_details_do_not_expose_non_http_error_strings() {
        let network = TransportError::Network(
            "error sending request for url (https://user:secret@example.com/v1/responses?api_key=secret&token=leaked)"
                .to_string(),
        );
        let build = TransportError::Build(
            "invalid header value for authorization bearer secret-token".to_string(),
        );

        let network_details = user_visible_transport_retry_details(&network);
        assert_eq!(network_details, "Network error, retrying");
        assert!(!network_details.contains("api_key"));
        assert!(!network_details.contains("token"));
        assert!(!network_details.contains("user:secret"));

        let build_details = user_visible_transport_retry_details(&build);
        assert_eq!(build_details, "Request build error");
        assert!(!build_details.contains("secret-token"));
    }

    #[test]
    fn telemetry_error_messages_preserve_non_http_details() {
        let network = TransportError::Network("dns lookup failed".to_string());
        let build = TransportError::Build("invalid header value".to_string());
        let stream = ApiError::Stream("socket closed".to_string());

        assert_eq!(
            telemetry_transport_error_message(&network),
            "dns lookup failed"
        );
        assert_eq!(
            telemetry_transport_error_message(&build),
            "invalid header value"
        );
        assert_eq!(telemetry_api_error_message(&stream), "socket closed");
    }
}
