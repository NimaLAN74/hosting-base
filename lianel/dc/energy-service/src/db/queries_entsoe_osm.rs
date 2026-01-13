use sqlx::{PgPool, Row};
use sqlx::types::BigDecimal;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Utc};
use std::result::Result;
use crate::models::*;

pub async fn get_electricity_timeseries_records(
    pool: &PgPool,
    country_code: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    production_type: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<(Vec<ElectricityTimeseriesRecord>, i64), sqlx::Error> {
    // Build WHERE conditions
    let mut conditions = vec!["1=1".to_string()];
    let mut bind_count = 0;

    if country_code.is_some() {
        bind_count += 1;
        conditions.push(format!("country_code = ${}", bind_count));
    }
    if start_date.is_some() {
        bind_count += 1;
        conditions.push(format!("timestamp_utc >= ${}", bind_count));
    }
    if end_date.is_some() {
        bind_count += 1;
        conditions.push(format!("timestamp_utc <= ${}", bind_count));
    }
    if production_type.is_some() {
        bind_count += 1;
        conditions.push(format!("production_type = ${}", bind_count));
    }

    let where_clause = conditions.join(" AND ");
    bind_count += 1;
    let limit_param = bind_count;
    bind_count += 1;
    let offset_param = bind_count;

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) FROM fact_electricity_timeseries WHERE {}",
        where_clause
    );

    let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
    if let Some(cc) = country_code {
        count_q = count_q.bind(cc);
    }
    if let Some(sd) = start_date {
        count_q = count_q.bind(sd);
    }
    if let Some(ed) = end_date {
        count_q = count_q.bind(ed);
    }
    if let Some(pt) = production_type {
        count_q = count_q.bind(pt);
    }

    let total = count_q.fetch_one(pool).await?;

    // Get records
    let query_str = format!(
        r#"
        SELECT 
            id,
            timestamp_utc,
            country_code,
            bidding_zone,
            production_type,
            load_mw,
            generation_mw,
            resolution,
            quality_flag
        FROM fact_electricity_timeseries
        WHERE {}
        ORDER BY timestamp_utc DESC, country_code
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut query = sqlx::query(&query_str);
    if let Some(cc) = country_code {
        query = query.bind(cc);
    }
    if let Some(sd) = start_date {
        query = query.bind(sd);
    }
    if let Some(ed) = end_date {
        query = query.bind(ed);
    }
    if let Some(pt) = production_type {
        query = query.bind(pt);
    }
    query = query.bind(limit as i64);
    query = query.bind(offset as i64);

    let rows = query.fetch_all(pool).await?;

    let records: Vec<ElectricityTimeseriesRecord> = rows
        .iter()
        .map(|row| ElectricityTimeseriesRecord {
            id: row.get::<i64, _>(0),
            timestamp_utc: row.get::<DateTime<Utc>, _>(1),
            country_code: row.get::<String, _>(2),
            bidding_zone: row.get::<Option<String>, _>(3),
            production_type: row.get::<Option<String>, _>(4),
            load_mw: row.get::<Option<BigDecimal>, _>(5).and_then(|v| v.to_f64()),
            generation_mw: row.get::<Option<BigDecimal>, _>(6).and_then(|v| v.to_f64()),
            resolution: row.get::<String, _>(7),
            quality_flag: row.get::<Option<String>, _>(8),
        })
        .collect();

    Ok((records, total))
}

pub async fn get_geo_feature_records(
    pool: &PgPool,
    region_id: Option<&str>,
    feature_name: Option<&str>,
    snapshot_year: Option<i32>,
    limit: u32,
    offset: u32,
) -> Result<(Vec<GeoFeatureRecord>, i64), sqlx::Error> {
    // Build WHERE conditions
    let mut conditions = vec!["1=1".to_string()];
    let mut bind_count = 0;

    if region_id.is_some() {
        bind_count += 1;
        conditions.push(format!("region_id = ${}", bind_count));
    }
    if feature_name.is_some() {
        bind_count += 1;
        conditions.push(format!("feature_name = ${}", bind_count));
    }
    if snapshot_year.is_some() {
        bind_count += 1;
        conditions.push(format!("snapshot_year = ${}", bind_count));
    }

    let where_clause = conditions.join(" AND ");
    bind_count += 1;
    let limit_param = bind_count;
    bind_count += 1;
    let offset_param = bind_count;

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) FROM fact_geo_region_features WHERE {}",
        where_clause
    );

    let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
    if let Some(rid) = region_id {
        count_q = count_q.bind(rid);
    }
    if let Some(fn) = feature_name {
        count_q = count_q.bind(fn);
    }
    if let Some(sy) = snapshot_year {
        count_q = count_q.bind(sy);
    }

    let total = count_q.fetch_one(pool).await?;

    // Get records
    let query_str = format!(
        r#"
        SELECT 
            id,
            region_id,
            feature_name,
            feature_value,
            snapshot_year
        FROM fact_geo_region_features
        WHERE {}
        ORDER BY region_id, feature_name, snapshot_year DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut query = sqlx::query(&query_str);
    if let Some(rid) = region_id {
        query = query.bind(rid);
    }
    if let Some(fn) = feature_name {
        query = query.bind(fn);
    }
    if let Some(sy) = snapshot_year {
        query = query.bind(sy);
    }
    query = query.bind(limit as i64);
    query = query.bind(offset as i64);

    let rows = query.fetch_all(pool).await?;

    let records: Vec<GeoFeatureRecord> = rows
        .iter()
        .map(|row| GeoFeatureRecord {
            id: row.get::<i64, _>(0),
            region_id: row.get::<String, _>(1),
            feature_name: row.get::<String, _>(2),
            feature_value: row.get::<BigDecimal, _>(3).to_f64().unwrap_or(0.0),
            snapshot_year: row.get::<i32, _>(4),
        })
        .collect();

    Ok((records, total))
}

