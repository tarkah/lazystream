use crate::{
    log_error,
    opt::{CastCommand, Command, Opt, RecordCommand},
    stream::{Game, LazyStream, Stream},
};
use async_std::{process, task};
use chrono::Local;
use failure::{bail, format_err, Error, ResultExt};
use http::Uri;
use mdns::RecordKind;
use read_input::prelude::*;
use std::{
    collections::HashMap, io::Write, net::Ipv4Addr, path::PathBuf, process::Stdio, time::Duration,
};

pub fn run(opts: Opt) {
    task::block_on(async {
        if let Err(e) = process(opts).await {
            log_error(&e);
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

    let (game, mut stream, command, restart, proxy) = match &opts.command {
        Command::Record { command } => process_record(&opts, command).await?,
        Command::Cast { command } => process_cast(&opts, command).await?,
        _ => bail!("Wrong command for module"),
    };

    println!();
    while let Err(_) = stream.master_link(&opts.cdn).await {
        println!("Stream not available yet, will check again soon...");
        task::sleep(Duration::from_secs(60 * 30)).await;
    }
    let link = stream.master_link(&opts.cdn).await?;

    task::spawn_blocking(move || streamlink(link, game, stream, command, restart, proxy)).await?;

    Ok(())
}

async fn process_record(
    opts: &Opt,
    command: &RecordCommand,
) -> Result<(Game, Stream, StreamlinkCommand, bool, Option<Uri>), Error> {
    match command {
        RecordCommand::Select {
            output,
            restart,
            proxy,
        } => {
            check_output(&output)?;
            let (game, stream) = crate::select::process(opts, true).await?;

            let streamlink_command = StreamlinkCommand::from(command);
            Ok((game, stream, streamlink_command, *restart, proxy.clone()))
        }
        RecordCommand::Team {
            team_abbrev,
            restart,
            feed_type,
            output,
            proxy,
        } => {
            check_output(&output)?;

            let lazy_stream = LazyStream::new(opts).await?;
            lazy_stream.check_team_abbrev(&team_abbrev)?;
            println!("Found matching team for {}", team_abbrev);

            if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                println!("Game found for today");

                let stream = game
                    .stream_with_feed_or_default(feed_type, team_abbrev)
                    .await?;
                println!("Using stream feed {}", stream.feed_type);

                let streamlink_command = StreamlinkCommand::from(command);
                Ok((game, stream, streamlink_command, *restart, proxy.clone()))
            } else {
                bail!("There are no games today for {}", team_abbrev);
            }
        }
    }
}

async fn process_cast(
    opts: &Opt,
    command: &CastCommand,
) -> Result<(Game, Stream, StreamlinkCommand, bool, Option<Uri>), Error> {
    task::spawn_blocking(check_vlc).await.context(format_err!(
        "Could not find and run VLC. Please ensure it is installed \
         and accessible from your PATH"
    ))?;

    match command {
        CastCommand::Select { restart, proxy } => {
            let (game, stream) = crate::select::process(opts, true).await?;

            let cast_devices = task::spawn_blocking(|| {
                print!("\nSearching for cast devices...");
                let _ = std::io::stdout().flush();
                find_cast_devices()
            })
            .await?;

            let cast_ip = select_cast_device(cast_devices)?;
            println!("\nUsing cast device {}\n", cast_ip);

            let streamlink_command = StreamlinkCommand::cast_with_ip(cast_ip);

            Ok((game, stream, streamlink_command, *restart, proxy.clone()))
        }
        CastCommand::Team {
            team_abbrev,
            restart,
            feed_type,
            proxy,
            ..
        } => {
            let lazy_stream = LazyStream::new(opts).await?;
            lazy_stream.check_team_abbrev(&team_abbrev)?;
            println!("Found matching team for {}", team_abbrev);

            if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                println!("Game found for today");

                let stream = game
                    .stream_with_feed_or_default(feed_type, team_abbrev)
                    .await?;
                println!("Using stream feed {}", stream.feed_type);

                let streamlink_command = StreamlinkCommand::from(command);
                Ok((game, stream, streamlink_command, *restart, proxy.clone()))
            } else {
                bail!("There are no games today for {}", team_abbrev);
            }
        }
    }
}

enum StreamlinkCommand {
    Record { output: PathBuf },
    Cast { cast_ip: Ipv4Addr },
}

impl StreamlinkCommand {
    fn cast_with_ip(addr: Ipv4Addr) -> Self {
        StreamlinkCommand::Cast { cast_ip: addr }
    }
}

impl From<&RecordCommand> for StreamlinkCommand {
    fn from(cmd: &RecordCommand) -> Self {
        match cmd {
            RecordCommand::Select { output, .. } => StreamlinkCommand::Record {
                output: output.clone(),
            },
            RecordCommand::Team { output, .. } => StreamlinkCommand::Record {
                output: output.clone(),
            },
        }
    }
}

impl From<&CastCommand> for StreamlinkCommand {
    fn from(cmd: &CastCommand) -> Self {
        match cmd {
            CastCommand::Select { .. } => StreamlinkCommand::Cast {
                cast_ip: [0, 0, 0, 0].into(),
            },
            CastCommand::Team { cast_ip, .. } => StreamlinkCommand::Cast { cast_ip: *cast_ip },
        }
    }
}

fn streamlink(
    link: String,
    game: Game,
    stream: Stream,
    mut command: StreamlinkCommand,
    restart: bool,
    proxy: Option<Uri>,
) -> Result<(), Error> {
    match &command {
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

    let vlc_cmd = if cfg!(target_os = "windows") {
        "vlc.exe"
    } else {
        "cvlc"
    };

    let hls_link = format!("hlsvariant://{}", link);

    let mut args = vec![
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
    ];

    if restart {
        args.push("--hls-live-restart");
    }

    let mut _proxy = String::new();
    if let Some(ref proxy) = proxy {
        _proxy = proxy.to_string();
        args.push("--https-proxy");
        args.push(&_proxy);
    }

    let mut _arg;
    match &mut command {
        StreamlinkCommand::Record { output } => {
            let filename = format!(
                "{} {} @ {} {}.mp4",
                game.game_date
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %-I:%M %p"),
                game.away_team.name,
                game.home_team.name,
                stream.feed_type
            );
            output.push(filename);

            _arg = output.display().to_string();

            args.push("-o");
            args.push(_arg.as_str());
        }
        StreamlinkCommand::Cast { cast_ip } => {
            _arg = if cfg!(target_os = "windows") {
                format!(
                    "{} -I dummy --sout \"#chromecast\" \
                     --sout-chromecast-ip={} \
                     --demux-filter=demux_chromecast",
                    vlc_cmd, cast_ip,
                )
            } else {
                format!(
                    "{} --sout \"#chromecast\" \
                     --sout-chromecast-ip={} \
                     --demux-filter=demux_chromecast",
                    vlc_cmd, cast_ip,
                )
            };

            args.push("--player");
            args.push(_arg.as_str());
        }
    }

    let result = std::process::Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .spawn()?
        .wait()?;

    if !result.success() {
        bail!("StreamLink failed");
    }

    match &command {
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
        let cmd = "cvlc";

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

    let metadata = directory.metadata().context(format_err!(
        "Could not get output directory metadata. Do you have permissions for this folder?"
    ))?;

    if metadata.permissions().readonly() {
        bail!(
            "Output directory is read only, please change permissions or \
             specify a directory you have permissions for"
        );
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
