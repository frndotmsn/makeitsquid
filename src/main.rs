use std::{collections::HashMap, mem, sync::{Arc, LazyLock, Mutex}};

use axum::{extract::FromRequestParts, http::StatusCode, response::{Html, IntoResponse, Redirect}, routing::{get, post}, Form, Router
};

use logged_in_player::LoggedInPlayer;
use player::Player;
use player_manager::PlayerManager;
use serde::Deserialize;
use templates::{index::IndexTemplateContext, lobby::{LobbyTemplateContext, PlayerHTMLInfo}, ingame::IngameTemplateContext};
use tinytemplate::TinyTemplate;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

mod templates;
mod unlabeled_meme;
mod labeled_meme;
mod player_manager;
mod logged_in_player;
mod player;

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
                        .route("/ingame", get(ingame))
                        .route("/reroll", post(reroll))
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
    tt.add_template("ingame.html", include_str!(r"..\templates\ingame.html")).unwrap();
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

static PLAYERS_IN_LOBBY: LazyLock<Arc<Mutex<HashMap<String, Player>>>> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

fn get_player_by_id(id: String) -> Option<Player> {
    PLAYERS_IN_LOBBY.lock().unwrap().get(&id).cloned()
}

async fn join_lobby(
    // using axum::extract::Form here, consider using axum_extra::extract::Form
    // 
    // https://docs.rs/axum-extra/0.10.0/axum_extra/extract/struct.Form.html#differences-from-axumextractform
    player_manager: player_manager::PlayerManager,
    Form(join_lobby_payload): Form<JoinLobbyPayload>,
) -> impl IntoResponse {
    let Some(player_name) = join_lobby_payload.player_name_if_valid() else {
        return StatusCode::BAD_REQUEST.into_response()
    };

    player_manager.sign_in_as_guest(player_name).await;
    Redirect::to("/lobby").into_response()
}

async fn lobby(
    LoggedInPlayer(logged_in_player): LoggedInPlayer
) -> Html<String> {
    let players: Vec<PlayerHTMLInfo> = PLAYERS_IN_LOBBY.lock().unwrap()
        .values()
        .map(|player|
            PlayerHTMLInfo::new(player.name.clone(),  player.id == logged_in_player.id)
        ).collect::<Vec<_>>();

    let context = LobbyTemplateContext {
        players
    };
    let rendered = TT.with(|tt| tt.render("lobby.html", &context)).unwrap();
    Html(rendered)
}

#[derive(Clone)]
struct MemeTemplate {
    pub id: String,
    pub uri: String,
}

impl MemeTemplate {
    pub fn new(uri: String) -> MemeTemplate {
        MemeTemplate { id: uuid::Uuid::new_v4().to_string(), uri }
    }
}

struct MemeTemplatePool {
    templates: Vec<MemeTemplate>
}

impl MemeTemplatePool {
    pub fn pick_id(&self) -> Option<String> {
        use rand::seq::IndexedRandom;

        self.templates.choose(&mut rand::rng()).cloned().map(|meme_template| meme_template.id)
    }

    pub fn pick_omit(&self, meme_template_ids_to_omit: &[String]) -> Option<String> {
        use rand::seq::IndexedRandom;

        let availible_ids = self.templates.iter().filter_map(|meme_template| if meme_template_ids_to_omit.contains(&meme_template.id) { None } else {Some(meme_template.id.clone()) }).collect::<Vec<_>>();
        availible_ids.choose(&mut rand::rng()).cloned()
    }
}

static MEME_TEMPLATE_POOL: LazyLock<Arc<Mutex<MemeTemplatePool>>> = LazyLock::new(|| Arc::new(Mutex::new(MemeTemplatePool { templates: vec![
    MemeTemplate::new("https://upload.wikimedia.org/wikipedia/en/3/34/RickAstleyNeverGonnaGiveYouUp7InchSingleCover.jpg".to_owned()),
    MemeTemplate::new("https://th.bing.com/th/id/OIP.o-KHiUpgsdqVeaG-wkhfnwHaEK?rs=1&pid=ImgDetMain".to_owned()),
    MemeTemplate::new("https://th.bing.com/th/id/OIP.WOki_Ng83gsk1xioaX3BPgHaG8?rs=1&pid=ImgDetMain".to_owned()),
    ]
})));

async fn ingame(
    player_manager: PlayerManager,
) -> Html<String> {

    let no_of_labels = 2;
    let labels = (1..=no_of_labels).into_iter().collect::<Box<[i32]>>();

    let template_image_uri = player_manager.get_meme_template_for_user().await.uri;

    let context = IngameTemplateContext {
        remaining_rerolls: 3,
        template_image_uri,
        labels,
    };
    let rendered = TT.with(|tt| tt.render("ingame.html", &context)).unwrap();
    Html(rendered)
}

async fn reroll(
    player_manager: PlayerManager,
) -> impl IntoResponse {
    player_manager.reroll_meme_template().await;
    return Redirect::to("/ingame");
}
