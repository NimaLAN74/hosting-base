use sqlx::{PgPool, Row};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use crate::models::{RequestHistory, Control, ControlWithRequirements, RequirementRef, EvidenceItem, RemediationTask, ControlTest};

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
        // TIMESTAMP WITHOUT TIME ZONE is read as NaiveDateTime, then convert to UTC
        let created_at_naive: NaiveDateTime = row.get("created_at");
        let created_at = DateTime::<Utc>::from_utc(created_at_naive, Utc);
        
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

// --- Phase 4: controls, requirements, evidence ---

fn row_naive_to_utc(naive: NaiveDateTime) -> DateTime<Utc> {
    #[allow(deprecated)]
    DateTime::<Utc>::from_utc(naive, Utc)
}

/// List all controls (no pagination for now)
pub async fn list_controls(pool: &PgPool) -> Result<Vec<Control>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, internal_id, name, description, category, created_at
        FROM comp_ai.controls
        ORDER BY internal_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let created_at_naive: NaiveDateTime = row.get("created_at");
        out.push(Control {
            id: row.get("id"),
            internal_id: row.get("internal_id"),
            name: row.get("name"),
            description: row.get("description"),
            category: row.get("category"),
            created_at: row_naive_to_utc(created_at_naive),
        });
    }
    Ok(out)
}

/// Get one control with its requirement mappings (framework code + title)
pub async fn get_control_with_requirements(
    pool: &PgPool,
    control_id: i64,
) -> Result<Option<ControlWithRequirements>, sqlx::Error> {
    let control_row = sqlx::query(
        r#"
        SELECT id, internal_id, name, description, category, created_at
        FROM comp_ai.controls WHERE id = $1
        "#,
    )
    .bind(control_id)
    .fetch_optional(pool)
    .await?;

    let Some(crow) = control_row else {
        return Ok(None);
    };

    let created_at_naive: NaiveDateTime = crow.get("created_at");
    let control = Control {
        id: crow.get("id"),
        internal_id: crow.get("internal_id"),
        name: crow.get("name"),
        description: crow.get("description"),
        category: crow.get("category"),
        created_at: row_naive_to_utc(created_at_naive),
    };

    let req_rows = sqlx::query(
        r#"
        SELECT r.code, r.title, f.slug AS framework_slug
        FROM comp_ai.requirements r
        JOIN comp_ai.frameworks f ON f.id = r.framework_id
        JOIN comp_ai.control_requirements cr ON cr.requirement_id = r.id
        WHERE cr.control_id = $1
        ORDER BY f.slug, r.code
        "#,
    )
    .bind(control_id)
    .fetch_all(pool)
    .await?;

    let requirements: Vec<RequirementRef> = req_rows
        .into_iter()
        .map(|row| RequirementRef {
            code: row.get("code"),
            title: row.get("title"),
            framework_slug: row.get("framework_slug"),
        })
        .collect();

    Ok(Some(ControlWithRequirements {
        id: control.id,
        internal_id: control.internal_id,
        name: control.name,
        description: control.description,
        category: control.category,
        created_at: control.created_at,
        requirements,
    }))
}

/// List requirements from DB, optionally filtered by framework slug (Phase 5 audit view).
pub async fn list_requirements(
    pool: &PgPool,
    framework_slug: Option<&str>,
) -> Result<Vec<crate::models::RequirementListItem>, sqlx::Error> {
    let rows = if let Some(slug) = framework_slug {
        sqlx::query(
            r#"
            SELECT r.id, f.slug AS framework_slug, r.code, r.title, r.description
            FROM comp_ai.requirements r
            JOIN comp_ai.frameworks f ON f.id = r.framework_id
            WHERE f.slug = $1
            ORDER BY r.code
            "#,
        )
        .bind(slug)
    } else {
        sqlx::query(
            r#"
            SELECT r.id, f.slug AS framework_slug, r.code, r.title, r.description
            FROM comp_ai.requirements r
            JOIN comp_ai.frameworks f ON f.id = r.framework_id
            ORDER BY f.slug, r.code
            "#,
        )
    }
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        out.push(crate::models::RequirementListItem {
            id: row.get("id"),
            framework_slug: row.get("framework_slug"),
            code: row.get("code"),
            title: row.get("title"),
            description: row.get("description"),
        });
    }
    Ok(out)
}

