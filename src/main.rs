mod auth;
mod config;
mod errors;
mod models;
mod routes;
mod state;
mod youtube;

use std::time::Duration;

use axum::Router;
use config::AppConfig;
use errors::AppResult;
use routes::app_router;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> AppResult<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_origin
                .parse::<http::HeaderValue>()
                .map_err(|_| errors::AppError::Config("Invalid FRONTEND_ORIGIN".to_owned()))?,
        )
        .allow_credentials(true)
        .allow_headers([http::header::CONTENT_TYPE])
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PATCH,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ]);

    let state = AppState {
        pool,
        config: config.clone(),
    };

    let app: Router = app_router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(config.address).await?;
    tracing::info!("urlvibe-rs listening on {}", config.address);

    axum::serve(listener, app).await.map_err(|error| {
        tracing::error!("server error: {error}");
        errors::AppError::Internal
    })
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "urlvibe_rs=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
