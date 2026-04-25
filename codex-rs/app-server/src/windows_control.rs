#![cfg_attr(not(windows), allow(dead_code))]

use crate::config_api::ConfigApi;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::ConfigLayer;
use codex_app_server_protocol::ConfigLayerSource;
use codex_app_server_protocol::ConfigReadParams;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::MergeStrategy;
use codex_core::ProviderRuntimeRefreshAllLoadedReport;
use codex_core::ThreadManager;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;

const APP_SERVERS_DIR_NAME: &str = "app_servers";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct WindowsAppServerControlPlane {
    #[cfg(windows)]
    inner: Option<WindowsAppServerControlPlaneInner>,
}

#[cfg(windows)]
#[derive(Debug)]
struct WindowsAppServerControlPlaneInner {
    shutdown: CancellationToken,
    heartbeat_handle: JoinHandle<()>,
    pipe_handle: JoinHandle<()>,
    registry_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppServerInstanceRegistration {
    instance_id: String,
    pid: u32,
    control_endpoint: String,
    started_at: String,
    heartbeat_at: String,
}

#[derive(Debug, Deserialize)]
struct ControlRequest {
    op: String,
    #[serde(default)]
    source_provider_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PingResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct RefreshAllLoadedThreadsResponse {
    ok: bool,
    total_threads: usize,
    applied_thread_ids: Vec<String>,
    queued_thread_ids: Vec<String>,
    failed_threads: Vec<RefreshFailure>,
}

#[derive(Debug, Serialize)]
struct RefreshFailure {
    thread_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct EffectiveProvidersResponse {
    ok: bool,
    current_model_provider_id: String,
    current_model_provider_writable: bool,
    providers: Vec<EffectiveProviderEntry>,
}

#[derive(Debug, Serialize)]
struct EffectiveProviderEntry {
    provider_id: String,
    display_name: String,
    has_base_url: bool,
    has_experimental_bearer_token: bool,
}

#[derive(Debug, Serialize)]
struct ApplySelectedProviderRuntimeResponse {
    ok: bool,
    outcome: &'static str,
    source_provider_id: Option<String>,
    current_model_provider_id: Option<String>,
    message: Option<String>,
    total_threads: usize,
    applied_thread_ids: Vec<String>,
    queued_thread_ids: Vec<String>,
    failed_threads: Vec<RefreshFailure>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
}

impl WindowsAppServerControlPlane {
    #[cfg(not(windows))]
    pub(crate) async fn start(
        _codex_home: PathBuf,
        _thread_manager: Arc<ThreadManager>,
        _config_api: ConfigApi,
    ) -> io::Result<Self> {
        Ok(Self {})
    }

    #[cfg(windows)]
    pub(crate) async fn start(
        codex_home: PathBuf,
        thread_manager: Arc<ThreadManager>,
        config_api: ConfigApi,
    ) -> io::Result<Self> {
        use tokio::net::windows::named_pipe::NamedPipeServer;

        let instance_id = uuid::Uuid::now_v7().to_string();
        let control_endpoint = format!(r"\\.\pipe\codex-app-server-{instance_id}");
        let registry_dir = codex_home.join(APP_SERVERS_DIR_NAME);
        std::fs::create_dir_all(&registry_dir)?;
        let registry_path = registry_dir.join(format!("{instance_id}.json"));
        let initial_pipe_server: NamedPipeServer = create_named_pipe_server(&control_endpoint)?;

        let started_at = timestamp_now();
        let registration = AppServerInstanceRegistration {
            instance_id,
            pid: std::process::id(),
            control_endpoint: control_endpoint.clone(),
            started_at: started_at.clone(),
            heartbeat_at: started_at,
        };
        write_registration_atomically(&registry_path, &registration)?;

        let shutdown = CancellationToken::new();
        let heartbeat_handle = tokio::spawn(run_heartbeat_loop(
            registry_path.clone(),
            registration,
            shutdown.clone(),
        ));
        let pipe_handle = tokio::spawn(run_named_pipe_server(
            initial_pipe_server,
            control_endpoint,
            thread_manager,
            config_api,
            shutdown.clone(),
        ));

        Ok(Self {
            inner: Some(WindowsAppServerControlPlaneInner {
                shutdown,
                heartbeat_handle,
                pipe_handle,
                registry_path,
            }),
        })
    }

