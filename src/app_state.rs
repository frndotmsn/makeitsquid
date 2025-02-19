use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub database_connection_pool: PgPool
}
