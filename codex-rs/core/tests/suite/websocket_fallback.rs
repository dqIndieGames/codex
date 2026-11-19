use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_model_provider_info::WireApi;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::TempDirExt;
use core_test_support::responses;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::local_requests;
use core_test_support::test_codex::test_codex;
use core_test_support::test_codex::turn_permission_fields;
use pretty_assertions::assert_eq;
use tokio::time::Duration;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::http::Method;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

#[path = "websocket_fallback_close.rs"]
mod close_code;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_fallback_switches_to_http_after_three_upgrade_required_failures() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    Mock::given(method("GET"))
        .and(path_regex(".*/responses$"))
        .respond_with(ResponseTemplate::new(426))
        .mount(&server)
        .await;

    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let mut builder = test_codex().with_config({
        let base_url = format!("{}/v1", server.uri());
        move |config| {
            config.model_provider.base_url = Some(base_url);
            config.model_provider.wire_api = WireApi::Responses;
            config.model_provider.supports_websockets = true;
            // 真值：功能清单 §3.1、§3.4。426 与普通失败相同，第三次后且预算允许时转 HTTP。
            config.model_provider.stream_max_retries = Some(3);
            config.model_provider.request_max_retries = Some(0);
        }
    });
    let test = builder.build(&server).await?;

    test.submit_turn("hello").await?;

    let requests = server.received_requests().await.unwrap_or_default();
    let websocket_attempts = requests
        .iter()
        .filter(|req| req.method == Method::GET && req.url.path().ends_with("/responses"))
        .count();
    let http_attempts = requests
        .iter()
        .filter(|req| req.method == Method::POST && req.url.path().ends_with("/responses"))
        .count();

    // 一次后台预热不计入用户逻辑请求；目标请求连续失败三次后才转 HTTP。
    assert_eq!(websocket_attempts, 4);
    assert_eq!(http_attempts, 1);
    assert_eq!(response_mock.requests().len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_fallback_does_not_add_http_after_retry_budget_is_exhausted() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let mut builder = test_codex().with_config({
        let base_url = format!("{}/v1", server.uri());
        move |config| {
            config.model_provider.base_url = Some(base_url);
            config.model_provider.wire_api = WireApi::Responses;
            config.model_provider.supports_websockets = true;
            config.model_provider.stream_max_retries = Some(2);
            config.model_provider.request_max_retries = Some(0);
        }
    });
    let test = builder.build(&server).await?;

    test.submit_turn("hello").await?;

    let requests = server.received_requests().await.unwrap_or_default();
    let websocket_attempts = requests
        .iter()
        .filter(|req| req.method == Method::GET && req.url.path().ends_with("/responses"))
        .count();
    let http_attempts = requests
        .iter()
        .filter(|req| req.method == Method::POST && req.url.path().ends_with("/responses"))
        .count();

    // 真值：功能清单 §3.1、§3.4。初次尝试加两次重试已耗尽预算，不得补发 HTTP。
    assert_eq!(websocket_attempts, 4);
    assert_eq!(http_attempts, 0);
    assert!(response_mock.requests().is_empty());

    Ok(())
}

