use serde::Serialize;

#[derive(Serialize)]
pub struct LobbyTemplateContext {
    pub player_name: String
}
