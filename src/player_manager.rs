use axum::{extract::FromRequestParts, http::StatusCode};
use tower_sessions::Session;

use crate::{get_player_by_id, MemeTemplate, MemeTemplatePool, Player, MEME_TEMPLATE_POOL, PLAYERS_IN_LOBBY};

pub struct PlayerManager(Session);

impl PlayerManager {
    const PLAYER_ID_KEY: &'static str = "player.id";

    pub async fn sign_in_as_guest(&self, name: String) {
        let player = Player::new_guest(name);
        self.0.insert(PlayerManager::PLAYER_ID_KEY, player.id.clone()).await.unwrap();
        PLAYERS_IN_LOBBY.lock().unwrap().insert(player.id.clone(), player);
    }

    pub async fn get_logged_in_player_id(&self) -> Result<String, (StatusCode, &'static str)> {
        let player_id: String = self.0.get(PlayerManager::PLAYER_ID_KEY).await.unwrap().ok_or((StatusCode::BAD_REQUEST, "player not found"))?;
        Ok(player_id)
    }

    pub async fn get_logged_in_player(&self) -> Result<Player, (StatusCode, &'static str)> {
        let logged_in_player_id = self.get_logged_in_player_id().await?;
        let logged_in_player = get_player_by_id(logged_in_player_id).ok_or((StatusCode::BAD_REQUEST, "player not found"))?;
        Ok(logged_in_player)
    }

    pub async fn get_meme_template_for_user(&self) -> MemeTemplate {
        let meme_template_id_for_user = self.get_meme_template_id_for_user().await;
        let meme_template = MEME_TEMPLATE_POOL.lock().unwrap().templates.iter().find(|meme_template| meme_template.id == meme_template_id_for_user).cloned().unwrap();
        meme_template
    }

    pub async fn get_meme_template_id_for_user(&self) -> String {
        // TODO: make sure different users get different memes through pool manipulation or something
        let logged_in_player: Player = self.get_logged_in_player().await.unwrap();

        let meme_template_id_for_user = logged_in_player.selected_meme_template_id.unwrap_or_else(|| {
            let template_image_id = MEME_TEMPLATE_POOL.lock().unwrap().pick_id().unwrap();

            // todo: move this over to playermanager
            PLAYERS_IN_LOBBY.lock().unwrap().get_mut(&logged_in_player.id).unwrap().selected_meme_template_id = Some(template_image_id.clone());
            template_image_id
        });
        meme_template_id_for_user
    }

    pub async fn reroll_meme_template(&self) {
        // TODO: dont choose the same meme again!
        let logged_in_player_id = self.get_logged_in_player_id().await.unwrap();

        let mut players = PLAYERS_IN_LOBBY.lock().unwrap();
        let logged_in_player = players.get_mut(&logged_in_player_id).unwrap();
        logged_in_player.seen_meme_templates.push(logged_in_player.selected_meme_template_id.clone().unwrap());

        let template_image_uri = MEME_TEMPLATE_POOL.lock().unwrap().pick_omit(&logged_in_player.seen_meme_templates).unwrap();
        logged_in_player.selected_meme_template_id = Some(template_image_uri.clone());
    }
}

impl<S> FromRequestParts<S> for PlayerManager
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async  fn from_request_parts(parts: &mut axum::http::request::Parts,state: &S,) -> Result<Self,Self::Rejection> {
        let session: Session = Session::from_request_parts(parts, state).await?;
        Ok(PlayerManager(session))
    }
}
