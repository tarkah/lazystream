use crate::{
    log_error,
    opt::{CastCommand, Command, Opt},
    stream::LazyStream,
};
use async_std::{process, task};
use failure::{bail, format_err, Error, ResultExt};

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
    task::spawn_blocking(check_vlc).await.context(format_err!(
        "Could not find and run VLC. Please ensure it is installed \
         and accessible from your PATH"
    ))?;

    if let Command::Cast { command } = &opts.command {
        let (mut stream, cast_ip, restart, proxy) = match command {
            CastCommand::Select {
                cast_ip,
                restart,
                proxy,
            } => {
                let (_, stream) = crate::select::process(&opts, true).await?;
                (stream, cast_ip.clone(), *restart, proxy.clone())
            }
            CastCommand::Team {
                team_abbrev,
                restart,
                feed_type,
                cast_ip,
                proxy,
            } => {
                let lazy_stream = LazyStream::new(&opts).await?;
                lazy_stream.check_team_abbrev(&team_abbrev)?;
                println!("Found matching team for {}", team_abbrev);
                if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                    let stream = game
                        .stream_with_feed_or_default(feed_type, team_abbrev)
                        .await?;
                    println!("Using stream feed {}", stream.feed_type);
                    (stream, cast_ip.clone(), *restart, proxy.clone())
                } else {
                    bail!("There are no games today for {}", team_abbrev);
                }
            }
        };

        let link = stream.master_link(&opts.cdn).await?;

        task::spawn_blocking(move || cast(link, cast_ip, restart, proxy)).await?;
    }

    Ok(())
}

fn cast(link: String, cast_ip: String, restart: bool, proxy: String) -> Result<(), Error> {
    println!("Casting to {}\n\n============================\n", cast_ip);

    let cmd = if cfg!(target_os = "windows") {
        "streamlink.exe"
    } else {
        "streamlink"
    };

    let vlc_cmd = if cfg!(target_os = "windows") {
        "cvlc.exe"
    } else {
        "cvlc"
    };

    let hls_link = format!("hlsvariant://{}", link);
    let player_arg = format!(
        "{} --sout \"#chromecast\" \
         --sout-chromecast-ip={} \
         --demux-filter=demux_chromecast",
        vlc_cmd, cast_ip,
    );

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
        "--player",
        player_arg.as_str(),
    ];

    if restart {
        args.push("--hls-live-restart");
    }

    if proxy != "" {
        args.push("--https-proxy");
        args.push(proxy.as_str());
    }

    let result = std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::inherit())
        .spawn()?
        .wait()?;

    if !result.success() {
        bail!("StreamLink failed");
    }

    println!("\n============================\n\nStream finshed");

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

    if !output.status.success() && &std_out[0..10] != "streamlink" {
        bail!("Couldn't run streamlink");
    }

    Ok(())
}

fn check_vlc() -> Result<(), Error> {
    let cmd = if cfg!(target_os = "windows") {
        "cvlc.exe"
    } else {
        "cvlc"
    };

    let output = std::process::Command::new(cmd).arg("--version").output()?;
    let std_out = String::from_utf8(output.stdout)?;

    if !output.status.success() && &std_out[0..3] != "VLC" {
        bail!("Couldn't run VLC");
    }

    Ok(())
}
