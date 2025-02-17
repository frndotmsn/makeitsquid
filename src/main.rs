#![forbid(unused_must_use)]

use std::sync::Arc;

use anyhow::anyhow;
use app_error::AppError;
use app_state::AppState;
use axum::{response::{Html, IntoResponse, Redirect}, routing::{get, post}, Form, Router
};

use dto::{meme_template::MemeTemplate, player::Player};
use logged_in_player::LoggedInPlayer;
use player_manager::PlayerManager;
use serde::Deserialize;
use sqlx::SqlitePool;
use templates::{index::IndexTemplateContext, lobby::{LobbyTemplateContext, PlayerHTMLInfo}, ingame::IngameTemplateContext};
use tinytemplate::TinyTemplate;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

mod templates;
mod player_manager;
mod logged_in_player;
mod dto;
mod app_state;
mod app_error;

#[tokio::main]
async fn main() {
    let database_connection_pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!()
        .run(&database_connection_pool)
        .await
        .unwrap();

    let uris_of_meme_templates_to_insert = [
        "https://upload.wikimedia.org/wikipedia/en/3/34/RickAstleyNeverGonnaGiveYouUp7InchSingleCover.jpg",
        "https://th.bing.com/th/id/OIP.o-KHiUpgsdqVeaG-wkhfnwHaEK?rs=1&pid=ImgDetMain",
        "https://th.bing.com/th/id/OIP.WOki_Ng83gsk1xioaX3BPgHaG8?rs=1&pid=ImgDetMain",
        ];
    
    for meme_template_uri in uris_of_meme_templates_to_insert {
        sqlx::query("INSERT INTO meme_templates (uri) VALUES ($1)")
            .bind(meme_template_uri)
            .execute(&database_connection_pool)
            .await
            .unwrap();
    }
    
    // no memstore in prod for serverless
    let session_store = MemoryStore::default();
    // much more setup in prod
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true);

    let state = Arc::new(AppState {
        database_connection_pool,
    });

    let app = Router::new()
                .route("/", get(index))
                .route("/join_lobby", post(join_lobby))
                .route("/lobby", get(lobby))
                .route("/ingame", get(ingame))
                .route("/reroll", post(reroll))
                .nest_service("/public", ServeDir::new("public"))
                .layer(session_layer)
                .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn get_tt() -> TinyTemplate<'static> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index.html", include_str!(r"../templates/index.html")).unwrap();
    tt.add_template("lobby.html", include_str!(r"../templates/lobby.html")).unwrap();
    tt.add_template("ingame.html", include_str!(r"../templates/ingame.html")).unwrap();
    tt
}

thread_local! {
    static TT: TinyTemplate<'static> = get_tt();
}

async fn index(
) -> Html<String> {
    let context = IndexTemplateContext {};
    let rendered = TT.with(|tt| tt.render("index.html", &context)).unwrap();
    Html(rendered)
}

#[derive(Deserialize)]
struct JoinLobbyPayload {
    player_name: String
}

impl JoinLobbyPayload {
    fn is_valid(&self) -> bool {
        !self.player_name.is_empty()
    }

    fn player_name_if_valid(&self) -> Option<String> {
        if self.is_valid() {
            Some(self.player_name.clone())
        } else { None }
    }
}

async fn join_lobby(
    // using axum::extract::Form here, consider using axum_extra::extract::Form
    // 
    // https://docs.rs/axum-extra/0.10.0/axum_extra/extract/struct.Form.html#differences-from-axumextractform
    player_manager: player_manager::PlayerManager,
    Form(join_lobby_payload): Form<JoinLobbyPayload>,
) -> Result<impl IntoResponse, AppError> {
    let Some(player_name) = join_lobby_payload.player_name_if_valid() else {
        return Err(anyhow!("Bad request!").into());
    };

    player_manager.sign_in_as_guest(player_name).await?;
    Ok(Redirect::to("/lobby").into_response())
}

