//! Proxy name deduplication registry.
//!
//! When multiple proxies share the same name, the registry appends a
//! zero-padded numeric suffix to keep names unique (e.g. "HK-01", "HK-02").

use std::collections::HashMap;

/// Tracks proxy names to ensure uniqueness within a subscription.
#[derive(Debug, Clone, Default)]
pub struct NameRegistry {
    names: HashMap<String, u32>,
}

impl NameRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a proxy name and return a unique version.
    ///
    /// The first occurrence returns the name unchanged.
    /// Subsequent occurrences return `{name}-{index:02d}`.
    pub fn register(&mut self, name: &str) -> String {
        if let Some(count) = self.names.get_mut(name) {
            if *count == 0 {
                *count = 1;
                return name.to_string();
            }
            let suffix = *count;
            *count += 1;
            return format!("{name}-{suffix:02}");
        }
        // Key absent: first occurrence, must insert.
        self.names.insert(name.to_string(), 1);
        name.to_string()
    }

    /// Clear all registered names.
    pub fn clear(&mut self) {
        self.names.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_occurrence_unchanged() {
        let mut reg = NameRegistry::new();
        assert_eq!(reg.register("HK"), "HK");
    }

    #[test]
    fn duplicates_get_suffixed() {
        let mut reg = NameRegistry::new();
        assert_eq!(reg.register("HK"), "HK");
        assert_eq!(reg.register("HK"), "HK-01");
        assert_eq!(reg.register("HK"), "HK-02");
    }

    #[test]
    fn different_names_independent() {
        let mut reg = NameRegistry::new();
        assert_eq!(reg.register("HK"), "HK");
        assert_eq!(reg.register("JP"), "JP");
        assert_eq!(reg.register("HK"), "HK-01");
        assert_eq!(reg.register("JP"), "JP-01");
    }

    #[test]
    fn clear_resets_state() {
        let mut reg = NameRegistry::new();
        reg.register("HK");
        reg.register("HK");
        reg.clear();
        assert_eq!(reg.register("HK"), "HK");
    }

    #[test]
    fn empty_name_handled() {
        let mut reg = NameRegistry::new();
        assert_eq!(reg.register(""), "");
        assert_eq!(reg.register(""), "-01");
    }
}
