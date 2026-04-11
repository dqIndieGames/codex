use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::create_mock_responses_server_sequence;
use app_test_support::create_request_user_input_sse_response;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
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
use std::path::Path;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn write_provider_refresh_config(
    codex_home: &Path,
    server_uri: &str,
    base_url: &str,
    token: &str,
) -> std::io::Result<()> {
    write_provider_refresh_config_with_agent_config(
        codex_home,
        server_uri,
        base_url,
        token,
        /*relative_agent_config_file*/ None,
    )
}

fn write_provider_refresh_config_with_agent_config(
    codex_home: &Path,
    server_uri: &str,
    base_url: &str,
    token: &str,
    relative_agent_config_file: Option<&str>,
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

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{base_url}"
experimental_bearer_token = "{token}"
wire_api = "responses"
supports_websockets = true
request_max_retries = 0
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

    std::fs::write(
        codex_home.join("config.toml"),
        config_toml,
    )
}

async fn init_mcp(codex_home: &Path) -> Result<McpProcess> {
    let mut mcp = McpProcess::new(codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;
    Ok(mcp)
}

async fn start_thread(mcp: &mut McpProcess) -> Result<String> {
    let request_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
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
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
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
async fn thread_provider_runtime_refresh_returns_queued_for_active_thread() -> Result<()> {
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

    let request = timeout(DEFAULT_READ_TIMEOUT, mcp.read_stream_until_request_message()).await??;
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
    assert_eq!(response.status, ThreadProviderRuntimeRefreshStatus::Queued);
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_invalid_request_when_provider_is_missing() -> Result<()>
{
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
    assert!(error.error.message.contains("failed to refresh provider runtime"));
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_returns_invalid_request_for_invalid_user_config() -> Result<()>
{
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
    assert!(error.error.message.contains("failed to refresh provider runtime"));
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_reports_mixed_statuses() -> Result<()> {
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
    let request = timeout(DEFAULT_READ_TIMEOUT, mcp.read_stream_until_request_message()).await??;
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
    assert!(response.queued_thread_ids.contains(&active_thread_id));
    assert!(response.failed_threads.is_empty());
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_keeps_failed_threads_empty_for_relative_agent_config_file() -> Result<()>
{
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
    let request = timeout(DEFAULT_READ_TIMEOUT, mcp.read_stream_until_request_message()).await??;
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
    assert!(response.queued_thread_ids.contains(&active_thread_id));
    assert!(response.failed_threads.is_empty());
    Ok(())
}

#[tokio::test]
async fn thread_provider_runtime_refresh_all_loaded_treats_zero_loaded_threads_as_success() -> Result<()>
{
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
