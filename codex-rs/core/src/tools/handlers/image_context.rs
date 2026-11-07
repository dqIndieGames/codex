use crate::function_tool::FunctionCallError;
use crate::tools::context::{FunctionToolOutput, ToolInvocation, ToolPayload, boxed_tool_output};
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::{CoreToolRuntime, ToolExecutor};
use codex_tools::{JsonSchema, ResponsesApiTool, ToolName, ToolSpec};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

pub struct ImageContextHandler;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    action: String,
    #[serde(default)]
    image_ids: Vec<String>,
    #[serde(default)]
    analysis_complete: bool,
    #[serde(default = "details_required_default")]
    details_required: bool,
    minimum_long_edge: Option<u32>,
}

fn details_required_default() -> bool { true }

impl ToolExecutor<ToolInvocation> for ImageContextHandler {
    fn tool_name(&self) -> ToolName { ToolName::plain("image_context") }
    fn supports_parallel_tool_calls(&self) -> bool { false }
    fn spec(&self) -> ToolSpec {
        let properties = BTreeMap::from([
            ("action".to_owned(), JsonSchema::string_enum(vec![json!("list"), json!("protect"), json!("release")], None)),
            ("image_ids".to_owned(), JsonSchema::array(JsonSchema::string(None), None)),
            ("analysis_complete".to_owned(), JsonSchema::boolean(None)),
            ("details_required".to_owned(), JsonSchema::boolean(None)),
            ("minimum_long_edge".to_owned(), JsonSchema::integer(Some("Minimum readable long edge in pixels; positive integer.".to_owned()))),
        ]);
        ToolSpec::Function(ResponsesApiTool {
            name: "image_context".to_owned(),
            description: "Manage image dependencies for request-size recovery. All new image batches are protected. List returns chronological image IDs and batch IDs. Release an image only after you have actually analyzed it AND its visual details are no longer required for the current task; set analysis_complete=true and details_required=false. Never release merely because a request succeeded, another image arrived, or you want to make room. Keep comparison/reference images protected. Protect can specify a minimum readable long edge in pixels (default 1280, capped by original size). Released images may progressively become thumbnails or placeholders after repeated failures; the original remains recoverable. This tool does not itself compress or delete an image.".to_owned(),
            strict: false, defer_loading: None,
            parameters: JsonSchema::object(properties, Some(vec!["action".to_owned()]), Some(false.into())),
            output_schema: None,
        })
    }
    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where ToolInvocation: 'a {
        Box::pin(async move {
            let ToolPayload::Function { arguments } = invocation.payload else {
                return Err(FunctionCallError::RespondToModel("image_context expects function arguments".to_owned()));
            };
            let args: Args = parse_arguments(&arguments)?;
            if !matches!(args.action.as_str(), "list" | "protect" | "release")
                || (args.action != "list" && args.image_ids.is_empty())
                || (args.action == "release" && (!args.analysis_complete || args.details_required))
                || args.minimum_long_edge == Some(0) {
                return Err(FunctionCallError::RespondToModel("Use list, protect, or release. Release requires explicit image_ids, analysis_complete=true and details_required=false.".to_owned()));
            }
            let result = invocation.session.update_image_dependencies(
                &args.action, &args.image_ids, args.minimum_long_edge,
            ).await.map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
            Ok(boxed_tool_output(FunctionToolOutput::from_text(result.to_string(), Some(true))))
        })
    }
}

impl CoreToolRuntime for ImageContextHandler {}
