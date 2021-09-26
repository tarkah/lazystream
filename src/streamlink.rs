use crate::{
    log_error,
    opt::{CastCommand, Command, Opt, PlayCommand, Quality, RecordCommand},
    stream::{Game, LazyStream, Stream},
};
use async_std::{process, task};
use chrono::Local;
use failure::{bail, format_err, Error, ResultExt};
use isahc::http::Uri;
use mdns::RecordKind;
use read_input::prelude::*;
use std::{
    collections::HashMap, io::Write, net::Ipv4Addr, path::PathBuf, process::Stdio, time::Duration,
};

pub fn run(opts: Opt) {
    task::block_on(async {
        if let Err(e) = process(opts).await {
            log_error(e.as_fail());
            process::exit(1);
        };
    });
}

async fn process(opts: Opt) -> Result<(), Error> {
    task::spawn_blocking(check_streamlink)
        .await
        .context(format_err!(
            "Could not find and run Streamlink. Please ensure it is installed \
             and accessible from your PATH"
        ))?;

    let (game, mut stream, command, restart, proxy, offset, quality) = match &opts.command {
        Command::Play { command } => process_play(&opts, command).await?,
        Command::Record { command } => process_record(&opts, command).await?,
        Command::Cast { command } => process_cast(&opts, command).await?,
        _ => bail!("Wrong command for module"),
    };

    println!();
    while stream.master_link(opts.cdn).await.is_err() {
        if opts.disable_retry {
            bail!("Stream not available yet");
        }

        println!("Stream not available yet, will check again soon...");
        task::sleep(Duration::from_secs(60 * 30)).await;
    }
    let link = if let Some(quality) = quality {
        stream.quality_link(opts.cdn, quality).await?
    } else {
        stream.master_link(opts.cdn).await?
    };

    let args = StreamlinkArgs {
        link,
        game,
        stream,
        command,
        restart,
        proxy,
        offset,
        quality,
    };

    task::spawn_blocking(move || streamlink(args)).await?;

    Ok(())
}

async fn process_play(
    opts: &Opt,
    command: &PlayCommand,
) -> Result<
    (
        Game,
        Stream,
        StreamlinkCommand,
        bool,
        Option<Uri>,
        Option<String>,
        Option<Quality>,
    ),
    Error,
> {
    match command {
        PlayCommand::Select {
            restart,
            proxy,
            offset,
            ..
        } => {
            let (game, stream) = crate::select::process(opts, true).await?;

            let streamlink_command = StreamlinkCommand::from(command);
            Ok((
                game,
                stream,
                streamlink_command,
                *restart,
                proxy.clone(),
                offset.clone(),
                opts.quality,
            ))
        }
        PlayCommand::Team {
            team_abbrev,
            restart,
            feed_type,
            proxy,
            offset,
            ..
        } => {
            let lazy_stream = LazyStream::new(opts).await?;
            lazy_stream.check_team_abbrev(&team_abbrev)?;
            println!("Found matching team for {}", team_abbrev);

            if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                println!("Game found for today");

                let stream = game
                    .stream_with_feed_or_default(*feed_type, team_abbrev)
                    .await?;
                println!("Using stream feed {}", stream.feed_type);

                let streamlink_command = StreamlinkCommand::from(command);
                Ok((
                    game,
                    stream,
                    streamlink_command,
                    *restart,
                    proxy.clone(),
                    offset.clone(),
                    opts.quality,
                ))
            } else {
                bail!("There are no games today for {}", team_abbrev);
            }
        }
    }
}

async fn process_record(
    opts: &Opt,
    command: &RecordCommand,
) -> Result<
    (
        Game,
        Stream,
        StreamlinkCommand,
        bool,
        Option<Uri>,
        Option<String>,
        Option<Quality>,
    ),
    Error,
