use crate::settings::Settings;
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_paths::expand;
use fh_plugin::{Source, detached};
use ignore::{WalkBuilder, WalkState};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

const MAX_RESULTS: usize = 8;
const MAX_CANDIDATES: usize = 200;
const MIN_TERM: usize = 2;

pub struct Find {
    matcher: Matcher,
    buffer: Vec<char>,
    matches: Vec<PathBuf>,
}

impl Find {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            buffer: Vec::new(),
            matches: Vec::new(),
        }
    }

    fn term(query: &str) -> Option<&str> {
        let query = query.trim();
        (query.chars().count() >= MIN_TERM).then_some(query)
    }

    fn candidates(term: &str) -> Vec<PathBuf> {
        let settings = Self::config();
        let Some((first, rest)) = settings.roots.split_first() else {
            return Vec::new();
        };
        let mut builder = WalkBuilder::new(expand(first));

        rest.iter().for_each(|root| {
            builder.add(expand(root));
        });

        builder
            .hidden(true)
            .git_ignore(true)
            .max_depth(Some(settings.max_depth))
            .threads(0);

        let found = Arc::new(Mutex::new(Vec::new()));
        let needle = term.to_lowercase();

        builder.build_parallel().run(|| {
            let found = Arc::clone(&found);
            let needle = needle.clone();

            Box::new(move |entry| {
                let Ok(entry) = entry else {
                    return WalkState::Continue;
                };

                if !entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&needle)
                {
                    return WalkState::Continue;
                }

                let mut found = found.lock().expect("a walker thread panicked");
                if found.len() >= MAX_CANDIDATES {
                    return WalkState::Quit;
                }

                found.push(entry.path().to_path_buf());
                WalkState::Continue
            })
        });

        Arc::try_unwrap(found)
            .map(|found| found.into_inner().expect("walker thread panicked"))
            .unwrap_or_default()
    }
}

impl Default for Find {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for Find {
    const NAME: &'static str = "find";
    type Settings = Settings;

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.matches.clear();

        let Some(term) = Self::term(query) else {
            return Vec::new();
        };
        let candidates = Self::candidates(term);
        let pattern = Pattern::parse(term, CaseMatching::Ignore, Normalization::Smart);
        let Self {
            matcher, buffer, ..
        } = self;
        let mut scored: Vec<(u32, PathBuf)> = candidates
            .into_iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().into_owned();
                let score = pattern.score(Utf32Str::new(&name, buffer), matcher)?;

                Some((score, path))
            })
            .collect();

        scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        scored.truncate(MAX_RESULTS);
        self.matches = scored.into_iter().map(|(_, path)| path).collect();
        self.matches
            .iter()
            .enumerate()
            .map(|(id, path)| PluginSearchResult {
                id: id as Indice,
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                description: path
                    .parent()
                    .map(|parent| parent.to_string_lossy().into())
                    .unwrap_or_default(),
                keywords: None,
                icon: Some(IconSource::Name("text-x-generic".into())),
                exec: None,
                window: None,
            })
            .collect()
    }

    fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(path) = self.matches.get(id as usize) else {
            return Vec::new();
        };
        detached("xdg-open", &[&path.to_string_lossy()]);
        vec![PluginResponse::Close]
    }
}

#[cfg(test)]
mod tests {
    use super::Find;

    #[test]
    fn short_terms_dont_walk() {
        assert!(Find::term("a").is_none());
        assert!(Find::term("ab").is_some());
    }

    #[test]
    fn whole_query_is_term() {
        assert_eq!(Find::term("report.odt"), Some("report.odt"));
        assert_eq!(Find::term("  two words  "), Some("two words"));
    }
}
