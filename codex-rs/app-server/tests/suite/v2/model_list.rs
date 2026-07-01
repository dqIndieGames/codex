use std::time::Duration;

use anyhow::Error;
use anyhow::Result;
use app_test_support::ChatGptAuthFixture;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use app_test_support::write_chatgpt_auth;
use app_test_support::write_models_cache;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::Model;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::ModelServiceTier;
use codex_app_server_protocol::ModelUpgradeInfo;
use codex_app_server_protocol::ReasoningEffortOption;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadProviderRuntimeRefreshAllLoadedParams;
use codex_config::types::AuthCredentialsStoreMode;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ModelsResponse;
use core_test_support::responses::mount_models_once;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

fn remote_model(slug: &str, display_name: &str) -> Result<ModelInfo> {
    Ok(serde_json::from_value(json!({
        "slug": slug,
        "display_name": display_name,
        "description": format!("Remote model {slug}"),
        "default_reasoning_level": "max",
        "supported_reasoning_levels": [
            {"effort": "max", "description": "Maximum"}
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "minimal_client_version": [0, 1, 0],
        "supported_in_api": true,
        "priority": 0,
        "upgrade": null,
        "base_instructions": "base instructions",
        "supports_reasoning_summaries": false,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10_000},
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": 272_000,
        "max_context_window": 272_000,
        "experimental_supported_tools": [],
    }))?)
}

fn model_from_preset(preset: &ModelPreset) -> Model {
    Model {
        id: preset.id.clone(),
        model: preset.model.clone(),
        upgrade: preset.upgrade.as_ref().map(|upgrade| upgrade.id.clone()),
        upgrade_info: preset.upgrade.as_ref().map(|upgrade| ModelUpgradeInfo {
            model: upgrade.id.clone(),
            upgrade_copy: upgrade.upgrade_copy.clone(),
            model_link: upgrade.model_link.clone(),
            migration_markdown: upgrade.migration_markdown.clone(),
        }),
        availability_nux: preset.availability_nux.clone().map(Into::into),
        display_name: preset.display_name.clone(),
        description: preset.description.clone(),
        hidden: !preset.show_in_picker,
        supported_reasoning_efforts: preset
            .supported_reasoning_efforts
            .iter()
            .map(|preset| ReasoningEffortOption {
                reasoning_effort: preset.effort.clone(),
                description: preset.description.clone(),
            })
            .collect(),
        default_reasoning_effort: preset.default_reasoning_effort.clone(),
        input_modalities: preset.input_modalities.clone(),
        // `write_models_cache()` round-trips through a simplified ModelInfo fixture that does not
        // preserve personality placeholders in base instructions, so app-server list results from
        // cache report `supports_personality = false`.
        // todo(sayan): fix, maybe make roundtrip use ModelInfo only
        supports_personality: false,
        additional_speed_tiers: preset.additional_speed_tiers.clone(),
        service_tiers: preset
            .service_tiers
            .iter()
            .map(|service_tier| ModelServiceTier {
                id: service_tier.id.clone(),
                name: service_tier.name.clone(),
                description: service_tier.description.clone(),
            })
            .collect(),
        default_service_tier: preset.default_service_tier.clone(),
        is_default: preset.is_default,
    }
}

fn expected_visible_models() -> Vec<Model> {
    // Filter by supported_in_api to support testing with both ChatGPT and non-ChatGPT auth modes.
    let mut presets = ModelPreset::filter_by_auth(
        codex_core::test_support::all_model_presets().clone(),
        /*chatgpt_mode*/ false,
    );

    // Mirror `ModelsManager::build_available_models()` default selection after auth filtering.
    ModelPreset::mark_default_by_picker_visibility(&mut presets);

    presets
        .iter()
        .filter(|preset| preset.show_in_picker)
        .map(model_from_preset)
        .collect()
}

#[tokio::test]
async fn list_models_returns_all_models_with_large_limit() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    let expected_models = expected_visible_models();

    assert_eq!(items, expected_models);
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn list_models_includes_hidden_models() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: Some(true),
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;

    assert!(items.iter().any(|item| item.hidden));
    assert!(next_cursor.is_none());
    Ok(())
}

