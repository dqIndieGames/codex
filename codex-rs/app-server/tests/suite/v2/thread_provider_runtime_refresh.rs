use anyhow::Context;
use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::create_final_assistant_message_sse_response;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::create_mock_responses_server_sequence;
use app_test_support::create_mock_responses_server_sequence_after_one_503;
use app_test_support::create_request_user_input_sse_response;
use app_test_support::to_response;
use app_test_support::write_models_cache;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::ConfigWriteResponse;
use codex_app_server_protocol::ErrorNotification;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshAllLoadedParams;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshAllLoadedResponse;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshParams;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshResponse;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshStatus;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput as V2UserInput;
use codex_app_server_protocol::WriteStatus;
use codex_core::test_support::all_model_presets;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn write_provider_refresh_config(
    codex_home: &Path,
    server_uri: &str,
    base_url: &str,
    token: &str,
) -> std::io::Result<()> {
    write_provider_refresh_config_with_agent_config(
        codex_home, server_uri, base_url, token, /*relative_agent_config_file*/ None,
    )
}

fn write_provider_refresh_config_with_agent_config(
    codex_home: &Path,
    server_uri: &str,
    base_url: &str,
    token: &str,
    relative_agent_config_file: Option<&str>,
) -> std::io::Result<()> {
    write_provider_refresh_config_with_agent_config_and_request_retries(
        codex_home,
        server_uri,
        base_url,
        token,
        relative_agent_config_file,
        /*request_max_retries*/ 0,
        /*supports_websockets*/ true,
    )
}

fn write_provider_refresh_config_with_agent_config_and_request_retries(
    codex_home: &Path,
    server_uri: &str,
    base_url: &str,
    token: &str,
    relative_agent_config_file: Option<&str>,
    request_max_retries: u64,
    supports_websockets: bool,
) -> std::io::Result<()> {
    if let Some(relative_agent_config_file) = relative_agent_config_file {
        let role_config_path = codex_home.join(relative_agent_config_file);
        if let Some(parent) = role_config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            &role_config_path,
            "model = \"mock-model\"\ndeveloper_instructions = \"Review carefully\"\n",
        )?;
    }

    let mut config_toml = format!(
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"
chatgpt_base_url = "{server_uri}"

[features]
plugins = false
default_mode_request_user_input = true

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{base_url}"
experimental_bearer_token = "{token}"
wire_api = "responses"
supports_websockets = {supports_websockets}
request_max_retries = {request_max_retries}
stream_max_retries = 0
"#
    );
    if let Some(relative_agent_config_file) = relative_agent_config_file {
        config_toml.push_str(&format!(
            r#"
[agents.reviewer]
description = "Reviewer role"
config_file = "{relative_agent_config_file}"
"#
        ));
    }

    std::fs::write(codex_home.join("config.toml"), config_toml)
}

fn write_provider_refresh_config_with_service_tier_runtime(
    codex_home: &Path,
    server_uri: &str,
    model: &str,
    base_url: &str,
    token: &str,
    force_service_tier_priority: bool,
    fast_mode: bool,
) -> std::io::Result<()> {
    let config_toml = format!(
        r#"
model = "{model}"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"
chatgpt_base_url = "{server_uri}"
force_service_tier_priority = {force_service_tier_priority}
service_tier = "fast"

[features]
plugins = false
default_mode_request_user_input = true
fast_mode = {fast_mode}

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{base_url}"
experimental_bearer_token = "{token}"
wire_api = "responses"
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
"#
    );

    std::fs::write(codex_home.join("config.toml"), config_toml)
}

