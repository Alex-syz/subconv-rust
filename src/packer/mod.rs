//! Clash config assembly and YAML output.
//!
//! The `pack()` function is the main entry point. It takes proxy nodes,
//! subscription URLs, and a template configuration, then assembles a complete
//! Mihomo YAML config string.

pub mod group_builder;
pub mod provider;
pub mod types;


use crate::config::template_config::TemplateConfig;
use crate::converter::proxy::Proxy;
use crate::SubconvError;

use self::group_builder::{build_all_groups, GroupContext};
use self::provider::{build_proxy_providers, build_rule_providers};
use self::types::ClashConfig;

/// Assemble a complete Mihomo YAML configuration.
///
/// # Arguments
///
/// * `proxies` - Primary standalone proxy nodes
/// * `standby_proxies` - Standby standalone proxy nodes
/// * `urls` - Primary subscription URLs
/// * `standby_urls` - Standby subscription URLs
/// * `interval` - Proxy provider refresh interval (in seconds, as a string)
/// * `base_url` - Base URL of this SubConv instance (for provider/rule proxying)
/// * `template_name` - Template name (affects rule URL construction)
/// * `template_config` - Parsed template configuration
/// * `short` - If true, omit the HEAD section (base config)
/// * `notproxyrule` - If true, rule URLs are not proxied through /proxy
/// * `provider_proxy_names` - Proxy names found inside providers (for regex matching)
/// * `domain` - The domain of this SubConv instance (added as a DIRECT rule)
#[allow(clippy::too_many_arguments)]
pub fn pack(
    proxies: Vec<Proxy>,
    standby_proxies: Vec<Proxy>,
    urls: &[String],
    standby_urls: &[String],
    interval: &str,
    base_url: &str,
    template_name: &str,
    template_config: &TemplateConfig,
    short: bool,
    notproxyrule: bool,
    provider_proxy_names: &[String],
    domain: &str,
) -> Result<String, SubconvError> {
    // 1. HEAD section
    let head = if short {
        serde_yaml::Value::Null
    } else {
        template_config.head.clone()
    };

    // 2. Standalone proxies
    let all_proxies: Vec<Proxy> = proxies
        .into_iter()
        .chain(standby_proxies)
        .collect();

    let (proxies_field, proxies_name, proxies_standby_name) =
        collect_standalone_proxy_names(&all_proxies);

    // 3. Proxy providers
    let interval_val: u32 = interval.parse().unwrap_or_else(|e| {
        tracing::warn!(interval, error = %e, "invalid interval, using default 300");
        300
    });

    let (proxy_providers, subscription_names, _provider_names) = build_proxy_providers(
        urls,
        standby_urls,
        interval_val,
        base_url,
        &template_config.test_url,
    );

    // Build standby list: primary + standby provider names
    let mut standby_names = subscription_names.clone();
    for i in 0..standby_urls.len() {
        standby_names.push(format!("subscriptionsub{i}"));
    }

    // 4. Proxy groups
    let ctx = GroupContext {
        template: template_config,
        subscriptions: &subscription_names,
        standby: &standby_names,
        proxies_name: &proxies_name,
        proxies_standby_name: &proxies_standby_name,
        provider_proxy_names,
    };

    let (proxy_groups, _discarded) = build_all_groups(&ctx);

    // 5. Rule providers and rules
    let (rule_providers, rules) = build_rule_providers(
        &template_config.ruleset,
        base_url,
        template_name,
        notproxyrule,
        domain,
    );

    // 6. Assemble final config
    let config = ClashConfig {
        head,
        proxies: proxies_field,
        proxy_providers: if proxy_providers.is_empty() {
            None
        } else {
            Some(proxy_providers)
        },
        proxy_groups,
        rule_providers: if rule_providers.is_empty() {
            None
        } else {
            Some(rule_providers)
        },
        rules,
    };

    // 7. Serialize to YAML
    let yaml = serde_yaml::to_string(&config)?;
    Ok(yaml)
}

