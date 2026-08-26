---
title: Aura RS Backend
emoji: 🦀
colorFrom: red
colorTo: gray
sdk: docker
app_port: 7860
pinned: false
---

# Zenplay Backend

Zenplay Backend is the Rust API service for the Aura-rs application. It provides authentication, playlist-label management, and YouTube library operations for the React frontend.

The service is built with `Axum` and `Tokio`, uses `PostgreSQL` via `sqlx`, and is designed to run against a Neon database. It stores user accounts, saved YouTube items, favorite status, and playlist labels inferred from item rows.

## What This Service Does

- Registers users with `first name`, `email`, `username`, and `password`
- Authenticates users with `email or username + password`
- Issues JWT-based sessions through an HTTP-only cookie
- Stores YouTube items with:
  - original YouTube URL
  - normalized YouTube video ID
  - custom title
  - single playlist label
  - favorite flag
- Returns playlist labels by grouping saved items
- Supports playlist label rename by bulk-updating matching rows
- Enforces one saved video per `(user, label, video_id)` combination

## Core API

### Authentication

- `POST /api/auth/signup`
- `POST /api/auth/login`
- `POST /api/auth/logout`
- `GET /api/auth/me`

### Library

- `GET /api/labels`
- `GET /api/items?view=all`
- `GET /api/items?view=favorites`
- `GET /api/items?label=:label`
- `POST /api/items`
- `PATCH /api/items/:item_id`
- `DELETE /api/items/:item_id`
- `PATCH /api/labels/:label/rename`

### Health

- `GET /health`

## Data Model

### `users`

- `id`
- `first_name`
- `email`
- `username`
- `password_hash`
- `created_at`
- `updated_at`

### `items`

- `id`
- `user_id`
- `youtube_url`
- `youtube_video_id`
- `title`
- `label`
- `is_favorite`
- `created_at`
- `updated_at`

Playlist labels are not stored in a separate table. A playlist exists only when at least one item is assigned to that label.

## Security and Validation

- Passwords are hashed with `argon2`
- Sessions are signed with JWT and stored in an HTTP-only cookie
- CORS is restricted to the configured frontend origin
- Supported YouTube URL formats are validated and normalized before persistence
- Authenticated routes load the current user from the signed cookie

## Environment Variables

See [.env.example](./.env.example).

Required configuration:

- `DATABASE_URL`
- `JWT_SECRET`
- `FRONTEND_ORIGIN`

Common runtime configuration:

- `HOST`
- `PORT`
- `RUST_LOG`
- `APP_ENV`
- `COOKIE_SECURE`

## Local Development

1. Copy `.env.example` to `.env`
2. Set a valid `DATABASE_URL`
3. Start the server:

```bash
cargo run
```

On startup the service:

- loads environment variables
- connects to PostgreSQL
- runs SQL migrations from `migrations/`
- starts the HTTP server

## Notes

- This backend is intended to be deployed separately from the frontend
- It is currently designed for private user libraries only
- Playlist grouping is label-based rather than using a separate playlist table

- Command for deploying to Hugging Face Spaces:
```bash
git push hf master:main
```
