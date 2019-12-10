use crate::opt::Opt;
use crate::{log_error, HOST};
use async_std::{fs, sync::Mutex, task};
use chrono::{DateTime, Local, Utc};
use failure::{bail, Error, ResultExt};
use futures::{future, AsyncReadExt};
use http_client::{native::NativeClient, Body, HttpClient};
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

    let client = stats_api::Client::new();

    let date = if opts.date.is_some() {
        opts.date.unwrap()
    } else {
        Local::today().naive_local()
    };
    let todays_schedule = client.get_schedule_for(date).await?;

    let mut games = vec![];
    for game in todays_schedule.games {
        let mut game_data = GameData::new(&game);

        let game_content = client.get_game_content(game.game_pk).await?;

        let preview_items = game_content.editorial.preview.items;
        if let Some(items) = preview_items {
            if let Some(preview) = items.first() {
                game_data.description = Some(preview.subhead.clone());
            }
        }

        for epg in game_content.media.epg {
            if epg.title == "NHLTV" {
                if let Some(items) = epg.items {
                    let client = NativeClient::default();
                    let date = todays_schedule.date.format("%Y-%m-%d");

                    let streams = Mutex::new(vec![]);
                    let tasks = items
                        .into_iter()
                        .map(|stream| {
                            async {
                                let url = format!(
                                    "{}/getM3U8.php?league=nhl&date={}&id={}&cdn=akc",
                                    HOST, &date, &stream.media_playback_id
                                );

                                if let Ok(m3u8) = get_m3u8(&client, url).await {
                                    let mut streams = streams.lock().await;
                                    streams.push((stream.media_feed_type, m3u8));
                                };
                            }
                        })
                        .collect::<Vec<_>>();

                    future::join_all(tasks).await;

                    let streams = streams.lock().await.clone();

                    for (feed_type, url) in streams {
                        let stream = Stream { feed_type, url };
                        game_data.streams.push(stream);
                    }
                }
            }
        }

        games.push(game_data);
    }

    if let Some(path) = opts.xmltv_output {
        let path = path.with_extension("m3u");
        create_playlist(path.clone(), games.clone(), true).await?;

        let path = path.with_extension("xml");
        create_xmltv(path, games).await?;
    } else if let Some(path) = opts.playlist_output {
        let path = path.with_extension("m3u");
        create_playlist(path, games, false).await?;
    }

    Ok(())
}

async fn create_playlist(path: PathBuf, games: Vec<GameData>, xmltv: bool) -> Result<(), Error> {
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
                1000 + id,
                1000 + id,
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

async fn create_xmltv(path: PathBuf, games: Vec<GameData>) -> Result<(), Error> {
    let mut xmltv = String::new();
    xmltv.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE tv SYSTEM \"xmltv.dd\">\
         \n\
         \n  <tv generator-info-name=\"lazystream\" source-info-name=\"lazystream - 1.3.0\">",
    );

    let mut id: u32 = 0;
    while id < 100 {
        let record = format!(
            "\n    <channel id=\"{}\">\
             \n      <display-name>Lazyman {}</display-name>\
             \n      <icon src=\"\"></icon>\
             \n    </channel>",
            1000 + id,
            id + 1
        );
        xmltv.push_str(&record);
        id += 1;
    }

    let mut id: u32 = 0;
    for game in games.iter() {
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
                 \n      <icon src=\"\"></icon>\
                 \n    </programme>",
                1000 + id,
                start.format("%Y%m%d"),
                start.format("%:z"),
                stop.format("%Y%m%d"),
                stop.format("%:z"),
                title,
                description,
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

async fn get_m3u8(client: &NativeClient, url: String) -> Result<String, Error> {
    let uri = url.parse::<http::Uri>().context("Failed to build URI")?;
    let request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let resp = client.send(request).await?;

    let mut body = resp.into_body();
    let mut body_text = String::new();
    body.read_to_string(&mut body_text)
        .await
        .context("Failed to read response body text")?;

    if !&body_text[..].starts_with("https") {
        bail!("Game hasn't started");
    }

    Ok(body_text)
}

#[derive(Debug, Clone)]
struct GameData {
    home: String,
    away: String,
    description: Option<String>,
    date: DateTime<Utc>,
    streams: Vec<Stream>,
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
        }
    }
}