#[tokio::test]
async fn list_models_uses_chatgpt_remote_catalog_as_source_of_truth() -> Result<()> {
    let server = MockServer::start().await;
    let mut remote_model = remote_model("chatgpt-remote-only", "ChatGPT Remote Only")?;
    remote_model.supported_reasoning_levels = serde_json::from_value(json!([
        {"effort": "max", "description": "Maximum"},
        {"effort": "low", "description": "Low"},
        {"effort": "focused", "description": "Focused"}
    ]))?;
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model.clone()],
        },
    )
    .await;

    let codex_home = TempDir::new()?;
    let server_uri = server.uri();
    std::fs::write(
        codex_home.path().join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
openai_base_url = "{server_uri}/v1"
"#
        ),
    )?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("chatgpt-access-token").plan_type("pro"),
        AuthCredentialsStoreMode::File,
    )?;

    let mut mcp =
        TestAppServer::new_with_env(codex_home.path(), &[("OPENAI_API_KEY", None)]).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
        })
        .await?;

    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let ModelListResponse {
        data: items,
        next_cursor,
    } = to_response::<ModelListResponse>(response)?;
    let mut expected_presets: Vec<ModelPreset> = vec![remote_model.into()];
    ModelPreset::mark_default_by_picker_visibility(&mut expected_presets);
    let mut expected_items = expected_presets
        .iter()
        .map(model_from_preset)
        .collect::<Vec<_>>();
    expected_items[0].supported_reasoning_efforts = vec![
        ReasoningEffortOption {
            reasoning_effort: "max".parse().map_err(Error::msg)?,
            description: "Maximum".to_string(),
        },
        ReasoningEffortOption {
            reasoning_effort: "low".parse().map_err(Error::msg)?,
            description: "Low".to_string(),
        },
        ReasoningEffortOption {
            reasoning_effort: "focused".parse().map_err(Error::msg)?,
            description: "Focused".to_string(),
        },
    ];

    assert_eq!(items, expected_items);
    assert!(next_cursor.is_none());
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    Ok(())
}

#[tokio::test]
async fn list_models_follows_provider_runtime_refresh_to_provider_token_endpoint() -> Result<()> {
    let old_server = MockServer::start().await;
    let new_server = MockServer::start().await;
    let old_model = remote_model("old-provider-model", "Old Provider Model")?;
    let new_model = remote_model("new-provider-model", "New Provider Model")?;
    Mock::given(method("GET"))
        .and(path_regex(".*/models$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(ModelsResponse {
                    models: vec![old_model.clone()],
                }),
        )
        .mount(&old_server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(".*/models$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(ModelsResponse {
                    models: vec![new_model.clone()],
                }),
        )
        .mount(&new_server)
        .await;

    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{old_base_url}"
experimental_bearer_token = "old-provider-token"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#,
            old_base_url = old_server.uri(),
        ),
    )?;

    let mut mcp =
        TestAppServer::new_with_env(codex_home.path(), &[("OPENAI_API_KEY", None)]).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let first: ModelListResponse = to_response(response)?;
    assert!(first.data.iter().any(|model| model.model == old_model.slug));

    std::fs::write(
        codex_home.path().join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{new_base_url}"
experimental_bearer_token = "new-provider-token"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#,
            new_base_url = new_server.uri(),
        ),
    )?;

    let refresh_id = mcp
        .send_raw_request(
            "thread/providerRuntime/refreshAllLoaded",
            Some(serde_json::to_value(
                ThreadProviderRuntimeRefreshAllLoadedParams::default(),
            )?),
        )
        .await?;
    let _: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(refresh_id)),
    )
    .await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: Some(100),
            cursor: None,
            include_hidden: None,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let second: ModelListResponse = to_response(response)?;
    assert!(second.data.iter().any(|model| model.model == new_model.slug));
    assert!(!second.data.iter().any(|model| model.model == old_model.slug));
    Ok(())
}

#[tokio::test]
async fn list_models_pagination_works() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let expected_models = expected_visible_models();
    let mut cursor = None;
    let mut items = Vec::new();

    for _ in 0..expected_models.len() {
        let request_id = mcp
            .send_list_models_request(ModelListParams {
                limit: Some(1),
                cursor: cursor.clone(),
                include_hidden: None,
            })
            .await?;

        let response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
        )
        .await??;

        let ModelListResponse {
            data: page_items,
            next_cursor,
        } = to_response::<ModelListResponse>(response)?;

        assert_eq!(page_items.len(), 1);
        items.extend(page_items);

        if let Some(next_cursor) = next_cursor {
            cursor = Some(next_cursor);
        } else {
            assert_eq!(items, expected_models);
            return Ok(());
        }
    }

    panic!(
        "model pagination did not terminate after {} pages",
        expected_models.len()
    );
}

#[tokio::test]
async fn list_models_rejects_invalid_cursor() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path())?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_list_models_request(ModelListParams {
            limit: None,
            cursor: Some("invalid".to_string()),
            include_hidden: None,
        })
        .await?;

    let error: JSONRPCError = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.id, RequestId::Integer(request_id));
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(error.error.message, "invalid cursor: invalid");
    Ok(())
}