/// Collect proxy names from standalone proxies.
///
/// Returns `(proxies_field, primary_names, standby_names)` where:
/// - `proxies_field` is `Some(all_proxies)` if non-empty, else `None`
/// - `primary_names` contains names from the first `proxies.len()` entries
/// - `standby_names` contains names from the remaining entries
fn collect_standalone_proxy_names(
    all_proxies: &[Proxy],
) -> (Option<Vec<Proxy>>, Vec<String>, Vec<String>) {
    if all_proxies.is_empty() {
        return (None, vec![], vec![]);
    }

    let names: Vec<String> = all_proxies.iter().map(|p| p.name().to_string()).collect();

    // In the Python code, proxiesName includes all standalone proxy names
    // (both primary and standby), while proxiesStandbyName also includes all.
    // The distinction is that proxiesName is used for non-manual groups
    // and proxiesStandbyName for manual groups.
    //
    // For now, both lists contain all standalone proxy names, matching the
    // Python behavior where urlstandalone names go into both lists.
    let proxies_field = if all_proxies.is_empty() {
        None
    } else {
        Some(all_proxies.to_vec())
    };

    (proxies_field, names.clone(), names)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::template_config::Group;
    use crate::types::NodeGroupType;

    fn make_template(groups: Vec<Group>) -> TemplateConfig {
        TemplateConfig {
            head: serde_yaml::from_str("mixed-port: 7890").unwrap(),
            test_url: "https://www.gstatic.com/generate_204".into(),
            ruleset: vec![],
            custom_proxy_group: groups,
        }
    }

    fn make_group(
        name: &str,
        group_type: NodeGroupType,
        rule: bool,
        manual: bool,
        prior: Option<&str>,
        regex: Option<&str>,
    ) -> Group {
        Group {
            name: name.into(),
            group_type,
            rule,
            manual,
            prior: prior.map(|s| s.into()),
            regex: regex.map(|s| s.into()),
        }
    }

    #[test]
    fn pack_produces_valid_yaml_with_head() {
        let template = make_template(vec![
            make_group("Auto", NodeGroupType::UrlTest, false, false, None, None),
        ]);

        let result = pack(
            vec![],
            vec![],
            &["https://sub.example.com".into()],
            &[],
            "300",
            "https://conv.example.com/",
            "zju",
            &template,
            false,
            false,
            &[],
            "conv.example.com",
        )
        .unwrap();

        assert!(result.contains("mixed-port: 7890"));
        assert!(result.contains("proxy-groups:"));
        assert!(result.contains("proxy-providers:"));
        assert!(result.contains("rules:"));
    }

    #[test]
    fn pack_short_mode_omits_head() {
        let template = make_template(vec![
            make_group("Auto", NodeGroupType::UrlTest, false, false, None, None),
        ]);

        let result = pack(
            vec![],
            vec![],
            &["https://sub.example.com".into()],
            &[],
            "300",
            "https://conv.example.com/",
            "zju",
            &template,
            true,
            false,
            &[],
            "conv.example.com",
        )
        .unwrap();

        assert!(!result.contains("mixed-port: 7890"));
    }

    #[test]
    fn pack_with_standalone_proxies() {
        use crate::converter::proxy::{HttpSocksProxy, Proxy};

        let proxy = Proxy::Http(HttpSocksProxy {
            name: "my-http".into(),
            server: "1.2.3.4".into(),
            port: 8080,
            username: None,
            password: None,
            tls: None,
            skip_cert_verify: None,
        });

        let template = make_template(vec![
            make_group("Auto", NodeGroupType::UrlTest, false, false, None, None),
        ]);

        let result = pack(
            vec![proxy],
            vec![],
            &[],
            &[],
            "300",
            "https://conv.example.com/",
            "zju",
            &template,
            false,
            false,
            &[],
            "conv.example.com",
        )
        .unwrap();

        assert!(result.contains("my-http"));
    }

    #[test]
    fn collect_standalone_proxy_names_empty() {
        let (field, primary, standby) = collect_standalone_proxy_names(&[]);
        assert!(field.is_none());
        assert!(primary.is_empty());
        assert!(standby.is_empty());
    }

    #[test]
    fn pack_no_providers_no_proxies_still_works() {
        let template = make_template(vec![]);

        let result = pack(
            vec![],
            vec![],
            &[],
            &[],
            "300",
            "https://conv.example.com/",
            "zju",
            &template,
            false,
            false,
            &[],
            "conv.example.com",
        )
        .unwrap();

        // Should still produce valid YAML with empty proxy-groups
        assert!(result.contains("proxy-groups: []"));
    }
}
