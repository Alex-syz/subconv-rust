//! Proxy group construction logic.
//!
//! Transforms `TemplateConfig::custom_proxy_group` definitions into the
//! `ProxyGroup` values that appear in the final Mihomo YAML. The root
//! "🚀 节点选择" group is always emitted first; rule-select groups and
//! node groups follow in template order.

use std::collections::HashSet;

use regex::Regex;

use crate::config::template_config::{Group, TemplateConfig};

use super::types::{
    NodeGroup, NodeGroupType, ProxyGroup, ProxyGroupKind, RuleSelectGroup, SelectType,
};

// ── Context passed into group building ──────────────────────────────────────

/// Everything the group builder needs from the caller.
pub struct GroupContext<'a> {
    pub template: &'a TemplateConfig,
    /// Primary provider names (e.g., ["subscription0", "subscription1"]).
    pub subscriptions: &'a [String],
    /// All provider names including standby (e.g., ["subscription0", "subscriptionsub0"]).
    pub standby: &'a [String],
    /// Names of primary standalone proxies.
    pub proxies_name: &'a [String],
    /// Names of standby standalone proxies.
    pub proxies_standby_name: &'a [String],
    /// All proxy names found inside providers (for regex matching).
    pub provider_proxy_names: &'a [String],
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Build all proxy groups from the template configuration.
///
/// Returns `(groups, discarded_names)` where `discarded_names` are group names
/// that had no matching nodes and were removed from the root group.
pub fn build_all_groups(ctx: &GroupContext) -> (Vec<ProxyGroup>, Vec<String>) {
    let mut groups: Vec<ProxyGroup> = Vec::new();
    let mut discarded: Vec<String> = Vec::new();

    // 1. Root group "🚀 节点选择"
    if !ctx.template.custom_proxy_group.is_empty() {
        groups.push(build_root_group(ctx.template));
    }

    // 2. Each group definition
    for group_def in &ctx.template.custom_proxy_group {
        let group_type = group_def.group_type;

        if group_def.rule && group_type == NodeGroupType::Select {
            // Rule-based select group
            groups.push(build_rule_select_group(group_def, ctx.template));
        } else {
            // Node group (may be discarded if no matching nodes)
            match build_node_group(group_def, group_type, ctx) {
                Some(g) => groups.push(g),
                None => {
                    discarded.push(group_def.name.clone());
                }
            }
        }
    }

    // 3. Remove discarded group names from root group
    if !discarded.is_empty() {
        remove_discarded_from_root(&mut groups, &discarded);
    }

    // 4. Validate references: remove proxy names that don't exist
    let valid_names = collect_valid_names(&groups, ctx);
    validate_group_references(&mut groups, &valid_names);

    (groups, discarded)
}

// ── Root group ──────────────────────────────────────────────────────────────

/// Build the root "🚀 节点选择" select group.
///
/// Its proxies list contains all non-rule group names plus "DIRECT".
fn build_root_group(template: &TemplateConfig) -> ProxyGroup {
    let mut proxies: Vec<String> = template
        .custom_proxy_group
        .iter()
        .filter(|g| !g.rule)
        .map(|g| g.name.clone())
        .collect();
    proxies.push("DIRECT".into());

    ProxyGroup {
        name: "🚀 节点选择".into(),
        kind: ProxyGroupKind::RuleSelect(RuleSelectGroup {
            group_type: SelectType::Select,
            proxies,
        }),
    }
}

// ── Rule-select group ───────────────────────────────────────────────────────

