mod api;
mod app;
mod auth;
mod config;
mod game_runtime;
mod persistence;
mod state;
mod view;
mod ws;

use app::build_router;
use config::{resolve_auto_run_migrations, resolve_database_url};
#[cfg(test)]
use config::{resolve_auto_run_migrations_from, resolve_database_url_from};
use persistence::{ensure_schema_ready, fetch_max_cutthroat_game_id_in_db, run_persistence_worker};
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let js_base = std::env::var("JS_INTERNAL_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:1337".to_string());
    let bind_addr = std::env::var("RUST_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());

    let http = reqwest::Client::new();
    let auth_cache = Arc::new(Mutex::new(HashMap::new()));
    let (runtime_tx, runtime_rx) = mpsc::channel(256);
    let (persistence_tx, persistence_rx) = mpsc::channel(256);

    let (initial_next_game_id, persistence_pool) = match resolve_database_url() {
        Ok(database_url) => {
            let auto_run_migrations = resolve_auto_run_migrations();
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;
            ensure_schema_ready(&pool, auto_run_migrations).await?;
            let max_persisted_cutthroat_game_id = fetch_max_cutthroat_game_id_in_db(&pool).await?;
            (
                max_persisted_cutthroat_game_id.saturating_add(1).max(1),
                Some(pool),
            )
        }
        #[cfg(feature = "e2e-seed")]
        Err(_) => {
            info!(
                "no database URL set; running cutthroat server without persistence because `e2e-seed` is enabled"
            );
            (1_i64, None)
        }
        #[cfg(not(feature = "e2e-seed"))]
        Err(err) => return Err(err),
    };

    tokio::spawn(game_runtime::runtime_task(
        runtime_rx,
        persistence_tx,
        initial_next_game_id,
    ));
    if let Some(pool) = persistence_pool {
        tokio::spawn(run_persistence_worker(persistence_rx, pool));
    } else {
        drop(persistence_rx);
    }

    let state = AppState {
        js_base,
        http,
        auth_cache,
        runtime_tx,
    };

    let app = build_router(state);

    info!(
        "cutthroat server listening on {} (initial_next_game_id={})",
        bind_addr, initial_next_game_id
    );
    #[cfg(feature = "e2e-seed")]
    info!("cutthroat e2e seed endpoint enabled at /cutthroat/api/test/games/seed-tokenlog");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{resolve_auto_run_migrations_from, resolve_database_url_from};
    use crate::game_runtime::SeatEntry;
    use crate::view::response::usernames_from_seats;

    #[test]
    fn resolve_database_url_prefers_cutthroat_specific_url() {
        let resolved = resolve_database_url_from(
            Some("postgres://cutthroat".to_string()),
            Some("postgres://fallback".to_string()),
        )
        .expect("database url");
        assert_eq!(resolved, "postgres://cutthroat");
    }

    #[test]
    fn resolve_database_url_uses_fallback_when_primary_missing() {
        let resolved = resolve_database_url_from(None, Some("postgres://fallback".to_string()))
            .expect("database url");
        assert_eq!(resolved, "postgres://fallback");
    }

    #[test]
    fn resolve_database_url_requires_any_url() {
        let err = resolve_database_url_from(None, None).expect_err("expected failure");
        assert!(
            err.to_string().contains("CUTTHROAT_DATABASE_URL"),
            "error should explain required env vars"
        );
    }

    #[test]
    fn auto_run_migrations_defaults_to_false() {
        assert!(!resolve_auto_run_migrations_from(None));
    }

    #[test]
    fn auto_run_migrations_enabled_only_for_true() {
        assert!(resolve_auto_run_migrations_from(Some("true".to_string())));
        assert!(resolve_auto_run_migrations_from(Some(" TRUE ".to_string())));
        assert!(!resolve_auto_run_migrations_from(Some("1".to_string())));
        assert!(!resolve_auto_run_migrations_from(Some("false".to_string())));
    }

    #[test]
    fn usernames_from_seats_maps_by_seat_index() {
        let seats = vec![
            SeatEntry {
                seat: 2,
                user_id: 12,
                username: "carol".to_string(),
                ready: true,
            },
            SeatEntry {
                seat: 0,
                user_id: 10,
                username: "alice".to_string(),
                ready: true,
            },
            SeatEntry {
                seat: 1,
                user_id: 11,
                username: "bob".to_string(),
                ready: true,
            },
        ];
        let names = usernames_from_seats(&seats).expect("expected full seat map");
        assert_eq!(
            names,
            ["alice".to_string(), "bob".to_string(), "carol".to_string()]
        );
    }
}
