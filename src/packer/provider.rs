//! Provider construction: proxy-providers and rule-providers.
//!
//! Builds the `proxy-providers` and `rule-providers` sections of the Mihomo
//! YAML, plus the `rules` list. Proxy-provider URLs are proxied through
//! `{base_url}provider?...`; rule-provider URLs go through
//! `{base_url}proxy?...` unless `notproxyrule` is set.

use indexmap::IndexMap;

use crate::config::template_config::RuleEntry;

use super::types::{
    HealthCheck, ProviderType, ProxyProvider, RuleProvider, RuleProviderType,
};

// ── Proxy providers ─────────────────────────────────────────────────────────

/// Build `proxy-providers` from subscription URLs.
///
/// Returns `(providers, provider_names, all_proxy_names_in_providers)`.
///
/// - Primary URLs produce names like `subscription0`, `subscription1`, ...
/// - Standby URLs produce names like `subscriptionsub0`, `subscriptionsub1`, ...
pub fn build_proxy_providers(
    urls: &[String],
    standby_urls: &[String],
    interval: u32,
    base_url: &str,
    test_url: &str,
) -> (IndexMap<String, ProxyProvider>, Vec<String>, Vec<String>) {
    let mut providers: IndexMap<String, ProxyProvider> = IndexMap::new();
    let mut provider_names: Vec<String> = Vec::new();
    // all_proxy_names is populated by the caller from subscription parsing;
    // we cannot know proxy names inside remote providers at this point.
    let all_proxy_names: Vec<String> = Vec::new();

    // Primary subscriptions
    for (i, url) in urls.iter().enumerate() {
        let name = format!("subscription{i}");
        let path = format!("./sub/{name}.yaml");

        let proxied_url = format!(
            "{}provider?url={}",
            base_url,
            urlencoding::encode(url)
        );

        providers.insert(
            name.clone(),
            ProxyProvider {
                provider_type: ProviderType::Http,
                url: proxied_url,
                interval: Some(interval),
                path: Some(path),
                health_check: Some(HealthCheck {
                    enable: true,
                    url: test_url.into(),
                    interval: Some(60),
                }),
            },
        );
        provider_names.push(name);
    }

    // Standby subscriptions
    for (i, url) in standby_urls.iter().enumerate() {
        let name = format!("subscriptionsub{i}");
        let path = format!("./sub/{name}.yaml");

        let proxied_url = format!(
            "{}provider?url={}",
            base_url,
            urlencoding::encode(url)
        );

        providers.insert(
            name,
            ProxyProvider {
                provider_type: ProviderType::Http,
                url: proxied_url,
                interval: Some(interval),
                path: Some(path),
                health_check: Some(HealthCheck {
                    enable: true,
                    url: test_url.into(),
                    interval: Some(60),
                }),
            },
        );
    }

    (providers, provider_names, all_proxy_names)
}

// ── Rule providers ──────────────────────────────────────────────────────────