    #[cfg(not(windows))]
    pub(crate) async fn shutdown(self) -> io::Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    pub(crate) async fn shutdown(mut self) -> io::Result<()> {
        let Some(inner) = self.inner.take() else {
            return Ok(());
        };

        inner.shutdown.cancel();
        let _ = inner.heartbeat_handle.await;
        let _ = inner.pipe_handle.await;
        remove_registration_file(&inner.registry_path)
    }
}

#[cfg(windows)]
async fn run_heartbeat_loop(
    registry_path: PathBuf,
    registration: AppServerInstanceRegistration,
    shutdown: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tokio::time::sleep(HEARTBEAT_INTERVAL) => {
                let updated = AppServerInstanceRegistration {
                    heartbeat_at: timestamp_now(),
                    ..registration.clone()
                };
                if let Err(err) = write_registration_atomically(&registry_path, &updated) {
                    warn!("failed to update app-server heartbeat registration: {err}");
                }
            }
        }
    }
}

#[cfg(windows)]
async fn run_named_pipe_server(
    mut server: tokio::net::windows::named_pipe::NamedPipeServer,
    control_endpoint: String,
    thread_manager: Arc<ThreadManager>,
    config_api: ConfigApi,
    shutdown: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            connect_result = server.connect() => {
                if let Err(err) = connect_result {
                    warn!("named pipe control connection failed: {err}");
                    server = match create_named_pipe_server(&control_endpoint) {
                        Ok(server) => server,
                        Err(err) => {
                            warn!("failed to recreate named pipe control server: {err}");
                            break;
                        }
                    };
                    continue;
                }

                let connected = server;
                server = match create_named_pipe_server(&control_endpoint) {
                    Ok(server) => server,
                    Err(err) => {
                        warn!("failed to create next named pipe control server: {err}");
                        break;
                    }
                };

                let thread_manager = Arc::clone(&thread_manager);
                let config_api = config_api.clone();
                tokio::spawn(async move {
                    if let Err(err) =
                        handle_named_pipe_client(connected, thread_manager, config_api).await
                    {
                        warn!("named pipe control request failed: {err}");
                    }
                });
            }
        }
    }
}

#[cfg(windows)]
fn create_named_pipe_server(
    control_endpoint: &str,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    use tokio::net::windows::named_pipe::ServerOptions;

    ServerOptions::new().create(control_endpoint)
}

#[cfg(windows)]
async fn handle_named_pipe_client(
    pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    thread_manager: Arc<ThreadManager>,
    config_api: ConfigApi,
) -> io::Result<()> {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::io::BufReader;

    let (read_half, mut write_half) = tokio::io::split(pipe);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Ok(());
    }

    let request: ControlRequest = serde_json::from_str(line.trim()).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid control request JSON: {err}"),
        )
    })?;

    let response = match request.op.as_str() {
        "ping" => serde_json::to_vec(&PingResponse { ok: true }).map_err(io::Error::other)?,
        "refresh_all_loaded_threads" => {
            let report = thread_manager.refresh_all_loaded_provider_runtime().await;
            serde_json::to_vec(&refresh_response_from_report(report)).map_err(io::Error::other)?
        }
        "list_effective_providers" => {
            let response = match list_effective_providers(&config_api).await {
                Ok(response) => serde_json::to_value(response).map_err(io::Error::other)?,
                Err(err) => serde_json::to_value(ErrorResponse {
                    ok: false,
                    error: err.message,
                })
                .map_err(io::Error::other)?,
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        "apply_provider_runtime_from_effective_provider" => serde_json::to_vec(
            &apply_provider_runtime_from_effective_provider(
                &config_api,
                &thread_manager,
                request.source_provider_id.as_deref(),
            )
            .await,
        )
        .map_err(io::Error::other)?,
        other => serde_json::to_vec(&ErrorResponse {
            ok: false,
            error: format!("unsupported control operation: {other}"),
        })
        .map_err(io::Error::other)?,
    };

    write_half.write_all(&response).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;
    Ok(())
}

#[cfg(windows)]
fn refresh_response_from_report(
    report: ProviderRuntimeRefreshAllLoadedReport,
) -> RefreshAllLoadedThreadsResponse {
    RefreshAllLoadedThreadsResponse {
        ok: true,
        total_threads: report.total_threads,
        applied_thread_ids: report
            .applied_thread_ids
            .into_iter()
            .map(|thread_id| thread_id.to_string())
            .collect(),
        queued_thread_ids: report
            .queued_thread_ids
            .into_iter()
            .map(|thread_id| thread_id.to_string())
            .collect(),
        failed_threads: report
            .failed_threads
            .into_iter()
            .map(|failure| RefreshFailure {
                thread_id: failure.thread_id.to_string(),
                message: failure.message,
            })
            .collect(),
    }
}

#[cfg(windows)]
async fn list_effective_providers(
    config_api: &ConfigApi,
) -> Result<EffectiveProvidersResponse, JSONRPCErrorError> {
    let effective_config = config_api.load_latest_config(/*fallback_cwd*/ None).await?;
    let current_model_provider_id = effective_config.model_provider_id.clone();
    let read_response = config_api
        .read(ConfigReadParams {
            include_layers: true,
            cwd: None,
        })
        .await?;
    let current_model_provider_writable = read_response
        .layers
        .as_ref()
        .and_then(|layers| find_user_layer(layers))
        .map(|user_layer| {
            json_path_exists(
                &user_layer.config,
                &["model_providers", current_model_provider_id.as_str()],
            )
        })
        .unwrap_or(false);

    let mut providers = read_response
        .layers
        .as_ref()
        .and_then(|layers| find_user_layer(layers))
        .and_then(user_model_provider_entries)
        .unwrap_or_default();
    providers.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));

    Ok(EffectiveProvidersResponse {
        ok: true,
        current_model_provider_id,
        current_model_provider_writable,
        providers,
    })
}

