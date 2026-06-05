//! Verifies that the agent retries when the SSE stream terminates before
//! delivering a `response.completed` event.

use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::start_streaming_sse_server;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;

fn sse_incomplete() -> String {
    responses::sse(vec![serde_json::json!({
        "type": "response.output_item.done",
    })])
}

fn streaming_sse_model_provider(server_uri: &str, stream_max_retries: u64) -> ModelProviderInfo {
    ModelProviderInfo {
        name: "openai".into(),
        base_url: Some(format!("{server_uri}/v1")),
        // Environment variable that should exist in the test environment.
        // ModelClient will return an error if the environment variable for the
        // provider is not set.
        env_key: Some("PATH".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(0),
        stream_max_retries: Some(stream_max_retries),
        stream_idle_timeout_ms: Some(2000),
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retries_on_early_close() {
    skip_if_no_network!();

    let incomplete_sse = sse_incomplete();
    let completed_sse = responses::sse_completed("resp_ok");

    let (server, _) = start_streaming_sse_server(vec![
        vec![StreamingSseChunk {
            gate: None,
            body: incomplete_sse,
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: completed_sse,
        }],
    ])
    .await;

    // Configure retry behavior explicitly to avoid mutating process-wide
    // environment variables.

    // Exercise retry path: first attempt yields incomplete stream, so allow 1 retry.
    let model_provider = streaming_sse_model_provider(server.uri(), /*stream_max_retries*/ 1);

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
        })
        .build_with_streaming_server(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    // Wait until TurnComplete (should succeed after retry).
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = server.requests().await;
    assert_eq!(
        requests.len(),
        2,
        "expected retry after incomplete SSE stream"
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rotates_prompt_cache_key_after_three_early_closes() {
    skip_if_no_network!();

    let incomplete_sse = sse_incomplete();
    let completed_sse = responses::sse_completed("resp_ok");

    let (server, _) = start_streaming_sse_server(vec![
        vec![StreamingSseChunk {
            gate: None,
            body: incomplete_sse.clone(),
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: incomplete_sse.clone(),
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: incomplete_sse,
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: completed_sse,
        }],
    ])
    .await;

    let model_provider = streaming_sse_model_provider(server.uri(), /*stream_max_retries*/ 3);

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
        })
        .build_with_streaming_server(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = server.requests().await;
    assert_eq!(
        requests.len(),
        4,
        "expected three incomplete attempts followed by a recovered request"
    );
    let request_bodies = requests
        .iter()
        .map(|body| serde_json::from_slice::<serde_json::Value>(body).expect("request body JSON"))
        .collect::<Vec<_>>();
    let cache_keys = request_bodies
        .iter()
        .map(|body| {
            body["prompt_cache_key"]
                .as_str()
                .expect("prompt_cache_key")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(cache_keys[0], cache_keys[1]);
    assert_eq!(cache_keys[1], cache_keys[2]);
    assert!(
        cache_keys[3].starts_with(&cache_keys[0]),
        "recovery cache key should keep the thread id prefix: {cache_keys:?}"
    );
    assert!(
        cache_keys[3].ends_with(":retry-recovery:1"),
        "fourth request should rotate prompt_cache_key after three retryable failures: {cache_keys:?}"
    );

    server.shutdown().await;
}
