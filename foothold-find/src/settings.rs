use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub roots: Vec<String>,
    pub max_depth: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: vec!["~".into()],
            max_depth: 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn empty_is_home() {
        let settings: Settings = toml::from_str("").expect("parses");
        assert_eq!(settings.max_depth, 6);
        assert_eq!(settings.roots, vec!["~"]);
    }

    #[test]
    fn key_leaves_others_alone() {
        let settings: Settings = toml::from_str("max_depth = 3").expect("parses");
        assert_eq!(settings.max_depth, 3);
        assert_eq!(settings.roots, vec!["~"]);
    }
}
