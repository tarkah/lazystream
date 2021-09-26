use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub link: String,
    pub abbreviation: Option<String>,
    #[serde(default)]
    pub team_name: String,
    pub location_name: Option<String>,
    pub first_year_of_play: Option<String>,
    pub short_name: Option<String>,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    #[serde(default)]
    pub dates: Vec<Schedule>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub date: NaiveDate,
    #[serde(default)]
    pub games: Vec<ScheduleGame>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGame {
    pub game_pk: u64,
    #[serde(default)]
    pub link: String,
    pub date: DateTime<Utc>,
    #[serde(default)]
    pub game_type: String,
    #[serde(default)]
    pub season: String,
    pub teams: ScheduleGameTeams,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGameTeams {
    pub away: ScheduleGameTeam,
    pub home: ScheduleGameTeam,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGameTeam {
    pub score: Option<u8>,
    pub detail: ScheduleGameTeamDetail,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGameTeamDetail {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub link: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentResponse {
    pub editorial: GameContentEditorial,
    pub media: GameContentMedia,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentMedia {
    pub epg: Option<Vec<GameContentEpg>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentEpg {
    #[serde(default)]
    pub title: String,
    pub items: Option<Vec<GameContentEpgItem>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentEpgItem {
    pub media_feed_type: Option<String>,
    pub call_letters: Option<String>,
    pub media_state: Option<String>,
    pub id: Option<u32>,
    pub media_playback_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentEditorial {
    #[serde(deserialize_with = "fail_as_none")]
    pub preview: Option<GameContentEditorialItem>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentEditorialItem {
    #[serde(default)]
    pub title: String,
    pub items: Option<Vec<GameContentEditorialItemArticle>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentEditorialItemArticle {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub subhead: String,
    #[serde(default)]
    pub seo_title: String,
    #[serde(default)]
    pub seo_description: String,
    #[serde(deserialize_with = "fail_as_none")]
    pub media: Option<GameContentArticleMedia>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentArticleMedia {
    #[serde(default)]
    pub r#type: String,
    pub image: GameContentArticleMediaImage,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentArticleMediaImage {
    pub cuts: GameContentArticleMediaImageCut,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameContentArticleMediaImageCutDetail {
    #[serde(default)]
    pub aspect_ratio: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
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
