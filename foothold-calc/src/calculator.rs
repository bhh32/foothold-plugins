use crate::{parser::evaluate, token::tokenize};
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_plugin::Source;

#[derive(Default)]
pub struct Calculator {
    outcome: Option<String>,
}

impl Source for Calculator {
    const NAME: &'static str = "calculator";
    type Settings = ();

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.outcome = None;

        let query = query.trim();
        let (expression, explicit) = match query.strip_prefix('=') {
            Some(rest) => (rest.trim(), true),
            None => (query, false),
        };
        let Ok(tokens) = tokenize(expression) else {
            return Vec::new();
        };

        // A lone number is not a calculation, give apps instead
        if !explicit && tokens.len() < 2 {
            return Vec::new();
        }

        let Ok(value) = evaluate(&tokens) else {
            return Vec::new();
        };

        // Division by zero and the like, either infinity or NaN
        if !value.is_finite() {
            return Vec::new();
        }

        let formatted = format_value(value);
        self.outcome = Some(formatted.clone());

        vec![PluginSearchResult {
            id: 0,
            name: formatted,
            description: expression.to_owned(),
            keywords: None,
            icon: Some(IconSource::Name("accessories-calculator".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        // Fill without close: the answer replaces the expression and the launcher
        // stays open
        match self.outcome.as_deref() {
            Some(value) => vec![PluginResponse::Fill(format!("= {value}"))],
            None => Vec::new(),
        }
    }
}

fn format_value(value: f64) -> String {
    if value.abs() < 1e15 {
        let rounded = (value * 1e10).round() / 1e10;
        format!("{rounded}")
    } else {
        format!("{value}")
    }
}