async fn init_mcp(codex_home: &Path) -> Result<McpProcess> {
    let mut mcp = McpProcess::new(codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;
    Ok(mcp)
}

async fn start_thread(mcp: &mut McpProcess) -> Result<String> {
    start_thread_with_model(mcp, "mock-model").await
}

async fn start_thread_with_model(mcp: &mut McpProcess, model: &str) -> Result<String> {
    let request_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some(model.to_string()),
            ..Default::default()
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response(response)?;
    Ok(thread.id)
}

async fn refresh_thread(
    mcp: &mut McpProcess,
    thread_id: &str,
) -> Result<ThreadProviderRuntimeRefreshResponse> {
    let request_id = mcp
        .send_raw_request(
            "thread/providerRuntime/refresh",
            Some(serde_json::to_value(ThreadProviderRuntimeRefreshParams {
                thread_id: thread_id.to_string(),
            })?),
        )
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    to_response(response)
}

async fn refresh_thread_error(mcp: &mut McpProcess, thread_id: &str) -> Result<JSONRPCError> {
    let request_id = mcp
        .send_raw_request(
            "thread/providerRuntime/refresh",
            Some(serde_json::to_value(ThreadProviderRuntimeRefreshParams {
                thread_id: thread_id.to_string(),
            })?),
        )
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    Ok(error)
}

async fn refresh_all_loaded(
    mcp: &mut McpProcess,
) -> Result<ThreadProviderRuntimeRefreshAllLoadedResponse> {
    let request_id = mcp
        .send_raw_request(
            "thread/providerRuntime/refreshAllLoaded",
            Some(serde_json::to_value(
                ThreadProviderRuntimeRefreshAllLoadedParams::default(),
            )?),
        )
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    to_response(response)
}

async fn run_text_turn(mcp: &mut McpProcess, thread_id: &str, text: &str) -> Result<()> {
    let request_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread_id.to_string(),
            input: vec![V2UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let TurnStartResponse { turn } = to_response(response)?;
    assert!(!turn.id.is_empty());

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;
    Ok(())
}

async fn start_text_turn(mcp: &mut McpProcess, thread_id: &str, text: &str) -> Result<String> {
    let request_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread_id.to_string(),
            input: vec![V2UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let TurnStartResponse { turn } = to_response(response)?;
    assert!(!turn.id.is_empty());
    Ok(turn.id)
}

fn response_request_authorization_headers(requests: &[wiremock::Request]) -> Vec<Option<String>> {
    requests
        .iter()
        .filter(|request| request.url.path().ends_with("/responses"))
        .map(|request| request.header("authorization"))
        .collect()
}

fn response_request_bodies(requests: &[wiremock::Request]) -> Result<Vec<serde_json::Value>> {
    requests
        .iter()
        .filter(|request| request.url.path().ends_with("/responses"))
        .map(|request| request.body_json::<serde_json::Value>().map_err(Into::into))
        .collect()
}

fn service_tier_model_id() -> Result<String> {
    let model = all_model_presets()
        .iter()
        .find(|preset| preset.show_in_picker && !preset.service_tiers.is_empty())
        .context("bundled model catalog should include a picker model with service tiers")?;
    Ok(model.id.clone())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_applied_for_idle_thread() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;

    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        "https://refreshed.example.com/v1",
        "new-token",
    )?;

    let response = refresh_thread(&mut mcp, &thread_id).await?;
    assert_eq!(response.thread_id, thread_id);
    assert_eq!(response.status, ThreadProviderRuntimeRefreshStatus::Applied);
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_applied_for_active_thread() -> Result<()> {
    let responses = vec![create_request_user_input_sse_response("call-1")?];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;

    let turn_request_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![V2UserInput::Text {
                text: "ask for confirmation".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let turn_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_request_id)),
    )
    .await??;
    let TurnStartResponse { turn, .. } = to_response(turn_response)?;

    let request = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_request_message(),
    )
    .await??;
    let ServerRequest::ToolRequestUserInput { params, .. } = request else {
        panic!("expected ToolRequestUserInput request");
    };
    assert_eq!(params.thread_id, thread_id);
    assert_eq!(params.turn_id, turn.id);

    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        "https://refreshed.example.com/v1",
        "new-token",
    )?;

    let response = refresh_thread(&mut mcp, &thread_id).await?;
    assert_eq!(response.thread_id, thread_id);
    assert_eq!(response.status, ThreadProviderRuntimeRefreshStatus::Applied);
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_invalid_request_when_provider_is_missing()
-> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;

    std::fs::write(
        codex_home.path().join("config.toml"),
        r#"model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"
"#,
    )?;

    let error = refresh_thread_error(&mut mcp, &thread_id).await?;
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        error
            .error
            .message
            .contains("failed to refresh provider runtime")
    );
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_invalid_request_for_invalid_user_config()
-> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;

    std::fs::write(codex_home.path().join("config.toml"), "model_provider = [")?;

    let error = refresh_thread_error(&mut mcp, &thread_id).await?;
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        error
            .error
            .message
            .contains("failed to refresh provider runtime")
    );
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_reports_applied_statuses() -> Result<()> {
    let responses = vec![create_request_user_input_sse_response("call-1")?];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let idle_thread_id = start_thread(&mut mcp).await?;
    let active_thread_id = start_thread(&mut mcp).await?;

    let turn_request_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: active_thread_id.clone(),
            input: vec![V2UserInput::Text {
                text: "ask for confirmation".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_request_id)),
    )
    .await??;
    let request = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_request_message(),
    )
    .await??;
    let ServerRequest::ToolRequestUserInput { .. } = request else {
        panic!("expected ToolRequestUserInput request");
    };

    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        "https://refreshed.example.com/v1",
        "new-token",
    )?;

    let response = refresh_all_loaded(&mut mcp).await?;
    assert_eq!(response.total_threads, 2);
    assert!(response.applied_thread_ids.contains(&idle_thread_id));
    assert!(response.applied_thread_ids.contains(&active_thread_id));
    assert!(response.queued_thread_ids.is_empty());
    assert!(response.failed_threads.is_empty());
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_keeps_failed_threads_empty_for_relative_agent_config_file()
-> Result<()> {
    let responses = vec![create_request_user_input_sse_response("call-1")?];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config_with_agent_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
        Some("./agents/reviewer.toml"),
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let idle_thread_id = start_thread(&mut mcp).await?;
    let active_thread_id = start_thread(&mut mcp).await?;

    let turn_request_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: active_thread_id.clone(),
            input: vec![V2UserInput::Text {
                text: "ask for confirmation".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_request_id)),
    )
    .await??;
    let request = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_request_message(),
    )
    .await??;
    let ServerRequest::ToolRequestUserInput { .. } = request else {
        panic!("expected ToolRequestUserInput request");
    };

    write_provider_refresh_config_with_agent_config(
        codex_home.path(),
        &server.uri(),
        "https://refreshed.example.com/v1",
        "new-token",
        Some("./agents/reviewer.toml"),
    )?;

    let response = refresh_all_loaded(&mut mcp).await?;
    assert_eq!(response.total_threads, 2);
    assert!(response.applied_thread_ids.contains(&idle_thread_id));
    assert!(response.applied_thread_ids.contains(&active_thread_id));
    assert!(response.queued_thread_ids.is_empty());
    assert!(response.failed_threads.is_empty());
    Ok(())
}

