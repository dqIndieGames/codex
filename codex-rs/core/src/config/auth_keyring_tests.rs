use super::*;
use codex_config::ConfigLayerStack;
use codex_config::ConfigRequirements;
use codex_config::ConfigRequirementsToml;
use codex_config::FeatureRequirementsToml;
use codex_config::RequirementSource;
use codex_config::Sourced;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::ForcedChatgptWorkspaceIds;
use codex_config::types::AuthCredentialsStoreMode;
use codex_features::FeaturesToml;
use codex_protocol::config_types::ForcedLoginMethod;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

#[test]
fn auth_account_names_are_safe_path_components() {
    for accepted in ["work", "work-1", "工作"] {
        assert_eq!(validate_auth_account_name(accepted), Ok(()));
    }

    for rejected in [
        "", " work", "work ", ".", "..", "a/b", "a\\b", "a:b", "a*", "CON", "con.txt",
        "COM1", "lpt9.log",
    ] {
        assert!(
            validate_auth_account_name(rejected).is_err(),
            "unsafe account name should be rejected: {rejected:?}"
        );
    }
    assert!(validate_auth_account_name(&"a".repeat(65)).is_err());
}

#[test]
fn auth_account_resolves_only_the_authentication_storage_root() -> std::io::Result<()> {
    let codex_home = Path::new("codex-home");
    assert_eq!(resolve_auth_storage_home(codex_home, None)?, codex_home);
    assert_eq!(
        resolve_auth_storage_home(codex_home, Some("工作"))?,
        codex_home.join("accounts").join("工作")
    );
    Ok(())
}

#[test]
fn account_store_mode_forces_file_and_rejects_conflicting_managed_policy()
-> std::io::Result<()> {
    assert_eq!(
        resolve_account_auth_store_mode(
            true,
            None,
            AuthCredentialsStoreMode::Keyring,
        )?,
        AuthCredentialsStoreMode::File
    );
    assert_eq!(
        resolve_account_auth_store_mode(
            true,
            Some(AuthCredentialsStoreMode::File),
            AuthCredentialsStoreMode::Keyring,
        )?,
        AuthCredentialsStoreMode::File
    );
    assert_eq!(
        resolve_account_auth_store_mode(
            true,
            Some(AuthCredentialsStoreMode::Keyring),
            AuthCredentialsStoreMode::File,
        )
        .expect_err("managed keyring policy must not be bypassed")
        .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        resolve_account_auth_store_mode(
            false,
            Some(AuthCredentialsStoreMode::Keyring),
            AuthCredentialsStoreMode::File,
        )?,
        AuthCredentialsStoreMode::Keyring
    );
    Ok(())
}

#[test]
fn bootstrap_default_account_preserves_configured_storage() -> std::io::Result<()> {
    let codex_home = Path::new("codex-home");
    let default = bootstrap_auth_config(
        codex_home,
        &config_toml_load_result(
            ConfigToml {
                cli_auth_credentials_store: Some(AuthCredentialsStoreMode::Keyring),
                ..Default::default()
            },
            None,
        )?,
    )?;
    assert_eq!(default.codex_home, codex_home);
    assert_eq!(
        default.auth_credentials_store_mode,
        AuthCredentialsStoreMode::Keyring
    );
    Ok(())
}

#[test]
fn resolve_bootstrap_auth_keyring_backend_kind_uses_secret_auth_storage_feature()
-> std::io::Result<()> {
    let config_toml = ConfigToml {
        features: Some(FeaturesToml::from(BTreeMap::from([(
            "secret_auth_storage".to_string(),
            true,
        )]))),
        ..Default::default()
    };
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml,
            /*feature_requirements*/ None,
        )?)?,
        AuthKeyringBackendKind::Secrets
    );

    let config_toml = ConfigToml {
        features: Some(FeaturesToml::from(BTreeMap::from([(
            "secret_auth_storage".to_string(),
            false,
        )]))),
        ..Default::default()
    };
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml.clone(),
            /*feature_requirements*/ None,
        )?)?,
        AuthKeyringBackendKind::Direct
    );

    let requirements = Sourced::new(
        FeatureRequirementsToml {
            entries: BTreeMap::from([("secret_auth_storage".to_string(), true)]),
        },
        RequirementSource::Unknown,
    );
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml,
            Some(requirements),
        )?)?,
        AuthKeyringBackendKind::Secrets
    );

    Ok(())
}

