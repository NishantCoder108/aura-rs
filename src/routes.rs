use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgDatabaseError};
use uuid::Uuid;

use crate::{
    auth::{
        CurrentUser, create_token, hash_password, normalize_email, normalize_username,
        verify_password,
    },
    errors::{AppError, AppResult},
    models::{AuthUser, ItemRecord, LabelSummary, UserRecord},
    state::AppState,
    youtube::extract_video_id,
};
pub fn app_router() -> Router<AppState> {
    let labels_router = Router::new()
        .route("/", get(list_labels))
        .route("/{label}/rename", patch(rename_label));

    let items_router = Router::new()
        .route("/", get(list_items).post(create_item))
        .route("/{item_id}", patch(update_item).delete(delete_item));

    let user_router = Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me));

    Router::new()
        .nest("/api/auth", user_router)
        .nest("/api/items", items_router)
        .nest("/api/labels", labels_router)
        .route("/api/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    user: AuthUser,
    token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignupRequest {
    first_name: String,
    email: String,
    username: String,
    password: String,
}

async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    validate_signup(&payload)?;

    let first_name = payload.first_name.trim().to_owned();
    let email = normalize_email(&payload.email);
    let username = normalize_username(&payload.username);
    let password_hash = hash_password(payload.password.trim())?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (first_name, email, username, password_hash)
        VALUES ($1, $2, $3, $4)
        RETURNING id, first_name, email, username, password_hash
        "#,
    )
    .bind(first_name)
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(map_database_error)?;

    let token = create_token(user.id, &state.config.jwt_secret)?;
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user: user.into(),
            token: Some(token),
        }),
    ))
}

#[derive(Deserialize)]
struct LoginRequest {
    identifier: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    if payload.identifier.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Identifier and password are required".to_owned(),
        ));
    }

    let identifier = payload.identifier.trim().to_ascii_lowercase();
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, first_name, email, username, password_hash
        FROM users
        WHERE email = $1 OR username = $1
        "#,
    )
    .bind(identifier)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_owned()))?;

    verify_password(payload.password.trim(), &user.password_hash)
        .map_err(|_| AppError::Unauthorized("Invalid credentials".to_owned()))?;

    let token = create_token(user.id, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse {
        user: user.into(),
        token: Some(token),
    }))
}

async fn logout() -> AppResult<StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

async fn me(CurrentUser(user): CurrentUser) -> Json<AuthResponse> {
    Json(AuthResponse { user, token: None })
}

