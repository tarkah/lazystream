use chrono::{format::ParseError, NaiveDate};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "lazystream",
    about = "Easily get LazyMan stream links, output directly or to m3u / xmltv formats.",
    version = "1.4.0",
    author = "tarkah <admin@tarkah.dev>"
)]
pub struct Opt {
    #[structopt(long, parse(try_from_str = parse_date), name = "YYYYMMDD")]
    /// Specify what date to generate stream links for, defaults to today
    pub date: Option<NaiveDate>,
    #[structopt(long, parse(from_os_str))]
    /// Generate a .m3u playlist file for all games
    pub playlist_output: Option<PathBuf>,
    #[structopt(long, parse(from_os_str))]
    /// Generate a .xml XMLTV file for all games with corresponding .m3u playlist file
    pub xmltv_output: Option<PathBuf>,
    #[structopt(long, default_value = "1000")]
    /// Specify the starting channel number for the XMLVTV output
    pub xmltv_start_channel: u32,
}

pub fn parse_opts() -> OutputType {
    let opts = Opt::from_args();

    if opts.playlist_output.is_some() || opts.xmltv_output.is_some() {
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