/// List evidence for a control (or all if control_id is None)
pub async fn list_evidence(
    pool: &PgPool,
    control_id: Option<i64>,
    limit: u32,
    offset: u32,
) -> Result<Vec<EvidenceItem>, sqlx::Error> {
    let limit = limit.min(100);
    let rows = if let Some(cid) = control_id {
        sqlx::query(
            r#"
            SELECT id, control_id, type, source, description, link_url, collected_at, created_by
            FROM comp_ai.evidence
            WHERE control_id = $1
            ORDER BY collected_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(cid)
        .bind(limit as i64)
        .bind(offset as i64)
    } else {
        sqlx::query(
            r#"
            SELECT id, control_id, type, source, description, link_url, collected_at, created_by
            FROM comp_ai.evidence
            ORDER BY collected_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
    }
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let collected_at_naive: NaiveDateTime = row.get("collected_at");
        out.push(EvidenceItem {
            id: row.get("id"),
            control_id: row.get("control_id"),
            r#type: row.get("type"),
            source: row.get("source"),
            description: row.get("description"),
            link_url: row.get("link_url"),
            collected_at: row_naive_to_utc(collected_at_naive),
            created_by: row.get("created_by"),
        });
    }
    Ok(out)
}

/// Controls that have no evidence (Phase 5 gaps).
pub async fn list_controls_without_evidence(pool: &PgPool) -> Result<Vec<Control>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT c.id, c.internal_id, c.name, c.description, c.category, c.created_at
        FROM comp_ai.controls c
        LEFT JOIN comp_ai.evidence e ON e.control_id = c.id
        WHERE e.id IS NULL
        ORDER BY c.internal_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let created_at_naive: NaiveDateTime = row.get("created_at");
        out.push(Control {
            id: row.get("id"),
            internal_id: row.get("internal_id"),
            name: row.get("name"),
            description: row.get("description"),
            category: row.get("category"),
            created_at: row_naive_to_utc(created_at_naive),
        });
    }
    Ok(out)
}

/// List remediation tasks (optionally filter by control_id).
pub async fn list_remediation_tasks(
    pool: &PgPool,
    control_id: Option<i64>,
) -> Result<Vec<RemediationTask>, sqlx::Error> {
    let rows = if let Some(cid) = control_id {
        sqlx::query(
            r#"
            SELECT id, control_id, assigned_to, due_date, status, notes, created_at, updated_at
            FROM comp_ai.remediation_tasks
            WHERE control_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(cid)
    } else {
        sqlx::query(
            r#"
            SELECT id, control_id, assigned_to, due_date, status, notes, created_at, updated_at
            FROM comp_ai.remediation_tasks
            ORDER BY updated_at DESC
            "#,
        )
    }
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let created_at_naive: NaiveDateTime = row.get("created_at");
        let updated_at_naive: NaiveDateTime = row.get("updated_at");
        out.push(RemediationTask {
            id: row.get("id"),
            control_id: row.get("control_id"),
            assigned_to: row.get("assigned_to"),
            due_date: row.get::<Option<NaiveDate>, _>("due_date"),
            status: row.get("status"),
            notes: row.get("notes"),
            created_at: row_naive_to_utc(created_at_naive),
            updated_at: row_naive_to_utc(updated_at_naive),
        });
    }
    Ok(out)
}

