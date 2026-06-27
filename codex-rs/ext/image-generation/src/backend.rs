use codex_api::ImageEditRequest;
use codex_api::ImageGenerationRequest;
use codex_api::ImageResponse;
use codex_api::ImagesClient;
use codex_api::RequestTelemetry;
use codex_api::ReqwestTransport;
use codex_api::TransportError;
use codex_login::default_client::build_reqwest_client;
use codex_model_provider::create_model_provider;
use http::HeaderMap;
use http::StatusCode;
use std::sync::Arc;
use std::time::Duration;

use crate::extension::ImageGenerationExtensionRuntime;

#[derive(Clone)]
pub(crate) struct CodexImagesBackend {
    runtime: Arc<ImageGenerationExtensionRuntime>,
}

struct ImageRequestTelemetry {
    runtime: Arc<ImageGenerationExtensionRuntime>,
    generation: u64,
}

impl RequestTelemetry for ImageRequestTelemetry {
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

impl CodexImagesBackend {
    /// Creates a backend that sends image requests through the active model provider.
    pub(crate) fn new(runtime: Arc<ImageGenerationExtensionRuntime>) -> Self {
        Self { runtime }
    }

    /// Resolves the provider and auth required for the current image API request.
    async fn client(&self) -> Result<(ImagesClient<ReqwestTransport>, u64), String> {
        let snapshot = self.runtime.snapshot();
        let provider = create_model_provider(
            snapshot.config.provider.clone(),
            Some(snapshot.auth_manager.clone()),
        );
        let api_provider = provider
            .api_provider()
            .await
            .map_err(|err| err.to_string())?;
        let auth = provider.api_auth().await.map_err(|err| err.to_string())?;
        let client = ImagesClient::new(
            ReqwestTransport::new(build_reqwest_client()),
            api_provider,
            auth,
        )
        .with_telemetry(Some(Arc::new(ImageRequestTelemetry {
            runtime: self.runtime.clone(),
            generation: snapshot.generation,
        })));
        Ok((client, snapshot.generation))
    }

    /// Sends a standalone image generation request through the configured Images client.
    pub(crate) async fn generate(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageResponse, String> {
        loop {
            let (client, generation) = self.client().await?;
            match client.generate(&request, HeaderMap::new()).await {
                Ok(response) => return Ok(response),
                Err(codex_api::ApiError::Transport(TransportError::RetryInterrupted(_)))
                    if !self.runtime.matches_generation(generation) =>
                {
                    continue;
                }
                Err(err) => return Err(err.to_string()),
            }
        }
    }

    /// Sends a standalone image edit request through the configured Images client.
    pub(crate) async fn edit(&self, request: ImageEditRequest) -> Result<ImageResponse, String> {
        loop {
            let (client, generation) = self.client().await?;
            match client.edit(&request, HeaderMap::new()).await {
                Ok(response) => return Ok(response),
                Err(codex_api::ApiError::Transport(TransportError::RetryInterrupted(_)))
                    if !self.runtime.matches_generation(generation) =>
                {
                    continue;
                }
                Err(err) => return Err(err.to_string()),
            }
        }
    }
}