/// Build a rule-based select group (rule=true, type=select).
///
/// The `prior` field controls the order of the built-in policies:
/// - "DIRECT" → [DIRECT, REJECT, 🚀节点选择, ...non_rule_groups]
/// - "REJECT" → [REJECT, DIRECT, 🚀节点选择, ...non_rule_groups]
/// - other    → [🚀节点选择, ...non_rule_groups, DIRECT, REJECT]
fn build_rule_select_group(group_def: &Group, template: &TemplateConfig) -> ProxyGroup {
    let non_rule_names: Vec<String> = template
        .custom_proxy_group
        .iter()
        .filter(|g| !g.rule)
        .map(|g| g.name.clone())
        .collect();

    let proxies = match group_def.prior.as_deref() {
        Some("DIRECT") => {
            let mut p = vec![
                "DIRECT".into(),
                "REJECT".into(),
                "🚀 节点选择".into(),
            ];
            p.extend(non_rule_names);
            p
        }
        Some("REJECT") => {
            let mut p = vec![
                "REJECT".into(),
                "DIRECT".into(),
                "🚀 节点选择".into(),
            ];
            p.extend(non_rule_names);
            p
        }
        _ => {
            let mut p = vec!["🚀 节点选择".into()];
            p.extend(non_rule_names);
            p.push("DIRECT".into());
            p.push("REJECT".into());
            p
        }
    };

    ProxyGroup {
        name: group_def.name.clone(),
        kind: ProxyGroupKind::RuleSelect(RuleSelectGroup {
            group_type: SelectType::Select,
            proxies,
        }),
    }
}

// ── Node group ──────────────────────────────────────────────────────────────

/// Build a node group (rule=false, or non-select type).
///
/// Returns `None` only if the group has no viable data source:
/// - Empty regex (meaningless filter)
/// - Invalid regex
/// - No providers AND no standalone proxies available
///
/// When a regex is present but no proxy names match (e.g. partial fetch failure),
/// the group is kept with `use` + `filter` so that Mihomo can filter nodes from
/// the provider on its own refresh cycle. This prevents groups disappearing when
/// upstream subscriptions are temporarily unreachable.
fn build_node_group(
    group_def: &Group,
    group_type: NodeGroupType,
    ctx: &GroupContext,
) -> Option<ProxyGroup> {
    let mut node_group = NodeGroup {
        group_type,
        use_providers: None,
        proxies: None,
        filter: None,
        url: None,
        interval: None,
        tolerance: None,
        strategy: None,
    };

    match &group_def.regex {
        Some(regex_pattern) if regex_pattern.is_empty() => {
            // Empty regex is meaningless — discard this group.
            return None;
        }
        Some(regex_pattern) => {
            // Compile regex with case-insensitive flag
            let compiled = match Regex::new(&format!("(?i){regex_pattern}")) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        "invalid regex '{}' for group '{}': {e}",
                        regex_pattern,
                        group_def.name
                    );
                    return None;
                }
            };

            let filter_str = regex_pattern.clone();

            if group_def.manual {
                // Manual mode: always set `use` if standby providers exist,
                // regardless of whether current proxy names match the regex.
                // Mihomo will apply the filter on its own provider refresh.
                if !ctx.standby.is_empty() {
                    node_group.use_providers = Some(ctx.standby.to_vec());
                }
                // Still filter standalone proxies by regex for direct inclusion.
                let proxy_matches: Vec<String> = ctx.proxies_standby_name.iter()
                    .filter(|p| compiled.is_match(p))
                    .cloned()
                    .collect();
                if !proxy_matches.is_empty() {
                    node_group.proxies = Some(proxy_matches);
                }
            } else {
                // Auto mode: always set `use` if subscriptions exist,
                // regardless of whether current proxy names match the regex.
                if !ctx.subscriptions.is_empty() {
                    node_group.use_providers = Some(ctx.subscriptions.to_vec());
                }
                // Still filter standalone proxies by regex for direct inclusion.
                let proxy_matches: Vec<String> = ctx.proxies_name.iter()
                    .filter(|p| compiled.is_match(p))
                    .cloned()
                    .collect();
                if !proxy_matches.is_empty() {
                    node_group.proxies = Some(proxy_matches);
                }
            }

            // Discard only when there is truly no data source for this group
            // — no providers to pull from, AND no matching standalone proxies.
            if node_group.use_providers.is_none() && node_group.proxies.is_none() {
                return None;
            }

            node_group.filter = Some(filter_str);
        }
        _ => {
            // No regex: include all providers/proxies
            if group_def.manual {
                if !ctx.standby.is_empty() {
                    node_group.use_providers = Some(ctx.standby.to_vec());
                }
                if !ctx.proxies_standby_name.is_empty() {
                    node_group.proxies = Some(ctx.proxies_standby_name.to_vec());
                }
            } else {
                if !ctx.subscriptions.is_empty() {
                    node_group.use_providers = Some(ctx.subscriptions.to_vec());
                }
                if !ctx.proxies_name.is_empty() {
                    node_group.proxies = Some(ctx.proxies_name.to_vec());
                }
            }
        }
    }

    // Add health-check fields for types that need them
    if group_type.needs_health_check() {
        node_group.url = Some(ctx.template.test_url.clone());
        node_group.interval = Some(60);
        node_group.tolerance = Some(50);
    }

    // Load-balance gets a strategy
    if group_type == NodeGroupType::LoadBalance {
        node_group.strategy = Some("consistent-hashing".into());
    }

    Some(ProxyGroup {
        name: group_def.name.clone(),
        kind: ProxyGroupKind::NodeGroup(node_group),
    })
}

