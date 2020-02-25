use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct Team {
    pub id: u32,
    pub name: String,
    pub link: String,
    pub abbreviation: String,
    pub team_name: String,
    pub location_name: Option<String>,
    pub first_year_of_play: Option<String>,
    pub short_name: String,
    pub active: bool,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleResponse {
    pub dates: Vec<Schedule>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct Schedule {
    pub date: NaiveDate,
    pub games: Vec<ScheduleGame>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleGame {
    pub game_pk: u64,
    pub link: String,
    pub date: DateTime<Utc>,
    pub game_type: String,
    pub season: String,
    pub teams: ScheduleGameTeams,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleGameTeams {
    pub away: ScheduleGameTeam,
    pub home: ScheduleGameTeam,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleGameTeam {
    pub score: Option<u8>,
    pub detail: ScheduleGameTeamDetail,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleGameTeamDetail {
    pub id: u32,
    pub name: String,
    pub link: String,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentResponse {
    pub editorial: GameContentEditorial,
    pub media: GameContentMedia,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentMedia {
    pub epg: Option<Vec<GameContentEpg>>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentEpg {
    pub title: String,
    pub items: Option<Vec<GameContentEpgItem>>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentEpgItem {
    pub media_feed_type: Option<String>,
    pub call_letters: Option<String>,
    pub media_state: Option<String>,
    pub id: Option<u32>,
    pub media_playback_id: Option<String>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentEditorial {
    #[serde(deserialize_with = "fail_as_none")]
    pub preview: Option<GameContentEditorialItem>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentEditorialItem {
    pub title: String,
    pub items: Option<Vec<GameContentEditorialItemArticle>>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentEditorialItemArticle {
    pub r#type: String,
    pub headline: String,
    pub subhead: String,
    pub seo_title: String,
    pub seo_description: String,
    #[serde(deserialize_with = "fail_as_none")]
    pub media: Option<GameContentArticleMedia>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentArticleMedia {
    pub r#type: String,
    pub image: GameContentArticleMediaImage,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentArticleMediaImage {
    pub cuts: GameContentArticleMediaImageCut,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentArticleMediaImageCut {
    pub cut_2208_1242: GameContentArticleMediaImageCutDetail,
    pub cut_2048_1152: GameContentArticleMediaImageCutDetail,
    pub cut_1704_960: GameContentArticleMediaImageCutDetail,
    pub cut_1536_864: GameContentArticleMediaImageCutDetail,
    pub cut_1284_722: GameContentArticleMediaImageCutDetail,
    pub cut_1136_640: GameContentArticleMediaImageCutDetail,
    pub cut_1024_576: GameContentArticleMediaImageCutDetail,
    pub cut_960_540: GameContentArticleMediaImageCutDetail,
    pub cut_768_432: GameContentArticleMediaImageCutDetail,
    pub cut_640_360: GameContentArticleMediaImageCutDetail,
    pub cut_568_320: GameContentArticleMediaImageCutDetail,
    pub cut_372_210: GameContentArticleMediaImageCutDetail,
    pub cut_320_180: GameContentArticleMediaImageCutDetail,
    pub cut_248_140: GameContentArticleMediaImageCutDetail,
    pub cut_124_70: GameContentArticleMediaImageCutDetail,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Clone)]
pub struct GameContentArticleMediaImageCutDetail {
    pub aspect_ratio: String,
    pub width: u32,
    pub height: u32,
    pub src: String,
}

fn fail_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let result = T::deserialize(de);
    match result {
        Ok(t) => Ok(Some(t)),
        Err(_) => Ok(None),
    }
}
