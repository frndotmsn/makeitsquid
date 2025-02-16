use serde::Serialize;

#[derive(Serialize)]
pub struct PlayerHTMLInfo {
    name: String,
    is_logged_in_user: bool, 
}

impl PlayerHTMLInfo {
    pub fn new(name: String, is_logged_in_user: bool) -> PlayerHTMLInfo {
        PlayerHTMLInfo {
            name,
            is_logged_in_user,
        }
    } 
}

#[derive(Serialize)]
pub struct LobbyTemplateContext {
    pub players: Vec<PlayerHTMLInfo>
}
