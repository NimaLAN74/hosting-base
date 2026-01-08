use sqlx::{PgPool, Row};
use crate::models::*;

pub async fn get_energy_records(
    pool: &PgPool,
    country_code: Option<&str>,
    year: Option<i32>,
    product_code: Option<&str>,
    flow_code: Option<&str>,
    source_table: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<(Vec<EnergyRecord>, i64), sqlx::Error> {
    // Build WHERE conditions dynamically
    let mut conditions = vec!["e.source_system = 'eurostat'".to_string()];
    let mut bind_count = 0;

    if country_code.is_some() {
        bind_count += 1;
        conditions.push(format!("e.country_code = ${}", bind_count));
    }
    if year.is_some() {
        bind_count += 1;
        conditions.push(format!("e.year = ${}", bind_count));
    }
    if product_code.is_some() {
        bind_count += 1;
        conditions.push(format!("e.product_code = ${}", bind_count));
    }
    if flow_code.is_some() {
        bind_count += 1;
        conditions.push(format!("e.flow_code = ${}", bind_count));
    }
    if source_table.is_some() {
        bind_count += 1;
        conditions.push(format!("e.source_table = ${}", bind_count));
    }

    let where_clause = conditions.join(" AND ");
    bind_count += 1;
    let limit_param = bind_count;
    bind_count += 1;
    let offset_param = bind_count;

    // Get total count (simplified - count all matching records)
    let count_query = format!(
        "SELECT COUNT(*) FROM fact_energy_annual e WHERE {}",
        where_clause
    );
    
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
    if let Some(cc) = country_code {
        count_q = count_q.bind(cc);
    }
    if let Some(y) = year {
        count_q = count_q.bind(y);
    }
    if let Some(pc) = product_code {
        count_q = count_q.bind(pc);
    }
    if let Some(fc) = flow_code {
        count_q = count_q.bind(fc);
    }
    if let Some(st) = source_table {
        count_q = count_q.bind(st);
    }
    let total = count_q.fetch_one(pool).await?;

    // Main query
    let query_str = format!(
        r#"
        SELECT 
            e.id,
            e.country_code,
            c.country_name,
            e.year,
            e.product_code,
            p.product_name,
            e.flow_code,
            f.flow_name,
            e.value_gwh,
            e.unit,
            e.source_table,
            e.ingestion_timestamp
        FROM fact_energy_annual e
        LEFT JOIN dim_country c ON e.country_code = c.country_code
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code
        WHERE {}
        ORDER BY e.year DESC, e.country_code
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut query = sqlx::query(&query_str);
    
    if let Some(cc) = country_code {
        query = query.bind(cc);
    }
    if let Some(y) = year {
        query = query.bind(y);
    }
    if let Some(pc) = product_code {
        query = query.bind(pc);
    }
    if let Some(fc) = flow_code {
        query = query.bind(fc);
    }
    if let Some(st) = source_table {
        query = query.bind(st);
    }
    query = query.bind(limit as i64);
    query = query.bind(offset as i64);

    let rows = query.fetch_all(pool).await?;

    let records: Vec<EnergyRecord> = rows
        .iter()
        .map(|row| EnergyRecord {
            id: row.get(0),
            country_code: row.get(1),
            country_name: row.get(2),
            year: row.get(3),
            product_code: row.get(4),
            product_name: row.get(5),
            flow_code: row.get(6),
            flow_name: row.get(7),
            // Convert NUMERIC to f64
            value_gwh: row.get::<rust_decimal::Decimal, _>(8).to_f64().unwrap_or(0.0),
            unit: row.get(9),
            source_table: row.get(10),
            ingestion_timestamp: row.get(11),
        })
        .collect();

    Ok((records, total))
}

pub async fn get_energy_summary(
    pool: &PgPool,
    country_code: Option<&str>,
    year: Option<i32>,
    group_by: &str,
) -> Result<Vec<(String, f64, i64)>, sqlx::Error> {
    // Validate group_by to prevent SQL injection
    let group_column = match group_by {
        "country" => "e.country_code",
        "year" => "e.year::text",
        "product" => "e.product_code",
        "flow" => "e.flow_code",
        _ => "e.country_code",
    };

    let mut where_clauses = vec!["e.source_system = 'eurostat'".to_string()];
    let mut bind_idx = 1;
    
    if let Some(_) = country_code {
        where_clauses.push(format!("e.country_code = ${}", bind_idx));
        bind_idx += 1;
    }
    if let Some(_) = year {
        where_clauses.push(format!("e.year = ${}", bind_idx));
        bind_idx += 1;
    }

    let where_clause = where_clauses.join(" AND ");

    let query_str = format!(
        r#"
        SELECT 
            {} as group_key,
            SUM(e.value_gwh) as total_gwh,
            COUNT(*) as record_count
        FROM fact_energy_annual e
        WHERE {}
        GROUP BY {}
        ORDER BY total_gwh DESC
        "#,
        group_column, where_clause, group_column
    );

    let mut query = sqlx::query(&query_str);
    
    if let Some(cc) = country_code {
        query = query.bind(cc);
    }
    if let Some(y) = year {
        query = query.bind(y);
    }

    let rows = query.fetch_all(pool).await?;

    let results: Vec<(String, f64, i64)> = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>(0),
                row.get::<rust_decimal::Decimal, _>(1).to_f64().unwrap_or(0.0),
                row.get::<i64, _>(2),
            )
        })
        .collect();

    Ok(results)
}

pub async fn get_database_stats(pool: &PgPool) -> Result<(i64, i64, i64, i64), sqlx::Error> {
    // First check if the table exists
    let table_exists: bool = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name = 'fact_energy_annual'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        // Table doesn't exist yet - return zeros
        return Ok((0, 0, 0, 0));
    }

    // Table exists - get stats
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_records,
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years,
            COUNT(DISTINCT source_table) as tables
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok((
        row.get::<i64, _>(0),
        row.get::<i64, _>(1),
        row.get::<i64, _>(2),
        row.get::<i64, _>(3),
    ))
}

