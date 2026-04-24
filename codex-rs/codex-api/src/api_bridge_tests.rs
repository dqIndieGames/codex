use super::*;
use base64::Engine;
use pretty_assertions::assert_eq;

fn non_usage_limit_429() -> ApiError {
    ApiError::Transport(TransportError::Http {
        status: http::StatusCode::TOO_MANY_REQUESTS,
        url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
        headers: None,
        body: Some(r#"{"error":{"type":"rate_limit_error","message":"try again"}}"#.to_string()),
    })
}

#[test]
fn map_api_error_maps_server_overloaded() {
    let err = map_api_error(ApiError::ServerOverloaded);
    assert!(matches!(err, CodexErr::ServerOverloaded));
}

#[test]
fn map_api_error_maps_server_overloaded_from_503_body() {
    let body = serde_json::json!({
        "error": {
            "code": "server_is_overloaded"
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::SERVICE_UNAVAILABLE,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(body),
    }));

    assert!(matches!(err, CodexErr::ServerOverloaded));
}

#[test]
fn map_api_error_maps_cyber_policy_from_400_body() {
    let body = serde_json::json!({
        "error": {
            "message": "This request has been flagged for potentially high-risk cyber activity.",
            "type": "invalid_request",
            "param": null,
            "code": "cyber_policy"
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::BAD_REQUEST,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(body),
    }));

    let CodexErr::CyberPolicy { message } = err else {
        panic!("expected CodexErr::CyberPolicy, got {err:?}");
    };
    assert_eq!(
        message,
        "This request has been flagged for potentially high-risk cyber activity."
    );
}

#[test]
fn map_api_error_maps_wrapped_websocket_cyber_policy_from_400_body() {
    let body = serde_json::json!({
        "type": "error",
        "status": 400,
        "error": {
            "message": "This websocket request was flagged.",
            "type": "invalid_request",
            "code": "cyber_policy"
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::BAD_REQUEST,
        url: Some("ws://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(body),
    }));

    let CodexErr::CyberPolicy { message } = err else {
        panic!("expected CodexErr::CyberPolicy, got {err:?}");
    };
    assert_eq!(message, "This websocket request was flagged.");
}

#[test]
fn map_api_error_uses_cyber_policy_fallback_for_missing_message() {
    let body = serde_json::json!({
        "error": {
            "code": "cyber_policy"
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::BAD_REQUEST,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(body),
    }));

    let CodexErr::CyberPolicy { message } = err else {
        panic!("expected CodexErr::CyberPolicy, got {err:?}");
    };
    assert_eq!(
        message,
        "This request has been flagged for possible cybersecurity risk."
    );
}

#[test]
fn map_api_error_keeps_unknown_400_errors_generic() {
    let body = serde_json::json!({
        "error": {
            "message": "Some other bad request.",
            "code": "some_other_policy"
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::BAD_REQUEST,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: None,
        body: Some(body.clone()),
    }));

    let CodexErr::InvalidRequest(message) = err else {
        panic!("expected CodexErr::InvalidRequest, got {err:?}");
    };
    assert_eq!(message, body);
}

#[test]
fn map_api_error_maps_usage_limit_limit_name_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACTIVE_LIMIT_HEADER,
        http::HeaderValue::from_static("codex_other"),
    );
    headers.insert(
        "x-codex-other-limit-name",
        http::HeaderValue::from_static("codex_other"),
    );
    let body = serde_json::json!({
        "error": {
            "type": "usage_limit_reached",
            "plan_type": "pro",
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::TOO_MANY_REQUESTS,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: Some(headers),
        body: Some(body),
    }));

    let CodexErr::UsageLimitReached(usage_limit) = err else {
        panic!("expected CodexErr::UsageLimitReached, got {err:?}");
    };
    assert_eq!(
        usage_limit
            .rate_limits
            .as_ref()
            .and_then(|snapshot| snapshot.limit_name.as_deref()),
        Some("codex_other")
    );
}

#[test]
fn map_api_error_does_not_fallback_limit_name_to_limit_id() {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACTIVE_LIMIT_HEADER,
        http::HeaderValue::from_static("codex_other"),
    );
    let body = serde_json::json!({
        "error": {
            "type": "usage_limit_reached",
            "plan_type": "pro",
        }
    })
    .to_string();
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::TOO_MANY_REQUESTS,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: Some(headers),
        body: Some(body),
    }));

    let CodexErr::UsageLimitReached(usage_limit) = err else {
        panic!("expected CodexErr::UsageLimitReached, got {err:?}");
    };
    assert_eq!(
        usage_limit
            .rate_limits
            .as_ref()
            .and_then(|snapshot| snapshot.limit_name.as_deref()),
        None
    );
}

