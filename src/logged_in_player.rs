use axum::{extract::FromRequestParts, http::StatusCode};

use crate::{player_manager::PlayerManager, Player};

pub struct LoggedInPlayer(pub Player);

impl<S> FromRequestParts<S> for LoggedInPlayer
where
    S: Send + Sync,
{

    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut axum::http::request::Parts,state: &S,) -> Result<Self, Self::Rejection> {
        let player_manager = PlayerManager::from_request_parts(parts, state).await?;

        let player = player_manager.get_logged_in_player().await?;

        let logged_in_player: LoggedInPlayer = LoggedInPlayer(player);
        Ok(logged_in_player)
    }
}
