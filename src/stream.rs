use crate::{
    generic_stats_api::client::Client,
    generic_stats_api::model::{
        GameContentArticleMediaImageCut, GameContentEditorialItem, GameContentResponse, Team,
    },
    opt::{Cdn, FeedType, Opt, Quality, Sport},
    HOST,
};
use chrono::{DateTime, Local, NaiveDate, Utc};
use failure::{bail, format_err, Error, ResultExt};
use futures::{future, AsyncReadExt};
use http_client::{native::NativeClient, Body, HttpClient};
use std::{collections::HashMap, str::FromStr};

pub struct LazyStream {
    pub opts: Opt,
    games: Vec<Game>,
    teams: Vec<Team>,
}

impl LazyStream {
    pub async fn new(opts: &Opt) -> Result<Self, Error> {
        let date = if opts.date.is_some() {
            opts.date.clone().unwrap()
        } else {
            Local::today().naive_local()
        };

        let client = Client::new(&opts.sport);
        let schedule = client.get_schedule_for(date).await?;
        let teams = client.get_teams().await?;

        let mut games = vec![];
        for game in schedule.games {
            let game_pk = game.game_pk;
            let game_date = game.date;
            let home_team = teams
                .iter()
                .find(|team| team.id == game.teams.home.detail.id)
                .unwrap();
            let away_team = teams
                .iter()
                .find(|team| team.id == game.teams.away.detail.id)
                .unwrap();

            let game = Game::new(
                opts.sport.clone(),
                game_pk,
                game_date,
                date,
                home_team.clone(),
                away_team.clone(),
            );
            games.push(game);
        }

        Ok(LazyStream {
            opts: opts.clone(),
            games,
            teams,
        })
    }

    pub fn date(&self) -> NaiveDate {
        if self.opts.date.is_some() {
            self.opts.date.clone().unwrap()
        } else {
            Local::today().naive_local()
        }
    }

    pub fn games(&self) -> Vec<Game> {
        self.games.clone()
    }

    pub fn check_team_abbrev(&self, team_abbrev: &str) -> Result<(), Error> {
        if self
            .teams
            .iter()
            .any(|team| team.abbreviation == team_abbrev)
        {
            Ok(())
        } else {
            bail!("Team abbreviation {} does not exist", team_abbrev);
        }
    }

    pub fn game_with_team_abbrev(&self, team_abbrev: &str) -> Option<Game> {
        let game_idx = self.games.iter().position(|game| {
            game.home_team.abbreviation == team_abbrev || game.away_team.abbreviation == team_abbrev
        });
        if let Some(index) = game_idx {
            Some(self.games[index].clone())
        } else {
            None
        }
    }

    #[allow(clippy::drop_ref)]
    pub async fn resolve_with_master_link(&mut self, cdn: &Cdn) {
        let tasks: Vec<_> = self
            .games
            .iter_mut()
            .map(|game| {
                async {
                    game.resolve_streams_master_link(cdn).await;
                    drop(game);
                }
            })
            .collect();

        future::join_all(tasks).await;
    }

    #[allow(clippy::drop_ref)]
    pub async fn resolve_with_quality_link(&mut self, cdn: &Cdn, quality: &Quality) {
        let tasks: Vec<_> = self
            .games
            .iter_mut()
            .map(|game| {
                async {
                    game.resolve_streams_quality_link(cdn, quality).await;
                    drop(game);
                }
            })
            .collect();

        future::join_all(tasks).await;
    }
}

#[derive(Clone)]
pub struct Game {
    sport: Sport,
    pub game_pk: u64,
    pub game_date: DateTime<Utc>,
    pub selected_date: NaiveDate,
    pub streams: Option<HashMap<FeedType, Stream>>,
    pub home_team: Team,
    pub away_team: Team,
    pub game_content: Option<GameContentResponse>,
}

impl Game {
    fn new(
        sport: Sport,
        game_pk: u64,
        game_date: DateTime<Utc>,
        selected_date: NaiveDate,
        home_team: Team,
        away_team: Team,
    ) -> Self {
        Game {
            sport,
            game_pk,
            game_date,
            selected_date,
            streams: None,
            home_team,
            away_team,
            game_content: None,
        }
    }