// ── Post-processing helpers ─────────────────────────────────────────────────

/// Remove discarded group names from the root group's proxies list.
fn remove_discarded_from_root(groups: &mut [ProxyGroup], discarded: &[String]) {
    let Some(root) = groups.first_mut() else {
        return;
    };
    let ProxyGroupKind::RuleSelect(ref mut rs) = root.kind else {
        return;
    };
    rs.proxies.retain(|p| !discarded.contains(p));
}

/// Collect all valid proxy/group names that can be referenced.
fn collect_valid_names(groups: &[ProxyGroup], ctx: &GroupContext) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    names.insert("DIRECT".into());
    names.insert("REJECT".into());

    for g in groups {
        names.insert(g.name.clone());
    }

    for p in ctx.proxies_standby_name {
        names.insert(p.clone());
    }

    names
}

/// Validate that all proxy references in node groups point to existing names.
///
/// Invalid references are silently removed. This mirrors the Python behavior
/// where `proxygroup["proxies"]` is filtered against `proxyGroupAndProxyList`.
fn validate_group_references(groups: &mut [ProxyGroup], valid_names: &HashSet<String>) {
    for group in groups.iter_mut() {
        match group.kind {
            ProxyGroupKind::NodeGroup(ref mut ng) => {
                if let Some(ref mut proxies) = ng.proxies {
                    proxies.retain(|p| valid_names.contains(p));
                }
            }
            ProxyGroupKind::RuleSelect(ref mut rs) => {
                rs.proxies.retain(|p| valid_names.contains(p));
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_template(groups: Vec<Group>) -> TemplateConfig {
        TemplateConfig {
            head: serde_yaml::Value::Null,
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
    fn root_group_contains_non_rule_groups_and_direct() {
        let template = make_template(vec![
            make_group("HK", NodeGroupType::UrlTest, false, false, None, Some("HK")),
            make_group("Ad", NodeGroupType::Select, true, false, Some("REJECT"), None),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into(), "HK-2".into()],
        };

        let (groups, _) = build_all_groups(&ctx);

        // Root group is first
        let root = &groups[0];
        assert_eq!(root.name, "🚀 节点选择");
        if let ProxyGroupKind::RuleSelect(ref rs) = root.kind {
            // Should contain "HK" (non-rule) and "DIRECT", but NOT "Ad" (rule)
            assert!(rs.proxies.contains(&"HK".to_string()));
            assert!(rs.proxies.contains(&"DIRECT".to_string()));
            assert!(!rs.proxies.contains(&"Ad".to_string()));
        } else {
            panic!("root group should be RuleSelect");
        }
    }

    #[test]
    fn rule_select_with_prior_direct() {
        let template = make_template(vec![
            make_group("Ad", NodeGroupType::Select, true, false, Some("DIRECT"), None),
        ]);
        let subs: Vec<String> = vec![];
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &[],
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &[],
        };

        let (groups, _) = build_all_groups(&ctx);
        let ad = groups.iter().find(|g| g.name == "Ad").unwrap();
        if let ProxyGroupKind::RuleSelect(ref rs) = ad.kind {
            assert_eq!(rs.proxies[0], "DIRECT");
            assert_eq!(rs.proxies[1], "REJECT");
        } else {
            panic!("Ad should be RuleSelect");
        }
    }

    #[test]
    fn rule_select_with_prior_reject() {
        let template = make_template(vec![
            make_group("Ad", NodeGroupType::Select, true, false, Some("REJECT"), None),
        ]);
        let subs: Vec<String> = vec![];
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &[],
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &[],
        };

        let (groups, _) = build_all_groups(&ctx);
        let ad = groups.iter().find(|g| g.name == "Ad").unwrap();
        if let ProxyGroupKind::RuleSelect(ref rs) = ad.kind {
            assert_eq!(rs.proxies[0], "REJECT");
            assert_eq!(rs.proxies[1], "DIRECT");
        } else {
            panic!("Ad should be RuleSelect");
        }
    }

    #[test]
    fn node_group_with_matching_regex_is_kept() {
        let template = make_template(vec![
            make_group("HK", NodeGroupType::UrlTest, false, false, None, Some("HK")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into(), "US-1".into()],
        };

        let (groups, discarded) = build_all_groups(&ctx);
        assert!(discarded.is_empty());
        let hk = groups.iter().find(|g| g.name == "HK").unwrap();
        if let ProxyGroupKind::NodeGroup(ref ng) = hk.kind {
            assert_eq!(ng.group_type, NodeGroupType::UrlTest);
            assert!(ng.filter.as_ref().unwrap().contains("HK"));
            assert!(ng.use_providers.is_some());
            assert!(ng.url.is_some()); // health-check
        } else {
            panic!("HK should be NodeGroup");
        }
    }

    #[test]
    fn node_group_with_no_matching_regex_kept_with_use_filter() {
        // When providers exist but no proxy names match the regex (partial fetch failure),
        // the group should be kept with `use` + `filter` so Mihomo can filter on refresh.
        let template = make_template(vec![
            make_group("JP", NodeGroupType::UrlTest, false, false, None, Some("JP")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into(), "US-1".into()],
        };

        let (groups, discarded) = build_all_groups(&ctx);
        assert!(discarded.is_empty());
        let jp = groups.iter().find(|g| g.name == "JP").unwrap();
        if let ProxyGroupKind::NodeGroup(ref ng) = jp.kind {
            // Group is kept with use + filter even though no names matched
            assert!(ng.use_providers.is_some());
            assert!(ng.filter.as_ref().unwrap().contains("JP"));
        } else {
            panic!("JP should be NodeGroup");
        }
    }

    #[test]
    fn node_group_with_no_providers_and_no_match_is_discarded() {
        // When there are NO providers and no matching standalone proxies,
        // the group is truly discarded — nothing to pull from.
        let template = make_template(vec![
            make_group("JP", NodeGroupType::UrlTest, false, false, None, Some("JP")),
        ]);
        let subs: Vec<String> = vec![];
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &[],
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into(), "US-1".into()],
        };

        let (groups, discarded) = build_all_groups(&ctx);
        assert!(discarded.contains(&"JP".to_string()));
        assert!(groups.iter().find(|g| g.name == "JP").is_none());
    }

    #[test]
    fn group_kept_with_use_filter_stays_in_root() {
        // When a group is kept (has providers, regex doesn't match current names),
        // it should still appear in the root group's proxies list.
        let template = make_template(vec![
            make_group("JP", NodeGroupType::UrlTest, false, false, None, Some("JP")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into()],
        };

        let (groups, _) = build_all_groups(&ctx);
        let root = &groups[0];
        if let ProxyGroupKind::RuleSelect(ref rs) = root.kind {
            // JP is kept (has provider), so it should appear in root
            assert!(rs.proxies.contains(&"JP".to_string()));
        }
    }

    #[test]
    fn load_balance_gets_strategy_and_health_check() {
        let template = make_template(vec![
            make_group("LB", NodeGroupType::LoadBalance, false, false, None, Some("HK")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into()],
        };

        let (groups, _) = build_all_groups(&ctx);
        let lb = groups.iter().find(|g| g.name == "LB").unwrap();
        if let ProxyGroupKind::NodeGroup(ref ng) = lb.kind {
            assert_eq!(ng.group_type, NodeGroupType::LoadBalance);
            assert_eq!(ng.strategy.as_deref(), Some("consistent-hashing"));
            assert!(ng.url.is_some());
            assert_eq!(ng.interval, Some(60));
            assert_eq!(ng.tolerance, Some(50));
        }
    }

    #[test]
    fn manual_mode_uses_standby_providers() {
        let template = make_template(vec![
            make_group("HK", NodeGroupType::Select, false, true, None, Some("HK")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = vec!["subscription0".into(), "subscriptionsub0".into()];
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &["HK-main".into()],
            proxies_standby_name: &["HK-standby".into()],
            provider_proxy_names: &["HK-1".into()],
        };

        let (groups, _) = build_all_groups(&ctx);
        let hk = groups.iter().find(|g| g.name == "HK").unwrap();
        if let ProxyGroupKind::NodeGroup(ref ng) = hk.kind {
            // Manual mode should use standby providers
            assert_eq!(
                ng.use_providers.as_deref(),
                Some(vec!["subscription0".to_string(), "subscriptionsub0".to_string()]).as_deref()
            );
            // And standby proxy names that match
            assert_eq!(
                ng.proxies.as_deref(),
                Some(vec!["HK-standby".to_string()]).as_deref()
            );
        }
    }

    #[test]
    fn no_regex_includes_all_providers() {
        let template = make_template(vec![
            make_group("All", NodeGroupType::Select, false, false, None, None),
        ]);
        let subs: Vec<String> = vec!["subscription0".into(), "subscription1".into()];
        let standby: Vec<String> = subs.clone();
        let proxies = vec!["p1".into(), "p2".into()];
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &proxies,
            proxies_standby_name: &proxies,
            provider_proxy_names: &[],
        };

        let (groups, _) = build_all_groups(&ctx);
        let all_group = groups.iter().find(|g| g.name == "All").unwrap();
        if let ProxyGroupKind::NodeGroup(ref ng) = all_group.kind {
            assert!(ng.use_providers.is_some());
            assert!(ng.proxies.is_some());
            assert!(ng.filter.is_none());
        }
    }

    #[test]
    fn empty_regex_discards_group() {
        let template = make_template(vec![
            make_group("Empty", NodeGroupType::UrlTest, false, false, None, Some("")),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            provider_proxy_names: &["HK-1".into()],
        };

        let (groups, discarded) = build_all_groups(&ctx);
        assert!(discarded.contains(&"Empty".to_string()));
        assert!(groups.iter().find(|g| g.name == "Empty").is_none());
    }

    #[test]
    fn rule_select_keeps_kept_node_group_refs() {
        // Template has a node group "AI" with regex and a rule-select group "Ad"
        // that references "AI". With providers available, AI is kept (use + filter)
        // even though no proxy names currently match, so "Ad" should still have "AI".
        let template = make_template(vec![
            make_group("AI", NodeGroupType::Select, false, false, None, Some("AI")),
            make_group("Ad", NodeGroupType::Select, true, false, Some("REJECT"), None),
        ]);
        let subs: Vec<String> = vec!["subscription0".into()];
        let standby: Vec<String> = subs.clone();
        let ctx = GroupContext {
            template: &template,
            subscriptions: &subs,
            standby: &standby,
            proxies_name: &[],
            proxies_standby_name: &[],
            // No AI nodes — only HK nodes
            provider_proxy_names: &["HK-1".into(), "HK-2".into()],
        };

        let (groups, discarded) = build_all_groups(&ctx);
        // AI is kept because providers exist (use + filter)
        assert!(discarded.is_empty());
        assert!(groups.iter().find(|g| g.name == "AI").is_some());

        // Ad should still contain "AI" in its proxies
        let ad = groups.iter().find(|g| g.name == "Ad").unwrap();
        if let ProxyGroupKind::RuleSelect(ref rs) = ad.kind {
            assert!(rs.proxies.contains(&"AI".to_string()));
            assert!(rs.proxies.contains(&"REJECT".to_string()));
            assert!(rs.proxies.contains(&"DIRECT".to_string()));
        } else {
            panic!("Ad should be RuleSelect");
        }
    }
}
