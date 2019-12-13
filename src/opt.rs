use crate::VERSION;
use chrono::{format::ParseError, NaiveDate};
use failure::{bail, Error};
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "lazystream",
    about = "Easily get LazyMan stream links, output directly or to m3u / xmltv formats.",
    version = VERSION,
    author = "tarkah <admin@tarkah.dev>"
)]
pub struct Opt {
    #[structopt(long, parse(try_from_str = parse_date), name = "YYYYMMDD")]
    /// Specify what date to generate stream links for, defaults to today
    pub date: Option<NaiveDate>,
    #[structopt(long, parse(try_from_str), default_value = Cdn::Akc.into())]
    /// Specify which CDN to use: 'akc' or 'l3c'
    pub cdn: Cdn,
    #[structopt(long, parse(try_from_str))]
    /// Specify a quality to use, otherwise stream will be adaptive.
    ///
    /// Must be one of: '720p60', '720p', '540p', '504p', '360p', '288p', '224p', '216p'
    pub quality: Option<Quality>,
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

#[derive(Debug)]
pub enum Cdn {
    Akc,
    L3c,
}

impl From<Cdn> for &str {
    fn from(cdn: Cdn) -> &'static str {
        match cdn {
            Cdn::Akc => "akc",
            Cdn::L3c => "l3c",
        }
    }
}

impl FromStr for Cdn {
    type Err = Error;

    fn from_str(s: &str) -> Result<Cdn, Error> {
        match s {
            "akc" => Ok(Cdn::Akc),
            "l3c" => Ok(Cdn::L3c),
            _ => bail!("Option must match 'akc' or 'l3c'"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Quality {
    _720p60,
    _720p,
    _540p,
    _504p,
    _360p,
    _288p,
    _224p,
    _216p,
}

impl From<Quality> for &str {
    fn from(quality: Quality) -> &'static str {
        match quality {
            Quality::_720p60 => "72060",
            Quality::_720p => "720",
            Quality::_540p => "540",
            Quality::_504p => "504",
            Quality::_360p => "360",
            Quality::_288p => "288",
            Quality::_224p => "224",
            Quality::_216p => "216",
        }
    }
}

impl FromStr for Quality {
    type Err = Error;

    fn from_str(s: &str) -> Result<Quality, Error> {
        match s {
            "720p60" => Ok(Quality::_720p60),
            "720p" => Ok(Quality::_720p),
            "540p" => Ok(Quality::_540p),
            "504p" => Ok(Quality::_504p),
            "360p" => Ok(Quality::_360p),
            "288p" => Ok(Quality::_288p),
            "224p" => Ok(Quality::_224p),
            "216p" => Ok(Quality::_216p),
            _ => bail!(
                "\n\nMust be one of: '720p60', '720p', '540p', '504p', '360p', '288p', '224p', '216p'"
            ),
        }
    }
}
