use crate::VERSION;
use chrono::{format::ParseError, NaiveDate};
use failure::{bail, Error};
use isahc::http::Uri;
use std::{path::PathBuf, str::FromStr};
use structopt::{clap::AppSettings::DeriveDisplayOrder, StructOpt};

pub fn parse_opts() -> OutputType {
    let opts = Opt::from_args();

    match opts.command {
        Command::Select { .. } => OutputType::Select(opts),
        Command::Generate { .. } => OutputType::Generate(opts),
        Command::Play { .. } => OutputType::Play(opts),
        Command::Record { .. } => OutputType::Record(opts),
        Command::Cast { .. } => OutputType::Cast(opts),
        Command::Completions { .. } => OutputType::Completions(opts),
    }
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(
    name = "lazystream",
    about = "Easily get LazyMan stream links, output directly or to m3u / xmltv formats. Streams can also be recorded or casted.",
    version = VERSION,
    author = "tarkah <admin@tarkah.dev>",
    setting = DeriveDisplayOrder,
)]
pub struct Opt {
    #[structopt(subcommand)]
    pub command: Command,
    #[structopt(long, parse(try_from_str), default_value = Sport::Nhl.into(), global = true, possible_values(&["mlb","nhl"]))]
    /// Specify which sport to get streams for
    pub sport: Sport,
    #[structopt(long, parse(try_from_str = parse_date), value_name = "YYYYMMDD", global = true)]
    /// Specify what date to use for games, defaults to today
    pub date: Option<NaiveDate>,
    #[structopt(long, parse(try_from_str), default_value = Cdn::Akc.into(), global = true, possible_values(&["akc","l3c"]))]
    /// Specify which CDN to use
    pub cdn: Cdn,
    #[structopt(long, parse(try_from_str), global = true, possible_values(&["720p60","720p","540p","504p","360p","288p","224p","216p"]))]
    /// Specify a quality to use, otherwise stream will be adaptive
    pub quality: Option<Quality>,
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
pub enum Command {
    #[structopt(usage = "lazystream select [--resolve] [OPTIONS]")]
    /// Select stream link via command line
    Select {
        #[structopt(long)]
        /// Resolve url to the actual hls link, if it's available
        resolve: bool,
    },
    #[structopt(usage = "lazystream generate <SUBCOMMAND> [OPTIONS]", setting = DeriveDisplayOrder)]
    /// Generate an xmltv and/or playlist formatted output for all games
    Generate {
        #[structopt(subcommand)]
        command: GenerateCommand,
    },
    #[structopt(usage = "lazystream play <SUBCOMMAND> [OPTIONS]", setting = DeriveDisplayOrder)]
    /// Play a game with VLC, requires StreamLink and VLC
    ///
    /// Game can be chosen from command line with 'select' subcommand or supplied
    /// in advanced with 'team' subcommand. A custom player can be used over VLC, see
    /// https://streamlink.github.io/players.html for a list of supported players.
    Play {
        #[structopt(subcommand)]
        command: PlayCommand,
    },
    #[structopt(usage = "lazystream record <SUBCOMMAND> [OPTIONS]", setting = DeriveDisplayOrder)]
    /// Record a game, requires StreamLink
    ///
    /// Game can be chosen from command line with 'select' subcommand or supplied
    /// in advanced with 'team' subcommand
    Record {
        #[structopt(subcommand)]
        command: RecordCommand,
    },
    #[structopt(usage = "lazystream cast <SUBCOMMAND> [OPTIONS]", setting = DeriveDisplayOrder)]
    /// Cast a game, requires StreamLink and VLC
    ///
    /// Game can be chosen from command line with 'select' subcommand or supplied
    /// in advanced with 'team' subcommand
    Cast {
        #[structopt(subcommand)]
        command: CastCommand,
    },
    #[structopt(usage = "lazystream completions <SHELL> <TARGET_DIR>")]
    /// Output shell completions to a target directory
    Completions {
        #[structopt(name = "SHELL", possible_values(&["bash", "fish", "zsh"]), default_value = "bash")]
        /// Specify a shell
        shell: String,
        #[structopt(name = "TARGET_DIR", parse(from_os_str))]
        /// Target directory to save completions
        target: PathBuf,
    },
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
pub enum PlayCommand {
    #[structopt(
        usage = "lazystream play select [--restart --proxy <PROXY> --passthrough] [OPTIONS]"
    )]
    /// Select a game from the command line to play in VLC (or --custom-player <PATH>)
    Select {
        #[structopt(long)]
        /// If live, restart the stream from the beginning
        restart: bool,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long)]
        /// Pass stream directly to VLC, this allows playback seeking
        passthrough: bool,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long, name = "PATH", parse(from_os_str))]
        /// Path to custom player supported by Streamlink (VLC, mpv & more)
        ///
        /// See https://streamlink.github.io/players.html for list of supported players
        custom_player: Option<PathBuf>,
    },
    #[structopt(
        usage = "lazystream play team <TEAM> [--restart --feed-type <feed-type> --proxy <PROXY> --passthrough] [OPTIONS]"
    )]
    /// Specify team abbreviation. If / when stream is available, will play in VLC (or --custom-player <PATH>)
    ///
    /// Example: 'lazystream play team VGK' will play the stream for the
    /// Golden Knights game in VLC.
    ///
    /// The program will stay running if a game is scheduled for the day, but stream is not yet
    /// available. Program will periodically check for the stream availability and once live,
    /// will pass that stream to VLC (or --custom-player <PATH>) to play.
    Team {
        #[structopt(name = "TEAM")]
        /// Team abbreviation
        team_abbrev: String,
        #[structopt(long)]
        /// If live, restart the stream from the beginning and record the entire thing
        restart: bool,
        #[structopt(long, parse(try_from_str), possible_values(&["HOME", "AWAY", "FRENCH", "COMPOSITE", "NATIONAL"]))]
        /// Specify the feed type to download. Will default to supplied
        /// team's applicable Home / Away feed
        feed_type: Option<FeedType>,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long)]
        /// Pass stream directly to VLC, this allows playback seeking
        passthrough: bool,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long, name = "PATH", parse(from_os_str))]
        /// Path to custom player supported by Streamlink (VLC, mpv & more)
        ///
        /// See https://streamlink.github.io/players.html for list of supported players
        custom_player: Option<PathBuf>,
    },
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
pub enum RecordCommand {
    #[structopt(
        usage = "lazystream record select <OUTPUT_DIR> [--restart --proxy <PROXY>] [OPTIONS]"
    )]
    /// Select a game from the command line to record to OUTPUT DIR
    Select {
        #[structopt(name = "OUTPUT_DIR", parse(from_os_str))]
        /// Directory to save game recordings
        output: PathBuf,
        #[structopt(long)]
        /// If live, restart the stream from the beginning and record the entire thing
        restart: bool,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long)]
        /// Specify the name / language of the audio source you'd like to use E.g. "en" or "English" for English track
        audio_source: Option<String>,
    },
    #[structopt(
        usage = "lazystream record team <TEAM> <OUTPUT_DIR> [--restart --feed-type <feed-type> --proxy <PROXY>] [OPTIONS]"
    )]
    /// Specify team abbreviation. If / when stream is available, will record to OUTPUT DIR.
    ///
    /// Example: 'lazystream record team VGK /tmp/game.mp4' will download the stream for the
    /// Golden Knights game to /tmp/game.mp4.
    ///
    /// The program will stay running if a game is scheduled for the day, but stream is not yet
    /// available. Program will periodically check for the stream availability and once live,
    /// will pass that stream to StreamLink to be downloaded.
    Team {
        #[structopt(name = "TEAM")]
        /// Team abbreviation
        team_abbrev: String,
        #[structopt(name = "OUTPUT_DIR", parse(from_os_str))]
        /// Directory to save game recordings
        output: PathBuf,
        #[structopt(long)]
        /// If live, restart the stream from the beginning and record the entire thing
        restart: bool,
        #[structopt(long, parse(try_from_str), possible_values(&["HOME", "AWAY", "FRENCH", "COMPOSITE", "NATIONAL"]))]
        /// Specify the feed type to download. Will default to supplied
        /// team's applicable Home / Away feed
        feed_type: Option<FeedType>,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long)]
        /// Specify the name / language of the audio source you'd like to use E.g. "en" or "English" for English track
        audio_source: Option<String>,
    },
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
pub enum CastCommand {
    #[structopt(usage = "lazystream cast select [--restart --proxy <PROXY>] [OPTIONS]")]
    /// Select a game and chromecast device from the command line to cast to
    Select {
        #[structopt(long)]
        /// If live, restart the stream from the beginning and cast the entire thing
        restart: bool,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long)]
        /// Specify the name / language of the audio source you'd like to use E.g. "en" or "English" for English track
        audio_source: Option<String>,
    },
    #[structopt(
        usage = "lazystream cast team <TEAM> <CHROMECAST_HOST> [--restart --feed-type <feed-type> --proxy <PROXY>] [OPTIONS]"
    )]
    /// Specify team abbreviation. If / when stream is available, will cast to CHROMECAST_HOST
    ///
    /// Example: 'lazystream cast team VGK 192.16.0.100' will cast the stream for the
    /// Golden Knights game to the Chromecast at 192.168.0.100.
    Team {
        #[structopt(name = "TEAM")]
        /// Team abbreviation
        team_abbrev: String,
        #[structopt(name = "CHROMECAST_HOST")]
        /// IP / Hostname of the Chromecast
        cast_host: String,
        #[structopt(long)]
        /// If live, restart the stream from the beginning and cast the entire thing
        restart: bool,
        #[structopt(long, parse(try_from_str), possible_values(&["HOME", "AWAY", "FRENCH", "COMPOSITE", "NATIONAL"]))]
        /// Specify the feed type to cast. Will default to supplied
        /// team's applicable Home / Away feed
        feed_type: Option<FeedType>,
        #[structopt(long, parse(try_from_str))]
        /// Proxy server address to be passed to Streamlink
        proxy: Option<Uri>,
        #[structopt(long, value_name = "[HH:]MM:SS", parse(try_from_str = parse_offset))]
        /// Amount of time to skip from the beginning of the stream. For live streams, this is a negative offset from the end of the stream (rewind).
        offset: Option<String>,
        #[structopt(long)]
        /// Specify the name / language of the audio source you'd like to use E.g. "en" or "English" for English track
        audio_source: Option<String>,
    },
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
pub enum GenerateCommand {
    #[structopt(usage = "lazystream generate playlist <FILE> [OPTIONS]")]
    /// Generate a .m3u playlist file for all games
    Playlist {
        #[structopt(name = "FILE", parse(from_os_str))]
        /// File path to save .m3u output
        file: PathBuf,
    },
    #[structopt(usage = "lazystream generate xmltv <FILE> [--start-channel INT] [OPTIONS]")]
    /// Generate a .xml XMLTV file for all games with corresponding .m3u playlist file
    Xmltv {
        #[structopt(name = "FILE", parse(from_os_str))]
        /// File path to save output, will save both .m3u and .xml files
        file: PathBuf,
        #[structopt(long, default_value = "1000")]
        /// Specify the starting channel number for the XMLVTV output
        start_channel: u32,
        #[structopt(long, default_value = "Lazyman")]
        /// Specify the channel name prefix
        channel_prefix: String,
    },
}