    pub async fn streams(&mut self) -> Result<HashMap<FeedType, Stream>, Error> {
        if self.streams.is_none() {
            let mut streams = HashMap::new();
            let game_content = self.game_content().await?;

            if let Some(epg) = game_content.media.epg {
                for epg in epg {
                    if epg.title == "NHLTV" || epg.title == "MLBTV" {
                        if let Some(items) = epg.items {
                            for item in items {
                                if let Some(feed_type) = item.media_feed_type {
                                    let id = match self.sport {
                                        Sport::Mlb => format!("{}", item.id.unwrap()),
                                        Sport::Nhl => item.media_playback_id.unwrap(),
                                    };
                                    let feed_type = FeedType::from_str(feed_type.as_str())?;

                                    let stream = Stream::new(
                                        id,
                                        feed_type.clone(),
                                        self.game_date,
                                        self.selected_date,
                                    );
                                    streams.insert(feed_type, stream);
                                }
                            }
                        }
                    }
                }
            }
            self.streams = Some(streams.clone());
            Ok(streams)
        } else {
            Ok(self.streams.clone().unwrap())
        }
    }

    pub async fn game_content(&mut self) -> Result<GameContentResponse, Error> {
        if self.game_content.is_none() {
            let client = Client::new(&self.sport);
            let game_content = client.get_game_content(self.game_pk).await?;
            self.game_content = Some(game_content.clone());
            Ok(game_content)
        } else {
            Ok(self.game_content.clone().unwrap())
        }
    }

    pub async fn game_cuts(&mut self) -> Option<GameContentArticleMediaImageCut> {
        let game_content = self.game_content().await.ok()?;

        if let Some(GameContentEditorialItem {
            items: Some(items), ..
        }) = game_content.editorial.preview
        {
            let item = items.get(0)?;

            if let Some(media) = item.media.clone() {
                return Some(media.image.cuts);
            }
        }
        None
    }

    pub async fn description(&mut self) -> Option<String> {
        let game_content = self.game_content().await.ok()?;

        if let Some(GameContentEditorialItem {
            items: Some(items), ..
        }) = game_content.editorial.preview
        {
            let item = &items.get(0)?;

            return Some(item.subhead.clone());
        }
        None
    }

    pub async fn stream_with_feed_or_default(
        &mut self,
        feed_type: &Option<FeedType>,
        team_abbrev: &str,
    ) -> Result<Stream, Error> {
        let mut streams = if self.streams.is_none() {
            self.streams().await?
        } else {
            self.streams.clone().unwrap()
        };

        let mut feed_type: FeedType = if let Some(feed_type) = feed_type {
            feed_type.clone()
        } else if self.home_team.abbreviation == team_abbrev {
            FeedType::Home
        } else {
            FeedType::Away
        };

        if !streams.contains_key(&feed_type) {
            if streams.contains_key(&FeedType::National) {
                feed_type = FeedType::National
            } else if streams.contains_key(&FeedType::Home) {
                feed_type = FeedType::Home
            } else {
                feed_type = FeedType::Away
            }
        }

        if let Some(stream) = streams.remove(&feed_type) {
            Ok(stream)
        } else {
            bail!("Couldn't find any ");
        }
    }

    async fn resolve_streams(&mut self) {
        let _ = self.streams().await;
    }

    #[allow(clippy::drop_ref)]
    async fn resolve_streams_master_link(&mut self, cdn: &Cdn) {
        if self.streams.is_none() {
            self.resolve_streams().await;
        }

        let tasks: Vec<_> = self
            .streams
            .as_mut()
            .unwrap()
            .iter_mut()
            .map(|(_, stream)| {
                async {
                    stream.resolve_master_link(cdn).await;
                    drop(stream);
                }
            })
            .collect();

        future::join_all(tasks).await;
    }

    #[allow(clippy::drop_ref)]
    async fn resolve_streams_quality_link(&mut self, cdn: &Cdn, quality: &Quality) {
        if self.streams.is_none() {
            self.resolve_streams().await;
        }
        let tasks: Vec<_> = self
            .streams
            .as_mut()
            .unwrap()
            .iter_mut()
            .map(|(_, stream)| {
                async {
                    stream.resolve_quality_link(cdn, quality).await;
                    drop(stream);
                }
            })
            .collect();

        future::join_all(tasks).await;
    }
}

#[derive(Clone)]
#[allow(clippy::option_option)]
pub struct Stream {
    id: String,
    pub feed_type: FeedType,
    game_date: DateTime<Utc>,
    selected_date: NaiveDate,
    master_link: Option<Option<String>>,
    master_m3u8: Option<String>,
    quality_link: Option<Option<String>>,
}

impl Stream {
    fn new(
        id: String,
        feed_type: FeedType,
        game_date: DateTime<Utc>,
        selected_date: NaiveDate,
    ) -> Self {
        Stream {
            id,
            feed_type,
            game_date,
            selected_date,
            master_link: None,
            master_m3u8: None,
            quality_link: None,
        }
    }

