use sqlx::Sqlite;

#[derive(Clone)]
pub struct AppState {
    pub database_connection_pool: sqlx::Pool<Sqlite>
}