/// Build `rule-providers` and `rules` list from the template's RULESET.
///
/// Returns `(rule_providers, rules)`.
///
/// Rule name deduplication uses an incrementing counter instead of random
/// numbers, producing deterministic output.
///
/// URL transformation:
/// - `[]`-prefixed entries are inline rules (no provider needed).
/// - If `notproxyrule` is true, original URLs are used as-is.
/// - Otherwise, URLs are proxied through `{base_url}proxy?url=...&template=...`.
pub fn build_rule_providers(
    ruleset: &[RuleEntry],
    base_url: &str,
    template_name: &str,
    notproxyrule: bool,
    domain: &str,
) -> (IndexMap<String, RuleProvider>, Vec<String>) {
    let mut rule_providers: IndexMap<String, RuleProvider> = IndexMap::new();
    let mut rules: Vec<String> = Vec::new();

    // First rule: direct the domain
    rules.push(format!("DOMAIN,{domain},DIRECT"));

    // Track name collisions for deterministic deduplication
    let mut name_counts: IndexMap<String, u32> = IndexMap::new();

    for entry in ruleset {
        let rule_url = &entry.url;

        // Inline rule (e.g., []GEOIP,CN or []MATCH)
        if let Some(inline) = rule_url.strip_prefix("[]") {
            if inline == "FINAL" || inline == "MATCH" {
                rules.push(format!("MATCH,{}", entry.action));
            } else {
                rules.push(format!("{inline},{}", entry.action));
            }
            continue;
        }

        // Derive name from URL's last path segment (without extension)
        let base_name = derive_rule_name(rule_url);

        // Deduplicate with incrementing counter
        let unique_name = match name_counts.get_mut(&base_name) {
            Some(count) => {
                *count += 1;
                format!("{base_name}{}", *count)
            }
            None => {
                name_counts.insert(base_name.clone(), 0);
                base_name
            }
        };

        rules.push(format!("RULE-SET,{unique_name},{}", entry.action));

        // Transform URL if needed
        let final_url = if notproxyrule {
            rule_url.clone()
        } else {
            let mut params = vec![format!("url={}", urlencoding::encode(rule_url))];
            if template_name != "meta-rules" {
                params.push(format!("template={}", urlencoding::encode(template_name)));
            }
            format!("{}proxy?{}", base_url, params.join("&"))
        };

        let path = format!("./rule/{unique_name}.txt");

        rule_providers.insert(
            unique_name,
            RuleProvider {
                provider_type: RuleProviderType::Http,
                behavior: entry.behavior.clone(),
                format: Some(entry.format.clone()),
                url: Some(final_url),
                path: Some(path),
                interval: Some(86400 * 7),
            },
        );
    }

    (rule_providers, rules)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Derive a rule name from a URL by taking the last path segment without
/// its file extension.
fn derive_rule_name(url: &str) -> String {
    // Parse the URL and extract the last path segment, avoiding
    // lifetime issues with the parsed URL.
    let parsed = url::Url::parse(url);
    let segment = parsed
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|mut s| s.next_back().map(|seg| seg.to_string()))
        })
        .unwrap_or_else(|| "rule".into());

    // Remove file extension
    if let Some(dot_pos) = segment.rfind('.') {
        segment[..dot_pos].to_string()
    } else {
        segment
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RuleProviderBehavior, RuleProviderFormat};

    #[test]
    fn build_proxy_providers_creates_primary_and_standby() {
        let urls = vec!["https://sub1.example.com".into()];
        let standby = vec!["https://sub2.example.com".into()];

        let (providers, names, _) = build_proxy_providers(
            &urls,
            &standby,
            300,
            "https://conv.example.com/",
            "https://test.example.com",
        );

        assert!(providers.contains_key("subscription0"));
        assert!(providers.contains_key("subscriptionsub0"));
        assert_eq!(names, vec!["subscription0"]);
    }

    #[test]
    fn proxy_provider_url_uses_base_url() {
        let urls = vec!["https://sub.example.com/list".into()];

        let (providers, _, _) = build_proxy_providers(
            &urls,
            &[],
            300,
            "https://conv.example.com/",
            "https://test.example.com",
        );

        let p = &providers["subscription0"];
        assert!(p.url.starts_with("https://conv.example.com/provider?url="));
        assert_eq!(p.interval, Some(300));
        assert!(p.path.as_ref().unwrap().contains("subscription0"));
    }

    #[test]
    fn build_rule_providers_basic() {
        let entries = vec![
            RuleEntry {
                action: "DIRECT".into(),
                url: "https://rules.example.com/direct.yaml".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
            RuleEntry {
                action: "PROXY".into(),
                url: "https://rules.example.com/proxy.yaml".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
        ];

        let (providers, rules) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            false,
            "conv.example.com",
        );

        assert!(providers.contains_key("direct"));
        assert!(providers.contains_key("proxy"));
        assert_eq!(rules[0], "DOMAIN,conv.example.com,DIRECT");
        assert!(rules.iter().any(|r| r.starts_with("RULE-SET,direct,")));
        assert!(rules.iter().any(|r| r.starts_with("RULE-SET,proxy,")));
    }

    #[test]
    fn inline_rules_no_provider() {
        let entries = vec![
            RuleEntry {
                action: "DIRECT".into(),
                url: "[]GEOIP,CN".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
            RuleEntry {
                action: "PROXY".into(),
                url: "[]MATCH".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
        ];

        let (providers, rules) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            false,
            "conv.example.com",
        );

        assert!(providers.is_empty());
        assert!(rules.iter().any(|r| r == "GEOIP,CN,DIRECT"));
        assert!(rules.iter().any(|r| r == "MATCH,PROXY"));
    }

    #[test]
    fn notproxyrule_uses_raw_urls() {
        let entries = vec![RuleEntry {
            action: "DIRECT".into(),
            url: "https://rules.example.com/direct.yaml".into(),
            behavior: RuleProviderBehavior::Classical,
            format: RuleProviderFormat::Text,
        }];

        let (providers, _) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            true,
            "conv.example.com",
        );

        let p = &providers["direct"];
        assert_eq!(p.url.as_deref(), Some("https://rules.example.com/direct.yaml"));
    }

    #[test]
    fn rule_name_deduplication_uses_counter() {
        let entries = vec![
            RuleEntry {
                action: "DIRECT".into(),
                url: "https://rules.example.com/list.yaml".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
            RuleEntry {
                action: "PROXY".into(),
                url: "https://other.example.com/list.yaml".into(),
                behavior: RuleProviderBehavior::Classical,
                format: RuleProviderFormat::Text,
            },
        ];

        let (providers, rules) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            false,
            "conv.example.com",
        );

        // First gets "list", second gets "list1"
        assert!(providers.contains_key("list"));
        assert!(providers.contains_key("list1"));
        assert!(rules.iter().any(|r| r.starts_with("RULE-SET,list,")));
        assert!(rules.iter().any(|r| r.starts_with("RULE-SET,list1,")));
    }

    #[test]
    fn template_name_added_to_proxy_url_when_not_default() {
        let entries = vec![RuleEntry {
            action: "DIRECT".into(),
            url: "https://rules.example.com/direct.yaml".into(),
            behavior: RuleProviderBehavior::Classical,
            format: RuleProviderFormat::Text,
        }];

        let (providers, _) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            false,
            "conv.example.com",
        );

        let p = &providers["direct"];
        let url = p.url.as_deref().unwrap();
        assert!(url.contains("template=zju"));
    }

    #[test]
    fn default_template_is_omitted_from_proxy_url() {
        let entries = vec![RuleEntry {
            action: "DIRECT".into(),
            url: "https://rules.example.com/direct.yaml".into(),
            behavior: RuleProviderBehavior::Classical,
            format: RuleProviderFormat::Text,
        }];

        let (providers, _) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "meta-rules",
            false,
            "conv.example.com",
        );

        let url = providers["direct"].url.as_deref().unwrap();
        assert!(!url.contains("template="));
    }

    #[test]
    fn derive_rule_name_extracts_last_segment() {
        assert_eq!(
            derive_rule_name("https://rules.example.com/path/to/my-rules.yaml"),
            "my-rules"
        );
    }

    #[test]
    fn derive_rule_name_no_extension() {
        assert_eq!(
            derive_rule_name("https://rules.example.com/path/to/myrules"),
            "myrules"
        );
    }

    #[test]
    fn final_inline_rule_becomes_match() {
        let entries = vec![RuleEntry {
            action: "PROXY".into(),
            url: "[]FINAL".into(),
            behavior: RuleProviderBehavior::Classical,
            format: RuleProviderFormat::Text,
        }];

        let (_, rules) = build_rule_providers(
            &entries,
            "https://conv.example.com/",
            "zju",
            false,
            "conv.example.com",
        );

        assert!(rules.iter().any(|r| r == "MATCH,PROXY"));
    }

}