#[tokio::test]
async fn provider_runtime_refresh_updates_force_priority_and_fast_mode() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    write_provider_refresh_config_with_service_tier_runtime(
        codex_home.path(),
        &server.uri(),
        "mock-model",
        &format!("{}/v1", server.uri()),
        "token",
        /*force_service_tier_priority*/ true,
        /*fast_mode*/ true,
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;
    run_text_turn(&mut mcp, &thread_id, "before refresh").await?;

    let requests_before_refresh = server.received_requests().await.unwrap_or_default();
    let bodies_before_refresh = response_request_bodies(&requests_before_refresh)?;
    let first_body = bodies_before_refresh
        .last()
        .expect("expected request before refresh");
    assert_eq!(first_body["service_tier"].as_str(), Some("priority"));

    write_provider_refresh_config_with_service_tier_runtime(
        codex_home.path(),
        &server.uri(),
        "mock-model",
        &format!("{}/v1", server.uri()),
        "token",
        /*force_service_tier_priority*/ false,
        /*fast_mode*/ false,
    )?;

    let refresh_response = refresh_thread(&mut mcp, &thread_id).await?;
    assert_eq!(refresh_response.thread_id, thread_id);
    assert_eq!(
        refresh_response.status,
        ThreadProviderRuntimeRefreshStatus::Applied
    );

    run_text_turn(&mut mcp, &thread_id, "after refresh").await?;

    let requests_after_refresh = server.received_requests().await.unwrap_or_default();
    let bodies_after_refresh = response_request_bodies(&requests_after_refresh)?;
    assert!(
        bodies_after_refresh.len() > bodies_before_refresh.len(),
        "expected another Responses request after provider runtime refresh"
    );
    let refreshed_body = &bodies_after_refresh[bodies_before_refresh.len()];
    assert!(
        !refreshed_body
            .as_object()
            .expect("request body should be an object")
            .contains_key("service_tier"),
        "request after refresh should not keep stale priority service tier: {refreshed_body}"
    );
    Ok(())
}