> {
    match command {
        RecordCommand::Select {
            output,
            restart,
            proxy,
            offset,
            ..
        } => {
            check_output(&output)?;
            let (game, stream) = crate::select::process(opts, true).await?;

            let streamlink_command = StreamlinkCommand::from(command);
            Ok((
                game,
                stream,
                streamlink_command,
                *restart,
                proxy.clone(),
                offset.clone(),
                opts.quality,
            ))
        }
        RecordCommand::Team {
            team_abbrev,
            restart,
            feed_type,
            output,
            proxy,
            offset,
            ..
        } => {
            check_output(&output)?;

            let lazy_stream = LazyStream::new(opts).await?;
            lazy_stream.check_team_abbrev(&team_abbrev)?;
            println!("Found matching team for {}", team_abbrev);

            if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                println!("Game found for today");

                let stream = game
                    .stream_with_feed_or_default(*feed_type, team_abbrev)
                    .await?;
                println!("Using stream feed {}", stream.feed_type);

                let streamlink_command = StreamlinkCommand::from(command);
                Ok((
                    game,
                    stream,
                    streamlink_command,
                    *restart,
                    proxy.clone(),
                    offset.clone(),
                    opts.quality,
                ))
            } else {
                bail!("There are no games today for {}", team_abbrev);
            }
        }
    }
}

async fn process_cast(
    opts: &Opt,
    command: &CastCommand,
) -> Result<
    (
        Game,
        Stream,
        StreamlinkCommand,
        bool,
        Option<Uri>,
        Option<String>,
        Option<Quality>,
    ),
    Error,
> {
    task::spawn_blocking(check_vlc).await.context(format_err!(
        "Could not find and run VLC. Please ensure it is installed \
         and accessible from your PATH"
    ))?;

    match command {
        CastCommand::Select {
            restart,
            proxy,
            offset,
            audio_source,
        } => {
            let (game, stream) = crate::select::process(opts, true).await?;

            let cast_devices = task::spawn_blocking(|| {
                print!("\nSearching for cast devices...");
                let _ = std::io::stdout().flush();
                find_cast_devices()
            })
            .await?;

            let cast_ip = select_cast_device(cast_devices)?;
            println!("\nUsing cast device {}\n", cast_ip);

            let streamlink_command =
                StreamlinkCommand::cast_with_ip(cast_ip.to_string(), audio_source.clone());

            Ok((
                game,
                stream,
                streamlink_command,
                *restart,
                proxy.clone(),
                offset.clone(),
                opts.quality,
            ))
        }
        CastCommand::Team {
            team_abbrev,
            restart,
            feed_type,
            proxy,
            offset,
            ..
        } => {
            let lazy_stream = LazyStream::new(opts).await?;
            lazy_stream.check_team_abbrev(&team_abbrev)?;
            println!("Found matching team for {}", team_abbrev);

            if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                println!("Game found for today");

                let stream = game
                    .stream_with_feed_or_default(*feed_type, team_abbrev)
                    .await?;
                println!("Using stream feed {}", stream.feed_type);

                let streamlink_command = StreamlinkCommand::from(command);
                Ok((
                    game,
                    stream,
                    streamlink_command,
                    *restart,
                    proxy.clone(),
                    offset.clone(),
                    opts.quality,
                ))
            } else {
                bail!("There are no games today for {}", team_abbrev);
            }
        }
    }
}

#[derive(PartialEq)]
enum StreamlinkCommand {
    Play {
        passthrough: bool,
        custom_player: Option<PathBuf>,
    },
    Record {
        output: PathBuf,
        audio_source: Option<String>,
    },
    Cast {
        cast_host: String,
        audio_source: Option<String>,
    },
}

impl StreamlinkCommand {
    fn cast_with_ip(addr: String, audio_source: Option<String>) -> Self {
        StreamlinkCommand::Cast {
            cast_host: addr,
            audio_source,
        }
    }
}

impl From<&PlayCommand> for StreamlinkCommand {
    fn from(cmd: &PlayCommand) -> Self {
        match cmd {
            PlayCommand::Select {
                passthrough,
                custom_player,
                ..
            } => StreamlinkCommand::Play {
                passthrough: *passthrough,
                custom_player: custom_player.clone(),
            },
            PlayCommand::Team {
                passthrough,
                custom_player,
                ..
            } => StreamlinkCommand::Play {
                passthrough: *passthrough,
                custom_player: custom_player.clone(),
            },
        }
    }
}

