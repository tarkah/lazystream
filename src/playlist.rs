use crate::{
    log_error,
    opt::{Opt, Quality},
    stream::{get_master_m3u8, get_master_url, get_quality_url},
    HOST, VERSION,
};
use async_std::{fs, sync::Mutex, task};
use chrono::{DateTime, Local, Utc};
use failure::Error;
use futures::future::join_all;
use stats_api::model::GameContentResponse;
use std::{path::PathBuf, process};

pub fn run(opts: Opt) {
    task::block_on(async {
        if let Err(e) = process(opts).await {
            log_error(&e);
            process::exit(1);
        };
    });
}

async fn process(opts: Opt) -> Result<(), Error> {
    if opts.xmltv_output.is_some() {
        println!("Creating .m3u & .xml for XMLTV...");
    } else if opts.playlist_output.is_some() {
        println!("Creating playlist file...");
    }

    let quality = opts.quality.clone();
    let cdn: &str = opts.cdn.into();

    let client = stats_api::Client::new();

    let date = if opts.date.is_some() {
        opts.date.unwrap()
    } else {
        Local::today().naive_local()
    };
    let todays_schedule = client.get_schedule_for(date).await?;
    let date = todays_schedule.date.format("%Y-%m-%d").to_string();

    let games = Mutex::new(vec![]);
    let tasks: Vec<_> = todays_schedule
        .games
        .into_iter()
        .map(|game| {
            async {
                let game_data = GameData::new(&game);

                let client = stats_api::Client::new();
                if let Ok(game_content) = client.get_game_content(game.game_pk).await {
                    let game_data = add_game_media_items(game_data, &game_content);

                    let game_data =
                        add_game_streams(game_data, game_content, &date, &cdn, &quality).await;

                    games.lock().await.push(game_data);
                }
                drop(game);
            }
        })
        .collect();

    join_all(tasks).await;
    let games = games.into_inner();

    if let Some(path) = opts.xmltv_output {
        let path = path.with_extension("m3u");
        create_playlist(path.clone(), games.clone(), true, opts.xmltv_start_channel).await?;

        let path = path.with_extension("xml");
        create_xmltv(path, games, opts.xmltv_start_channel).await?;
    } else if let Some(path) = opts.playlist_output {
        let path = path.with_extension("m3u");
        create_playlist(path, games, false, opts.xmltv_start_channel).await?;
    }

    Ok(())
}

fn add_game_media_items(mut game_data: GameData, game_content: &GameContentResponse) -> GameData {
    let preview_items = &game_content.editorial.preview.items;
    if let Some(items) = preview_items {
        if let Some(preview) = items.first() {
            game_data.description = Some(preview.subhead.clone());

            if let Some(ref media) = preview.media {
                if media.r#type == "photo" {
                    game_data.cuts = Some(media.image.cuts.clone());
                }
            }
        }
    }

    game_data
}

async fn add_game_streams(
    game_data: GameData,
    game_content: GameContentResponse,
    date: &str,
    cdn: &str,
    quality: &Option<Quality>,
) -> GameData {
    let game_data = Mutex::new(game_data);

    let tasks: Vec<_> = game_content
        .media
        .epg
        .into_iter()
        .map(|epg| {
            async {
                if epg.title == "NHLTV" {
                    if let Some(ref items) = epg.items {
                        let mut streams = vec![];
                        for item in items {
                            let url = format!(
                                "{}/getM3U8.php?league=nhl&date={}&id={}&cdn={}",
                                HOST, &date, &item.media_playback_id, cdn,
                            );

                            if let Ok(master_url) = get_master_url(&url).await {
                                if let Some(quality) = quality {
                                    if let Ok(master_m3u8) = get_master_m3u8(&master_url).await {
                                        if let Ok(quality_url) = get_quality_url(
                                            &master_url,
                                            &master_m3u8,
                                            quality.clone(),
                                        ) {
                                            streams
                                                .push((item.media_feed_type.clone(), quality_url));
                                        }
                                    }
                                } else {
                                    streams.push((item.media_feed_type.clone(), master_url));
                                }
                            }
                        }

                        for (feed_type, url) in streams {
                            let stream = Stream { feed_type, url };
                            game_data.lock().await.streams.push(stream);
                        }
                    }
                }
                drop(epg);
            }
        })
        .collect();

    join_all(tasks).await;

    game_data.into_inner()
}

