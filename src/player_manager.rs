use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::FromRequestParts;
use sqlx::{error::{DatabaseError, ErrorKind}, sqlite::SqliteError, Sqlite, SqlitePool};
use tower_sessions::Session;

use crate::{app_error::AppError, app_state::AppState, MemeTemplate, Player};

pub struct PlayerManager {
    session: Session,
    database_connection_pool: sqlx::Pool<Sqlite>
}

async fn select_new_meme_template(database_connection_pool: &sqlx::Pool<Sqlite>, player_id: i32) -> anyhow::Result<Option<i32>> {
    // TODO: exclude meme templaces being labelled by other users and the previous one!
    // selects meme_templates being currently labelled by users
    // also includes the meme_template being labelled by the current user
    /* let _ = sqlx::query_scalar("SELECT (id) FROM (meme_templates INNER JOIN players ON players.selected_meme_template = meme_templates.id)")
        .fetch_all(&pool)
        .await?; */
    // old version without excluding meme templates
    /* let new_meme_template_id: i32 = sqlx::query_scalar("SELECT (id) FROM meme_templates ORDER BY RANDOM() LIMIT 1")
        .fetch_one(&self.database_connection_pool)
        .await?; */

    // but we also need to exclude meme_templates that were previously seen by the current user!
    // achieve this through a union (ALL???? performance? dunno) in the except clause!

    // due to technical reasons we need a subquery
    // https://stackoverflow.com/questions/31500558/sqlite-except-order-by-random
    // would panic when using fetch_one when no meme template can be selected
    // INNER JOIN because we need the values to match!
    // we need to use a subquery for the except clause as well
    // see https://www.sqlite.org/lang_select.html#compound
    let new_meme_template_id: Option<i32> = sqlx::query_scalar("
        SELECT (id)
        FROM (SELECT (id) FROM meme_templates
            EXCEPT SELECT (id) FROM
            (SELECT (meme_templates.id) FROM meme_templates
            INNER JOIN players
            ON players.selected_meme_template_id = meme_templates.id
            UNION
            SELECT (meme_templates.id) FROM meme_templates_seen_by_players
            INNER JOIN meme_templates
            ON meme_templates_seen_by_players.meme_template_id = meme_templates.id
            WHERE meme_templates_seen_by_players.player_id = $1)
        ) ORDER BY RANDOM() LIMIT 1")
        .bind(player_id)
        .fetch_optional(database_connection_pool)
        .await?;
    Ok(new_meme_template_id)
}

impl PlayerManager {
    const PLAYER_ID_KEY: &'static str = "player.id";

    pub async fn get_players_in_lobby(&self) -> anyhow::Result<Vec<Player>> {
        let players = sqlx::query_as("SELECT * FROM players")
            .fetch_all(&self.database_connection_pool)
            .await?;

        Ok(players)
    }

    pub async fn sign_in_as_guest(&self, name: String) -> anyhow::Result<()> {
        let player_id: i32 = sqlx::query_scalar("INSERT INTO players (name) VALUES ($1) RETURNING id")
            .bind(name)
            .fetch_one(&self.database_connection_pool)
            .await?;

        self.session.insert(PlayerManager::PLAYER_ID_KEY, player_id).await?;
        Ok(())
    }

    pub async fn get_logged_in_player_id(&self) -> anyhow::Result<i32> {
        // already is i32 because of serde!
        let player_id: i32 = self.session.get(PlayerManager::PLAYER_ID_KEY)
            .await?
            .ok_or(anyhow!("player id key not present or malformed in session"))?;
        Ok(player_id)
    }

    pub async fn get_player_by_id(&self, id: i32) -> anyhow::Result<Player> {
        let player: Player = sqlx::query_as("SELECT * FROM players WHERE id = $1")
            .bind(id)
            .fetch_one(&self.database_connection_pool)
            .await?;
        Ok(player)    
    }

    pub async fn get_logged_in_player(&self) -> anyhow::Result<Player> {
        let logged_in_player_id = self.get_logged_in_player_id().await?;
        let logged_in_player = self.get_player_by_id(logged_in_player_id)
            .await?;
        Ok(logged_in_player)
    }

    pub async fn get_meme_template(&self, meme_template_id: i32) -> anyhow::Result<Option<MemeTemplate>> {
        let meme_template: Option<MemeTemplate> = sqlx::query_as("SELECT * FROM meme_templates WHERE id = $1")
            .bind(meme_template_id)
            .fetch_optional(&self.database_connection_pool)
            .await?;
        Ok(meme_template)
    }

    pub async fn get_meme_template_id_for_player(&self, user_id: i32) -> anyhow::Result<Option<i32>> {
        // TODO: make sure different users get different memes through pool manipulation or something
        let id: Option<i32> = sqlx::query_scalar("SELECT (selected_meme_template_id) FROM players WHERE id = $1")
            .bind(user_id)
            // fetch_one instead of fetch_optional, because fetch_optional will always return Some(Option(...)) here and fetch_one does what we want
            // default value is NULL in SQL, but there is always a row present if id is valid!
            .fetch_one(&self.database_connection_pool)
            .await?;
        Ok(id)
    }

    pub async fn pick_meme_template(&self, player_id: i32) -> anyhow::Result<i32> {
        let new_meme_template_id: Option<i32> = select_new_meme_template(&self.database_connection_pool, player_id).await?;

        // need to prevent this somehow, prevent rerolling?
        // maybe limit max players depending on memes?
        let Some(new_meme_template_id) = new_meme_template_id else { return Err(anyhow!("couldnt pick a new meme template, as all were seen or in use")) };

        sqlx::query("UPDATE players SET selected_meme_template_id = $1 WHERE id = $2")
            .bind(new_meme_template_id)
            .bind(player_id)
            .execute(&self.database_connection_pool)
            .await?;

        // always do this after picking a meme instead of before choosing a new one
        // else a player might see a meme twice (but not through rerolling, just rejoining game) without it being registered
        sqlx::query("INSERT INTO meme_templates_seen_by_players (player_id, meme_template_id) VALUES ($1, $2)")
            .bind(player_id)
            .bind(new_meme_template_id)
            .execute(&self.database_connection_pool)
            .await.map_err(|err| {
                // a unique constraint violation happens when trying to insert the same meme_template_seen_by_players Row
                // this might indicate that the same meme template is being shown twice to the player
                if err.as_database_error().unwrap().kind() == ErrorKind::UniqueViolation {
                    anyhow!("trying to insert duplicate row into meme_templates_seen_by_players, might a meme template be shown twice to a player?")
                } else {
                    err.into()
                }
            })?;
        Ok(new_meme_template_id)
    }

    pub async fn get_remaining_rerolls(&self, player_id: i32) -> anyhow::Result<i32> {
        let rerolls_left: i32 = sqlx::query_scalar("SELECT (rerolls_left) FROM players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&self.database_connection_pool)
            .await?;

        Ok(rerolls_left)
    }

    pub fn can_reroll(&self, no_of_remaining_rerolls: i32) -> bool {
        no_of_remaining_rerolls > 0
    }

    pub async fn set_rerolls_left(&self, new_no_of_rerolls: i32, player_id: i32) -> anyhow::Result<()> {
        sqlx::query("UPDATE players SET rerolls_left = $1 WHERE id = $2")
            .bind(new_no_of_rerolls)
            .bind(player_id)
            .execute(&self.database_connection_pool)
            .await?;
        Ok(())
    }

    pub async fn reroll_meme_template(&self) -> anyhow::Result<()> {
        // sql transaction?
        let logged_in_player_id = self.get_logged_in_player_id().await?;
        let no_of_remaining_rerolls = self.get_remaining_rerolls(logged_in_player_id).await?;
        if !self.can_reroll(no_of_remaining_rerolls) {
            Err(anyhow!("cant reroll!"))?;
        }

        self.pick_meme_template(logged_in_player_id).await?;

        let no_of_remaining_rerolls = no_of_remaining_rerolls - 1;
        self.set_rerolls_left(no_of_remaining_rerolls, logged_in_player_id).await?;
        Ok(())
    }
}

impl FromRequestParts<Arc<AppState>> for PlayerManager
{
    type Rejection = AppError;

    async  fn from_request_parts(parts: &mut axum::http::request::Parts,state: &Arc<AppState>,) -> Result<Self,Self::Rejection> {
        let session: Session = Session::from_request_parts(parts, state).await.map_err(|_| anyhow!("invalid session"))?;
        let database_connection_pool: sqlx::Pool<Sqlite> = state.database_connection_pool.clone();
        Ok(PlayerManager { session, database_connection_pool })
    }
}

#[tokio::test]
async fn test_pick_meme() {
    let database_connection_pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    
    sqlx::migrate!()
        .run(&database_connection_pool)
        .await.unwrap();

        // due to technical reasons we need a subquery
        // https://stackoverflow.com/questions/31500558/sqlite-except-order-by-random
    let _: Option<i32> = select_new_meme_template(&database_connection_pool, 1).await.unwrap();
}