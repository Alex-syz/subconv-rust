//! Application-level configuration loaded from `config.yaml`.
//!
//! Environment variables override file values when set. If `config.yaml` is
//! absent the struct is populated entirely from defaults.

use std::path::Path;

use serde::Deserialize;

use crate::SubconvError;

// ---------------------------------------------------------------------------
// Default value helpers (serde `default = "…"`)
// ---------------------------------------------------------------------------

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8080
}
fn default_template() -> String {
    "meta-rules".into()
}
fn default_true() -> bool {
    true
}
fn default_cache_ttl() -> u64 {
    7200
}
fn default_cache_dir() -> String {
    "./cache".into()
}
fn default_cache_max_size() -> u64 {
    50
}
fn default_sub_cache_ttl() -> u64 {
    300
}
fn default_sub_cache_lock_timeout() -> u64 {
    3
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Top-level application configuration.
///
/// Field names use SCREAMING_SNAKE_CASE in YAML so that the config file
/// mirrors the original Python project's convention.  `#[serde(rename)]`
/// maps them to idiomatic Rust snake_case fields.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(rename = "DEFAULT_TEMPLATE", default = "default_template")]
    pub default_template: String,

    #[serde(rename = "DISALLOW_ROBOTS", default = "default_true")]
    pub disallow_robots: bool,

    /// Whitelist of remote templates the server is allowed to fetch.
    ///
    /// See `config.yaml.example` for detailed instructions on adding entries.
    #[serde(rename = "REMOTE_TEMPLATES", default)]
    pub remote_templates: Vec<RemoteTemplate>,

    #[serde(rename = "CACHE_TTL", default = "default_cache_ttl")]
    pub cache_ttl: u64,

    #[serde(rename = "CACHE_DIR", default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde(rename = "CACHE_MAX_SIZE_MB", default = "default_cache_max_size")]
    pub cache_max_size_mb: u64,

    #[serde(rename = "SUB_CACHE_TTL", default = "default_sub_cache_ttl")]
    pub sub_cache_ttl: u64,

    #[serde(rename = "SUB_CACHE_LOCK_TIMEOUT", default = "default_sub_cache_lock_timeout")]
    pub sub_cache_lock_timeout: u64,
}

/// A single remote template entry in the whitelist.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteTemplate {
    /// Short name used in URLs and the template dropdown.
    pub name: String,
    /// Raw download URL for the template YAML.
    pub url: String,
    /// Optional SHA-256 hex digest for integrity verification.
    /// When `None` the template is fetched without verification.
    #[serde(default)]
    pub sha256: Option<String>,
}

// ---------------------------------------------------------------------------
// Loading logic
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Load configuration from `config.yaml`.
    ///
    /// If the file does not exist, returns a fully-defaulted config so the
    /// service can start without any configuration file at all.
    pub fn load() -> Result<Self, SubconvError> {
        let path = Path::new("config.yaml");
        if !path.exists() {
            tracing::info!("config.yaml not found, using defaults");
            // Build a minimal YAML string so serde fills in defaults.
            let config: Self = serde_yaml::from_str("")?;
            return Ok(config);
        }

        let raw = std::fs::read_to_string(path)
            .map_err(|e| SubconvError::Config(format!("failed to read config.yaml: {e}")))?;
        let mut config: Self = serde_yaml::from_str(&raw)
            .map_err(|e| SubconvError::Config(format!("failed to parse config.yaml: {e}")))?;

        // Environment variable overrides (SUBCONV_ prefix).
        Self::apply_env_overrides(&mut config)?;

        Ok(config)
    }

    /// List all available template names: local YAML files + remote entries.
    pub fn available_templates(&self) -> Vec<String> {
        let mut names = Vec::new();

        // Scan local template directory.
        let template_dir = Path::new("template");
        if template_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(template_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().is_some_and(|ext| ext == "yaml") && p.is_file() {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            names.push(stem.to_string());
                        }
                    }
                }
            }
        }

        // Append remote template names.
        names.extend(self.remote_templates.iter().map(|rt| rt.name.clone()));

        names.sort();
        names.dedup(); // in case a remote name collides with a local one
        names
    }

    /// Return the configured default template name.
    pub fn default_template_name(&self) -> &str {
        &self.default_template
    }

    /// Look up a remote template by name.
    pub fn remote_template(&self, name: &str) -> Option<&RemoteTemplate> {
        self.remote_templates.iter().find(|rt| rt.name == name)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Apply `SUBCONV_*` environment variable overrides.
    fn apply_env_overrides(config: &mut Self) -> Result<(), SubconvError> {
        if let Ok(v) = std::env::var("SUBCONV_HOST") {
            config.host = v;
        }
        if let Ok(v) = std::env::var("SUBCONV_PORT") {
            config.port = v
                .parse()
                .map_err(|_| SubconvError::Config("SUBCONV_PORT is not a valid u16".into()))?;
        }
        if let Ok(v) = std::env::var("SUBCONV_DEFAULT_TEMPLATE") {
            config.default_template = v;
        }
        if let Ok(v) = std::env::var("SUBCONV_DISALLOW_ROBOTS") {
            config.disallow_robots = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_DISALLOW_ROBOTS must be true/false".into())
            })?;
        }
        if let Ok(v) = std::env::var("SUBCONV_CACHE_TTL") {
            config.cache_ttl = v
                .parse()
                .map_err(|_| SubconvError::Config("SUBCONV_CACHE_TTL is not a valid u64".into()))?;
        }
        if let Ok(v) = std::env::var("SUBCONV_CACHE_DIR") {
            config.cache_dir = v;
        }
        if let Ok(v) = std::env::var("SUBCONV_CACHE_MAX_SIZE_MB") {
            config.cache_max_size_mb = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_CACHE_MAX_SIZE_MB is not a valid u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("SUBCONV_SUB_CACHE_TTL") {
            config.sub_cache_ttl = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_SUB_CACHE_TTL is not a valid u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("SUBCONV_SUB_CACHE_LOCK_TIMEOUT") {
            config.sub_cache_lock_timeout = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_SUB_CACHE_LOCK_TIMEOUT is not a valid u64".into())
            })?;
        }
        Ok(())
    }
}
