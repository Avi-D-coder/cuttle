use crate::state::AppState;
use axum::http::{HeaderMap, StatusCode, header};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const AUTH_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub(crate) struct AuthUser {
    pub(crate) id: i64,
    pub(crate) username: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AuthCacheEntry {
    pub(crate) user: AuthUser,
    pub(crate) expires_at: Instant,
}

#[derive(Deserialize)]
struct AuthStatus {
    authenticated: bool,
    id: Option<i64>,
    username: Option<String>,
}

pub(crate) async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthUser, StatusCode> {
    let cookie = headers
        .get(header::COOKIE)
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let session_key = extract_session_id(cookie).unwrap_or_else(|| cookie.to_string());

    if let Some(user) = get_cached_user(&state.auth_cache, &session_key).await {
        return Ok(user);
    }

    let url = format!("{}/api/user/status", state.js_base);
    let res = state
        .http
        .get(url)
        .header(header::COOKIE, cookie)
        .send()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !res.status().is_success() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let body: AuthStatus = res.json().await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !body.authenticated {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let id = body.id.ok_or(StatusCode::UNAUTHORIZED)?;
    let username = body.username.ok_or(StatusCode::UNAUTHORIZED)?;
    let user = AuthUser { id, username };
    set_cached_user(&state.auth_cache, session_key, user.clone()).await;
    Ok(user)
}

fn extract_session_id(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix("cuttle.sid=").map(|val| val.to_string()))
}

async fn get_cached_user(
    cache: &Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    session_key: &str,
) -> Option<AuthUser> {
    let now = Instant::now();
    let mut guard = cache.lock().await;
    guard.retain(|_, entry| entry.expires_at > now);
    guard.get(session_key).map(|entry| entry.user.clone())
}

async fn set_cached_user(
    cache: &Arc<Mutex<HashMap<String, AuthCacheEntry>>>,
    session_key: String,
    user: AuthUser,
) {
    let mut guard = cache.lock().await;
    guard.insert(
        session_key,
        AuthCacheEntry {
            user,
            expires_at: Instant::now() + AUTH_CACHE_TTL,
        },
    );
}
