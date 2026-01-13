use sqlx::PgPool;
use anyhow::Result;

pub mod queries;
pub mod queries_entsoe_osm;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPool::connect(database_url).await?;
    Ok(pool)
}

