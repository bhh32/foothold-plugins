mod settings;
mod web;

use crate::web::Web;
use fh_plugin::{log_to_stderr, run};

fn main() {
    log_to_stderr();
    run(Web::default());
}
