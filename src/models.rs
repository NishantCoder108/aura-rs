use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub first_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: Uuid,
    pub first_name: String,
    pub email: String,
    pub username: String,
}

impl From<UserRecord> for AuthUser {
    fn from(value: UserRecord) -> Self {
        Self {
            id: value.id,
            first_name: value.first_name,
            email: value.email,
            username: value.username,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub youtube_url: String,
    pub youtube_video_id: String,
    pub title: String,
    pub label: String,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelSummary {
    pub label: String,
    pub item_count: i64,
}
