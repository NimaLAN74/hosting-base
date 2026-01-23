use std::env;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub keycloak_admin_user: String,
    pub keycloak_admin_password: String,
    pub keycloak_client_id: String,
    pub keycloak_client_secret: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String,
    pub comp_ai_api_key: Option<String>,
    pub comp_ai_model_path: Option<String>,
    pub comp_ai_max_tokens: u32,
    pub comp_ai_temperature: f64,
}

impl AppConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.postgres_user, self.postgres_password, self.postgres_host, self.postgres_port, self.postgres_db
        )
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3002".to_string())
                .parse()
                .context("Invalid PORT")?,
            keycloak_url: env::var("KEYCLOAK_URL")
                .unwrap_or_else(|_| "https://auth.lianel.se".to_string()),
            keycloak_realm: env::var("KEYCLOAK_REALM")
                .unwrap_or_else(|_| "lianel".to_string()),
            keycloak_admin_user: env::var("KEYCLOAK_ADMIN_USER")
                .context("KEYCLOAK_ADMIN_USER not set")?,
            keycloak_admin_password: env::var("KEYCLOAK_ADMIN_PASSWORD")
                .context("KEYCLOAK_ADMIN_PASSWORD not set")?,
            keycloak_client_id: env::var("COMP_AI_KEYCLOAK_CLIENT_ID")
                .unwrap_or_else(|_| "comp-ai-service".to_string()),
            keycloak_client_secret: env::var("COMP_AI_KEYCLOAK_CLIENT_SECRET")
                .context("COMP_AI_KEYCLOAK_CLIENT_SECRET not set")?,
            postgres_host: env::var("POSTGRES_HOST")
                .unwrap_or_else(|_| "172.18.0.1".to_string()),
            postgres_port: env::var("POSTGRES_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .context("Invalid POSTGRES_PORT")?,
            postgres_user: env::var("POSTGRES_USER")
                .unwrap_or_else(|_| "postgres".to_string()),
            postgres_password: env::var("POSTGRES_PASSWORD")
                .context("POSTGRES_PASSWORD not set")?,
            postgres_db: env::var("POSTGRES_DB")
                .unwrap_or_else(|_| "lianel_energy".to_string()),
            comp_ai_api_key: env::var("COMP_AI_API_KEY").ok(),
            comp_ai_model_path: env::var("COMP_AI_MODEL_PATH").ok(),
            comp_ai_max_tokens: env::var("COMP_AI_MAX_TOKENS")
                .unwrap_or_else(|_| "4096".to_string())
                .parse()
                .unwrap_or(4096),
            comp_ai_temperature: env::var("COMP_AI_TEMPERATURE")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
        })
    }
}
