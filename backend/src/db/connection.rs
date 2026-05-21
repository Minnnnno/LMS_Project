use sea_orm::{Database, DatabaseConnection};

pub async fn connect_db() -> DatabaseConnection {
    let database_url = std::env::var("DATABASE_URL")
    .expect("DATABASE_URL must be set");
    Database::connect(database_url)
        .await
        .expect("Failed to connect to database")
    
}