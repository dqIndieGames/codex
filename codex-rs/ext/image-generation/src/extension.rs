use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use codex_core::config::Config;
use codex_extension_api::ConfigContributor;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolContributor;
use codex_extension_api::ToolExecutor;
use codex_features::Feature;
use codex_login::AuthManager;
use codex_model_provider_info::ModelProviderInfo;

use crate::backend::CodexImagesBackend;
use crate::tool::ImageGenerationTool;

#[derive(Clone)]
struct ImageGenerationExtension {
    auth_manager: Arc<AuthManager>,
}

#[derive(Clone)]
pub(crate) struct ImageGenerationExtensionConfig {
    enabled: bool,
    pub(crate) provider: ModelProviderInfo,
}

pub(crate) struct ImageGenerationExtensionRuntime {
    auth_manager: Arc<AuthManager>,
    state: RwLock<ImageGenerationExtensionConfig>,
    generation: AtomicU64,
}

impl ImageGenerationExtensionRuntime {
    fn new(auth_manager: Arc<AuthManager>, config: ImageGenerationExtensionConfig) -> Self {
        Self {
            auth_manager,
            state: RwLock::new(config),
            generation: AtomicU64::new(0),
        }
    }

    fn update(&self, config: ImageGenerationExtensionConfig) {
        *self
            .state
            .write()
            .expect("image generation runtime lock poisoned") = config;
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn snapshot(&self) -> ImageGenerationExtensionRuntimeSnapshot {
        ImageGenerationExtensionRuntimeSnapshot {
            config: self
                .state
                .read()
                .expect("image generation runtime lock poisoned")
                .clone(),
            generation: self.generation.load(Ordering::Acquire),
            auth_manager: self.auth_manager.clone(),
        }
    }

    pub(crate) fn matches_generation(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }
}

pub(crate) struct ImageGenerationExtensionRuntimeSnapshot {
    pub(crate) config: ImageGenerationExtensionConfig,
    pub(crate) generation: u64,
    pub(crate) auth_manager: Arc<AuthManager>,
}

impl From<&Config> for ImageGenerationExtensionConfig {
    /// Resolves whether standalone image generation should be available for a thread.
    fn from(config: &Config) -> Self {
        Self {
            enabled: config.features.enabled(Feature::ImageGenExt)
                && config.model_provider.is_openai(),
            provider: config.model_provider.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ThreadLifecycleContributor<Config> for ImageGenerationExtension {
    /// Seeds image-generation availability when a thread begins.
    async fn on_thread_start(&self, input: ThreadStartInput<'_, Config>) {
        input.thread_store.insert(ImageGenerationExtensionRuntime::new(
            self.auth_manager.clone(),
            ImageGenerationExtensionConfig::from(input.config),
        ));
    }
}

impl ConfigContributor<Config> for ImageGenerationExtension {
    /// Refreshes image-generation availability after thread configuration changes.
    fn on_config_changed(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
        _previous_config: &Config,
        new_config: &Config,
    ) {
        thread_store
            .get_or_init(|| {
                ImageGenerationExtensionRuntime::new(
                    self.auth_manager.clone(),
                    ImageGenerationExtensionConfig::from(new_config),
                )
            })
            .update(ImageGenerationExtensionConfig::from(new_config));
    }
}

impl ToolContributor for ImageGenerationExtension {
    /// Creates the image-generation tool exposed by this installed extension.
    fn tools(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        let Some(runtime) = thread_store.get::<ImageGenerationExtensionRuntime>() else {
            return Vec::new();
        };
        let snapshot = runtime.snapshot();
        let config = snapshot.config;
        if !config.enabled || !self.auth_manager.current_auth_uses_codex_backend() {
            return Vec::new();
        }

        vec![Arc::new(ImageGenerationTool::new(CodexImagesBackend::new(
            runtime,
        )))]
    }
}

/// Installs the feature-gated standalone image-generation extension contributors.
pub fn install(registry: &mut ExtensionRegistryBuilder<Config>, auth_manager: Arc<AuthManager>) {
    let extension = Arc::new(ImageGenerationExtension { auth_manager });
    registry.thread_lifecycle_contributor(extension.clone());
    registry.config_contributor(extension.clone());
    registry.tool_contributor(extension);
}
