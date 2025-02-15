use std::{collections::HashMap, sync::{Arc, LazyLock, Mutex}};

use axum::{extract::FromRequestParts, http::StatusCode, response::{Html, IntoResponse, Redirect}, routing::{get, post}, Form, Router
};

use serde::Deserialize;
use templates::{index::IndexTemplateContext, lobby::LobbyTemplateContext};
use tinytemplate::TinyTemplate;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

mod templates;
mod unlabeled_meme;
mod labeled_meme;

#[tokio::main]
async fn main() {
    // no memstore in prod for serverless
    let session_store = MemoryStore::default();
    // much more setup in prod
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true);

    let app = Router::new()
                        .route("/", get(index))
                        .route("/join_lobby", post(join_lobby))
                        .route("/lobby", get(lobby))
                        .nest_service("/public", ServeDir::new("public"))
                        .layer(session_layer);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn get_tt() -> TinyTemplate<'static> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index.html", include_str!(r"..\templates\index.html")).unwrap();
    tt.add_template("lobby.html", include_str!(r"..\templates\lobby.html")).unwrap();
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

#[derive(Clone)]
struct Player {
    id: String,
    name: String,
}

impl Player {
    fn new_guest(name: String) -> Player {
        Player {
            id: uuid::Uuid::new_v4().to_string(),
            name
        }
    }
}

static PLAYERS_IN_LOBBY: LazyLock<Arc<Mutex<HashMap<String, Player>>>> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

const PLAYER_ID_KEY: &'static str = "player.id";

fn get_player_by_id(id: String) -> Option<Player> {
    PLAYERS_IN_LOBBY.lock().unwrap().get(&id).cloned()
}

struct PlayerManager(Session);

impl PlayerManager {
    async fn sign_in_as_guest(&self, name: String) {
        let player = Player::new_guest(name);
        self.0.insert(PLAYER_ID_KEY, player.id.clone()).await.unwrap();
        PLAYERS_IN_LOBBY.lock().unwrap().insert(player.id.clone(), player);
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

struct LoggedInPlayer(Player);

impl<S> FromRequestParts<S> for LoggedInPlayer
where
    S: Send + Sync,
{

    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut axum::http::request::Parts,state: &S,) -> Result<Self, Self::Rejection> {
        let session: Session = Session::from_request_parts(parts, state).await?;
        let player_id: String = session.get(PLAYER_ID_KEY).await.unwrap().ok_or((StatusCode::BAD_REQUEST, "player not found"))?;

        let player = get_player_by_id(player_id).ok_or((StatusCode::BAD_REQUEST, "player not found"))?;
        let logged_in_player: LoggedInPlayer = LoggedInPlayer(player);
        Ok(logged_in_player)
    }
}

async fn join_lobby(
    // using axum::extract::Form here, consider using axum_extra::extract::Form
    // 
    // https://docs.rs/axum-extra/0.10.0/axum_extra/extract/struct.Form.html#differences-from-axumextractform
    player_manager: PlayerManager,
    Form(join_lobby_payload): Form<JoinLobbyPayload>,
) -> impl IntoResponse {
    let Some(player_name) = join_lobby_payload.player_name_if_valid() else {
        return StatusCode::BAD_REQUEST.into_response()
    };

    player_manager.sign_in_as_guest(player_name).await;
    Redirect::to("/lobby").into_response()
}

async fn lobby(
    LoggedInPlayer(player): LoggedInPlayer
) -> Html<String> {
    let context = LobbyTemplateContext {
        player_name: player.name
    };
    let rendered = TT.with(|tt| tt.render("lobby.html", &context)).unwrap();
    Html(rendered)
}