async fn create_playlist(
    path: PathBuf,
    games: Vec<GameData>,
    xmltv: bool,
    start_channel: u32,
) -> Result<(), Error> {
    let mut m3u = String::new();
    m3u.push_str("#EXTM3U\n");

    let mut id: u32 = 0;
    for game in games.iter() {
        for stream in game.streams.iter() {
            let title = if xmltv {
                format!("Lazyman {}", id + 1)
            } else {
                format!(
                    "{} {} @ {} {}",
                    game.date
                        .with_timezone(&Local)
                        .time()
                        .format("%-I:%M %p")
                        .to_string(),
                    game.away,
                    game.home,
                    stream.feed_type,
                )
            };

            let record = format!(
                "#EXTINF:-1 CUID=\"{}\" tvg-id=\"{}\" tvg-name=\"Lazyman {}\",{}\n{}\n",
                start_channel + id,
                start_channel + id,
                id + 1,
                title,
                stream.url
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
    games: Vec<GameData>,
    start_channel: u32,
) -> Result<(), Error> {
    let mut xmltv = String::new();
    xmltv.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE tv SYSTEM \"xmltv.dd\">\
         \n\
         \n  <tv generator-info-name=\"lazystream\" source-info-name=\"lazystream - {}\">",
        VERSION
    ));

    let mut id: u32 = 0;
    while id < 100 {
        let record = format!(
            "\n    <channel id=\"{}\">\
             \n      <display-name>Lazyman {}</display-name>\
             \n      <icon src=\"http://home.windstream.net/dgrodecki/images/nhl/nhl_logo2.jpg\"></icon>\
             \n    </channel>",
            start_channel + id,
            id + 1
        );
        xmltv.push_str(&record);
        id += 1;
    }

    let mut id: u32 = 0;
    for game in games.iter() {
        let icons = if let Some(ref game_cuts) = game.cuts {
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

        for stream in game.streams.iter() {
            let start = Local::now();
            let stop = Local::now();
            let description = game.description.clone().unwrap_or_else(|| "".into());
            let title = format!(
                "{} {} {} @ {}",
                game.date
                    .with_timezone(&Local)
                    .time()
                    .format("%-I:%M %p")
                    .to_string(),
                stream.feed_type,
                game.away,
                game.home,
            );

            let record = format!(
                "\n    <programme channel=\"{}\" start=\"{}000000 {}\" stop=\"{}235959 {}\">\
                 \n      <title lang=\"en\">{}</title>\
                 \n      <desc lang=\"en\">{}</desc>\
                 {}\
                 \n    </programme>",
                start_channel + id,
                start.format("%Y%m%d"),
                start.format("%z"),
                stop.format("%Y%m%d"),
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

#[derive(Debug, Clone)]
struct GameData {
    home: String,
    away: String,
    description: Option<String>,
    date: DateTime<Utc>,
    streams: Vec<Stream>,
    cuts: Option<stats_api::model::GameContentArticleMediaImageCut>,
}

#[derive(Debug, Clone)]
struct Stream {
    feed_type: String,
    url: String,
}

impl GameData {
    fn new(game: &stats_api::model::ScheduleGame) -> Self {
        let home = game.teams.home.detail.name.clone();
        let away = game.teams.away.detail.name.clone();
        let date = game.date;
        let streams = vec![];

        GameData {
            home,
            away,
            description: None,
            date,
            streams,
            cuts: None,
        }
    }
}
