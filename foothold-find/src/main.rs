mod find;
mod settings;

use crate::find::Find;
use fh_plugin::{log_to_stderr, run};

fn main() {
    log_to_stderr();
    run(Find::new());
}
