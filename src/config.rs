use std::{env, net::SocketAddr};

use crate::errors::{AppError, AppResult};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub frontend_origin: String,
    pub address: SocketAddr,
    pub cookie_secure: bool,
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

        let address = format!("{host}:{port}")
            .parse()
            .map_err(|error| AppError::Config(format!("Invalid HOST/PORT combination: {error}")))?;

        Ok(Self {
            database_url,
            jwt_secret,
            frontend_origin,
            address,
            cookie_secure,
        })
    }
}
