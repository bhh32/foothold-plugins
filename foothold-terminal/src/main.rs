mod settings;
mod terminal;

use crate::terminal::Terminal;
use fh_plugin::{log_to_stderr, run};

fn main() {
    log_to_stderr();
    run(Terminal::new());
}
