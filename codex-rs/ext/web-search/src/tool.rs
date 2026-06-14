use codex_api::ReqwestTransport;
use codex_api::RequestTelemetry;
use codex_api::SearchClient;
use codex_api::SearchCommands;
use codex_api::SearchQuery;
use codex_api::SearchRequest;
use codex_api::TransportError;
use codex_core::web_search_action_detail;
use codex_extension_api::ExtensionTurnItem;
use codex_extension_api::FunctionCallError;
use codex_extension_api::ResponsesApiTool;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolExecutor;
use codex_extension_api::ToolName;
use codex_extension_api::ToolOutput;
use codex_extension_api::ToolSpec;
use codex_extension_api::parse_tool_input_schema_without_compaction;
use codex_login::default_client::build_reqwest_client;
use codex_model_provider::create_model_provider;
use codex_protocol::items::WebSearchItem;
use codex_protocol::models::WebSearchAction;
use codex_tools::ResponsesApiNamespace;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolExposure;
use codex_tools::default_namespace_description;
use http::HeaderMap;
use http::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::extension::WebSearchExtensionRuntime;
use crate::history::recent_input;
use crate::output::SearchOutput;
use crate::schema::commands_schema;

pub(crate) const WEB_NAMESPACE: &str = "web";
pub(crate) const RUN_TOOL_NAME: &str = "run";
const WEB_RUN_DESCRIPTION: &str = include_str!("../web_run_description.md");

pub(crate) struct WebSearchTool {
    pub(crate) session_id: String,
    pub(crate) runtime: Arc<WebSearchExtensionRuntime>,
}

struct WebSearchRequestTelemetry {
    runtime: Arc<WebSearchExtensionRuntime>,
    generation: u64,
}

impl RequestTelemetry for WebSearchRequestTelemetry {
    fn on_request(
        &self,
        _attempt: u64,
        _status: Option<StatusCode>,
        _error: Option<&TransportError>,
        _duration: Duration,
        _emit_log_trace: bool,
    ) {
    }

    fn can_continue_request_retry(&self) -> bool {
        self.runtime.matches_generation(self.generation)
    }
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for WebSearchTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(WEB_NAMESPACE, RUN_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        // parse schema without compaction that removes field metadata/descriptions to match hosted tool definition
        let parameters = match parse_tool_input_schema_without_compaction(&commands_schema()) {
            Ok(parameters) => parameters,
            Err(err) => panic!("search command schema should parse: {err}"),
        };

        ToolSpec::Namespace(ResponsesApiNamespace {
            name: WEB_NAMESPACE.to_string(),
            description: default_namespace_description(WEB_NAMESPACE),
            tools: vec![ResponsesApiNamespaceTool::Function(ResponsesApiTool {
                name: RUN_TOOL_NAME.to_string(),
                description: WEB_RUN_DESCRIPTION.to_string(),
                strict: false,
                parameters,
                output_schema: None,
                defer_loading: None,
            })],
        })
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let commands = parse_commands(&call)?;
        let command_action = command_action(&commands);
        call.turn_item_emitter
            .emit_started(web_search_item(&call.call_id, WebSearchAction::Other))
            .await;
        let response = loop {
            let snapshot = self.runtime.snapshot();
            let provider = create_model_provider(
                snapshot.config.provider.clone(),
                Some(snapshot.auth_manager.clone()),
            );
            let api_provider = provider
                .api_provider()
                .await
                .map_err(|err| FunctionCallError::Fatal(err.to_string()))?;
            let auth = provider
                .api_auth()
                .await
                .map_err(|err| FunctionCallError::Fatal(err.to_string()))?;
            let client =
                SearchClient::new(ReqwestTransport::new(build_reqwest_client()), api_provider, auth)
                    .with_telemetry(Some(Arc::new(WebSearchRequestTelemetry {
                        runtime: self.runtime.clone(),
                        generation: snapshot.generation,
                    })));
            let request = SearchRequest {
                id: self.session_id.clone(),
                model: call.model.clone(),
                reasoning: None,
                input: recent_input(call.conversation_history.items()),
                commands: Some(commands.clone()),
                settings: Some(snapshot.config.settings.clone()),
                max_output_tokens: Some(
                    u64::try_from(call.truncation_policy.token_budget()).unwrap_or(u64::MAX),
                ),
            };
            match client.search(&request, HeaderMap::new()).await {
                Ok(response) => break response,
                Err(codex_api::ApiError::Transport(TransportError::RetryInterrupted(_)))
                    if !self.runtime.matches_generation(snapshot.generation) =>
                {
                    continue;
                }
                Err(err) => return Err(FunctionCallError::Fatal(err.to_string())),
            }
        };
        call.turn_item_emitter
            .emit_completed(web_search_item(&call.call_id, command_action))
            .await;

        Ok(Box::new(SearchOutput::new(response.output)))
    }
}

