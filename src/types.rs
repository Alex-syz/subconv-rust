//! Shared domain types used across multiple modules.
//!
//! These types represent domain concepts (group types, rule behaviors, formats)
//! that are referenced by both the config layer (parsing templates) and the
//! packer layer (producing output). Placing them here avoids circular
//! dependencies between `config` and `packer`.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The `type` field for node groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeGroupType {
    Select,
    #[serde(rename = "url-test")]
    UrlTest,
    Fallback,
    #[serde(rename = "load-balance")]
    LoadBalance,
}

impl NodeGroupType {
    /// Whether this group type needs health-check configuration.
    pub fn needs_health_check(&self) -> bool {
        matches!(self, Self::UrlTest | Self::Fallback | Self::LoadBalance)
    }
}

/// Rule provider behavior: how rules are organized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleProviderBehavior {
    #[serde(rename = "classical")]
    Classical,
    #[serde(rename = "domain")]
    Domain,
    #[serde(rename = "ipcidr")]
    IpCidr,
    #[serde(rename = "meta")]
    Meta,
}

/// Rule provider format: how rule data is encoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleProviderFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "yaml")]
    Yaml,
    #[serde(rename = "mrs")]
    Mrs,
}

// Keep FromStr for backward compat with any remaining direct usage.
impl FromStr for NodeGroupType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "select" => Ok(Self::Select),
            "url-test" => Ok(Self::UrlTest),
            "fallback" => Ok(Self::Fallback),
            "load-balance" => Ok(Self::LoadBalance),
            _ => Err(()),
        }
    }
}
