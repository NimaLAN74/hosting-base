use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};
use crate::models::RequestHistory;

/// Save a request to the database
pub async fn save_request(
    pool: &PgPool,
    user_id: &str,
    request_text: &str,
    response_text: Option<&str>,
    model_used: Option<&str>,
    tokens_used: Option<i32>,
    processing_time_ms: i64,
    status: &str,
    error_message: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO comp_ai.requests (
            user_id, request_text, response_text, model_used, 
            tokens_used, processing_time_ms, status, error_message
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(request_text)
    .bind(response_text)
    .bind(model_used)
    .bind(tokens_used)
    .bind(processing_time_ms)
    .bind(status)
    .bind(error_message)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("id"))
}

/// Get request history for a user
pub async fn get_request_history(
    pool: &PgPool,
    user_id: &str,
    limit: u32,
    offset: u32,
) -> Result<(Vec<RequestHistory>, i64), sqlx::Error> {
    // Get total count
    let count_row = sqlx::query(
        r#"
        SELECT COUNT(*) as total
        FROM comp_ai.requests
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let total: i64 = count_row.get("total");

    // Get paginated results
    let rows = sqlx::query(
        r#"
        SELECT 
            id, user_id, request_text, response_text, model_used,
            tokens_used, created_at
        FROM comp_ai.requests
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await?;

    let mut history = Vec::new();
    for row in rows {
        let created_at: DateTime<Utc> = row.get("created_at");
        history.push(RequestHistory {
            id: row.get("id"),
            user_id: row.get("user_id"),
            prompt: row.get("request_text"),
            response: row.get("response_text"),
            model_used: row.get("model_used"),
            tokens_used: row.get("tokens_used"),
            created_at,
        });
    }

    Ok((history, total))
}
