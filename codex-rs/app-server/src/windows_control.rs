use codex_core::ThreadManager;
use serde::Deserialize;
use serde::Serialize;
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
struct ErrorResponse {
    ok: bool,
    error: String,
}

impl WindowsAppServerControlPlane {
    #[cfg(not(windows))]
    pub(crate) async fn start(
        _codex_home: PathBuf,
        _thread_manager: Arc<ThreadManager>,
    ) -> io::Result<Self> {
        Ok(Self {})
    }

    #[cfg(windows)]
    pub(crate) async fn start(
        codex_home: PathBuf,
        thread_manager: Arc<ThreadManager>,
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
                tokio::spawn(async move {
                    if let Err(err) = handle_named_pipe_client(connected, thread_manager).await {
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
            serde_json::to_vec(&RefreshAllLoadedThreadsResponse {
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
            })
            .map_err(io::Error::other)?
        }
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

    // SAFETY: Both paths are encoded as null-terminated UTF-16 strings and remain valid for the
    // duration of the call.
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
            ..registration.clone()
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
