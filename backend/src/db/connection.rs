use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

pub async fn connect_db() -> DatabaseConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let max_connections = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);

    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(max_connections)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300));

    Database::connect(options)
        .await
        .expect("Failed to connect to database")
}
