use std::sync::Arc;

use axum::extract::FromRequestParts;
use crate::{app_error::AppError, app_state::AppState, player_manager::PlayerManager, Player};

pub struct LoggedInPlayer(pub Player);

impl FromRequestParts<Arc<AppState>> for LoggedInPlayer
{

    type Rejection = AppError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts,state: &Arc<AppState>,) -> Result<Self, Self::Rejection> {
        let player_manager = PlayerManager::from_request_parts(parts, state).await?;

        let player = player_manager.get_logged_in_player().await?;

        let logged_in_player: LoggedInPlayer = LoggedInPlayer(player);
        Ok(logged_in_player)
    }
}
