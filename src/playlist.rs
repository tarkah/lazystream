use crate::{log_error, HOST};
use async_std::{fs, sync::Mutex, task};
use chrono::{DateTime, Local, Utc};
use failure::{bail, Error, ResultExt};
use futures::{future, AsyncReadExt};
use http_client::{native::NativeClient, Body, HttpClient};
use std::{path::PathBuf, process};

pub fn run(path: PathBuf) {
    task::block_on(async {
        if let Err(e) = process(path).await {
            log_error(&e);
            process::exit(1);
        };
    });
}

async fn process(path: PathBuf) -> Result<(), Error> {
    if let Some(extension) = path.extension() {
        if extension != "m3u" {
            bail!("Playlist file extension must be '.m3u'");
        }
    } else {
        bail!("Playlist file extension must be '.m3u'");
    }

    println!("Creating playlist...");

    let client = stats_api::Client::new();

    let today = Local::today().naive_local();
    let todays_schedule = client.get_schedule_for(today).await?;

    let mut games = vec![];
    for game in todays_schedule.games {
        let mut game_data = GameData::new(&game);

        let game_content = client.get_game_content(game.game_pk).await?;

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

    create_playlist(path, games).await?;

    Ok(())
}

async fn create_playlist(path: PathBuf, games: Vec<GameData>) -> Result<(), Error> {
    let mut records = vec![];
    for game in games {
        for stream in game.streams {
            let title = format!(
                "{} @ {} {} {}",
                game.away,
                game.home,
                game.date
                    .with_timezone(&Local)
                    .time()
                    .format("%-I:%M %p")
                    .to_string(),
                stream.feed_type
            );

            let record = format!("#EXTINF:-1,{}\n{}\n", title, stream.url);
            records.push(record);
        }
    }

    let mut m3u = String::new();
    m3u.push_str("#EXTM3U\n");
    for record in records {
        m3u.push_str(&record);
    }

    fs::write(&path, m3u).await?;

    println!("Playlist saved to: {:?}", path);

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

#[derive(Debug)]
struct GameData {
    home: String,
    away: String,
    date: DateTime<Utc>,
    streams: Vec<Stream>,
}

#[derive(Debug)]
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
            date,
            streams,
        }
    }
}
