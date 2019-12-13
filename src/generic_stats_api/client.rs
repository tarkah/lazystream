use super::model::*;
use crate::opt::Sport;
use failure::Error;
use stats_api::{MlbClient, NhlClient};

pub struct Client {
    mlb: MlbClient,
    nhl: NhlClient,
    sport: Sport,
}

impl Client {
    pub fn new(sport: &Sport) -> Self {
        let mlb = MlbClient::default();
        let nhl = NhlClient::default();

        Client {
            mlb,
            nhl,
            sport: sport.clone(),
        }
    }

    pub async fn get_schedule_for(&self, date: chrono::NaiveDate) -> Result<Schedule, Error> {
        let serialized = match &self.sport {
            Sport::Mlb => {
                let schedule = self.mlb.get_schedule_for(date).await?;
                serde_json::to_vec(&schedule)?
            }
            Sport::Nhl => {
                let schedule = self.nhl.get_schedule_for(date).await?;
                serde_json::to_vec(&schedule)?
            }
        };

        let schedule = serde_json::from_slice(&serialized)?;
        Ok(schedule)
    }

    pub async fn get_game_content(&self, game_pk: u64) -> Result<GameContentResponse, Error> {
        let serialized = match &self.sport {
            Sport::Mlb => {
                let game_content = self.mlb.get_game_content(game_pk).await?;
                serde_json::to_vec(&game_content)?
            }
            Sport::Nhl => {
                let game_content = self.nhl.get_game_content(game_pk).await?;
                serde_json::to_vec(&game_content)?
            }
        };

        let game_content = serde_json::from_slice(&serialized)?;
        Ok(game_content)
    }
}