impl From<&RecordCommand> for StreamlinkCommand {
    fn from(cmd: &RecordCommand) -> Self {
        match cmd {
            RecordCommand::Select {
                output,
                audio_source,
                ..
            } => StreamlinkCommand::Record {
                output: output.clone(),
                audio_source: audio_source.clone(),
            },
            RecordCommand::Team {
                output,
                audio_source,
                ..
            } => StreamlinkCommand::Record {
                output: output.clone(),
                audio_source: audio_source.clone(),
            },
        }
    }
}

impl From<&CastCommand> for StreamlinkCommand {
    fn from(cmd: &CastCommand) -> Self {
        match cmd {
            CastCommand::Select { audio_source, .. } => StreamlinkCommand::Cast {
                cast_host: "0.0.0.0".to_owned(),
                audio_source: audio_source.clone(),
            },
            CastCommand::Team {
                cast_host,
                audio_source,
                ..
            } => StreamlinkCommand::Cast {
                cast_host: cast_host.clone(),
                audio_source: audio_source.clone(),
            },
        }
    }
}

struct StreamlinkArgs {
    link: String,
    game: Game,
    stream: Stream,
    command: StreamlinkCommand,
    restart: bool,
    proxy: Option<Uri>,
    offset: Option<String>,
    quality: Option<Quality>,
}

fn streamlink(mut args: StreamlinkArgs) -> Result<(), Error> {
    match &args.command {
        StreamlinkCommand::Play { .. } => {
            println!("Passing game to player...\n\n============================\n")
        }
        StreamlinkCommand::Record { .. } => {
            println!("Recording with StreamLink...\n\n============================\n")
        }
        StreamlinkCommand::Cast { .. } => {
            println!("Casting with StreamLink...\n\n============================\n")
        }
    }

    let cmd = if cfg!(target_os = "windows") {
        "streamlink.exe"
    } else {
        "streamlink"
    };

    let mut player_cmd = if cfg!(target_os = "windows") {
        "vlc.exe"
    } else {
        "vlc"
    };

    let hls_link = if args.quality.is_some() {
        format!("hlsvariant://{}", args.link)
    } else {
        format!("hlsvariant://{} name_key=bitrate", args.link)
    };

    let mut command_args = vec![
        hls_link.as_str(),
        "best",
        "--force",
        "--http-no-ssl-verify",
        "--hls-segment-threads",
        "4",
        "--http-header",
        "User-Agent=User-Agent=Mozilla/5.0 (Windows NT 10.0; \
         Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko \
         Chrome/59.0.3071.115 Safari/537.36",
        "--retry-streams",
        "1",
        "--retry-open",
        "3",
        "--stream-types",
        "hls",
    ];

    if args.restart {
        command_args.push("--hls-live-restart");
    }

    let mut _proxy = String::new();
    if let Some(ref proxy) = args.proxy {
        _proxy = proxy.to_string();
        command_args.push("--https-proxy");
        command_args.push(&_proxy);
    }

    let mut _offset = String::new();
    if let Some(offset) = args.offset {
        _offset = offset;
        command_args.push("--hls-start-offset");
        command_args.push(&_offset);
    }

    let mut _arg;
    match &mut args.command {
        StreamlinkCommand::Play {
            passthrough,
            custom_player,
        } => {
            let title = format!(
                "{} @ {} - {} - {}",
                args.game.away_team.name,
                args.game.home_team.name,
                args.stream.feed_type,
                args.game
                    .game_date
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %-I:%M %p"),
            );
            _arg = title;

            if let Some(player) = custom_player {
                player_cmd = player.to_str().unwrap();
            }

            command_args.push("--hls-audio-select");
            command_args.push("*");
            command_args.push("--player");
            command_args.push(player_cmd);
            command_args.push("--title");
            command_args.push(_arg.as_str());

            if *passthrough {
                command_args.push("--player-passthrough");
                command_args.push("hls");
            }
        }
        StreamlinkCommand::Record {
            output,
            audio_source,
        } => {
            let filename = format!(
                "{} {} @ {} {}.mp4",
                args.game
                    .game_date
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H%M"),
                args.game.away_team.name,
                args.game.home_team.name,
                args.stream.feed_type
            );
            output.push(filename);

            if let Some(source) = audio_source {
                command_args.push("--hls-audio-select");
                command_args.push(source.as_str());
            }

            _arg = output.display().to_string();

            command_args.push("-o");
            command_args.push(_arg.as_str());
        }
        StreamlinkCommand::Cast {
            cast_host,
            audio_source,
        } => {
            _arg = format!(
                "{} -I dummy --sout \"#chromecast\" \
                     --sout-chromecast-ip={} \
                     --demux-filter=demux_chromecast",
                player_cmd, cast_host,
            );

            if let Some(source) = audio_source {
                command_args.push("--hls-audio-select");
                command_args.push(source.as_str());
            }

            command_args.push("--player");
            command_args.push(_arg.as_str());
        }
    }

    let result = std::process::Command::new(cmd)
        .args(command_args)
        .stdout(Stdio::inherit())
        .spawn()?
        .wait()?;

    if !result.success() {
        bail!("StreamLink failed");
    }

    match &args.command {
        StreamlinkCommand::Play { .. } => {
            println!("\n============================\n\nPlayback finshed")
        }
        StreamlinkCommand::Record { .. } => {
            println!("\n============================\n\nRecording finshed")
        }
        StreamlinkCommand::Cast { .. } => {
            println!("\n============================\n\nCasting finshed")
        }
    }

    Ok(())
}

