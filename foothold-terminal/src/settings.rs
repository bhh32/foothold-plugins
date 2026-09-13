use serde::Deserialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hold {
    // Keep the window open when a command fails so it's visible
    #[default]
    OnError,
    Always,
    Never,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Empty means $TERMINAL
    pub command: String,
    pub hold: Hold,
}

#[cfg(test)]
mod tests {
    use super::{Hold, Settings};

    #[test]
    fn an_empty_file_holds_on_error() {
        let settings: Settings = toml::from_str("").expect("parses");
        assert_eq!(settings.hold, Hold::OnError);
        assert!(settings.command.is_empty());
    }

    #[test]
    fn hold_is_kabab_case() {
        let mut settings: Settings = toml::from_str("hold = \"on-error\"").expect("parses");
        assert!(settings.hold == Hold::OnError);

        settings = toml::from_str("hold = \"never\"").expect("parses");
        assert!(settings.hold == Hold::Never)
    }
}
