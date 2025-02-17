#[derive(sqlx::FromRow)]
pub struct MemeTemplatesSeenByPlayer {
    pub player_id: i32,
    pub meme_template_id: i32,
}
