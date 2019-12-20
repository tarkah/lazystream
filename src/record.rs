use crate::{
    log_error,
    opt::{Command, Opt, RecordCommand},
    stream::{Game, LazyStream, Stream},
};
use async_std::{process, task};
use chrono::Local;
use failure::{bail, format_err, Error, ResultExt};
use http::Uri;
use std::{path::PathBuf, process::Stdio, time::Duration};

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

    if let Command::Record { command } = &opts.command {
        let (game, mut stream, output, restart, proxy) = match command {
            RecordCommand::Select {
                output,
                restart,
                proxy,
            } => {
                check_output(&output)?;
                let (game, stream) = crate::select::process(&opts, true).await?;
                (game, stream, output.clone(), *restart, proxy.clone())
            }
            RecordCommand::Team {
                team_abbrev,
                restart,
                feed_type,
                output,
                proxy,
            } => {
                check_output(&output)?;

                let lazy_stream = LazyStream::new(&opts).await?;
                lazy_stream.check_team_abbrev(&team_abbrev)?;
                println!("Found matching team for {}", team_abbrev);
                if let Some(mut game) = lazy_stream.game_with_team_abbrev(&team_abbrev) {
                    println!("Game found for today");
                    let stream = game
                        .stream_with_feed_or_default(feed_type, team_abbrev)
                        .await?;
                    println!("Using stream feed {}", stream.feed_type);
                    (game, stream, output.clone(), *restart, proxy.clone())
                } else {
                    bail!("There are no games today for {}", team_abbrev);
                }
            }
        };

        while let Err(_) = stream.master_link(&opts.cdn).await {
            println!("Stream not available yet, will check again soon...");
            task::sleep(Duration::from_secs(60 * 30)).await;
        }
        let link = stream.master_link(&opts.cdn).await?;

        task::spawn_blocking(move || record(link, game, stream, output, restart, proxy)).await?;
    }

    Ok(())
}

fn record(
    link: String,
    game: Game,
    stream: Stream,
    mut path: PathBuf,
    restart: bool,
    proxy: Option<Uri>,
) -> Result<(), Error> {
    println!("Recording with StreamLink...\n\n============================\n");
    let filename = format!(
        "{} {} @ {} {}.mp4",
        game.game_date
            .with_timezone(&Local)
            .format("%Y-%m-%d %-I:%M %p"),
        game.away_team.name,
        game.home_team.name,
        stream.feed_type
    );
    path.push(filename);

    let cmd = if cfg!(target_os = "windows") {
        "streamlink.exe"
    } else {
        "streamlink"
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

    let result = std::process::Command::new(cmd)
        .args(args)
        .arg("-o")
        .arg(path)
        .stdout(Stdio::inherit())
        .spawn()?
        .wait()?;

    if !result.success() {
        bail!("StreamLink failed");
    }

    println!("\n============================\n\nRecording finshed");

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
