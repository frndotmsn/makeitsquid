#[derive(sqlx::FromRow)]
pub struct MemeTemplate {
    pub id: i32,
    pub uri: String,
}