    pub fn host_link(&self, cdn: &Cdn) -> String {
        format!(
            "{}/getM3U8.php?league=nhl&date={}&id={}&cdn={}",
            HOST,
            self.selected_date.format("%Y-%m-%d"),
            self.id,
            cdn,
        )
    }

    pub async fn master_link(&mut self, cdn: &Cdn) -> Result<String, Error> {
        if self.master_link.is_none() {
            match get_master_link(&self.host_link(cdn)).await {
                Ok(master_link) => {
                    self.master_link = Some(Some(master_link.clone()));
                    Ok(master_link)
                }
                Err(e) => {
                    self.master_link = Some(None);
                    bail!(e);
                }
            }
        } else if let Some(master_link) = self.master_link.clone().unwrap() {
            Ok(master_link)
        } else {
            bail!("Master link is not avaialable");
        }
    }

    pub async fn quality_link(&mut self, cdn: &Cdn, quality: &Quality) -> Result<String, Error> {
        if self.quality_link.is_none() {
            if self.master_m3u8.is_none() {
                if let Ok(master_link) = self.master_link(cdn).await {
                    match get_master_m3u8(&master_link).await {
                        Err(e) => {
                            self.quality_link = Some(None);
                            bail!(e);
                        }
                        Ok(master_m3u8) => {
                            self.master_m3u8 = Some(master_m3u8);
                        }
                    }
                } else {
                    self.quality_link = Some(None);
                    bail!("Master link not available yet");
                }
            }
            let master_link = self.master_link.as_ref().unwrap().as_ref().unwrap();
            let master_m3u8 = self.master_m3u8.as_ref().unwrap();

            if let Ok(quality_link) = get_quality_link(master_link, master_m3u8, quality) {
                self.quality_link = Some(Some(quality_link.clone()));
                Ok(quality_link)
            } else {
                self.quality_link = Some(None);
                bail!("Link doesn't exist for specified quality");
            }
        } else if let Some(quality_link) = self.quality_link.clone().unwrap() {
            Ok(quality_link)
        } else {
            bail!("Could not get master m3u8 to build quality link");
        }
    }

    async fn resolve_master_link(&mut self, cdn: &Cdn) {
        let _ = self.master_link(cdn).await;
    }

    async fn resolve_quality_link(&mut self, cdn: &Cdn, quality: &Quality) {
        let _ = self.quality_link(cdn, quality).await;
    }
}

async fn get_master_link(url: &str) -> Result<String, Error> {
    let uri = url.parse::<http::Uri>().context("Failed to build URI")?;
    let request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let client = NativeClient::default();
    let resp = client.send(request).await?;

    let mut body = resp.into_body();
    let mut body_text = String::new();
    body.read_to_string(&mut body_text)
        .await
        .context("Failed to read response body text")?;

    if !&body_text[..].starts_with("https") {
        bail!("Stream not available yet");
    }

    Ok(body_text)
}

async fn get_master_m3u8(url: &str) -> Result<String, Error> {
    let uri = url.parse::<http::Uri>().context("Failed to build URI")?;
    let request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let client = NativeClient::default();
    let resp = client.send(request).await?;

    let mut body = resp.into_body();
    let mut body_text = String::new();
    body.read_to_string(&mut body_text)
        .await
        .context("Failed to read response body text")?;

    if body_text[..].starts_with("#EXTM3U") {
        return Ok(body_text);
    }

    bail!("Failed to get master m3u8");
}

fn get_quality_link(
    master_link: &str,
    master_m3u8: &str,
    quality: &Quality,
) -> Result<String, Error> {
    let quality_str: &str = quality.clone().into();
    let quality_check = format!("x{}", quality_str);

    let mut quality_idx = None;
    for (idx, line) in master_m3u8.lines().enumerate() {
        if (quality == &Quality::_720p60 && line.contains("FRAME-RATE"))
            || (quality != &Quality::_720p60 && line.contains(&quality_check))
        {
            quality_idx = Some(idx + 1);
        }
    }

    if let Some(idx) = quality_idx {
        let quality_line = master_m3u8
            .lines()
            .nth(idx)
            .ok_or_else(|| format_err!("No stream found matching quality specified"))?;

        let master_link_parts = master_link.rsplitn(2, '/').collect::<Vec<&str>>();
        if master_link_parts.len() == 2 {
            let quality_link = format!("{}/{}", master_link_parts[1], quality_line);

            return Ok(quality_link);
        }
    }

    bail!("No stream found matching quality specified");
}
