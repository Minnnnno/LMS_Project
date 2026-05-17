use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub async fn connect_db() -> Pool<Postgres> {

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL not found");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database")
}