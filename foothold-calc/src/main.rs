mod calculator;
mod error;
mod parser;
mod token;

use crate::calculator::Calculator;
use fh_plugin::{log_to_stderr, run};

fn main() {
    log_to_stderr();
    run(Calculator::default());
}
