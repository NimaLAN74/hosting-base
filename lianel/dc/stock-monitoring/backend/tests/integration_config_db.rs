//! TDD: integration tests for config and DB pool.
//! Require DATABASE_URL or POSTGRES_* env (e.g. in CI with Postgres service container).
//! Run with: DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres cargo test

use lianel_stock_monitoring_service::config::AppConfig;
use lianel_stock_monitoring_service::db;
use sqlx::PgPool;

#[tokio::test]
async fn config_from_env_produces_valid_database_url() {
    // In CI we set POSTGRES_PASSWORD etc.; from_env() must succeed and database_url() must be non-empty.
    let config = AppConfig::from_env();
    let config = config.expect("AppConfig::from_env() should succeed when POSTGRES_PASSWORD is set");
    let url = config.database_url();
    assert!(!url.is_empty(), "database_url must be non-empty");
    assert!(url.starts_with("postgresql://") || url.starts_with("postgres://"));
    assert!(!config.keycloak_jwks_url().is_empty());
    assert!(!config.keycloak_issuer().is_empty());
}

#[tokio::test]
async fn pool_connects_and_can_execute_simple_query() {
    let config = AppConfig::from_env().expect("config must load in CI");
    let pool = db::create_pool(&config.database_url())
        .await
        .expect("create_pool must succeed when Postgres is available");
    let one: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("SELECT 1 must succeed");
    assert_eq!(one.0, 1);
}

#[tokio::test]
async fn stock_monitoring_schema_exists_after_migration() {
    let config = AppConfig::from_env().expect("config must load in CI");
    let pool = db::create_pool(&config.database_url())
        .await
        .expect("pool must connect");
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'stock_monitoring')",
    )
    .fetch_one(&pool)
    .await
    .expect("query must succeed");
    assert!(exists.0, "schema stock_monitoring must exist after migration 022");
}