#[cfg(windows)]
async fn apply_provider_runtime_from_effective_provider(
    config_api: &ConfigApi,
    thread_manager: &ThreadManager,
    source_provider_id: Option<&str>,
) -> ApplySelectedProviderRuntimeResponse {
    let Some(source_provider_id) = source_provider_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return apply_failure_response(
            "provider_parse_failed",
            None,
            None,
            "missing source_provider_id".to_string(),
        );
    };

    let effective_config = match config_api.load_latest_config(/*fallback_cwd*/ None).await {
        Ok(config) => config,
        Err(err) => {
            return apply_failure_response(
                "provider_parse_failed",
                Some(source_provider_id.to_string()),
                None,
                err.message,
            );
        }
    };
    let current_model_provider_id = effective_config.model_provider_id.clone();

    let read_response = match config_api
        .read(ConfigReadParams {
            include_layers: true,
            cwd: None,
        })
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return apply_failure_response(
                "provider_parse_failed",
                Some(source_provider_id.to_string()),
                Some(current_model_provider_id),
                err.message,
            );
        }
    };
    let Some(user_layer) = read_response
        .layers
        .as_ref()
        .and_then(|layers| find_user_layer(layers))
    else {
        return apply_failure_response(
            "config_write_failed",
            Some(source_provider_id.to_string()),
            Some(current_model_provider_id),
            "user config layer is missing; current model_provider entry is not writable"
                .to_string(),
        );
    };

    let Some(source_provider) = user_layer_model_provider(&user_layer.config, source_provider_id)
    else {
        return apply_failure_response(
            "provider_parse_failed",
            Some(source_provider_id.to_string()),
            Some(current_model_provider_id),
            format!("source provider `{source_provider_id}` was not found in the user config"),
        );
    };
    let source_base_url = provider_field_as_non_empty_str(source_provider, "base_url");
    let source_experimental_bearer_token =
        provider_field_as_non_empty_str(source_provider, "experimental_bearer_token");
    let mut missing_fields = Vec::new();
    if source_base_url.is_none() {
        missing_fields.push("base_url");
    }
    if source_experimental_bearer_token.is_none() {
        missing_fields.push("experimental_bearer_token");
    }
    if !missing_fields.is_empty() {
        return apply_failure_response(
            "provider_field_missing",
            Some(source_provider_id.to_string()),
            Some(current_model_provider_id),
            format!(
                "source provider `{source_provider_id}` is missing required fields: {}",
                missing_fields.join(", ")
            ),
        );
    }
    if !json_path_exists(
        &user_layer.config,
        &["model_providers", current_model_provider_id.as_str()],
    ) {
        return apply_failure_response(
            "config_write_failed",
            Some(source_provider_id.to_string()),
            Some(current_model_provider_id),
            "current model_provider is not backed by a writable user config entry".to_string(),
        );
    }

    let write_result = config_api
        .batch_write(ConfigBatchWriteParams {
            edits: vec![
                ConfigEdit {
                    key_path: format!("model_providers.{current_model_provider_id}.base_url"),
                    value: json!(source_base_url),
                    merge_strategy: MergeStrategy::Replace,
                },
                ConfigEdit {
                    key_path: format!(
                        "model_providers.{current_model_provider_id}.experimental_bearer_token"
                    ),
                    value: json!(source_experimental_bearer_token),
                    merge_strategy: MergeStrategy::Replace,
                },
            ],
            file_path: None,
            expected_version: Some(user_layer.version.clone()),
            reload_user_config: true,
        })
        .await;
    if let Err(err) = write_result {
        return apply_failure_response(
            "config_write_failed",
            Some(source_provider_id.to_string()),
            Some(current_model_provider_id),
            err.message,
        );
    }

    let refresh_response =
        refresh_response_from_report(thread_manager.refresh_all_loaded_provider_runtime().await);
    let outcome = if refresh_response.failed_threads.is_empty() {
        "success"
    } else {
        "partial_failure"
    };

    ApplySelectedProviderRuntimeResponse {
        ok: refresh_response.failed_threads.is_empty(),
        outcome,
        source_provider_id: Some(source_provider_id.to_string()),
        current_model_provider_id: Some(current_model_provider_id),
        message: None,
        total_threads: refresh_response.total_threads,
        applied_thread_ids: refresh_response.applied_thread_ids,
        queued_thread_ids: refresh_response.queued_thread_ids,
        failed_threads: refresh_response.failed_threads,
    }
}

