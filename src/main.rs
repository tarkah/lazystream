use crate::opt::OutputType;
use colored::Colorize;

mod api;
mod completions;
mod generate;
mod opt;
mod select;
mod stream;
mod streamlink;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HOST: &str = "http://freesports.ddns.net";
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
        OutputType::Play(opts) => crate::streamlink::run(opts),
        OutputType::Record(opts) => crate::streamlink::run(opts),
        OutputType::Cast(opts) => crate::streamlink::run(opts),
        OutputType::Completions(opts) => crate::completions::run(opts),
    }
}

/// Log any errors and causes
pub fn log_error(e: &dyn failure::Fail) {
    let error_colored = "ERROR".red();
    eprintln!("\n{}: {}", error_colored, e);
    for cause in e.iter_causes() {
        let caused_colored = "Caused by:".yellow();
        eprintln!("\n{} {}", caused_colored, cause);
    }
}
