//! Empty `response.completed` after tools is treated as a retryable stream
//! disconnect. Completing with no assistant message and no prior tools must
//! still succeed so existing empty-complete fixtures keep working.

#![allow(clippy::expect_used)]

use std::time::Duration;

use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use codex_protocol::turn_input::TurnInputRequest;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use tokio::time::timeout;

const EMPTY_COMPLETED_MARKER: &str = "empty completed after tools";
const CLOSING_MESSAGE: &str = "closing after tools";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retries_empty_completed_after_tools_then_closes_with_assistant_text() {
    skip_if_no_network!();

    let server = core_test_support::responses::start_mock_server().await;
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-tools"),
                ev_function_call("call-empty-complete", "nonexistent", "{}"),
                ev_completed("resp-tools"),
            ]),
            sse(vec![
                ev_response_created("resp-empty"),
                ev_completed("resp-empty"),
            ]),
            sse(vec![
                ev_response_created("resp-close"),
                ev_assistant_message("msg-close", CLOSING_MESSAGE),
                ev_completed("resp-close"),
            ]),
        ],
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .expect("test codex");

    codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "run a tool then stop empty".into(),
            text_elements: Vec::new(),
        }]))
        .await
        .expect("submit turn");

    let mut saw_empty_complete_retry = false;
    let last_agent_message = loop {
        let event = timeout(Duration::from_secs(30), codex.next_event())
            .await
            .expect("timeout waiting for event")
            .expect("event stream ended");
        match event.msg {
            EventMsg::StreamError(stream_error) => {
                let details = stream_error.additional_details.unwrap_or_default();
                if details.contains(EMPTY_COMPLETED_MARKER) {
                    saw_empty_complete_retry = true;
                }
            }
            EventMsg::TurnComplete(complete) => break complete.last_agent_message,
            EventMsg::Error(error) => panic!("unexpected turn error: {error:?}"),
            _ => {}
        }
    };

    assert!(
        saw_empty_complete_retry,
        "expected a StreamError for empty completed after tools"
    );
    assert_eq!(last_agent_message.as_deref(), Some(CLOSING_MESSAGE));
    assert_eq!(
        mock.requests().len(),
        3,
        "tool call, empty complete retry, then closing assistant message"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_completed_without_tools_still_succeeds() {
    skip_if_no_network!();

    let server = core_test_support::responses::start_mock_server().await;
    let mock = mount_sse_sequence(
        &server,
        vec![sse(vec![
            ev_response_created("resp-empty"),
            ev_completed("resp-empty"),
        ])],
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .expect("test codex");

    codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "hello".into(),
            text_elements: Vec::new(),
        }]))
        .await
        .expect("submit turn");

    let mut saw_empty_complete_retry = false;
    let last_agent_message = loop {
        let event = timeout(Duration::from_secs(30), codex.next_event())
            .await
            .expect("timeout waiting for event")
            .expect("event stream ended");
        match event.msg {
            EventMsg::StreamError(stream_error) => {
                let details = stream_error.additional_details.unwrap_or_default();
                if details.contains(EMPTY_COMPLETED_MARKER) {
                    saw_empty_complete_retry = true;
                }
            }
            EventMsg::TurnComplete(complete) => break complete.last_agent_message,
            EventMsg::Error(error) => panic!("unexpected turn error: {error:?}"),
            _ => {}
        }
    };

    assert!(
        !saw_empty_complete_retry,
        "empty completed without tools must not be retried as a stream error"
    );
    assert_eq!(last_agent_message, None);
    assert_eq!(mock.requests().len(), 1);
}
