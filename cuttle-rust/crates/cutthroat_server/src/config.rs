pub(crate) fn resolve_database_url() -> Result<String, anyhow::Error> {
    resolve_database_url_from(
        std::env::var("CUTTHROAT_DATABASE_URL").ok(),
        std::env::var("DATABASE_URL").ok(),
    )
}

pub(crate) fn resolve_auto_run_migrations() -> bool {
    resolve_auto_run_migrations_from(std::env::var("CUTTHROAT_AUTO_RUN_MIGRATIONS").ok())
}

pub(crate) fn resolve_auto_run_migrations_from(value: Option<String>) -> bool {
    value
        .as_deref()
        .map(|raw| raw.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub(crate) fn resolve_database_url_from(
    cutthroat_database_url: Option<String>,
    database_url: Option<String>,
) -> Result<String, anyhow::Error> {
    cutthroat_database_url
        .or(database_url)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Missing database URL for cutthroat persistence. Set `CUTTHROAT_DATABASE_URL` (preferred) or `DATABASE_URL`."
            )
        })
}