pub enum OutputType {
    Generate(Opt),
    Select(Opt),
    Play(Opt),
    Record(Opt),
    Cast(Opt),
    Completions(Opt),
}

fn parse_date(src: &str) -> Result<NaiveDate, ParseError> {
    let s = src.replace("-", "");
    NaiveDate::parse_from_str(&s, "%Y%m%d")
}

#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display for Cdn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.clone().into();
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    _216p,
    _224p,
    _288p,
    _360p,
    _504p,
    _540p,
    _720p,
    _720p60,
}

impl Quality {
    pub const ALL: [Quality; 8] = [
        Quality::_720p60,
        Quality::_720p,
        Quality::_540p,
        Quality::_504p,
        Quality::_360p,
        Quality::_288p,
        Quality::_224p,
        Quality::_216p,
    ];
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
                "Must be one of: '720p60', '720p', '540p', '504p', '360p', '288p', '224p', '216p'"
            ),
        }
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.clone().into();
        write!(f, "{}", s)
    }
}

impl Quality {
    pub fn to_streamlink_quality<'a>(self) -> &'a str {
        match self {
            Quality::_720p60 => "720p_alt",
            Quality::_720p => "720p",
            Quality::_540p => "540p",
            Quality::_504p => "504p",
            Quality::_360p => "360p",
            Quality::_288p => "288p",
            Quality::_224p => "224p",
            Quality::_216p => "216p",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord)]
pub enum FeedType {
    National,
    Home,
    Away,
    French,
    Composite,
}

impl From<FeedType> for &str {
    fn from(feed_type: FeedType) -> &'static str {
        match feed_type {
            FeedType::Home => "HOME",
            FeedType::Away => "AWAY",
            FeedType::National => "NATIONAL",
            FeedType::French => "FRENCH",
            FeedType::Composite => "COMPOSITE",
        }
    }
}

