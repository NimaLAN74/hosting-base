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
    /// When set with comp_ai_ollama_model, use local Ollama instead of hosted API or mock.
    pub comp_ai_ollama_url: Option<String>,
    pub comp_ai_ollama_model: Option<String>,
    /// When true, if Ollama is configured but the request fails, return mock instead of 503.
    pub comp_ai_ollama_fallback_to_mock: bool,
    pub comp_ai_max_tokens: u32,
    pub comp_ai_temperature: f64,
    /// Max requests per window for rate limiting (0 = disabled).
    pub comp_ai_rate_limit_requests: u32,
    /// Rate limit window in seconds.
    pub comp_ai_rate_limit_window_secs: u64,
    /// Max prompt length in characters (0 = no limit).
    pub comp_ai_max_prompt_len: usize,
    /// Response cache TTL in seconds (0 = cache disabled).
    pub comp_ai_response_cache_ttl_secs: u64,
    /// Response cache max entries (when cache enabled).
    pub comp_ai_response_cache_max_entries: u64,
    /// Optional GitHub token for integrations (evidence collection).
    pub github_token: Option<String>,
    /// G3: Okta IdP integration. Domain (e.g. dev-12345.okta.com).
    pub okta_domain: Option<String>,
    /// G3: Okta API token (admin). Required for Okta evidence collection.
    pub okta_api_token: Option<String>,
    /// G4: AWS region for IAM evidence (e.g. us-east-1).
    pub aws_region: Option<String>,
    /// G4: AWS access key for IAM (optional if using instance role / env default).
    pub aws_access_key_id: Option<String>,
    /// G4: AWS secret key for IAM (optional if using instance role / env default).
    pub aws_secret_access_key: Option<String>,
    /// D3: M365 (Microsoft Graph) – tenant ID for OAuth (e.g. xxx.onmicrosoft.com or tenant GUID).
    pub m365_tenant_id: Option<String>,
    /// D3: M365 app (client) ID.
    pub m365_client_id: Option<String>,
    /// D3: M365 client secret.
    pub m365_client_secret: Option<String>,
    /// D3: Mailbox to read (user ID or userPrincipalName). Required for app-only.
    pub m365_mailbox_user_id: Option<String>,
    /// C3: Google Drive – folder ID to list (optional; can be overridden per request).
    pub google_drive_folder_id: Option<String>,
    /// C3: Path to service account JSON file (for Drive API).
    pub google_drive_credentials_path: Option<String>,
    /// Base directory for uploaded evidence files (Phase B). If unset, upload is disabled.
    pub comp_ai_evidence_storage_path: Option<String>,
    /// Max upload size in bytes (default 10 MB).
    pub comp_ai_evidence_max_file_bytes: u64,
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
            comp_ai_ollama_url: env::var("COMP_AI_OLLAMA_URL").ok(),
            comp_ai_ollama_model: env::var("COMP_AI_OLLAMA_MODEL").ok(),
            comp_ai_ollama_fallback_to_mock: env::var("COMP_AI_OLLAMA_FALLBACK_TO_MOCK")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            comp_ai_max_tokens: env::var("COMP_AI_MAX_TOKENS")
                .unwrap_or_else(|_| "4096".to_string())
                .parse()
                .unwrap_or(4096),
            comp_ai_temperature: env::var("COMP_AI_TEMPERATURE")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
            comp_ai_rate_limit_requests: env::var("COMP_AI_RATE_LIMIT_REQUESTS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            comp_ai_rate_limit_window_secs: env::var("COMP_AI_RATE_LIMIT_WINDOW_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            comp_ai_max_prompt_len: env::var("COMP_AI_MAX_PROMPT_LEN")
                .unwrap_or_else(|_| "32768".to_string())
                .parse()
                .unwrap_or(32768),
            comp_ai_response_cache_ttl_secs: env::var("COMP_AI_RESPONSE_CACHE_TTL_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            comp_ai_response_cache_max_entries: env::var("COMP_AI_RESPONSE_CACHE_MAX_ENTRIES")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            github_token: env::var("GITHUB_TOKEN").ok(),
            okta_domain: env::var("OKTA_DOMAIN").ok(),
            okta_api_token: env::var("OKTA_API_TOKEN").ok(),
            aws_region: env::var("AWS_REGION").ok(),
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            m365_tenant_id: env::var("M365_TENANT_ID").ok(),
            m365_client_id: env::var("M365_CLIENT_ID").ok(),
            m365_client_secret: env::var("M365_CLIENT_SECRET").ok(),
            m365_mailbox_user_id: env::var("M365_MAILBOX_USER_ID").ok(),
            google_drive_folder_id: env::var("GOOGLE_DRIVE_FOLDER_ID").ok(),
            google_drive_credentials_path: env::var("GOOGLE_DRIVE_CREDENTIALS_PATH").ok(),
            comp_ai_evidence_storage_path: env::var("COMP_AI_EVIDENCE_STORAGE_PATH").ok(),
            comp_ai_evidence_max_file_bytes: env::var("COMP_AI_EVIDENCE_MAX_FILE_BYTES")
                .unwrap_or_else(|_| "10485760".to_string()) // 10 MiB
                .parse()
                .unwrap_or(10 * 1024 * 1024),
        })
    }
}
