use super::*;
use codex_extension_api::ExtensionData;
use codex_extension_api::TurnItemContributor;
use codex_protocol::items::AgentMessageContent;
use pretty_assertions::assert_eq;
use std::sync::Arc;

struct RewriteAgentMessageContributor;

impl TurnItemContributor for RewriteAgentMessageContributor {
    fn contribute<'a>(
        &'a self,
        _thread_store: &'a ExtensionData,
        _turn_store: &'a ExtensionData,
        item: &'a mut TurnItem,
    ) -> codex_extension_api::ExtensionFuture<'a, Result<(), String>> {
        Box::pin(async move {
            if let TurnItem::AgentMessage(agent_message) = item {
                agent_message.content = vec![AgentMessageContent::Text {
                    text: "plan contributed assistant text".to_string(),
                }];
            }
            Ok(())
        })
    }
}

fn assistant_output_text(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: Some("msg-1".to_string()),
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

#[tokio::test]
async fn plan_mode_uses_contributed_turn_item_for_last_agent_message() {
    let (mut session, turn_context) = crate::session::tests::make_session_and_context().await;
    let mut builder = codex_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(RewriteAgentMessageContributor));
    session.services.extensions = Arc::new(builder.build());
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let mut state = PlanModeStreamState::new(&turn_context.sub_id);
    let mut last_agent_message = None;
    let item = assistant_output_text("original assistant text");

    let handled = handle_assistant_item_done_in_plan_mode(
        &session,
        &turn_context,
        &turn_store,
        &item,
        &mut state,
        /*previously_active_item*/ None,
        &mut last_agent_message,
    )
    .await;

    assert!(handled);
    assert_eq!(
        last_agent_message.as_deref(),
        Some("plan contributed assistant text")
    );
}

#[test]
fn retries_empty_completed_only_after_tools_and_within_cap() {
    assert!(should_retry_empty_completed_after_tools(
        /*had_tools_this_turn*/ true,
        /*needs_follow_up*/ false,
        /*last_agent_message*/ None,
        /*empty_complete_retries*/ 0,
    ));
    assert!(should_retry_empty_completed_after_tools(
        true, false, None, 2
    ));
    assert!(!should_retry_empty_completed_after_tools(
        true, false, None, 3
    ));
    assert!(!should_retry_empty_completed_after_tools(
        /*had_tools_this_turn*/ false,
        false,
        None,
        0,
    ));
    assert!(!should_retry_empty_completed_after_tools(
        true,
        /*needs_follow_up*/ true,
        None,
        0,
    ));
    assert!(!should_retry_empty_completed_after_tools(
        true,
        false,
        Some("closing message"),
        0,
    ));
}

#[test]
fn empty_completed_after_tools_stream_error_is_retryable() {
    let err = empty_completed_after_tools_stream_error();
    assert!(is_empty_completed_after_tools_stream(&err));
    assert!(err.is_retryable());
    assert!(!is_empty_completed_after_tools_stream(&CodexErr::Stream(
        "stream closed before response.completed".into(),
        None,
    )));
}
