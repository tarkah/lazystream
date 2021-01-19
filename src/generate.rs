use crate::{
    log_error,
    opt::{Cdn, Command, FeedType, GenerateCommand, Opt, Quality, Sport},
    stream::{Game, LazyStream},
    VERSION,
};
use async_std::{fs, process, task};
use chrono::{Duration, Local};
use failure::Error;
use std::path::PathBuf;

const NHL_ICON: &str = "https://upload.wikimedia.org/wikipedia/en/thumb/3/3a/05_NHL_Shield.svg/1200px-05_NHL_Shield.svg.png";
const MLB_ICON: &str = "https://upload.wikimedia.org/wikipedia/en/thumb/a/a6/Major_League_Baseball_logo.svg/1200px-Major_League_Baseball_logo.svg.png";

pub fn run(opts: Opt) {
    task::block_on(async {
        if let Err(e) = process(opts).await {
            log_error(&e);
            process::exit(1);
        };
    });
}

async fn process(opts: Opt) -> Result<(), Error> {
    if let Command::Generate { command } = &opts.command {
        match command {
            GenerateCommand::Xmltv { .. } => {
                println!("Creating .m3u & .xml for XMLTV...");
            }
            _ => println!("Creating playlist file..."),
        }
    }

    let mut lazy_stream = LazyStream::new(&opts).await?;

    if let Some(quality) = opts.quality {
        lazy_stream
            .resolve_with_quality_link(opts.cdn, quality)
            .await;
    } else {
        lazy_stream.resolve_with_master_link(opts.cdn).await;
    }

    let games = lazy_stream.games();

    if let Command::Generate { command } = opts.command {
        match command {
            GenerateCommand::Xmltv {
                file,
                start_channel,
                channel_prefix,
                exclude_feeds,
            } => {
                let path = file.with_extension("m3u");
                create_playlist(
                    path.clone(),
                    games.clone(),
                    opts.cdn,
                    opts.quality,
                    true,
                    start_channel,
                    Some(&channel_prefix),
                    &exclude_feeds,
                )
                .await?;

                let path = path.with_extension("xml");
                create_xmltv(
                    path,
                    games,
                    start_channel,
                    opts.sport,
                    &channel_prefix,
                    &exclude_feeds,
                )
                .await?;
            }
            GenerateCommand::Playlist {
                file,
                exclude_feeds,
            } => {
                let path = file.with_extension("m3u");
                create_playlist(
                    path,
                    games,
                    opts.cdn,
                    opts.quality,
                    false,
                    1000,
                    None,
                    &exclude_feeds,
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::clippy::too_many_arguments)]
async fn create_playlist(
    path: PathBuf,
    mut games: Vec<Game>,
    cdn: Cdn,
    quality: Option<Quality>,
    is_xmltv: bool,
    start_channel: u32,
    channel_prefix: Option<&str>,
    exclude_feeds: &[FeedType],
) -> Result<(), Error> {
    let mut m3u = String::new();
    m3u.push_str("#EXTM3U\n");

    let mut id: u32 = 0;
    for game in games.iter_mut() {
        for (_, stream) in game
            .streams
            .as_mut()
            .unwrap()
            .iter_mut()
            .filter(|(feed_type, _)| !exclude_feeds.contains(&feed_type))
        {
            let master_link = stream.master_link(cdn).await;

            let link = if let Some(quality) = quality {
                let quality_link = stream.quality_link(cdn, quality).await;

                quality_link.or(master_link)
            } else {
                master_link
            };

            let title = if is_xmltv {
                format!("{} {}", channel_prefix.unwrap(), id + 1)
            } else {
                format!(
                    "{} {} @ {} {}",
                    game.game_date
                        .with_timezone(&Local)
                        .time()
                        .format("%-I:%M %p")
                        .to_string(),
                    game.away_team.team_name,
                    game.home_team.team_name,
                    stream.feed_type,
                )
            };
            let record = format!(
                "#EXTINF:-1 CUID=\"{}\" tvg-id=\"{}\" tvg-name=\"{} {}\",{}\n{}\n",
                start_channel + id,
                start_channel + id,
                channel_prefix.unwrap_or("Lazyman"),
                id + 1,
                title,
                link.unwrap_or_else(|_| ".".to_string())
            );
            m3u.push_str(&record);
            id += 1;
        }
    }

    // Create additional blank records for all 100 channels
    if is_xmltv {
        let _id = id;
        for _ in _id..100 {
            let title = format!("{} {}", channel_prefix.unwrap(), id + 1);
            let record = format!(
                "#EXTINF:-1 CUID=\"{}\" tvg-id=\"{}\" tvg-name=\"{} {}\",{}\n.\n",
                start_channel + id,
                start_channel + id,
                channel_prefix.unwrap_or("Lazyman"),
                id + 1,
                title,
            );
            m3u.push_str(&record);
            id += 1;
        }
    }

    fs::write(&path, m3u).await?;

    println!("Playlist saved to: {:?}", path);

    Ok(())
}

async fn create_xmltv(
    path: PathBuf,
    mut games: Vec<Game>,
    start_channel: u32,
    sport: Sport,
    channel_prefix: &str,
    exclude_feeds: &[FeedType],
) -> Result<(), Error> {
    let mut xmltv = String::new();
    xmltv.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE tv SYSTEM \"xmltv.dd\">\
         \n\
         \n  <tv generator-info-name=\"lazystream\" source-info-name=\"lazystream - {}\">",
        VERSION
    ));

    let icon = match sport {
        Sport::Nhl => NHL_ICON,
        Sport::Mlb => MLB_ICON,
    };

    let mut id: u32 = 0;
    while id < 100 {
        let record = format!(
            "\n    <channel id=\"{}\">\
             \n      <display-name>{} {}</display-name>\
             \n      <icon src=\"{}\"></icon>\
             \n    </channel>",
            start_channel + id,
            channel_prefix,
            id + 1,
            icon
        );
        xmltv.push_str(&record);
        id += 1;
    }

    let mut id: u32 = 0;
    for game in games.iter_mut() {
        let icons = if let Some(game_cuts) = game.game_cuts().await {
            let cuts = vec![&game_cuts.cut_320_180, &game_cuts.cut_2048_1152];
            let mut icons = String::new();
            for cut in cuts {
                let icon = format!(
                    "\n      <icon src=\"{}\" width=\"{}\" height=\"{}\"></icon>",
                    cut.src, cut.width, cut.height,
                );
                icons.push_str(&icon);
            }
            icons
        } else {
            String::from("\n      <icon src=\"\"></icon>")
        };

        let mut description = game.description().await.unwrap_or_else(|| String::from(""));
        if description.is_empty() {
            description = format!(
                "Watch the {} take on the {}.",
                game.away_team.team_name, game.home_team.team_name
            );
        }

        for (_, stream) in game
            .streams
            .as_mut()
            .unwrap()
            .iter_mut()
            .filter(|(feed_type, _)| !exclude_feeds.contains(&feed_type))
        {
            let start = game.game_date.with_timezone(&Local);
            let stop = start + Duration::hours(4);
            let title = format!(
                "{} @ {} ({})",
                game.away_team.team_name, game.home_team.team_name, stream.feed_type
            );

            let record = format!(
                "\n    <programme channel=\"{}\" start=\"{} {}\" stop=\"{} {}\">\
                     \n      <title lang=\"en\">{}</title>\
                     \n      <desc lang=\"en\">{}</desc>\
                     \n      <category lang=\"en\">Sports</category>\
                     {}\
                     \n    </programme>",
                start_channel + id,
                start.format("%Y%m%d%H%M%S"),
                start.format("%z"),
                stop.format("%Y%m%d%H%M%S"),
                stop.format("%z"),
                title,
                description,
                icons,
            );
            xmltv.push_str(&record);
            id += 1;
        }
    }

    xmltv.push_str("\n  </tv>");

    fs::write(&path, xmltv).await?;

    println!("Xmltv file saved to: {:?}", path);

    Ok(())
}
