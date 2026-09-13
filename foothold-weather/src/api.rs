use crate::{
    regions::{country_code_for, state_in},
    settings::Units,
};
use reqwest::blocking::Client;
use serde::{Deserialize, de::DeserializeOwned};
use std::{cmp::Reverse, thread, time::Duration};
use tracing::warn;

const GEOCODE: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const AGENT: &str = concat!("foothold-weather/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub admin1: String,
    #[serde(default)]
    pub country: String,
}

impl Place {
    pub fn label(&self) -> String {
        [
            self.name.as_str(),
            self.admin1.as_str(),
            self.country.as_str(),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Geocoding {
    results: Vec<Place>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Current {
    pub temperature_2m: f64,
    pub apparent_temperature: f64,
    pub weather_code: u16,
    pub wind_speed_10m: f64,
}

impl Current {
    pub fn summary(&self, units: Units) -> String {
        let (description, _) = condition(self.weather_code);
        let symbol = units.symbol();

        format!(
            "{description}, {:.0}{symbol}, feels like {:.0}{symbol}, wind {:0} {}",
            self.temperature_2m,
            self.apparent_temperature,
            self.wind_speed_10m,
            units.wind(),
        )
    }

    pub fn icon(&self) -> &'static str {
        condition(self.weather_code).1
    }
}

#[derive(Debug, Deserialize)]
struct Forecast {
    current: Current,
}

fn condition(code: u16) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear sky", "weather-clear"),
        1 => ("Mainly clear", "weather-few-clouds"),
        2 => ("Partly cloudy", "weather-few-clouds"),
        3 => ("Overcast", "weather-overcast"),
        45 | 48 => ("Fog", "weather-fog"),
        51 | 53 | 55 => ("Drizzle", "weather-showers-scattered"),
        56 | 57 => ("Freezing drizzle", "weather-showers-scattered"),
        61 | 63 | 65 => ("Rain", "weather-showers"),
        66 | 67 => ("Freezing rain", "weather-showers"),
        71 | 73 | 75 => ("Snow", "weather-snow"),
        77 => ("Snow grains", "weather-snow"),
        (80..=82) => ("Rain showers", "weather-showers"),
        85 | 86 => ("Snow showers", "weather-snow"),
        95 => ("Thunderstorm", "weather-storm"),
        96 | 99 => ("Thunderstorm with hail", "weather-storm"),
        _ => ("Unknown", "weather-service-alert"),
    }
}

pub fn split_place(query: &str) -> (&str, Vec<&str>) {
    let mut parts = query
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let name = parts.next().unwrap_or_default();

    (name, parts.collect())
}

fn matches(place: &Place, hint: &str) -> bool {
    if place.admin1.eq_ignore_ascii_case(hint) || place.country.eq_ignore_ascii_case(hint) {
        return true;
    } else if let Some(country) = country_code_for(hint)
        && place.country.eq_ignore_ascii_case(country)
    {
        return true;
    }

    state_in(&place.country, hint).is_some_and(|state| place.admin1.eq_ignore_ascii_case(state))
}

pub fn pick(places: Vec<Place>, hints: &[&str]) -> Option<Place> {
    let best = places.iter().enumerate().max_by_key(|(idx, place)| {
        let score = hints.iter().filter(|hint| matches(place, hint)).count();

        (score, Reverse(*idx))
    })?;
    places.clone().into_iter().nth(best.0)
}

pub fn client() -> Client {
    Client::builder()
        .user_agent(AGENT)
        .timeout(TIMEOUT)
        .build()
        .unwrap_or_else(|_| Client::new())
}

pub fn geocode(client: &Client, name: &str) -> Vec<Place> {
    get::<Geocoding>(
        client,
        GEOCODE,
        &[
            ("name", name),
            ("count", "5"),
            ("language", "en"),
            ("format", "json"),
        ],
    )
    .map(|response| response.results)
    .unwrap_or_default()
}

pub fn current(client: &Client, place: &Place, units: Units) -> Option<Current> {
    let latitude = place.latitude.to_string();
    let longitude = place.longitude.to_string();

    get::<Forecast>(
        client,
        FORECAST,
        &[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            (
                "current",
                "temperature_2m,apparent_temperature,weather_code,wind_speed_10m",
            ),
            ("temperature_unit", units.temperature()),
            ("wind_speed_unit", units.wind()),
        ],
    )
    .map(|forecast| forecast.current)
}

fn get<T: DeserializeOwned>(client: &Client, url: &str, query: &[(&str, &str)]) -> Option<T> {
    let response = match client.get(url).query(query).send() {
        Ok(response) => response,
        Err(e) => {
            warn!(%e, url, "request failed");
            return None;
        }
    };
    let status = response.status();

    if !status.is_success() {
        let reason = response.text().unwrap_or_default();
        warn!(%status, url, %reason, "request rejected");
        return None;
    }

    match response.json() {
        Ok(value) => Some(value),
        Err(e) => {
            warn!(%e, url, "could not decode the response");
            None
        }
    }
}

pub fn warm(client: &Client) {
    let client = client.clone();
    thread::spawn(move || {
        let _ = client.head(GEOCODE).send();
        let _ = client.head(FORECAST).send();
    });
}

#[cfg(test)]
mod tests {
    use super::{Forecast, Geocoding, Place, condition, pick, split_place};
    use crate::settings::Units;

