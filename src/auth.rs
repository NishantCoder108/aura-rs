use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header, request::Parts},
};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    errors::{AppError, AppResult},
    models::{AuthUser, UserRecord},
    state::AppState,
};

pub const AUTH_COOKIE_NAME: &str = "urlvibe_token";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Debug, Clone)]
pub struct CurrentUser(pub AuthUser);

pub fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn normalize_username(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| {
            tracing::error!("password hash error: {error}");
            AppError::Internal
        })?
        .to_string();

    Ok(hashed_password)
}

pub fn verify_password(password: &str, password_hash: &str) -> AppResult<()> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|error| {
        tracing::error!("password hash parse error: {error}");
        AppError::Internal
    })?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|error| {
            tracing::error!("password verify error: {error}");
            AppError::Unauthorized("Invalid credentials".to_owned())
        })?;
    Ok(())
}

pub fn create_token(user_id: Uuid, secret: &str) -> AppResult<String> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
    };

    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn decode_token(token: &str, secret: &str) -> AppResult<Uuid> {
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::Unauthorized("Authentication failed".to_owned()))?;

    Uuid::parse_str(&claims.claims.sub)
        .map_err(|_| AppError::Unauthorized("Authentication failed".to_owned()))
}

pub fn auth_cookie(
    token: String,
    config: &AppConfig,
) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((AUTH_COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .same_site(config.cookie_same_site)
        .secure(config.cookie_secure)
        .max_age(TimeDuration::days(7))
        .expires(OffsetDateTime::now_utc() + TimeDuration::days(7))
        .build()
}

pub fn clear_auth_cookie(config: &AppConfig) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((AUTH_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(config.cookie_same_site)
        .secure(config.cookie_secure)
        .max_age(TimeDuration::seconds(0))
        .expires(OffsetDateTime::now_utc() - TimeDuration::days(1))
        .build()
}

async fn load_user(pool: &PgPool, user_id: Uuid) -> AppResult<AuthUser> {
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, first_name, email, username, password_hash
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Authentication required".to_owned()))?;

    Ok(user.into())
}

impl<S> FromRequestParts<S> for CurrentUser
where
    AppState: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> AppResult<Self> {
        let State(app_state) = State::<AppState>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Internal)?;
        let token = extract_bearer_token(parts)?
            .or_else(|| extract_cookie_token(parts))
            .ok_or_else(|| AppError::Unauthorized("Authentication required".to_owned()))?;

        let user_id = decode_token(&token, &app_state.config.jwt_secret)?;
        let user = load_user(&app_state.pool, user_id).await?;

        Ok(Self(user))
    }
}

fn extract_bearer_token(parts: &Parts) -> AppResult<Option<String>> {
    let Some(value) = parts.headers.get(header::AUTHORIZATION) else {
        return Ok(None);
    };

    let value = value
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_owned()))?;

    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(AppError::Unauthorized(
            "Authorization header must use Bearer token format".to_owned(),
        ));
    };

    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::Unauthorized(
            "Authorization token cannot be empty".to_owned(),
        ));
    }

    Ok(Some(token.to_owned()))
}

fn extract_cookie_token(parts: &Parts) -> Option<String> {
    let jar = CookieJar::from_headers(&parts.headers);
    jar.get(AUTH_COOKIE_NAME)
        .map(|cookie| cookie.value().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{create_token, decode_token, normalize_email, normalize_username};
    use uuid::Uuid;

    #[test]
    fn normalizes_identifier_input() {
        assert_eq!(normalize_email(" Test@Example.com "), "test@example.com");
        assert_eq!(normalize_username(" Alice "), "alice");
    }

    #[test]
    fn token_roundtrip() {
        let user_id = Uuid::new_v4();
        let token = create_token(user_id, "secret").unwrap();
        assert_eq!(decode_token(&token, "secret").unwrap(), user_id);
    }
}
