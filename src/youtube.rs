use std::collections::HashMap;

use url::Url;

use crate::errors::{AppError, AppResult};

pub fn extract_video_id(raw_url: &str) -> AppResult<String> {
    let parsed = Url::parse(raw_url.trim())
        .map_err(|_| AppError::BadRequest("Please provide a valid YouTube URL".to_owned()))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Please provide a valid YouTube URL".to_owned()))?
        .to_ascii_lowercase();

    let path = parsed.path().trim_matches('/');
    let params: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

    let candidate = match host.as_str() {
        "youtu.be" | "www.youtu.be" => Some(path.to_owned()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" => {
            if path == "watch" {
                params.get("v").cloned()
            } else if let Some(value) = path.strip_prefix("shorts/") {
                Some(value.to_owned())
            } else if let Some(value) = path.strip_prefix("embed/") {
                Some(value.to_owned())
            } else {
                None
            }
        }
        _ => None,
    };

    let video_id = candidate
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("Please provide a valid YouTube URL".to_owned()))?;

    if !video_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::BadRequest(
            "Unable to extract a valid YouTube video id".to_owned(),
        ));
    }

    Ok(video_id)
}

#[cfg(test)]
mod tests {
    use super::extract_video_id;

    #[test]
    fn parses_watch_urls() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn parses_short_urls() {
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn parses_shorts_urls() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn rejects_non_youtube_urls() {
        assert!(extract_video_id("https://example.com/video").is_err());
    }
}