#[cfg(windows)]
fn apply_failure_response(
    outcome: &'static str,
    source_provider_id: Option<String>,
    current_model_provider_id: Option<String>,
    message: String,
) -> ApplySelectedProviderRuntimeResponse {
    ApplySelectedProviderRuntimeResponse {
        ok: false,
        outcome,
        source_provider_id,
        current_model_provider_id,
        message: Some(message),
        total_threads: 0,
        applied_thread_ids: Vec::new(),
        queued_thread_ids: Vec::new(),
        failed_threads: Vec::new(),
    }
}

#[cfg(windows)]
fn find_user_layer(layers: &[ConfigLayer]) -> Option<&ConfigLayer> {
    layers
        .iter()
        .find(|layer| matches!(layer.name, ConfigLayerSource::User { .. }))
}

#[cfg(windows)]
fn json_path_exists(root: &Value, segments: &[&str]) -> bool {
    let mut current = root;
    for segment in segments {
        let Some(object) = current.as_object() else {
            return false;
        };
        let Some(next) = object.get(*segment) else {
            return false;
        };
        current = next;
    }
    true
}

#[cfg(windows)]
fn user_model_provider_entries(user_layer: &ConfigLayer) -> Option<Vec<EffectiveProviderEntry>> {
    let providers = user_layer.config.get("model_providers")?.as_object()?;
    Some(
        providers
            .iter()
            .map(|(provider_id, provider)| EffectiveProviderEntry {
                provider_id: provider_id.clone(),
                display_name: provider_field_as_non_empty_str(provider, "name")
                    .unwrap_or(provider_id)
                    .to_string(),
                has_base_url: provider_field_as_non_empty_str(provider, "base_url").is_some(),
                has_experimental_bearer_token: provider_field_as_non_empty_str(
                    provider,
                    "experimental_bearer_token",
                )
                .is_some(),
            })
            .collect(),
    )
}

#[cfg(windows)]
fn user_layer_model_provider<'a>(root: &'a Value, provider_id: &str) -> Option<&'a Value> {
    root.get("model_providers")?.as_object()?.get(provider_id)
}

#[cfg(windows)]
fn provider_field_as_non_empty_str<'a>(provider: &'a Value, field: &str) -> Option<&'a str> {
    provider
        .get(field)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
}

fn timestamp_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn write_registration_atomically(
    registry_path: &Path,
    registration: &AppServerInstanceRegistration,
) -> io::Result<()> {
    let Some(parent) = registry_path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("registry path has no parent: {}", registry_path.display()),
        ));
    };
    std::fs::create_dir_all(parent)?;

    let temp_path = registry_path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(registration).map_err(io::Error::other)?;
    std::fs::write(&temp_path, contents)?;
    replace_file_atomically(&temp_path, registry_path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING;
    use windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let moved = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::rename(source, destination)
}

fn remove_registration_file(registry_path: &Path) -> io::Result<()> {
    match std::fs::remove_file(registry_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn write_registration_atomically_rewrites_file_contents() {
        let tempdir = tempdir().expect("tempdir");
        let registry_path = tempdir.path().join("instance.json");
        let registration = AppServerInstanceRegistration {
            instance_id: "instance-1".to_string(),
            pid: 123,
            control_endpoint: r"\\.\pipe\codex-app-server-instance-1".to_string(),
            started_at: "2026-04-02T00:00:00Z".to_string(),
            heartbeat_at: "2026-04-02T00:00:05Z".to_string(),
        };

        write_registration_atomically(&registry_path, &registration).expect("write registration");
        let updated_registration = AppServerInstanceRegistration {
            heartbeat_at: "2026-04-02T00:00:10Z".to_string(),
            ..registration
        };
        write_registration_atomically(&registry_path, &updated_registration)
            .expect("rewrite registration");
        let contents = std::fs::read_to_string(&registry_path).expect("read registration");
        let written: AppServerInstanceRegistration =
            serde_json::from_str(&contents).expect("deserialize registration");

        assert_eq!(written.instance_id, "instance-1");
        assert_eq!(written.pid, 123);
        assert_eq!(written.heartbeat_at, "2026-04-02T00:00:10Z");
        assert!(contents.contains("\"instance_id\""));
        assert!(contents.contains("\"control_endpoint\""));
    }

    #[test]
    fn remove_registration_file_ignores_missing_file() {
        let tempdir = tempdir().expect("tempdir");
        let registry_path = tempdir.path().join("missing.json");

        remove_registration_file(&registry_path).expect("missing file should be ignored");
    }
}
