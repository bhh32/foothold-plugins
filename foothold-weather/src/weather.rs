use crate::{
    api::{Current, Place, client, current, geocode, pick, split_place, warm},
    settings::{Settings, Units},
};
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_plugin::{Source, Waker};
use reqwest::blocking::Client;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread,
    time::{Duration, Instant},
};

const DEBOUNCE: Duration = Duration::from_millis(250);

struct Entry {
    place: Place,
    current: Current,
    fetched: Instant,
}

pub struct Weather {
    client: Client,
    cache: HashMap<String, Entry>,
    updates: Receiver<(String, Entry)>,
    sender: Sender<(String, Entry)>,
    waker: Option<Waker>,
    generation: Arc<AtomicU64>,
}

impl Weather {
    pub fn new() -> Self {
        let (sender, updates) = channel();
        let client = client();

        warm(&client);

        Self {
            client,
            cache: HashMap::new(),
            updates,
            sender,
            waker: None,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    fn drain(&mut self) {
        while let Ok((key, entry)) = self.updates.try_recv() {
            self.cache.insert(key, entry);
        }
    }

    fn is_fresh(entry: Option<&Entry>, ttl: Duration) -> bool {
        entry.is_some_and(|entry| entry.fetched.elapsed() < ttl)
    }

    fn fetch(&self, key: &str, known: Option<Place>, units: Units) {
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let latest = Arc::clone(&self.generation);
        let sender = self.sender.clone();
        let waker = self.waker.clone();
        let client = self.client.clone();
        let key = key.to_owned();

        thread::spawn(move || {
            thread::sleep(DEBOUNCE);

            if latest.load(Ordering::Relaxed) != generation {
                return;
            }
            let place = match known {
                Some(place) => place,
                None => {
                    let (name, hints) = split_place(&key);
                    let Some(found) = pick(geocode(&client, name), &hints) else {
                        return;
                    };
                    found
                }
            };
            let Some(current) = current(&client, &place, units) else {
                return;
            };
            let _ = sender.send((
                key,
                Entry {
                    place,
                    current,
                    fetched: Instant::now(),
                },
            ));
            if let Some(waker) = waker {
                waker.wake();
            }
        });
    }

    fn row(entry: &Entry, units: Units) -> PluginSearchResult {
        PluginSearchResult {
            id: 0,
            name: entry.place.label(),
            description: entry.current.summary(units),
            keywords: None,
            icon: Some(IconSource::Name(entry.current.icon().to_owned())),
            exec: None,
            window: None,
        }
    }

    fn pending(place: &str) -> PluginSearchResult {
        PluginSearchResult {
            id: 0,
            name: place.to_owned(),
            description: "Fetching the forecast...".to_owned(),
            keywords: None,
            icon: Some(IconSource::Name("view-refresh".to_owned())),
            exec: None,
            window: None,
        }
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for Weather {
    const NAME: &'static str = "weather";
    type Settings = Settings;

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        let key = query.trim().to_owned();

        if key.is_empty() {
            return Vec::new();
        }
        self.drain();

        let settings = Self::config();
        let ttl = Duration::from_secs(settings.ttl_minutes * 60);

        if !Self::is_fresh(self.cache.get(&key), ttl) {
            let known = self.cache.get(&key).map(|entry| entry.place.clone());
            self.fetch(&key, known, settings.units);
        }

        // A stale entry still shows while its replacement is on the way
        match self.cache.get(&key) {
            Some(entry) => vec![Self::row(entry, settings.units)],
            None => vec![Self::pending(&key)],
        }
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        vec![PluginResponse::Close]
    }

    fn connect(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, Weather};
    use crate::{
        api::{Current, Place},
        settings::Units,
    };
    use std::time::{Duration, Instant};

    fn entry(age: Duration) -> Entry {
        Entry {
            place: Place {
                name: "Seattle".to_owned(),
                latitude: 47.6,
                longitude: -122.3,
                admin1: "WA".to_owned(),
                country: "USA".to_owned(),
            },
            current: Current {
                temperature_2m: 61.2,
                apparent_temperature: 60.1,
                weather_code: 3,
                wind_speed_10m: 5.4,
            },
            fetched: Instant::now() - age,
        }
    }

    #[test]
    fn nothing_cached_not_fresh() {
        assert!(!Weather::is_fresh(None, Duration::from_secs(900)));
    }

    #[test]
    fn entry_past_ttl_not_fresh() {
        let ttl = Duration::from_secs(900);

        assert!(Weather::is_fresh(
            Some(&entry(Duration::from_secs(60))),
            ttl
        ));
        assert!(!Weather::is_fresh(
            Some(&entry(Duration::from_secs(1000))),
            ttl
        ));
    }

    #[test]
    fn cached_entry_becomes_row() {
        let row = Weather::row(&entry(Duration::ZERO), Units::Imperial);
        assert_eq!(row.name, "Seattle, WA, USA");
        assert!(row.description.starts_with("Overcast, 61°F"));
    }

    #[test]
    fn placeholder_names_place_waiting_on() {
        let row = Weather::pending("Seattle");
        assert_eq!(row.name, "Seattle");
        assert!(!row.description.is_empty());
    }
}
