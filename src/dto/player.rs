#[derive(sqlx::FromRow, Debug)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub selected_meme_template_id: Option<i32>,
    pub rerolls_left: i32,
}