#[test_case::test_case(true, Some("invalid_workspace_selected"), 5, 0; "workspace denial on websocket")]
#[test_case::test_case(false, Some("invalid_workspace_selected"), 0, 8; "workspace denial on http")]
#[test_case::test_case(true, Some("codex_workspace_access_denied"), 5, 0; "workspace permission denied on websocket")]
#[test_case::test_case(true, Some("access_program_not_enabled"), 5, 0; "access program denial on websocket")]
#[test_case::test_case(false, Some("access_program_not_enabled"), 0, 8; "access program denial on http")]
#[test_case::test_case(true, None, 5, 0; "bare websocket and http denials")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forbidden_responses_obey_local3_explicit_retry_budget(
    supports_websockets: bool,
    code: Option<&str>,
    expected_upgrades: u64,
    expected_posts: u64,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut denial = ResponseTemplate::new(403);
    if let Some(code) = code {
        denial = denial.set_body_json(serde_json::json!({"detail": {"code": code}}));
    }
    for (verb, count) in [("GET", expected_upgrades), ("POST", expected_posts)] {
        Mock::given(method(verb))
            .and(path_regex(".*/responses$"))
            .respond_with(denial.clone())
            .expect(count)
            .mount(&server)
            .await;
    }

    let test = test_codex()
        .with_config(move |config| {
            config.model_provider.supports_websockets = supports_websockets;
            config.model_provider.stream_max_retries = Some(1);
            config.model_provider.request_max_retries = Some(1);
        })
        .build_with_auto_env(&server)
        .await?;

    // 真值：功能清单 §3.1。WS 每回合初次加一次重试；HTTP 每轮也有一次请求层重试。
    // 两个独立回合共享协议锁，但失败计数各自重新开始；后台预热另计一次 WS。
    test.submit_turn("first").await?;
    test.submit_turn("second").await?;
    server.verify().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_fallback_shows_every_retry_in_release_and_debug() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let mut builder = test_codex().with_config({
        let base_url = format!("{}/v1", server.uri());
        move |config| {
            config.model_provider.base_url = Some(base_url);
            config.model_provider.wire_api = WireApi::Responses;
            config.model_provider.supports_websockets = true;
            config.model_provider.stream_max_retries = Some(3);
            config.model_provider.request_max_retries = Some(0);
        }
    });
    let TestCodex {
        codex,
        session_configured,
        cwd,
        ..
    } = builder.build(&server).await?;
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd.path());

    codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                environments: Some(local_requests(cwd.abs())),
                approval_policy: Some(AskForApproval::Never),
                sandbox_policy: Some(sandbox_policy),
                permission_profile,
                collaboration_mode: Some(CollaborationMode {
                    mode: ModeKind::Default,
                    settings: Settings {
                        model: session_configured.model.clone(),
                        reasoning_effort: None,
                        developer_instructions: None,
                    },
                }),
                ..Default::default()
            }),
        )
        .await?;

    let mut stream_error_messages = Vec::new();
    loop {
        let event = timeout(Duration::from_secs(20), codex.next_event())
            .await
            .expect("timeout waiting for event")
            .expect("event stream ended unexpectedly")
            .msg;
        match event {
            EventMsg::StreamError(e) => stream_error_messages.push(e.message),
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    // 真值：功能清单 §3.7，正式版从第一次重试开始可见；累计序号不因切协议归零。
    for (index, message) in stream_error_messages.iter().enumerate() {
        assert!(message.starts_with(&format!("Reconnecting... {} ", index + 1)));
    }
    assert_eq!(stream_error_messages.len(), 3);
    assert!(stream_error_messages.last().unwrap().contains("HTTP"));
    assert_eq!(response_mock.requests().len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_fallback_is_sticky_across_turns() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
            sse(vec![ev_response_created("resp-2"), ev_completed("resp-2")]),
        ],
    )
    .await;

    let mut builder = test_codex().with_config({
        let base_url = format!("{}/v1", server.uri());
        move |config| {
            config.model_provider.base_url = Some(base_url);
            config.model_provider.wire_api = WireApi::Responses;
            config.model_provider.supports_websockets = true;
            config.model_provider.stream_max_retries = Some(3);
            config.model_provider.request_max_retries = Some(0);
        }
    });
    let test = builder.build(&server).await?;

    test.submit_turn("first").await?;
    test.submit_turn("second").await?;

    let requests = server.received_requests().await.unwrap_or_default();
    let websocket_attempts = requests
        .iter()
        .filter(|req| req.method == Method::GET && req.url.path().ends_with("/responses"))
        .count();
    let http_attempts = requests
        .iter()
        .filter(|req| req.method == Method::POST && req.url.path().ends_with("/responses"))
        .count();

    // WebSocket attempts all happen on the first turn:
    // 1 deferred request prewarm attempt (startup) + 3 stream attempts
    // (initial try + 3 retries with the last attempt over HTTP) before success.
    // Fallback is sticky, so the second turn stays on HTTP and adds no websocket attempts.
    assert_eq!(websocket_attempts, 4);
    assert_eq!(http_attempts, 2);
    assert_eq!(response_mock.requests().len(), 2);

    Ok(())
}