/// Get remediation task for a control (if any).
pub async fn get_remediation_by_control_id(
    pool: &PgPool,
    control_id: i64,
) -> Result<Option<RemediationTask>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, control_id, assigned_to, due_date, status, notes, created_at, updated_at
        FROM comp_ai.remediation_tasks
        WHERE control_id = $1
        "#,
    )
    .bind(control_id)
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    let created_at_naive: NaiveDateTime = r.get("created_at");
    let updated_at_naive: NaiveDateTime = r.get("updated_at");
    Ok(Some(RemediationTask {
        id: r.get("id"),
        control_id: r.get("control_id"),
        assigned_to: r.get("assigned_to"),
        due_date: r.get::<Option<NaiveDate>, _>("due_date"),
        status: r.get("status"),
        notes: r.get("notes"),
        created_at: row_naive_to_utc(created_at_naive),
        updated_at: row_naive_to_utc(updated_at_naive),
    }))
}

/// Create or update remediation task for a control (upsert by control_id).
/// Only provided fields are updated; missing fields leave existing values unchanged.
pub async fn upsert_remediation(
    pool: &PgPool,
    control_id: i64,
    assigned_to: Option<&str>,
    due_date: Option<NaiveDate>,
    status: Option<&str>,
    notes: Option<&str>,
) -> Result<RemediationTask, sqlx::Error> {
    let existing = get_remediation_by_control_id(pool, control_id).await?;
    if let Some(ex) = existing {
        let assigned_to = assigned_to.or(ex.assigned_to.as_deref());
        let due_date = due_date.or(ex.due_date);
        let status = status.as_deref().unwrap_or(&ex.status);
        let notes = notes.or(ex.notes.as_deref());
        let row = sqlx::query(
            r#"
            UPDATE comp_ai.remediation_tasks
            SET assigned_to = $2, due_date = $3, status = $4, notes = $5, updated_at = CURRENT_TIMESTAMP
            WHERE control_id = $1
            RETURNING id, control_id, assigned_to, due_date, status, notes, created_at, updated_at
            "#,
        )
        .bind(control_id)
        .bind(assigned_to)
        .bind(due_date)
        .bind(status)
        .bind(notes)
        .fetch_one(pool)
        .await?;
        let created_at_naive: NaiveDateTime = row.get("created_at");
        let updated_at_naive: NaiveDateTime = row.get("updated_at");
        return Ok(RemediationTask {
            id: row.get("id"),
            control_id: row.get("control_id"),
            assigned_to: row.get("assigned_to"),
            due_date: row.get::<Option<NaiveDate>, _>("due_date"),
            status: row.get("status"),
            notes: row.get("notes"),
            created_at: row_naive_to_utc(created_at_naive),
            updated_at: row_naive_to_utc(updated_at_naive),
        });
    }
    let status_val = status.unwrap_or("open");
    let row = sqlx::query(
        r#"
        INSERT INTO comp_ai.remediation_tasks (control_id, assigned_to, due_date, status, notes)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, control_id, assigned_to, due_date, status, notes, created_at, updated_at
        "#,
    )
    .bind(control_id)
    .bind(assigned_to)
    .bind(due_date)
    .bind(status_val)
    .bind(notes)
    .fetch_one(pool)
    .await?;

    let created_at_naive: NaiveDateTime = row.get("created_at");
    let updated_at_naive: NaiveDateTime = row.get("updated_at");
    Ok(RemediationTask {
        id: row.get("id"),
        control_id: row.get("control_id"),
        assigned_to: row.get("assigned_to"),
        due_date: row.get::<Option<NaiveDate>, _>("due_date"),
        status: row.get("status"),
        notes: row.get("notes"),
        created_at: row_naive_to_utc(created_at_naive),
        updated_at: row_naive_to_utc(updated_at_naive),
    })
}

/// Create an evidence record and link to control
pub async fn create_evidence(
    pool: &PgPool,
    control_id: i64,
    r#type: &str,
    source: Option<&str>,
    description: Option<&str>,
    link_url: Option<&str>,
    created_by: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO comp_ai.evidence (control_id, type, source, description, link_url, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, collected_at
        "#,
    )
    .bind(control_id)
    .bind(r#type)
    .bind(source)
    .bind(description)
    .bind(link_url)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("id"))
}

// --- Phase 6B: control_tests ---