async fn list_labels(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<LabelSummary>>> {
    let labels = sqlx::query_as::<_, LabelSummary>(
        r#"
        SELECT label, COUNT(*)::BIGINT AS item_count
        FROM items
        WHERE user_id = $1
        GROUP BY label
        ORDER BY LOWER(label), label
        "#,
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(labels))
}

#[derive(Debug, Deserialize)]
struct ItemListQuery {
    view: Option<String>,
    label: Option<String>,
}

async fn list_items(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<ItemListQuery>,
) -> AppResult<Json<Vec<ItemRecord>>> {
    let rows = match (query.view.as_deref(), query.label.as_deref()) {
        (Some(_), Some(_)) => {
            return Err(AppError::BadRequest(
                "Use either a view or a label filter".to_owned(),
            ));
        }
        (_, Some(label)) => {
            sqlx::query_as::<_, ItemRecord>(
                r#"
                SELECT id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
                FROM items
                WHERE user_id = $1 AND label = $2
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .bind(user.id)
            .bind(clean_label(label)?)
            .fetch_all(&pool)
            .await?
        }
        (Some("favorites"), None) => {  
            sqlx::query_as::<_, ItemRecord>(
                r#"
                SELECT id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
                FROM items
                WHERE user_id = $1 AND is_favorite = TRUE
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .bind(user.id)
            .fetch_all(&pool)
            .await?
        }
        (Some("all") | None, None) => {
            sqlx::query_as::<_, ItemRecord>(
                r#"
                SELECT id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
                FROM items
                WHERE user_id = $1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .bind(user.id)
            .fetch_all(&pool)
            .await?
        }
        (Some(_), None) => {
            return Err(AppError::BadRequest(
                "Unknown items view requested".to_owned(),
            ));
        }
    };

    Ok(Json(rows))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateItemRequest {
    youtube_url: String,
    title: String,
    label: String,
}

async fn create_item(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<CreateItemRequest>,
) -> AppResult<(StatusCode, Json<ItemRecord>)> {
    let title = clean_title(&payload.title)?;
    let label = clean_label(&payload.label)?;
    let youtube_url = payload.youtube_url.trim().to_owned();
    let youtube_video_id = extract_video_id(&youtube_url)?;

    let item = sqlx::query_as::<_, ItemRecord>(
        r#"
        INSERT INTO items (user_id, youtube_url, youtube_video_id, title, label)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
        "#,
    )
    .bind(user.id)
    .bind(youtube_url)
    .bind(youtube_video_id)
    .bind(title)
    .bind(label)
    .fetch_one(&pool)
    .await
    .map_err(map_database_error)?;

    Ok((StatusCode::CREATED, Json(item)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateItemRequest {
    title: Option<String>,
    label: Option<String>,
    is_favorite: Option<bool>,
}

async fn update_item(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
    Path(item_id): Path<Uuid>,
    Json(payload): Json<UpdateItemRequest>,
) -> AppResult<Json<ItemRecord>> {
    let existing = sqlx::query_as::<_, ItemRecord>(
        r#"
        SELECT id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
        FROM items
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(item_id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Item not found".to_owned()))?;

    let title = match payload.title {
        Some(value) => clean_title(&value)?,
        None => existing.title,
    };
    let label = match payload.label {
        Some(value) => clean_label(&value)?,
        None => existing.label,
    };
    let is_favorite = payload.is_favorite.unwrap_or(existing.is_favorite);

    let updated = sqlx::query_as::<_, ItemRecord>(
        r#"
        UPDATE items
        SET title = $1, label = $2, is_favorite = $3, updated_at = NOW()
        WHERE id = $4 AND user_id = $5
        RETURNING id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
        "#,
    )
    .bind(title)
    .bind(label)
    .bind(is_favorite)
    .bind(item_id)
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .map_err(map_database_error)?;

    Ok(Json(updated))
}

async fn delete_item(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
    Path(item_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let result = sqlx::query(
        r#"
        DELETE FROM items
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(item_id)
    .bind(user.id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Item not found".to_owned()));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameLabelRequest {
    new_label: String,
}

async fn rename_label(
    State(pool): State<PgPool>,
    CurrentUser(user): CurrentUser,
    Path(label): Path<String>,
    Json(payload): Json<RenameLabelRequest>,
) -> AppResult<Json<Vec<ItemRecord>>> {
    let old_label = clean_label(&label)?;
    let new_label = clean_label(&payload.new_label)?;

    if old_label == new_label {
        return Err(AppError::BadRequest(
            "New label must be different from the current label".to_owned(),
        ));
    }

    let updated = sqlx::query_as::<_, ItemRecord>(
        r#"
        UPDATE items
        SET label = $1, updated_at = NOW()
        WHERE user_id = $2 AND label = $3
        RETURNING id, user_id, youtube_url, youtube_video_id, title, label, is_favorite, created_at, updated_at
        "#,
    )
    .bind(new_label)
    .bind(user.id)
    .bind(old_label)
    .fetch_all(&pool)
    .await
    .map_err(map_database_error)?;

    if updated.is_empty() {
        return Err(AppError::NotFound("Playlist label not found".to_owned()));
    }

    Ok(Json(updated))
}

fn validate_signup(payload: &SignupRequest) -> AppResult<()> {
    if payload.first_name.trim().is_empty()
        || payload.email.trim().is_empty()
        || payload.username.trim().is_empty()
        || payload.password.trim().is_empty()
    {
        return Err(AppError::BadRequest(
            "First name, email, username, and password are required".to_owned(),
        ));
    }

    if payload.password.trim().len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".to_owned(),
        ));
    }

    Ok(())
}

fn clean_title(value: &str) -> AppResult<String> {
    let title = value.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("Title is required".to_owned()));
    }

    Ok(title.to_owned())
}

fn clean_label(value: &str) -> AppResult<String> {
    let label = value.trim();
    if label.is_empty() {
        return Err(AppError::BadRequest(
            "Playlist label is required".to_owned(),
        ));
    }

    Ok(label.to_owned())
}

fn map_database_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        let pg_error: &PgDatabaseError = database_error.downcast_ref::<PgDatabaseError>();
        match pg_error.code() {
            "23505" => {
                return AppError::Conflict(
                    "This record already exists for the selected playlist".to_owned(),
                );
            }
            "23503" => {
                return AppError::BadRequest("Referenced record does not exist".to_owned());
            }
            _ => {}
        }
    }

    AppError::Database(error)
}
