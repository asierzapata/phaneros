use std::path::Path;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

/// Open (creating if absent) the SQLite database at `database_path` and bring its
/// schema up to date. The `migrations/` directory is embedded at compile time by
/// `sqlx::migrate!()`, so a fresh deployment self-initializes on first boot.
pub async fn connect(database_path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
