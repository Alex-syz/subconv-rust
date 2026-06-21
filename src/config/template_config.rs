//! Template configuration describing how to transform a subscription.
//!
//! Templates are YAML files that define:
//! - `HEAD`: base Mihomo config (mixed-port, DNS, etc.)
//! - `TEST_URL`: URL used to test proxy connectivity
//! - `RULESET`: list of rule sources (action + URL + optional behavior/format)
//! - `CUSTOM_PROXY_GROUP`: proxy groups to generate

use std::path::Path;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::SubconvError;
use crate::types::{NodeGroupType, RuleProviderBehavior, RuleProviderFormat};

// ---------------------------------------------------------------------------
// Default value helpers
// ---------------------------------------------------------------------------

fn default_test_url() -> String {
    "https://www.gstatic.com/generate_204".into()
}
fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Parsed template configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateConfig {
    /// Base Mihomo configuration (mixed-port, DNS, etc.).
    /// Stored as raw YAML value to preserve ordering and structure.
    #[serde(rename = "HEAD", default)]
    pub head: serde_yaml::Value,

    /// URL used to test proxy connectivity.
    #[serde(rename = "TEST_URL", default = "default_test_url")]
    pub test_url: String,

    /// Rule sources: each entry defines an action (target group) and a URL.
    #[serde(rename = "RULESET", default, deserialize_with = "deserialize_ruleset")]
    pub ruleset: Vec<RuleEntry>,

    /// Proxy groups to generate.
    #[serde(rename = "CUSTOM_PROXY_GROUP", default)]
    pub custom_proxy_group: Vec<Group>,
}

/// A single rule source entry.
///
/// Supports both legacy 2-element array format and extended 4-element format:
/// - `[action, url]` → behavior="classical", format="text"
/// - `[action, url, behavior, format]` → explicit values
#[derive(Debug, Clone)]
pub struct RuleEntry {
    /// Target policy group name (e.g., "🎯 全球直连").
    pub action: String,
    /// Rule URL or inline rule (e.g., `[]GEOIP,CN`).
    pub url: String,
    /// Rule behavior.
    pub behavior: RuleProviderBehavior,
    /// Rule format.
    pub format: RuleProviderFormat,
}

/// A proxy group definition.
#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    /// Group name displayed in Mihomo UI.
    pub name: String,
    /// Group type.
    #[serde(rename = "type")]
    pub group_type: NodeGroupType,
    /// Whether this group appears in rule-based routing.
    #[serde(default = "default_true")]
    pub rule: bool,
    /// Whether this group includes manual node selection.
    #[serde(default)]
    pub manual: bool,
    /// Default policy for the group (e.g., "PROXY", "DIRECT").
    #[serde(default)]
    pub prior: Option<String>,
    /// Regex pattern to filter nodes by name.
    #[serde(default)]
    pub regex: Option<String>,
}

// ---------------------------------------------------------------------------
// Custom deserializer for RuleEntry (backward compatibility)
// ---------------------------------------------------------------------------