#[tokio::test]
async fn provider_runtime_refresh_enables_fast_mode_for_next_turn() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    let model = service_tier_model_id()?;
    write_models_cache(codex_home.path())?;
    write_provider_refresh_config_with_service_tier_runtime(
        codex_home.path(),
        &server.uri(),
        &model,
        &format!("{}/v1", server.uri()),
        "token",
        /*force_service_tier_priority*/ false,
        /*fast_mode*/ false,
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread_with_model(&mut mcp, &model).await?;
    run_text_turn(&mut mcp, &thread_id, "before refresh").await?;

    let requests_before_refresh = server.received_requests().await.unwrap_or_default();
    let bodies_before_refresh = response_request_bodies(&requests_before_refresh)?;
    let first_body = bodies_before_refresh
        .last()
        .expect("expected request before refresh");
    assert!(
        !first_body
            .as_object()
            .expect("request body should be an object")
            .contains_key("service_tier"),
        "request before refresh should not use fast service tier while fast_mode is false: {first_body}"
    );

    write_provider_refresh_config_with_service_tier_runtime(
        codex_home.path(),
        &server.uri(),
        &model,
        &format!("{}/v1", server.uri()),
        "token",
        /*force_service_tier_priority*/ false,
        /*fast_mode*/ true,
    )?;

    let refresh_response = refresh_thread(&mut mcp, &thread_id).await?;
    assert_eq!(refresh_response.thread_id, thread_id);
    assert_eq!(
        refresh_response.status,
        ThreadProviderRuntimeRefreshStatus::Applied
    );

    run_text_turn(&mut mcp, &thread_id, "after refresh").await?;

    let requests_after_refresh = server.received_requests().await.unwrap_or_default();
    let bodies_after_refresh = response_request_bodies(&requests_after_refresh)?;
    assert!(
        bodies_after_refresh.len() > bodies_before_refresh.len(),
        "expected another Responses request after provider runtime refresh"
    );
    let refreshed_body = &bodies_after_refresh[bodies_before_refresh.len()];
    assert_eq!(
        refreshed_body["service_tier"].as_str(),
        Some("priority"),
        "request after refresh should use fast service tier from refreshed fast_mode: {refreshed_body}"
    );
    Ok(())
}

#[tokio::test]
async fn config_batch_write_refreshes_loaded_thread_provider_runtime() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;
    run_text_turn(&mut mcp, &thread_id, "first request").await?;

    let requests_before_write = server.received_requests().await.unwrap_or_default();
    let headers_before_write = response_request_authorization_headers(&requests_before_write);
    assert!(
        !headers_before_write.is_empty(),
        "expected a Responses API request before config write"
    );
    assert!(
        headers_before_write
            .iter()
            .all(|header| header.as_deref() == Some("Bearer old-token")),
        "requests before config write should use old token: {headers_before_write:?}"
    );

    let batch_id = mcp
        .send_config_batch_write_request(ConfigBatchWriteParams {
            file_path: Some(codex_home.path().join("config.toml").display().to_string()),
            edits: vec![ConfigEdit {
                key_path: "model_providers.mock_provider.experimental_bearer_token".to_string(),
                value: json!("new-token"),
                merge_strategy: MergeStrategy::Replace,
            }],
            expected_version: None,
            reload_user_config: false,
        })
        .await?;
    let batch_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(batch_id)),
    )
    .await??;
    let batch_write: ConfigWriteResponse = to_response(batch_response)?;
    assert_eq!(batch_write.status, WriteStatus::Ok);

    run_text_turn(&mut mcp, &thread_id, "second request").await?;

    let requests_after_write = server.received_requests().await.unwrap_or_default();
    let headers_after_write = response_request_authorization_headers(&requests_after_write);
    assert!(
        headers_after_write.len() > headers_before_write.len(),
        "expected another Responses API request after config write"
    );
    assert!(
        headers_after_write[headers_before_write.len()..]
            .iter()
            .all(|header| header.as_deref() == Some("Bearer new-token")),
        "requests after config write should use new token: {headers_after_write:?}"
    );
    Ok(())
}