impl FromStr for FeedType {
    type Err = Error;

    fn from_str(s: &str) -> Result<FeedType, Error> {
        match s {
            "HOME" => Ok(FeedType::Home),
            "AWAY" => Ok(FeedType::Away),
            "FRENCH" => Ok(FeedType::French),
            "COMPOSITE" => Ok(FeedType::Composite),
            "NATIONAL" => Ok(FeedType::National),
            _ => bail!("Must be one of: 'HOME', 'AWAY', 'FRENCH', 'COMPOSITE', 'NATIONAL'"),
        }
    }
}

impl std::fmt::Display for FeedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.clone().into();
        write!(f, "{}", s)
    }
}

fn parse_offset(s: &str) -> Result<String, Error> {
    let re = regex::Regex::new(r"^(\d{2}:)?\d{2}:\d{2}$").unwrap();
    if re.is_match(s) {
        return Ok(s.to_owned());
    }
    bail!("Offset must be supplied as [HH:]MM:SS");
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sport {
    Mlb,
    Nhl,
}

impl From<Sport> for &str {
    fn from(sport: Sport) -> &'static str {
        match sport {
            Sport::Mlb => "MLB",
            Sport::Nhl => "nhl",
        }
    }
}

impl FromStr for Sport {
    type Err = Error;

    fn from_str(s: &str) -> Result<Sport, Error> {
        match s {
            "mlb" => Ok(Sport::Mlb),
            "nhl" => Ok(Sport::Nhl),
            _ => bail!("Option must match 'mlb' or 'nhl'"),
        }
    }
}

impl std::fmt::Display for Sport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.clone().into();
        write!(f, "{}", s)
    }
}
