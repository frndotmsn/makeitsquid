use std::{collections::HashMap, sync::{atomic::AtomicI32, Arc, LazyLock, Mutex}};

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
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
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


struct SessionID(String);

impl SessionID {
    const SESSION_ID_KEY: &'static str = "id";
}

static COUNTER: LazyLock<Arc<AtomicI32>> = LazyLock::new(|| Arc::new(0.into())); 

impl<S> FromRequestParts<S> for SessionID
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S,) -> Result<Self,Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;

        let session_id_str: Option<String> = session
            .get(Self::SESSION_ID_KEY)
            .await
            .unwrap();

        match session_id_str {
            Some(session_id_str) => Ok(Self(session_id_str)),
            None => {
                let new_session_id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst).to_string();
                session
                    .insert(Self::SESSION_ID_KEY, &new_session_id)
                    .await
                    .unwrap();
                Ok(Self(new_session_id))
            }
        }
    }
}

static PLAYERS_IN_LOBBY: LazyLock<Arc<Mutex<HashMap<String, String>>>> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn join_lobby(
    // using axum::extract::Form here, consider using axum_extra::extract::Form
    // 
    // https://docs.rs/axum-extra/0.10.0/axum_extra/extract/struct.Form.html#differences-from-axumextractform
    SessionID(session_id): SessionID,
    Form(join_lobby_payload): Form<JoinLobbyPayload>,
) -> impl IntoResponse {
    let Some(player_name) = join_lobby_payload.player_name_if_valid() else {
        return StatusCode::BAD_REQUEST.into_response()
    };

    PLAYERS_IN_LOBBY
        .lock()
        .unwrap()
        .insert(session_id, player_name);

    Redirect::to("/lobby").into_response()
}

async fn lobby(
    PlayerName(player_name): PlayerName
) -> Html<String> {
    let context = LobbyTemplateContext {
        player_name
    };
    let rendered = TT.with(|tt| tt.render("lobby.html", &context)).unwrap();
    Html(rendered)
}

struct PlayerName(String);

impl<S> FromRequestParts<S> for PlayerName
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S,) -> Result<Self,Self::Rejection> {
        let SessionID(session_id) = SessionID::from_request_parts(parts, state).await?;

        for (k,v) in PLAYERS_IN_LOBBY.lock().unwrap().iter()
        {
            println!("{k}: {v}");
        }

        match PLAYERS_IN_LOBBY.lock().unwrap().get(session_id.as_str()) {
            Some(player_name) => Ok(Self(player_name.clone())),
            None => Err((StatusCode::BAD_REQUEST, "Can't extract player name from session id, is the session id valid?"))
        }   
    }
}