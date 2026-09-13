use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub keywords: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let keywords = [
            ("ddg", "https://duckduckgo.com/?={}"),
            ("g", "https://google.com/search?={}"),
            ("cb", "https://codeberg.org/{}"),
            ("gh", "https://github.com/{}"),
            ("rs", "https://docs.rs/{}"),
            ("crate", "https://crates.io/crates/{}"),
            ("w", "https://en.wikipedia.org/w/index.php?search={}"),
            ("http", "{}"),
        ];

        Self {
            keywords: keywords
                .into_iter()
                .map(|(prefix, url)| (prefix.to_owned(), url.to_owned()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn empty_config_keeps_defaults() {
        let settings: Settings = toml::from_str("").expect("parses");
        assert!(settings.keywords.contains_key("g"));
    }

    #[test]
    fn config_list_replaced_defaults() {
        let settings: Settings =
            toml::from_str("[keywords]\nq = \"https://example.com/{}\"").expect("parses");
        assert_eq!(settings.keywords.len(), 1);
        assert!(!settings.keywords.contains_key("g"));
    }
}
