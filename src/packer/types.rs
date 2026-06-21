//! Clash config output types for the packer module.
//!
//! `ClashConfig` is the top-level structure that serializes to a complete
//! mihomo YAML config. It uses `#[serde(flatten)]` for the head section
//! (arbitrary key-values from the template) and typed fields for proxies,
//! proxy-groups, providers, and rules.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::converter::proxy::Proxy;

// Re-export shared domain types for convenience.
pub use crate::types::{NodeGroupType, RuleProviderBehavior, RuleProviderFormat};

// ── Top-level config ────────────────────────────────────────────────────────

/// Complete Mihomo configuration output.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClashConfig {
    /// Arbitrary key-values from the template (port, mode, dns, etc.).
    #[serde(flatten)]
    pub head: serde_yaml::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<Proxy>>,

    #[serde(rename = "proxy-providers", skip_serializing_if = "Option::is_none")]
    pub proxy_providers: Option<IndexMap<String, ProxyProvider>>,

    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,

    #[serde(rename = "rule-providers", skip_serializing_if = "Option::is_none")]
    pub rule_providers: Option<IndexMap<String, RuleProvider>>,

    pub rules: Vec<String>,
}

// ── Proxy group ─────────────────────────────────────────────────────────────

/// A proxy group entry. Rule-select groups only carry `name`, `type`, and
/// `proxies`; node groups carry additional fields like `use`, `filter`,
/// `url`, `interval`, `tolerance`, and `strategy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProxyGroup {
    pub name: String,
    #[serde(flatten)]
    pub kind: ProxyGroupKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProxyGroupKind {
    /// A rule-based select group (rule=true in config).
    RuleSelect(RuleSelectGroup),
    /// A node group (rule=false) with health-check and strategy support.
    NodeGroup(NodeGroup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSelectGroup {
    #[serde(rename = "type")]
    pub group_type: SelectType,
    pub proxies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    #[serde(rename = "type")]
    pub group_type: NodeGroupType,

    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_providers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
}

/// The `type` field for rule-select groups is always "select".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectType {
    #[serde(rename = "select")]
    Select,
}

// ── Proxy provider ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProxyProvider {
    #[serde(rename = "type")]
    pub provider_type: ProviderType,

    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    #[serde(rename = "http")]
    Http,
}

// ── Health check ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HealthCheck {
    pub enable: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

// ── Rule provider ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuleProvider {
    #[serde(rename = "type")]
    pub provider_type: RuleProviderType,

    pub behavior: RuleProviderBehavior,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<RuleProviderFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleProviderType {
    #[serde(rename = "http")]
    Http,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_select_group_serializes() {
        let group = ProxyGroup {
            name: "Ad Block".into(),
            kind: ProxyGroupKind::RuleSelect(RuleSelectGroup {
                group_type: SelectType::Select,
                proxies: vec!["REJECT".into(), "DIRECT".into()],
            }),
        };
        let yaml = serde_yaml::to_string(&group).unwrap();
        assert!(yaml.contains("name: Ad Block"));
        assert!(yaml.contains("type: select"));
        assert!(yaml.contains("- REJECT"));
        assert!(yaml.contains("- DIRECT"));
    }

    #[test]
    fn node_group_url_test() {
        let group = ProxyGroup {
            name: "Auto".into(),
            kind: ProxyGroupKind::NodeGroup(NodeGroup {
                group_type: NodeGroupType::UrlTest,
                use_providers: Some(vec!["subscription0".into()]),
                proxies: None,
                filter: None,
                url: Some("http://www.gstatic.com/generate_204".into()),
                interval: Some(60),
                tolerance: Some(50),
                strategy: None,
            }),
        };
        let yaml = serde_yaml::to_string(&group).unwrap();
        assert!(yaml.contains("type: url-test"));
        assert!(yaml.contains("use:"));
        assert!(yaml.contains("interval: 60"));
        assert!(yaml.contains("tolerance: 50"));
        // No strategy field since it's None
        assert!(!yaml.contains("strategy"));
    }

    #[test]
    fn load_balance_with_strategy() {
        let group = ProxyGroup {
            name: "LB".into(),
            kind: ProxyGroupKind::NodeGroup(NodeGroup {
                group_type: NodeGroupType::LoadBalance,
                use_providers: Some(vec!["subscription0".into()]),
                proxies: None,
                filter: Some("HK|TW".into()),
                url: Some("http://www.gstatic.com/generate_204".into()),
                interval: Some(60),
                tolerance: Some(50),
                strategy: Some("consistent-hashing".into()),
            }),
        };
        let yaml = serde_yaml::to_string(&group).unwrap();
        assert!(yaml.contains("type: load-balance"));
        assert!(yaml.contains("filter: HK|TW"));
        assert!(yaml.contains("strategy: consistent-hashing"));
    }

    #[test]
    fn proxy_provider_serializes() {
        let provider = ProxyProvider {
            provider_type: ProviderType::Http,
            url: "https://example.com/sub".into(),
            interval: Some(300),
            path: Some("./sub/subscription0.yaml".into()),
            health_check: Some(HealthCheck {
                enable: true,
                url: "http://www.gstatic.com/generate_204".into(),
                interval: Some(60),
            }),
        };
        let yaml = serde_yaml::to_string(&provider).unwrap();
        assert!(yaml.contains("type: http"));
        assert!(yaml.contains("health-check:"));
        assert!(yaml.contains("enable: true"));
    }

    #[test]
    fn rule_provider_serializes() {
        let provider = RuleProvider {
            provider_type: RuleProviderType::Http,
            behavior: RuleProviderBehavior::Classical,
            format: Some(RuleProviderFormat::Text),
            url: Some("https://example.com/rules".into()),
            path: Some("./rule/test.txt".into()),
            interval: Some(604800),
        };
        let yaml = serde_yaml::to_string(&provider).unwrap();
        assert!(yaml.contains("type: http"));
        assert!(yaml.contains("behavior: classical"));
        assert!(yaml.contains("format: text"));
    }

    #[test]
    fn node_group_type_needs_health_check() {
        assert!(!NodeGroupType::Select.needs_health_check());
        assert!(NodeGroupType::UrlTest.needs_health_check());
        assert!(NodeGroupType::Fallback.needs_health_check());
        assert!(NodeGroupType::LoadBalance.needs_health_check());
    }
}
