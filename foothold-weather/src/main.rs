mod api;
mod regions;
mod settings;
mod weather;

use crate::weather::Weather;
use fh_plugin::{log_to_stderr, run};

fn main() {
    log_to_stderr();
    run(Weather::new());
}
