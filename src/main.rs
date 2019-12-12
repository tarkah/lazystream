use crate::opt::OutputType;
use failure::Error;

mod normal;
mod opt;
mod playlist;
mod stream;

const VERSION: &str = "1.4.3";
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
        OutputType::Normal(opts) => crate::normal::run(opts),
        OutputType::Playlist(opts) => crate::playlist::run(opts),
    }
}

/// Log any errors and causes
pub fn log_error(e: &Error) {
    eprintln!("\nERROR: {}", e);
    for cause in e.iter_causes() {
        eprintln!("Caused by: {}", cause);
    }
}
