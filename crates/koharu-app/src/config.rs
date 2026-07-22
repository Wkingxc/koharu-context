use std::collections::BTreeMap;
use std::fs;
use std::io::Write;

use anyhow::{Context, Result};
use atomicwrites::{AtomicFile, OverwriteBehavior};
use camino::Utf8PathBuf;
use koharu_runtime::default_app_data_root;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

use crate::pipeline::{Artifact, Registry};

const CONFIG_FILE: &str = "config.toml";
const PROVIDER_SECRETS_DIR: &str = "secrets";
const PROVIDER_SECRETS_FILE: &str = "provider-api-keys.toml";
const REDACTED: &str = "[REDACTED]";

// ---------------------------------------------------------------------------
// RedactedSecret
// ---------------------------------------------------------------------------

/// A secret value that serializes as `"[REDACTED]"` but deserializes normally.
#[derive(Clone)]
pub struct RedactedSecret(secrecy::SecretString);

impl RedactedSecret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(secrecy::SecretString::from(value.into()))
    }

    pub fn expose(&self) -> &str {
        use secrecy::ExposeSecret;
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED)
    }
}

impl Serialize for RedactedSecret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(REDACTED)
    }
}

impl<'de> Deserialize<'de> for RedactedSecret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct AppConfig {
    pub data: DataConfig,
    pub http: HttpConfig,
    pub pipeline: PipelineConfig,
    pub providers: Vec<ProviderConfig>,
}

