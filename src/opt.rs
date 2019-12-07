use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "lazystream",
    about = "Easily get stream links for the current days NHL schedule.",
    version = "1.2.0",
    author = "tarkah <admin@tarkah.dev>"
)]
struct Opt {
    #[structopt(long, parse(from_os_str), name = "FILE")]
    /// Generate a .m3u playlist with all games currently playing
    playlist_output: Option<PathBuf>,
}

pub fn parse_opts() -> OutputType {
    let opt = Opt::from_args();

    if let Some(path) = opt.playlist_output {
        return OutputType::Playlist(path);
    }

    OutputType::Normal
}

pub enum OutputType {
    Playlist(PathBuf),
    Normal,
}