async fn lobby(
    player_manager: PlayerManager,
    LoggedInPlayer(logged_in_player): LoggedInPlayer,
) -> Result<Html<String>, AppError> {
    let players: Vec<Player> = player_manager.get_players_in_lobby().await?;
    let players: Vec<PlayerHTMLInfo> = players
        .iter().map(|player|
            PlayerHTMLInfo::new(player.name.clone(),  player.id == logged_in_player.id)
        ).collect::<Vec<_>>();

    let context = LobbyTemplateContext {
        players
    };
    let rendered = TT.with(|tt| tt.render("lobby.html", &context)).unwrap();
    Ok(Html(rendered))
}
async fn ingame(
    player_manager: PlayerManager,
) -> Result<Html<String>, AppError> {

    let no_of_labels = 2;
    let labels = (1..=no_of_labels).into_iter().collect::<Box<[i32]>>();

    let logged_in_player_id = player_manager.get_logged_in_player_id().await?;
    let meme_template_id = match player_manager.get_meme_template_id_for_player(logged_in_player_id).await? {
        Some(meme_template_id) => meme_template_id,
        None => {
            player_manager.pick_meme_template(logged_in_player_id).await?
        }
    };

    let template_image_uri = player_manager.get_meme_template(meme_template_id).await?.expect("wrong id requested!").uri;

    let remaining_rerolls = player_manager.get_remaining_rerolls(logged_in_player_id)
        .await?;

    let context = IngameTemplateContext {
        remaining_rerolls,
        template_image_uri,
        labels,
    };
    let rendered = TT.with(|tt| tt.render("ingame.html", &context)).unwrap();
    Ok(Html(rendered))
}

async fn reroll(
    player_manager: PlayerManager,
) -> Result<impl IntoResponse, AppError> {
    player_manager.reroll_meme_template().await?;
    Ok(Redirect::to("/ingame"))
}

#[tokio::test]
async fn test_db() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    sqlx::migrate!()
        .run(&pool)
        .await.unwrap();

    let player: dto::player::Player = sqlx::query_as("INSERT INTO players (name) VALUES ($1) RETURNING *")
        .bind("nijo")
        .fetch_one(&pool)
        .await.unwrap();

    assert_eq!(player.name, "nijo");
    assert_eq!(player.selected_meme_template_id, None);

    let meme_template: dto::meme_template::MemeTemplate = sqlx::query_as("INSERT INTO meme_templates (uri) VALUES ($1) RETURNING *")
        .bind("https://wikipedia.org/favicon.ico")
        .fetch_one(&pool)
        .await.unwrap();

    assert_eq!(meme_template.uri, "https://wikipedia.org/favicon.ico");

    let meme_templates_seen_by_player: Option<dto::meme_templates_seen_by_player::MemeTemplatesSeenByPlayer> = sqlx::query_as("SELECT * FROM meme_templates_seen_by_players WHERE player_id = $1 LIMIT 1")
        .bind(player.id)
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(meme_templates_seen_by_player.is_none());

    let meme_templates_seen_by_player: dto::meme_templates_seen_by_player::MemeTemplatesSeenByPlayer = sqlx::query_as("INSERT INTO meme_templates_seen_by_players (player_id, meme_template_id) VALUES ($1, $2) RETURNING *")
        .bind(player.id)
        .bind(meme_template.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let meme_templates_seen_by_player_new: dto::meme_templates_seen_by_player::MemeTemplatesSeenByPlayer = sqlx::query_as("SELECT * FROM meme_templates_seen_by_players WHERE player_id = $1 LIMIT 1")
        .bind(player.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(meme_templates_seen_by_player.meme_template_id, meme_templates_seen_by_player_new.meme_template_id);
    assert_eq!(meme_templates_seen_by_player.player_id, meme_templates_seen_by_player_new.player_id);

    // test rerolls!
}