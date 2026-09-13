use crate::settings::Settings;
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult, Usage};
use fh_plugin::{Source, detached};

#[derive(Default)]
pub struct Web {
    outcome: Option<String>,
}

impl Web {
    fn split(query: &str, settings: &Settings) -> Option<(String, String)> {
        let (prefix, terms) = query.trim().split_once(char::is_whitespace)?;
        let terms = terms.trim();

        if terms.is_empty() {
            return None;
        }

        let template = settings.keywords.get(prefix)?;

        Some((template.clone(), terms.to_owned()))
    }
}

impl Source for Web {
    const NAME: &'static str = "web";
    type Settings = Settings;

    fn isolates(&mut self, query: &str) -> bool {
        Self::split(query, &Self::config()).is_some()
    }

    fn usage(&mut self) -> Vec<Usage> {
        Self::config()
            .keywords
            .iter()
            .map(|(prefix, template)| {
                let host = host_of(template);
                let searches = template
                    .split("{}")
                    .next()
                    .is_some_and(|head| head.contains('?'));
                let example = match (&host, searches) {
                    (None, _) => format!("{prefix} example.com"),
                    (Some(_), true) => format!("{prefix} <terms>"),
                    (Some(_), false) => format!("{prefix} <path>"),
                };

                Usage {
                    prefix: prefix.clone(),
                    example,
                    description: host.unwrap_or_else(|| "Open a URL".to_owned()),
                }
            })
            .collect()
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        let Some((template, terms)) = Self::split(query, &Self::config()) else {
            self.outcome = None;
            return Vec::new();
        };
        let url = template.replace("{}", &encode(&terms));

        self.outcome = Some(url.clone());
        vec![PluginSearchResult {
            id: 0,
            name: terms,
            description: url,
            keywords: None,
            icon: Some(IconSource::Name("internet-web-browser".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        let Some(url) = self.outcome.as_deref() else {
            return Vec::new();
        };

        detached("xdg-open", &[url]);
        vec![PluginResponse::Close]
    }
}

// Helpers

fn encode(terms: &str) -> String {
    let mut encoded = String::with_capacity(terms.len());

    terms.bytes().for_each(|byte| match byte {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
            encoded.push(byte as char);
        }
        b' ' => encoded.push('+'),
        _ => encoded.push_str(&format!("%{byte:02X}")),
    });
    encoded
}

fn host_of(template: &str) -> Option<String> {
    let rest = template.split_once("://")?.1;
    let host = rest.split('/').next()?;

    (!host.is_empty() && !host.contains('{')).then(|| host.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Web, encode};
    use fh_plugin::Source;

    #[test]
    fn usage_distinguishes_searches_from_paths() {
        let usage = Web::default().usage();
        let google = usage
            .iter()
            .find(|url| url.prefix == "g")
            .expect("g exists");
        assert_eq!(google.example, "g <terms>");
        assert_eq!(google.description, "google.com");

        let github = usage
            .iter()
            .find(|url| url.prefix == "gh")
            .expect("gh exists");
        assert_eq!(github.example, "gh <path>");

        let raw = usage
            .iter()
            .find(|url| url.prefix == "http")
            .expect("http exists");
        assert_eq!(raw.description, "Open a URL");
    }

    #[test]
    fn spaces_become_plus() {
        assert_eq!(encode("rust traits"), "rust+traits");
    }

    #[test]
    fn specials_escaped() {
        assert_eq!(encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode("café"), "caf%C3%A9");
    }

    #[test]
    fn slashes_survive_encoding() {
        assert_eq!(encode("bhh32/wifi"), "bhh32/wifi");
        assert_eq!(encode("a b/c"), "a+b/c");
    }
}