/// List automated tests for a control.
pub async fn list_control_tests(pool: &PgPool, control_id: i64) -> Result<Vec<ControlTest>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, control_id, name, test_type, schedule, last_run_at, last_result, last_details, created_at, updated_at
        FROM comp_ai.control_tests
        WHERE control_id = $1
        ORDER BY name
        "#,
    )
    .bind(control_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let created_at_naive: NaiveDateTime = row.get("created_at");
        let updated_at_naive: NaiveDateTime = row.get("updated_at");
        let last_run_at: Option<NaiveDateTime> = row.get("last_run_at");
        out.push(ControlTest {
            id: row.get("id"),
            control_id: row.get("control_id"),
            name: row.get("name"),
            test_type: row.get("test_type"),
            schedule: row.get("schedule"),
            last_run_at: last_run_at.map(row_naive_to_utc),
            last_result: row.get("last_result"),
            last_details: row.get("last_details"),
            created_at: row_naive_to_utc(created_at_naive),
            updated_at: row_naive_to_utc(updated_at_naive),
        });
    }
    Ok(out)
}

/// List all control tests (optionally filter by control_id).
pub async fn list_all_control_tests(pool: &PgPool, control_id: Option<i64>) -> Result<Vec<ControlTest>, sqlx::Error> {
    let rows = if let Some(cid) = control_id {
        sqlx::query(
            r#"
            SELECT id, control_id, name, test_type, schedule, last_run_at, last_result, last_details, created_at, updated_at
            FROM comp_ai.control_tests
            WHERE control_id = $1
            ORDER BY control_id, name
            "#,
        )
        .bind(cid)
    } else {
        sqlx::query(
            r#"
            SELECT id, control_id, name, test_type, schedule, last_run_at, last_result, last_details, created_at, updated_at
            FROM comp_ai.control_tests
            ORDER BY control_id, name
            "#,
        )
    }
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let created_at_naive: NaiveDateTime = row.get("created_at");
        let updated_at_naive: NaiveDateTime = row.get("updated_at");
        let last_run_at: Option<NaiveDateTime> = row.get("last_run_at");
        out.push(ControlTest {
            id: row.get("id"),
            control_id: row.get("control_id"),
            name: row.get("name"),
            test_type: row.get("test_type"),
            schedule: row.get("schedule"),
            last_run_at: last_run_at.map(row_naive_to_utc),
            last_result: row.get("last_result"),
            last_details: row.get("last_details"),
            created_at: row_naive_to_utc(created_at_naive),
            updated_at: row_naive_to_utc(updated_at_naive),
        });
    }
    Ok(out)
}

/// Record a test run result (updates last_run_at, last_result, last_details).
pub async fn record_control_test_result(
    pool: &PgPool,
    test_id: i64,
    result: &str,
    details: Option<&str>,
) -> Result<ControlTest, sqlx::Error> {
    let row = sqlx::query(
        r#"
        UPDATE comp_ai.control_tests
        SET last_run_at = CURRENT_TIMESTAMP, last_result = $2, last_details = $3, updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING id, control_id, name, test_type, schedule, last_run_at, last_result, last_details, created_at, updated_at
        "#,
    )
    .bind(test_id)
    .bind(result)
    .bind(details)
    .fetch_one(pool)
    .await?;

    let created_at_naive: NaiveDateTime = row.get("created_at");
    let updated_at_naive: NaiveDateTime = row.get("updated_at");
    let last_run_at: Option<NaiveDateTime> = row.get("last_run_at");
    Ok(ControlTest {
        id: row.get("id"),
        control_id: row.get("control_id"),
        name: row.get("name"),
        test_type: row.get("test_type"),
        schedule: row.get("schedule"),
        last_run_at: last_run_at.map(row_naive_to_utc),
        last_result: row.get("last_result"),
        last_details: row.get("last_details"),
        created_at: row_naive_to_utc(created_at_naive),
        updated_at: row_naive_to_utc(updated_at_naive),
    })
}