#[tokio::test]
async fn retryable_503_request_emits_visible_retry_error_notification() -> Result<()> {
    let responses = vec![create_final_assistant_message_sse_response("Done")?];
    let server = create_mock_responses_server_sequence_after_one_503(responses).await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config_with_agent_config_and_request_retries(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "token",
        /*relative_agent_config_file*/ None,
        /*request_max_retries*/ 1,
        /*supports_websockets*/ false,
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;
    let turn_id = start_text_turn(&mut mcp, &thread_id, "trigger retry").await?;

    let retry_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("error"),
    )
    .await??;
    let retry_error: ErrorNotification = serde_json::from_value(
        retry_notification
            .params
            .expect("error notification should include params"),
    )?;
    assert!(retry_error.will_retry);
    assert_eq!(retry_error.thread_id, thread_id);
    assert_eq!(retry_error.turn_id, turn_id);
    assert!(retry_error.error.message.contains("503"));
    assert!(
        retry_error
            .error
            .additional_details
            .as_deref()
            .is_some_and(|details| details.contains("HTTP 503 Service Unavailable, retrying"))
    );
    assert_ne!(
        retry_error.error.additional_details.as_deref(),
        Some("http 503")
    );

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;
    Ok(())
}

#[tokio::test]
async fn provider_runtime_refresh_interrupts_active_request_retry_and_rebuilds_provider()
-> Result<()> {
    let old_server = wiremock::MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(".*/responses$"))
        .respond_with(ResponseTemplate::new(503).set_body_string("old provider unavailable"))
        .mount(&old_server)
        .await;

    let new_server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config_with_agent_config_and_request_retries(
        codex_home.path(),
        &old_server.uri(),
        &format!("{}/v1", old_server.uri()),
        "old-token",
        /*relative_agent_config_file*/ None,
        /*request_max_retries*/ 5,
        /*supports_websockets*/ false,
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let thread_id = start_thread(&mut mcp).await?;
    let turn_id = start_text_turn(&mut mcp, &thread_id, "trigger retry then refresh").await?;

    let retry_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("error"),
    )
    .await??;
    let retry_error: ErrorNotification = serde_json::from_value(
        retry_notification
            .params
            .expect("error notification should include params"),
    )?;
    assert!(retry_error.will_retry);
    assert_eq!(retry_error.thread_id, thread_id);
    assert_eq!(retry_error.turn_id, turn_id);
    assert!(retry_error.error.message.contains("503"));

    write_provider_refresh_config_with_agent_config_and_request_retries(
        codex_home.path(),
        &new_server.uri(),
        &format!("{}/v1", new_server.uri()),
        "new-token",
        /*relative_agent_config_file*/ None,
        /*request_max_retries*/ 5,
        /*supports_websockets*/ false,
    )?;

    let refresh_response = refresh_thread(&mut mcp, &thread_id).await?;
    assert_eq!(refresh_response.thread_id, thread_id);
    assert_eq!(
        refresh_response.status,
        ThreadProviderRuntimeRefreshStatus::Applied
    );

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    let new_requests = new_server.received_requests().await.unwrap_or_default();
    let new_headers = response_request_authorization_headers(&new_requests);
    assert!(
        new_headers
            .iter()
            .any(|header| header.as_deref() == Some("Bearer new-token")),
        "refreshed request should use new provider token: {new_headers:?}"
    );
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_treats_zero_loaded_threads_as_success()
-> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    write_provider_refresh_config(
        codex_home.path(),
        &server.uri(),
        &format!("{}/v1", server.uri()),
        "old-token",
    )?;

    let mut mcp = init_mcp(codex_home.path()).await?;
    let response = refresh_all_loaded(&mut mcp).await?;
    assert_eq!(response.total_threads, 0);
    assert!(response.applied_thread_ids.is_empty());
    assert!(response.queued_thread_ids.is_empty());
    assert!(response.failed_threads.is_empty());
    Ok(())
}