/// Deserialize `RULESET` supporting both 2-element and 4-element array formats.
fn deserialize_ruleset<'de, D>(deserializer: D) -> Result<Vec<RuleEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    struct RuleSetVisitor;

    impl<'de> Visitor<'de> for RuleSetVisitor {
        type Value = Vec<RuleEntry>;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a sequence of rule entries (2 or 4 element arrays)")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut entries = Vec::new();
            while let Some(item) = seq.next_element::<serde_yaml::Value>()? {
                let entry = parse_rule_entry(&item).map_err(de::Error::custom)?;
                entries.push(entry);
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_seq(RuleSetVisitor)
}

/// Parse a single RULESET item from YAML value.
fn parse_rule_entry(value: &serde_yaml::Value) -> Result<RuleEntry, String> {
    let arr = value
        .as_sequence()
        .ok_or_else(|| "RULESET entry must be an array".to_string())?;

    match arr.len() {
        2 => {
            // Legacy format: [action, url]
            let action = arr[0]
                .as_str()
                .ok_or_else(|| "RULESET action must be a string".to_string())?
                .to_string();
            let url = arr[1]
                .as_str()
                .ok_or_else(|| "RULESET url must be a string".to_string())?
                .to_string();
            Ok(RuleEntry {
                action,
                url,
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            })
        }
        4 => {
            // Extended format: [action, url, behavior, format]
            let action = arr[0]
                .as_str()
                .ok_or_else(|| "RULESET action must be a string".to_string())?
                .to_string();
            let url = arr[1]
                .as_str()
                .ok_or_else(|| "RULESET url must be a string".to_string())?
                .to_string();
            let behavior_str = arr[2]
                .as_str()
                .ok_or_else(|| "RULESET behavior must be a string".to_string())?;
            let format_str = arr[3]
                .as_str()
                .ok_or_else(|| "RULESET format must be a string".to_string())?;
            let behavior = serde_yaml::from_value(serde_yaml::Value::String(behavior_str.to_string()))
                .map_err(|e| format!("invalid behavior '{behavior_str}': {e}"))?;
            let format = serde_yaml::from_value(serde_yaml::Value::String(format_str.to_string()))
                .map_err(|e| format!("invalid format '{format_str}': {e}"))?;
            Ok(RuleEntry {
                action,
                url,
                behavior,
                format,
            })
        }
        n => Err(format!(
            "RULESET entry must have 2 or 4 elements, found {n}"
        )),
    }
}

// ---------------------------------------------------------------------------
// Loading logic
// ---------------------------------------------------------------------------

impl TemplateConfig {
    /// Load a template from a local YAML file.
    ///
    /// # Arguments
    /// * `name` - Template name (without `.yaml` extension)
    /// * `template_dir` - Directory containing template files
    pub fn load_from_file(name: &str, template_dir: &Path) -> Result<Self, SubconvError> {
        let path = template_dir.join(format!("{name}.yaml"));
        if !path.exists() {
            return Err(SubconvError::TemplateNotFound(format!(
                "template '{name}' not found in {}",
                template_dir.display()
            )));
        }

        let raw = std::fs::read_to_string(&path)
            .map_err(|e| SubconvError::Config(format!("failed to read {}: {e}", path.display())))?;
        let config: Self = serde_yaml::from_str(&raw).map_err(|e| {
            SubconvError::Config(format!("failed to parse {}: {e}", path.display()))
        })?;
        Ok(config)
    }

    /// Load a template from a remote URL.
    ///
    /// # Arguments
    /// * `name` - Template name (for error messages)
    /// * `url` - Remote URL to fetch
    /// * `sha256` - Optional hex digest for integrity verification
    /// * `client` - HTTP client to use
    pub async fn load_from_remote(
        name: &str,
        url: &str,
        sha256: Option<&str>,
        client: &reqwest::Client,
    ) -> Result<Self, SubconvError> {
        let resp = client.get(url).send().await.map_err(|e| {
            SubconvError::UpstreamFetch(format!("failed to fetch remote template '{name}': {e}"))
        })?;

        if !resp.status().is_success() {
            return Err(SubconvError::UpstreamFetch(format!(
                "remote template '{name}' returned HTTP {}",
                resp.status()
            )));
        }

        let raw = resp.text().await.map_err(|e| {
            SubconvError::UpstreamFetch(format!("failed to read remote template '{name}': {e}"))
        })?;

        // Verify SHA256 if provided.
        if let Some(expected) = sha256 {
            let mut hasher = Sha256::new();
            hasher.update(raw.as_bytes());
            let actual = hex::encode(hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(SubconvError::Config(format!(
                    "remote template '{name}' SHA256 mismatch: expected {expected}, got {actual}"
                )));
            }
        }

        let config: Self = serde_yaml::from_str(&raw).map_err(|e| {
            SubconvError::Config(format!("failed to parse remote template '{name}': {e}"))
        })?;
        Ok(config)
    }

    /// Resolve a template by name: first check local, then remote whitelist.
    ///
    /// # Arguments
    /// * `name` - Template name
    /// * `config` - Application config (contains remote whitelist)
    /// * `client` - HTTP client for remote fetches
    pub async fn resolve_template(
        name: &str,
        app_config: &super::AppConfig,
        client: &reqwest::Client,
    ) -> Result<Self, SubconvError> {
        let template_dir = Path::new("template");

        // Try local first.
        if template_dir.join(format!("{name}.yaml")).exists() {
            return Self::load_from_file(name, template_dir);
        }

        // Check remote whitelist.
        if let Some(rt) = app_config.remote_template(name) {
            return Self::load_from_remote(name, &rt.url, rt.sha256.as_deref(), client).await;
        }

        // Not found anywhere.
        let available = app_config.available_templates();
        Err(SubconvError::TemplateNotFound(format!(
            "template '{name}' not found (available: {})",
            available.join(", ")
        )))
    }

    /// Return all remote rule URLs in this template.
    ///
    /// Used to build the `/proxy` endpoint whitelist for SSRF protection.
    pub fn ruleset_urls(&self) -> Vec<&str> {
        self.ruleset
            .iter()
            .filter(|r| r.url.starts_with("http://") || r.url.starts_with("https://"))
            .map(|r| r.url.as_str())
            .collect()
    }
}