fn parse_commands(call: &ToolCall) -> Result<SearchCommands, FunctionCallError> {
    let arguments = call.function_arguments()?;
    if arguments.trim().is_empty() {
        return Ok(SearchCommands::default());
    }

    serde_json::from_str(arguments)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))
}

fn command_action(commands: &SearchCommands) -> WebSearchAction {
    commands
        .search_query
        .as_deref()
        .and_then(query_action)
        .or_else(|| commands.image_query.as_deref().and_then(query_action))
        .or_else(|| {
            commands
                .open
                .as_deref()
                .and_then(|operations| operations.first())
                .and_then(|operation| {
                    literal_url(&operation.ref_id)
                        .map(|url| WebSearchAction::OpenPage { url: Some(url) })
                })
        })
        .or_else(|| {
            commands
                .find
                .as_deref()
                .and_then(|operations| operations.first())
                .map(|operation| WebSearchAction::FindInPage {
                    url: literal_url(&operation.ref_id),
                    pattern: Some(operation.pattern.clone()),
                })
        })
        .unwrap_or(WebSearchAction::Other)
}

fn query_action(queries: &[SearchQuery]) -> Option<WebSearchAction> {
    match queries {
        [] => None,
        [query] => Some(WebSearchAction::Search {
            query: Some(query.q.clone()),
            queries: None,
        }),
        queries => Some(WebSearchAction::Search {
            query: None,
            queries: Some(queries.iter().map(|query| query.q.clone()).collect()),
        }),
    }
}

fn literal_url(ref_id: &str) -> Option<String> {
    Url::parse(ref_id).is_ok().then(|| ref_id.to_string())
}

fn web_search_item(call_id: &str, action: WebSearchAction) -> ExtensionTurnItem {
    ExtensionTurnItem::WebSearch(WebSearchItem {
        id: call_id.to_string(),
        query: web_search_action_detail(&action),
        action,
    })
}

#[cfg(test)]
mod tests {
    use codex_api::SearchCommands;
    use codex_protocol::models::WebSearchAction;
    use pretty_assertions::assert_eq;

    use super::command_action;

    #[test]
    fn command_action_reports_queries_and_navigation_detail() {
        let cases = [
            (
                r#"{"image_query":[{"q":"waterfalls"},{"q":"mountains"}]}"#,
                WebSearchAction::Search {
                    query: None,
                    queries: Some(vec!["waterfalls".to_string(), "mountains".to_string()]),
                },
            ),
            (
                r#"{"open":[{"ref_id":"https://example.com/docs"}]}"#,
                WebSearchAction::OpenPage {
                    url: Some("https://example.com/docs".to_string()),
                },
            ),
            (
                r#"{"find":[{"ref_id":"https://example.com/docs","pattern":"install"}]}"#,
                WebSearchAction::FindInPage {
                    url: Some("https://example.com/docs".to_string()),
                    pattern: Some("install".to_string()),
                },
            ),
            (
                r#"{"find":[{"ref_id":"turn0search0","pattern":"install"}]}"#,
                WebSearchAction::FindInPage {
                    url: None,
                    pattern: Some("install".to_string()),
                },
            ),
            (
                r#"{"open":[{"ref_id":"turn0search0"}]}"#,
                WebSearchAction::Other,
            ),
        ];

        for (arguments, expected) in cases {
            let commands: SearchCommands =
                serde_json::from_str(arguments).expect("valid search command arguments");
            assert_eq!(command_action(&commands), expected);
        }
    }
}
