use crate::opt::OutputType;
use colored::Colorize;
use failure::Error;

mod generate;
mod opt;
mod record;
mod select;
mod stream;

const VERSION: &str = "1.5.1";
const HOST: &str = "http://nhl.freegamez.ga";
const BANNER: &str = r#"
 |        \   __  /\ \   / ___|__ __|  _ \  ____|    \     \  | 
 |       _ \     /  \   /\___ \   |   |   | __|     _ \   |\/ | 
 |      ___ \   /      |       |  |   __ <  |      ___ \  |   | 
_____|_/    _\____|   _| _____/  _|  _| \_\_____|_/    _\_|  _| 
"#;

fn main() {
    let output_type = crate::opt::parse_opts();

    match output_type {
        OutputType::Select(opts) => crate::select::run(opts),
        OutputType::Generate(opts) => crate::generate::run(opts),
        OutputType::Record(opts) => crate::record::run(opts),
    }
}

/// Log any errors and causes
pub fn log_error(e: &Error) {
    let error_colored = "ERROR".red();
    eprintln!("\n{}: {}", error_colored, e);
    for cause in e.iter_causes() {
        let caused_colored = "Caused by:".yellow();
        eprintln!("\n{} {}", caused_colored, cause);
    }
}
