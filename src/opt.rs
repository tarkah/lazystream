use chrono::{format::ParseError, NaiveDate};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "lazystream",
    about = "Easily get stream links for the current days NHL schedule.",
    version = "1.3.0",
    author = "tarkah <admin@tarkah.dev>"
)]
pub struct Opt {
    #[structopt(long, parse(from_os_str), name = "FILE")]
    /// Generate a .m3u playlist with all games currently playing
    pub playlist_output: Option<PathBuf>,
    #[structopt(long, parse(try_from_str = parse_date), name = "YYYYMMDD")]
    /// Specify what date to generate stream links for, defaults to today
    pub date: Option<NaiveDate>,
}

pub fn parse_opts() -> OutputType {
    let opts = Opt::from_args();

    if opts.playlist_output.is_some() {
        return OutputType::Playlist(opts);
    }

    OutputType::Normal(opts)
}

pub enum OutputType {
    Playlist(Opt),
    Normal(Opt),
}

fn parse_date(src: &str) -> Result<NaiveDate, ParseError> {
    let s = src.replace("-", "");
    NaiveDate::parse_from_str(&s, "%Y%m%d")
}
