use crate::settings::{Hold, Settings};
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_plugin::Source;
use std::{
    env,
    process::{Command, Stdio},
    thread,
};

// $1 is the user's command
const RUN: &str = r#"eval "$1""#;
const HOLD_ON_ERROR: &str = r#"eval "$1"; status=$?; [ "$status" -eq 0 ] || { printf '\n[exited %d] Press Enter to close.' "$status"; read -r _; }"#;
const HOLD_ALWAYS: &str =
    r#"eval "$1"; printf '\n[exited %d] Press Enter to close.' "$?"; read -r _"#;

// Known terminals
const KNOWN: &[(&str, &[&str])] = &[
    ("foot", &["-e"]),
    ("alacritty", &["-e"]),
    ("kitty", &["-e"]),
    ("ghostty", &["-e"]),
    ("wezterm", &["start", "--"]),
    ("cosmic-term", &["-e"]),
    ("konsole", &["-e"]),
    ("gnome-terminal", &["--"]),
    ("ptyxis", &["--"]),
    ("xfce4-terminal", &["-x"]),
    ("x-terminal-emulator", &["-e"]),
    ("xterm", &["-e"]),
];

pub struct Emulator {
    pub program: String,
    pub separator: Vec<String>,
}

pub struct Terminal {
    emulator: Option<Emulator>,
    outcome: Option<String>,
}

impl Terminal {
    pub fn new() -> Self {
        let emulator = discover(&Self::config().command);

        if emulator.is_none() {
            eprintln!("no terminal found; set command in terminal.toml");
        }

        Self {
            emulator,
            outcome: None,
        }
    }

    // The launcher removes the routing prefix, so whatever arrives is the
    // command.
    fn command(query: &str) -> Option<&str> {
        let query = query.trim();

        (!query.is_empty()).then_some(query)
    }

    fn script(&self) -> &'static str {
        match Self::config().hold {
            Hold::OnError => HOLD_ON_ERROR,
            Hold::Always => HOLD_ALWAYS,
            Hold::Never => RUN,
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for Terminal {
    const NAME: &'static str = "terminal";
    type Settings = Settings;

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        let Some(command) = Self::command(query) else {
            self.outcome = None;
            return Vec::new();
        };
        self.outcome = Some(command.to_owned());
        let description = match &self.emulator {
            Some(emulator) => format!("Run in {}", emulator.program),
            None => "No terminal found".to_owned(),
        };

        vec![PluginSearchResult {
            id: 0,
            name: command.to_owned(),
            description,
            keywords: None,
            icon: Some(IconSource::Name("utilities-terminal".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        let (Some(command), Some(emulator)) = (self.outcome.as_deref(), self.emulator.as_ref())
        else {
            return Vec::new();
        };
        let spawned = Command::new(&emulator.program)
            .args(&emulator.separator)
            .args(["sh", "-c", self.script(), "sh", command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match spawned {
            Ok(mut child) => {
                thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => eprintln!("could not start {}: {e}", emulator.program),
        }

        vec![PluginResponse::Close]
    }
}

fn discover(configured: &str) -> Option<Emulator> {
    if !configured.is_empty() {
        return Some(Emulator {
            program: configured.to_owned(),
            separator: vec!["-e".to_owned()],
        });
    }

    if in_path("xdg-terminal-exec") {
        return Some(Emulator {
            program: "xdg-terminal-exec".to_owned(),
            separator: Vec::new(),
        });
    }

    if let Some(program) = env::var("TERMINAL")
        .ok()
        .filter(|program| !program.is_empty())
    {
        let separator = separator_for(&program);
        return Some(Emulator { program, separator });
    }

    KNOWN
        .iter()
        .find(|(program, _)| in_path(program))
        .map(|(program, separator)| Emulator {
            program: (*program).to_owned(),
            separator: separator.iter().map(|arg| (*arg).to_owned()).collect(),
        })
}

fn separator_for(program: &str) -> Vec<String> {
    KNOWN
        .iter()
        .find(|(known, _)| *known == program)
        .map(|(_, separator)| separator.iter().map(|arg| (*arg).into()).collect())
        .unwrap_or_else(|| vec!["-e".into()])
}

fn in_path(program: &str) -> bool {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|dir| dir.join(program).is_file()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{KNOWN, Terminal, discover, separator_for};

    #[test]
    fn the_query_is_the_command() {
        assert_eq!(Terminal::command("htop"), Some("htop"));
    }

    #[test]
    fn arguments_survive() {
        assert_eq!(
            Terminal::command("journalctl --user -f"),
            Some("journalctl --user -f")
        );
    }

    #[test]
    fn an_empty_query_is_no_command() {
        assert_eq!(Terminal::command(""), None);
        assert_eq!(Terminal::command("   "), None);
    }

    #[test]
    fn every_known_term_with_separator() {
        for (program, separator) in KNOWN {
            assert!(!separator.is_empty(), "{program} has no separator");
        }
    }

    #[test]
    fn configured_term_overrides_the_probe() {
        let emulator = discover("myterm").expect("configured terminal is used as given");

        assert_eq!(emulator.program, "myterm");
        assert_eq!(emulator.separator, vec!["-e"]);
    }

    #[test]
    fn known_program_keeps_its_separator() {
        assert_eq!(separator_for("gnome-terminal"), vec!["--"]);
        assert_eq!(separator_for("wezterm"), vec!["start", "--"]);
        assert_eq!(separator_for("cosmic-term"), vec!["-e"]);
    }

    #[test]
    fn unknown_program_gets_default_separator() {
        assert_eq!(separator_for("something-new"), vec!["-e"]);
    }
}