    fn place(name: &str, admin1: &str, country: &str) -> Place {
        Place {
            name: name.to_owned(),
            latitude: 0.0,
            longitude: 0.0,
            admin1: admin1.to_owned(),
            country: country.into(),
        }
    }

    #[test]
    fn bare_name_no_hints() {
        let (name, hints) = split_place("Seattle");
        assert_eq!(name, "Seattle");
        assert!(hints.is_empty());
    }

    #[test]
    fn commas_become_hints() {
        let (name, hints) = split_place("Seattle, WA, USA");
        assert_eq!(name, "Seattle");
        assert_eq!(hints, vec!["WA", "USA"]);
    }

    #[test]
    fn no_resuts_not_err() {
        let response: Geocoding =
            serde_json::from_str("{\"generationtime_ms\":0.1}").expect("parses");
        assert!(response.results.is_empty());
    }

    #[test]
    fn hint_picks_matching_region() {
        let places = vec![
            place("Springfield", "IL", "United States"),
            place("Springfield", "MO", "United States"),
        ];
        assert_eq!(pick(places, &["MO"]).expect("one matches").admin1, "MO");
    }

    #[test]
    fn unmatched_hint_falls_back_to_first() {
        let places = vec![place("Springfield", "IL", "US")];
        assert_eq!(pick(places, &["WA"]).expect("falls back").admin1, "IL");
    }

    #[test]
    fn label_skips_missing_parts() {
        assert_eq!(place("Seattle", "", "").label(), "Seattle");
        assert_eq!(place("Seattle", "WA", "USA").label(), "Seattle, WA, USA");
    }

    #[test]
    fn every_documented_code_mapped() {
        for code in [0, 1, 2, 3, 45, 51, 61, 66, 71, 77, 80, 85, 95, 96] {
            assert_ne!(condition(code).1, "weather-severe-alert");
        }
    }

    #[test]
    fn forecast_becomes_summary() {
        let forecast: Forecast = serde_json::from_str(
            r#"{"current":{"temperature_2m":61.2,"apparent_temperature":60.1,
                "weather_code":3,"wind_speed_10m":5.4}}"#,
        )
        .expect("parses");
        assert_eq!(
            forecast.current.summary(Units::Imperial),
            "Overcast, 61°F, feels like 60°F, wind 5.4 mph"
        );
    }

    #[test]
    fn every_unit_names_api_expectations() {
        assert_eq!(Units::Imperial.temperature(), "fahrenheit");
        assert_eq!(Units::Imperial.wind(), "mph");
        assert_eq!(Units::Metric.temperature(), "celsius");
        assert_eq!(Units::Metric.wind(), "kmh");
    }
}
