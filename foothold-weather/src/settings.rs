use serde::Deserialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Units {
    #[default]
    Imperial,
    Metric,
}

impl Units {
    // Open-meteo names thes in the query string
    pub fn temperature(self) -> &'static str {
        match self {
            Self::Imperial => "fahrenheit",
            Self::Metric => "celsius",
        }
    }

    pub fn wind(self) -> &'static str {
        match self {
            Self::Imperial => "mph",
            Self::Metric => "kmh",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Imperial => "°F",
            Self::Metric => "°C",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub units: Units,
    pub ttl_minutes: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            units: Units::Imperial,
            ttl_minutes: 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, Units};

    #[test]
    fn empty_file_is_imperial_and_15_min() {
        let settings: Settings = toml::from_str("").expect("parses");
        assert_eq!(settings.units, Units::Imperial);
        assert_eq!(settings.ttl_minutes, 15);
    }

    #[test]
    fn units_are_kebab_case() {
        let settings: Settings = toml::from_str("units = \"metric\"").expect("parses");
        assert_eq!(settings.units, Units::Metric);
        assert_eq!(settings.units.temperature(), "celsius");
    }
}
