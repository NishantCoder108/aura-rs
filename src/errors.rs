use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Config(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("migration error")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("token error")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("internal server error")]
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::Config(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_)
            | Self::Migration(_)
            | Self::Jwt(_)
            | Self::Io(_)
            | Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Config(message) => message.clone(),
            Self::Database(error) => {
                tracing::error!("database error: {error}");
                "Database request failed".to_owned()
            }
            Self::Migration(error) => {
                tracing::error!("migration error: {error}");
                "Database migration failed".to_owned()
            }
            Self::Jwt(error) => {
                tracing::error!("jwt error: {error}");
                "Authentication failed".to_owned()
            }
            Self::Io(error) => {
                tracing::error!("io error: {error}");
                "Server IO failed".to_owned()
            }
            Self::Internal => "Internal server error".to_owned(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(ErrorBody {
            error: self.message(),
        });

        (status, body).into_response()
    }
}