/// Engine selection for each pipeline stage.
/// Values are engine IDs (e.g. "pp-doclayout-v3", "comic-text-detector").
/// Empty string means use default.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct PipelineConfig {
    pub detector: String,
    pub font_detector: String,
    pub segmenter: String,
    pub bubble_segmenter: String,
    pub ocr: String,
    pub translator: String,
    pub inpainter: String,
    pub renderer: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            detector: "pp-doclayout-v3".to_string(),
            font_detector: "yuzumarker-font-detection".to_string(),
            segmenter: "comic-text-detector-seg".to_string(),
            bubble_segmenter: "speech-bubble-segmentation".to_string(),
            ocr: "paddle-ocr-vl-1.6".to_string(),
            translator: "llm".to_string(),
            inpainter: "lama-manga".to_string(),
            renderer: "koharu-renderer".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataConfig {
    #[schema(value_type = String)]
    pub path: Utf8PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct HttpConfig {
    pub connect_timeout: u64,
    pub read_timeout: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderConfig {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Populated from the local provider secrets file on `load()`, never written to config.toml.
    /// Serializes as `"[REDACTED]"` in API responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub api_key: Option<RedactedSecret>,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            path: default_app_data_root(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 20,
            read_timeout: 300,
            max_retries: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

pub fn config_path() -> Result<Utf8PathBuf> {
    Ok(default_app_data_root().join(CONFIG_FILE))
}

pub fn load() -> Result<AppConfig> {
    let path = config_path()?;
    let mut config: AppConfig = if path.exists() {
        let content =
            fs::read_to_string(&path).with_context(|| format!("failed to read `{path}`"))?;
        toml::from_str(&content).with_context(|| format!("failed to parse `{path}`"))?
    } else {
        let config = AppConfig::default();
        save(&config)?;
        config
    };

    if validate_pipeline_config(&mut config) {
        save(&config)?;
    }

    // A serialized `[REDACTED]` marker is never a usable secret. Real values
    // live only in the data directory's provider secrets file.
    for provider in &mut config.providers {
        provider.api_key = None;
    }
    let secrets = load_provider_secrets(&config.data.path).unwrap_or_else(|error| {
        tracing::warn!(%error, "failed to load local provider secrets; continuing without API keys");
        ProviderSecrets::default()
    });
    for provider in &mut config.providers {
        if let Some(key) = secrets
            .api_keys
            .get(&provider.id)
            .filter(|key| !key.trim().is_empty())
        {
            provider.api_key = Some(RedactedSecret::new(key));
        }
    }

    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir `{parent}`"))?;
    }
    // `RedactedSecret` serializes as `[REDACTED]`, so the real key is never
    // written to config.toml.
    let content = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(&path, content).with_context(|| format!("failed to write config to `{path}`"))
}

// ---------------------------------------------------------------------------
// Patch application
// ---------------------------------------------------------------------------

/// Apply a `ConfigPatch` in-place. Missing fields leave the existing value
/// alone. Providers are replaced wholesale (the list, not field-by-field).
pub fn apply_patch(config: &mut AppConfig, patch: koharu_core::ConfigPatch) {
    if let Some(data) = patch.data
        && let Some(path) = data.path
    {
        config.data.path = camino::Utf8PathBuf::from(path);
    }
    if let Some(http) = patch.http {
        if let Some(v) = http.connect_timeout {
            config.http.connect_timeout = v;
        }
        if let Some(v) = http.read_timeout {
            config.http.read_timeout = v;
        }
        if let Some(v) = http.max_retries {
            config.http.max_retries = v;
        }
    }
    if let Some(p) = patch.pipeline {
        if let Some(v) = p.detector {
            config.pipeline.detector = v;
        }
        if let Some(v) = p.font_detector {
            config.pipeline.font_detector = v;
        }
        if let Some(v) = p.segmenter {
            config.pipeline.segmenter = v;
        }
        if let Some(v) = p.bubble_segmenter {
            config.pipeline.bubble_segmenter = v;
        }
        if let Some(v) = p.ocr {
            config.pipeline.ocr = v;
        }
        if let Some(v) = p.translator {
            config.pipeline.translator = v;
        }
        if let Some(v) = p.inpainter {
            config.pipeline.inpainter = v;
        }
        if let Some(v) = p.renderer {
            config.pipeline.renderer = v;
        }
    }
    if let Some(providers) = patch.providers {
        let mut new_providers = Vec::with_capacity(providers.len());
        for p in providers {
            let existing = config.providers.iter().find(|e| e.id == p.id);
            let api_key = match p.api_key.as_deref() {
                Some(REDACTED) => existing.and_then(|e| e.api_key.clone()),
                Some("") => None,
                Some(s) => Some(RedactedSecret::new(s)),
                None => existing.and_then(|e| e.api_key.clone()),
            };
            new_providers.push(ProviderConfig {
                id: p.id,
                base_url: p
                    .base_url
                    .or_else(|| existing.and_then(|e| e.base_url.clone())),
                api_key,
            });
        }
        config.providers = new_providers;
    }

    validate_pipeline_config(config);
}

fn validate_pipeline_config(config: &mut AppConfig) -> bool {
    let defaults = PipelineConfig::default();
    let mut changed = false;

    changed |= validate_engine_name(
        "detector",
        &mut config.pipeline.detector,
        &defaults.detector,
        Artifact::TextBoxes,
    );
    changed |= validate_engine_name(
        "font_detector",
        &mut config.pipeline.font_detector,
        &defaults.font_detector,
        Artifact::FontPredictions,
    );
    changed |= validate_engine_name(
        "segmenter",
        &mut config.pipeline.segmenter,
        &defaults.segmenter,
        Artifact::SegmentMask,
    );
    changed |= validate_engine_name(
        "bubble_segmenter",
        &mut config.pipeline.bubble_segmenter,
        &defaults.bubble_segmenter,
        Artifact::BubbleMask,
    );
    changed |= validate_engine_name(
        "ocr",
        &mut config.pipeline.ocr,
        &defaults.ocr,
        Artifact::OcrText,
    );
    changed |= validate_engine_name(
        "translator",
        &mut config.pipeline.translator,
        &defaults.translator,
        Artifact::Translations,
    );
    changed |= validate_engine_name(
        "inpainter",
        &mut config.pipeline.inpainter,
        &defaults.inpainter,
        Artifact::Inpainted,
    );
    changed |= validate_engine_name(
        "renderer",
        &mut config.pipeline.renderer,
        &defaults.renderer,
        Artifact::FinalRender,
    );

    changed
}

fn validate_engine_name(
    field: &'static str,
    configured: &mut String,
    default: &str,
    artifact: Artifact,
) -> bool {
    let trimmed = configured.trim();
    let is_valid = !trimmed.is_empty()
        && Registry::providers(artifact)
            .into_iter()
            .any(|engine| engine.id == trimmed);

    if is_valid {
        if trimmed != configured {
            *configured = trimmed.to_string();
            return true;
        }
        return false;
    }

    if trimmed != default {
        tracing::warn!(
            field,
            configured_engine = configured.as_str(),
            default_engine = default,
            "invalid pipeline engine in config; resetting to default"
        );
    }
    *configured = default.to_string();
    true
}

// ---------------------------------------------------------------------------
// Secret handling
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
struct ProviderSecrets {
    #[serde(default)]
    api_keys: BTreeMap<String, String>,
}

fn provider_secrets_path(data_path: &Utf8PathBuf) -> Utf8PathBuf {
    data_path
        .join(PROVIDER_SECRETS_DIR)
        .join(PROVIDER_SECRETS_FILE)
}

fn load_provider_secrets(data_path: &Utf8PathBuf) -> Result<ProviderSecrets> {
    let path = provider_secrets_path(data_path);
    if !path.exists() {
        return Ok(ProviderSecrets::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read provider secrets `{path}`"))?;
    toml::from_str(&content).with_context(|| format!("failed to parse provider secrets `{path}`"))
}

/// Sync api_key fields to the current data directory's local secrets file.
/// The complete file is rewritten so removed providers and cleared keys do
/// not linger. `[REDACTED]` is never persisted as a real key.
pub fn sync_secrets(config: &AppConfig) -> Result<()> {
    let mut secrets = ProviderSecrets::default();
    for provider in &config.providers {
        if let Some(secret) = &provider.api_key
            && secret.expose() != REDACTED
            && !secret.expose().trim().is_empty()
        {
            secrets
                .api_keys
                .insert(provider.id.clone(), secret.expose().to_string());
        }
    }

    let path = provider_secrets_path(&config.data.path);
    let parent = path
        .parent()
        .context("provider secrets path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create provider secrets dir `{parent}`"))?;
    let content =
        toml::to_string_pretty(&secrets).context("failed to serialize provider secrets")?;
    AtomicFile::new(path.as_std_path(), OverwriteBehavior::AllowOverwrite)
        .write(|file| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(content.as_bytes())
        })
        .map_err(|error| match error {
            atomicwrites::Error::Internal(error) | atomicwrites::Error::User(error) => error,
        })
        .with_context(|| format!("failed to write provider secrets `{path}`"))?;
    restrict_secret_file_permissions(&path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_secret_file_permissions(path: &Utf8PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to secure provider secrets `{path}`"))
}

#[cfg(not(unix))]
fn restrict_secret_file_permissions(_path: &Utf8PathBuf) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use koharu_core::{ConfigPatch, PipelineConfigPatch};

    #[test]
    fn old_config_without_providers_still_loads() {
        let config: AppConfig = toml::from_str(
            r#"
                [data]
                path = "/tmp/test"
            "#,
        )
        .unwrap();

        assert_eq!(config.data.path, "/tmp/test");
        assert_eq!(config.http.connect_timeout, 20);
        assert_eq!(config.http.read_timeout, 300);
        assert_eq!(config.http.max_retries, 3);
        assert!(config.providers.is_empty());
    }

    #[test]
    fn partial_http_config_uses_defaults_for_missing_fields() {
        let config: AppConfig = toml::from_str(
            r#"
                [http]
                connect_timeout = 45
            "#,
        )
        .unwrap();

        assert_eq!(config.http.connect_timeout, 45);
        assert_eq!(config.http.read_timeout, 300);
        assert_eq!(config.http.max_retries, 3);
    }

    #[test]
    fn config_path_uses_appdata_layout() {
        let path = config_path().unwrap();
        assert_eq!(path.file_name(), Some("config.toml"));
        assert!(path.as_str().contains("Koharu"));
    }

    #[test]
    fn provider_secrets_round_trip_outside_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        let data_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let config = AppConfig {
            data: DataConfig {
                path: data_path.clone(),
            },
            providers: vec![ProviderConfig {
                id: "openai".to_string(),
                base_url: None,
                api_key: Some(RedactedSecret::new("sk-local-test")),
            }],
            ..Default::default()
        };

        sync_secrets(&config).unwrap();
        let saved = fs::read_to_string(provider_secrets_path(&data_path)).unwrap();
        assert!(saved.contains("sk-local-test"));
        assert_eq!(
            load_provider_secrets(&data_path)
                .unwrap()
                .api_keys
                .get("openai")
                .map(String::as_str),
            Some("sk-local-test")
        );
        let public_config = toml::to_string_pretty(&config).unwrap();
        assert!(!public_config.contains("sk-local-test"));
        assert!(public_config.contains(REDACTED));
    }

    #[cfg(unix)]
    #[test]
    fn provider_secrets_file_is_user_only_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let data_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let config = AppConfig {
            data: DataConfig {
                path: data_path.clone(),
            },
            providers: vec![ProviderConfig {
                id: "openai".to_string(),
                base_url: None,
                api_key: Some(RedactedSecret::new("secret")),
            }],
            ..Default::default()
        };

        sync_secrets(&config).unwrap();
        let mode = fs::metadata(provider_secrets_path(&data_path))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn invalid_pipeline_engines_reset_to_defaults() {
        let mut config = AppConfig::default();
        config.pipeline.detector = "bad-detector".to_string();
        config.pipeline.renderer = "bad-renderer".to_string();
        config.pipeline.ocr = String::new();

        let changed = validate_pipeline_config(&mut config);

        assert!(changed);
        assert_eq!(config.pipeline.detector, PipelineConfig::default().detector);
        assert_eq!(config.pipeline.renderer, PipelineConfig::default().renderer);
        assert_eq!(config.pipeline.ocr, PipelineConfig::default().ocr);
    }

    #[test]
    fn apply_patch_normalizes_invalid_pipeline_engine_names() {
        let mut config = AppConfig::default();
        apply_patch(
            &mut config,
            ConfigPatch {
                pipeline: Some(PipelineConfigPatch {
                    renderer: Some("not-a-renderer".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert_eq!(config.pipeline.renderer, PipelineConfig::default().renderer);
    }
}