fn check_streamlink() -> Result<(), Error> {
    let cmd = if cfg!(target_os = "windows") {
        "streamlink.exe"
    } else {
        "streamlink"
    };

    let output = std::process::Command::new(cmd).arg("--version").output()?;
    let std_out = String::from_utf8(output.stdout)?;

    if !output.status.success() && &std_out[0..11] != "streamlink" {
        bail!("Couldn't run streamlink");
    }

    Ok(())
}

fn check_vlc() -> Result<(), Error> {
    if !cfg!(target_os = "windows") {
        let cmd = "vlc";

        let output = std::process::Command::new(cmd).arg("--version").output()?;
        let std_out = String::from_utf8(output.stdout)?;

        if !output.status.success() && &std_out[0..3] != "VLC" {
            bail!("Couldn't run VLC");
        }
    }

    Ok(())
}

/// Make sure output directory exists and can be written to
fn check_output(directory: &PathBuf) -> Result<(), Error> {
    if !directory.is_dir() {
        bail!("Output diretory does not exist, please create it");
    }

    Ok(())
}

const SERVICE_NAME: &str = "_googlecast._tcp.local";

#[allow(clippy::unnecessary_unwrap)]
fn find_cast_devices() -> Result<HashMap<Ipv4Addr, String>, Error> {
    let mut devices = HashMap::new();

    for response in mdns::discover::all(SERVICE_NAME)
        .map_err(|_| format_err!("mDNS discovery failed"))?
        .timeout(Duration::from_secs(2))
    {
        let response = response.map_err(|_| format_err!("mDNS response failed"))?;

        let mut ip = None;
        let mut name = None;
        for record in response.records() {
            match record.kind {
                RecordKind::A(addr) => ip = Some(addr),
                RecordKind::TXT(ref data) => {
                    for item in data {
                        if &item[0..3] == "fn=" {
                            name = Some(item[3..].to_owned());
                        }
                    }
                }
                _ => {}
            }
        }

        if ip.is_some() && name.is_some() {
            devices.insert(ip.unwrap(), name.unwrap());
        }
    }

    Ok(devices)
}

fn select_cast_device(devices: HashMap<Ipv4Addr, String>) -> Result<Ipv4Addr, Error> {
    if devices.is_empty() {
        bail!("No castable devices found on LAN");
    }

    println!("\rPick a cast device...        \n");

    let mut device_addrs = vec![];
    for (idx, (ip, name)) in devices.iter().enumerate() {
        println!("{}) {} - {}", idx + 1, ip, name);
        device_addrs.push(ip);
    }

    let device_count = devices.len();
    let device_choice = input::<usize>()
        .msg("\n>>> ")
        .add_test(move |input| *input > 0 && *input <= device_count)
        .get();
    let addrs = device_addrs[(device_choice - 1)];

    Ok(*addrs)
}