#[test]
fn map_api_error_maps_payment_required_usage_limit_message() {
    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::PAYMENT_REQUIRED,
        url: Some("http://example.com/v1/responses".to_string()),
        headers: None,
        body: Some("Daily spending limit reached for today".to_string()),
    }));

    let CodexErr::UsageLimitReached(usage_limit) = err else {
        panic!("expected CodexErr::UsageLimitReached, got {err:?}");
    };
    assert_eq!(usage_limit.plan_type, None);
}

#[test]
fn map_api_error_extracts_identity_auth_details_from_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(REQUEST_ID_HEADER, http::HeaderValue::from_static("req-401"));
    headers.insert(CF_RAY_HEADER, http::HeaderValue::from_static("ray-401"));
    headers.insert(
        X_OPENAI_AUTHORIZATION_ERROR_HEADER,
        http::HeaderValue::from_static("missing_authorization_header"),
    );
    let x_error_json =
        base64::engine::general_purpose::STANDARD.encode(r#"{"error":{"code":"token_expired"}}"#);
    headers.insert(
        X_ERROR_JSON_HEADER,
        http::HeaderValue::from_str(&x_error_json).expect("valid x-error-json header"),
    );

    let err = map_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::UNAUTHORIZED,
        url: Some("https://chatgpt.com/backend-api/codex/models".to_string()),
        headers: Some(headers),
        body: Some(r#"{"detail":"Unauthorized"}"#.to_string()),
    }));

    let CodexErr::UnexpectedStatus(err) = err else {
        panic!("expected CodexErr::UnexpectedStatus, got {err:?}");
    };
    assert_eq!(err.request_id.as_deref(), Some("req-401"));
    assert_eq!(err.cf_ray.as_deref(), Some("ray-401"));
    assert_eq!(
        err.identity_authorization_error.as_deref(),
        Some("missing_authorization_header")
    );
    assert_eq!(err.identity_error_code.as_deref(), Some("token_expired"));
}

#[test]
fn map_responses_request_api_error_maps_non_usage_429_to_retry_limit() {
    let err = map_responses_request_api_error(non_usage_limit_429());

    let CodexErr::RetryLimit(err) = err else {
        panic!("expected CodexErr::RetryLimit, got {err:?}");
    };
    assert_eq!(err.status, http::StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn map_responses_stream_api_error_maps_non_usage_429_to_unexpected_status() {
    let err = map_responses_stream_api_error(non_usage_limit_429());

    let CodexErr::UnexpectedStatus(err) = err else {
        panic!("expected CodexErr::UnexpectedStatus, got {err:?}");
    };
    assert_eq!(err.status, http::StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn map_responses_stream_api_error_maps_wrapped_429_usage_limit_semantically() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-codex-primary-used-percent",
        http::HeaderValue::from_static("100.0"),
    );
    headers.insert(
        "x-codex-primary-window-minutes",
        http::HeaderValue::from_static("15"),
    );
    let err = map_responses_stream_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::TOO_MANY_REQUESTS,
        url: Some("wss://chatgpt.com/backend-api/codex/responses".to_string()),
        headers: Some(headers),
        body: Some(
            r#"{"error":{"type":"usage_limit_reached","message":"The usage limit has been reached","plan_type":"pro"}}"#
                .to_string(),
        ),
    }));

    let CodexErr::UsageLimitReached(err) = err else {
        panic!("expected CodexErr::UsageLimitReached for turn-level 429, got {err:?}");
    };
    assert_eq!(
        err.rate_limits
            .as_ref()
            .and_then(|snapshot| snapshot.primary.as_ref())
            .map(|window| window.used_percent),
        Some(100.0)
    );
}

#[test]
fn map_responses_stream_api_error_maps_wrapped_400_invalid_request_semantically() {
    let err = map_responses_stream_api_error(ApiError::Transport(TransportError::Http {
        status: http::StatusCode::BAD_REQUEST,
        url: Some("wss://chatgpt.com/backend-api/codex/responses".to_string()),
        headers: None,
        body: Some(
            r#"{"error":{"type":"invalid_request_error","message":"Model does not support image inputs"}}"#
                .to_string(),
        ),
    }));

    let CodexErr::InvalidRequest(message) = err else {
        panic!("expected CodexErr::InvalidRequest for turn-level 400, got {err:?}");
    };
    assert!(message.contains("does not support image inputs"));
}

#[test]
fn map_responses_stream_api_error_keeps_stream_level_server_overloaded_retryable() {
    let err = map_responses_stream_api_error(ApiError::ServerOverloaded);

    let CodexErr::Stream(message, delay) = err else {
        panic!("expected CodexErr::Stream for stream-level overload, got {err:?}");
    };
    assert_eq!(message, CodexErr::ServerOverloaded.to_string());
    assert_eq!(delay, None);
}
