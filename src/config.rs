use std::{env, net::SocketAddr};

use axum_extra::extract::cookie::SameSite;

use crate::errors::{AppError, AppResult};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub frontend_origin: String,
    pub address: SocketAddr,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| AppError::Config("DATABASE_URL is required".to_owned()))?;
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| AppError::Config("JWT_SECRET is required".to_owned()))?;
        let frontend_origin =
            env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_owned());
        let port = env::var("PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(8080);
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
        let cookie_secure = env::var("COOKIE_SECURE")
            .ok()
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or_else(|| {
                env::var("APP_ENV")
                    .map(|value| value.eq_ignore_ascii_case("production"))
                    .unwrap_or(false)
            });
        let cookie_same_site = env::var("COOKIE_SAME_SITE")
            .ok()
            .map(|value| parse_same_site(&value))
            .transpose()?
            .unwrap_or_else(|| {
                if cookie_secure {
                    SameSite::None
                } else {
                    SameSite::Lax
                }
            });

        let address = format!("{host}:{port}")
            .parse()
            .map_err(|error| AppError::Config(format!("Invalid HOST/PORT combination: {error}")))?;

        Ok(Self {
            database_url,
            jwt_secret,
            frontend_origin,
            address,
            cookie_secure,
            cookie_same_site,
        })
    }
}

fn parse_same_site(value: &str) -> AppResult<SameSite> {
    match value.trim().to_ascii_lowercase().as_str() {
        "lax" => Ok(SameSite::Lax),
        "strict" => Ok(SameSite::Strict),
        "none" => Ok(SameSite::None),
        _ => Err(AppError::Config(
            "COOKIE_SAME_SITE must be one of: lax, strict, none".to_owned(),
        )),
    }
}
