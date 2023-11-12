use std::collections::HashMap;

use reqwest::header;
use serde::{Deserialize, Serialize};

pub struct CTFdClient {
    client: reqwest::Client,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIResponse<T> {
    pub success: bool,
    pub errors: Option<Vec<String>>,
    pub data: Option<T>,
}

#[derive(Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Challenge {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ChallengeSolver {
    pub account_id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreboardEntry{
    pub id: i64,
    pub name: String,
}



pub(crate) type TeamId = i64;
pub(crate) type TeamPosition = i64;

impl CTFdClient {
    pub fn new(url: String, api_key: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Content-Type", header::HeaderValue::from_static("application/json"));

        let auth_value = format!("Token {}", api_key);
        headers.insert("Authorization", header::HeaderValue::from_str(&auth_value).unwrap());

        Self {
            client: reqwest::Client::builder().default_headers(headers).build().unwrap(),
            url,
        }
    }

    pub async fn get_challenges(&self) -> Result<Vec<Challenge>, reqwest::Error> {
        let url = format!("{}/api/v1/challenges", self.url);
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<APIResponse<Vec<Challenge>>>()
            .await?;

        Ok(response.data.unwrap())
    }

    pub async fn get_team(&self, team_id: i64) -> Result<Team, reqwest::Error>{
        let url = format!("{}/api/v1/teams/{}", self.url, team_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<APIResponse<Team>>()
            .await?;

        Ok(response.data.unwrap())
    }

    pub async fn get_user(&self, team_id: i64) -> Result<Team, reqwest::Error>{
        let url = format!("{}/api/v1/users/{}", self.url, team_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<APIResponse<Team>>()
            .await?;

        Ok(response.data.unwrap())
    }

    pub async fn get_top_10_teams(&self) -> Result<HashMap<TeamId, TeamPosition>, reqwest::Error> {
        let url = format!("{}/api/v1/scoreboard/top/10", self.url);
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<APIResponse<HashMap<i64, ScoreboardEntry>>>()
            .await?;

        let mut teams = HashMap::new();

        for (i, team) in response.data.unwrap() {
            teams.insert(team.id, i);
        }

        Ok(teams)
    }
}

impl Challenge {
    pub async fn get_solves(&self, client: &CTFdClient) -> Result<Vec<ChallengeSolver>, reqwest::Error> {
        let url = format!("{}/api/v1/challenges/{}/solves", client.url, self.id);
        let response = client
            .client
            .get(&url)
            .send()
            .await?
            .json::<APIResponse<Vec<ChallengeSolver>>>()
            .await?;

        Ok(response.data.unwrap())
    }
}