#[test]
fn managed_auth_restrictions_intersect_workspaces_and_fail_closed() {
    let config = ConfigToml {
        forced_login_method: None,
        forced_chatgpt_workspace_id: Some(ForcedChatgptWorkspaceIds::Multiple(vec![
            " denied ".to_string(),
            " allowed ".to_string(),
        ])),
        ..Default::default()
    };
    let mut requirements = ConfigRequirements {
        allowed_login_methods: Some(Sourced::new(
            vec![ForcedLoginMethod::Chatgpt],
            RequirementSource::Unknown,
        )),
        allowed_chatgpt_workspaces: Some(Sourced::new(
            vec!["allowed".to_string()],
            RequirementSource::Unknown,
        )),
        ..Default::default()
    };

    let bootstrap_config = ConfigTomlLoadResult {
        config_toml: config.clone(),
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements.clone(),
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };
    let auth_config = bootstrap_auth_config(Path::new("codex-home"), &bootstrap_config)
        .expect("policy should resolve");
    assert_eq!(auth_config.forced_login_method, None);
    assert!(auth_config.is_login_method_allowed(ForcedLoginMethod::Chatgpt));
    assert!(!auth_config.is_login_method_allowed(ForcedLoginMethod::Api));
    assert_eq!(
        auth_config.forced_chatgpt_workspace_id,
        Some(vec!["denied".to_string(), "allowed".to_string()])
    );
    assert_eq!(
        auth_config.effective_chatgpt_workspaces(),
        Some(vec!["allowed".to_string()])
    );

    requirements.allowed_chatgpt_workspaces =
        Some(Sourced::new(Vec::new(), RequirementSource::Unknown));
    let bootstrap_config = ConfigTomlLoadResult {
        config_toml: config,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };
    assert_eq!(
        bootstrap_auth_config(Path::new("codex-home"), &bootstrap_config)
            .expect_err("ChatGPT-only policy without an allowed workspace must fail")
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn bootstrap_auth_config_applies_managed_store_and_chatgpt_base_url() {
    let configured_store = AuthCredentialsStoreMode::File;
    let configured_url = "https://user.example/backend-api/";
    let managed_store = AuthCredentialsStoreMode::Keyring;
    let managed_url = "https://managed.example/backend-api/";
    let config_toml = ConfigToml {
        cli_auth_credentials_store: Some(configured_store),
        chatgpt_base_url: Some(configured_url.to_string()),
        ..Default::default()
    };
    let requirements = ConfigRequirements {
        cli_auth_credentials_store: Some(Sourced::new(managed_store, RequirementSource::Unknown)),
        chatgpt_base_url: Some(Sourced::new(
            managed_url.to_string(),
            RequirementSource::Unknown,
        )),
        ..Default::default()
    };
    let bootstrap_config = ConfigTomlLoadResult {
        config_toml,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };

    let auth_config = bootstrap_auth_config(Path::new("codex-home"), &bootstrap_config)
        .expect("managed authentication settings should resolve");

    assert_eq!(auth_config.auth_credentials_store_mode, managed_store);
    assert_eq!(auth_config.chatgpt_base_url.as_deref(), Some(managed_url));
}

fn config_toml_load_result(
    config_toml: ConfigToml,
    feature_requirements: Option<Sourced<FeatureRequirementsToml>>,
) -> std::io::Result<ConfigTomlLoadResult> {
    let requirements = ConfigRequirements {
        feature_requirements,
        ..Default::default()
    };
    Ok(ConfigTomlLoadResult {
        config_toml,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )?,
    })
}
