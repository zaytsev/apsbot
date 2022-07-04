use anyhow::{Context, Result};
use sqlx::{migrate::Migrator, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run(db: &PgPool) -> Result<()> {
    MIGRATOR
        .run(db)
        .await
        .context("Error running database migrations")
}

#[cfg(test)]
pub(crate) async fn apply_only(db: &PgPool, migrations: &[&str]) -> Result<()> {
    use std::borrow::Cow;
    use std::collections::hash_map::RandomState;
    use std::collections::HashSet;

    let migs: HashSet<String, RandomState> =
        HashSet::from_iter(migrations.into_iter().map(|s| s.to_string()));

    let selected_migs = MIGRATOR
        .iter()
        .filter(|m| migs.contains(m.description.as_ref()))
        .map(|m| m.clone())
        .collect();

    Migrator {
        migrations: Cow::Owned(selected_migs),
        ignore_missing: true,
    }
    .run(db)
    .await?;

    Ok(())
}
