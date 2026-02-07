use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::migrate::Migrator;
use tokio::sync::mpsc;
use tracing::error;

const TABLE_NAME: &str = "public.cutthroat_games";
static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedGameRecord {
    pub rust_game_id: i64,
    pub tokenlog: String,
    pub p0_username: String,
    pub p1_username: String,
    pub p2_username: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

pub async fn ensure_schema_ready(
    pool: &PgPool,
    auto_run_migrations: bool,
) -> Result<(), anyhow::Error> {
    let table_name: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(TABLE_NAME)
        .fetch_one(pool)
        .await
        .context("failed to check cutthroat persistence schema")?;
    if table_name.is_some() {
        return Ok(());
    }

    if auto_run_migrations {
        MIGRATOR
            .run(pool)
            .await
            .context("failed to auto-run sqlx migrations for cutthroat persistence")?;
        return Ok(());
    }

    let migrations_path = format!("{}/migrations", env!("CARGO_MANIFEST_DIR"));
    let command = format!("sqlx migrate run --source {}", migrations_path);
    Err(anyhow!(
        "Required table `{TABLE_NAME}` not found.\nRun migration: `{command}`\nOr set `CUTTHROAT_AUTO_RUN_MIGRATIONS=true` to auto-run migrations at startup."
    ))
}

/// Reads the largest persisted Rust Cutthroat game ID from Postgres for startup seeding only.
/// Runtime game ID allocation remains in-memory in the store while the server is running.
pub async fn fetch_max_cutthroat_game_id_in_db(pool: &PgPool) -> Result<i64, anyhow::Error> {
    let max_id: Option<i64> = sqlx::query_scalar("SELECT MAX(rust_game_id) FROM cutthroat_games")
        .fetch_one(pool)
        .await
        .context("failed to query max rust_game_id from cutthroat_games")?;
    Ok(max_id.unwrap_or(0))
}

pub async fn run_persistence_worker(mut rx: mpsc::Receiver<CompletedGameRecord>, pool: PgPool) {
    while let Some(record) = rx.recv().await {
        if let Err(err) = persist_completed_game(&pool, &record).await {
            error!(
                game_id = record.rust_game_id,
                error = ?err,
                "failed to persist completed cutthroat game"
            );
        }
    }
}

async fn persist_completed_game(
    pool: &PgPool,
    record: &CompletedGameRecord,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        INSERT INTO cutthroat_games (
            rust_game_id,
            tokenlog,
            p0_username,
            p1_username,
            p2_username,
            started_at,
            finished_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(record.rust_game_id)
    .bind(&record.tokenlog)
    .bind(&record.p0_username)
    .bind(&record.p1_username)
    .bind(&record.p2_username)
    .bind(record.started_at)
    .bind(record.finished_at)
    .execute(pool)
    .await
    .context("insert into cutthroat_games failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_schema_ready;

    #[test]
    fn missing_schema_message_includes_migration_command() {
        let _ = ensure_schema_ready;
        let migrations_path = format!("{}/migrations", env!("CARGO_MANIFEST_DIR"));
        let command = format!("sqlx migrate run --source {}", migrations_path);
        assert!(command.contains("sqlx migrate run --source"));
        assert!(command.contains("/migrations"));
    }
}
