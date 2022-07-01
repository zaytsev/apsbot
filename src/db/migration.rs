use anyhow::{Context, Result};
use sqlx::{migrate::Migrator, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run(db: &PgPool) -> Result<()> {
    MIGRATOR
        .run(db)
        .await
        .context("Error applying database migrations")
}
