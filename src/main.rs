use crate::opt::OutputType;
use failure::Error;

mod normal;
mod opt;
mod playlist;

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
        OutputType::Normal => crate::normal::run(),
        OutputType::Playlist(path) => crate::playlist::run(path),
    }
}

/// Log any errors and causes
pub fn log_error(e: &Error) {
    eprintln!("ERROR: {}", e);
    for cause in e.iter_causes() {
        eprintln!("Caused by: {}", cause);
    }
}
