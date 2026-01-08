use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: String,
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub keycloak_admin_user: String,
    pub keycloak_admin_password: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .unwrap_or(3001);

        // Build database URL from components or use full DATABASE_URL
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "172.18.0.1".to_string());
            let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
            let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
            let password = env::var("POSTGRES_PASSWORD")
                .expect("POSTGRES_PASSWORD must be set");
            let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "lianel_energy".to_string());
            
            format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, db)
        });

        let keycloak_url = env::var("KEYCLOAK_URL")
            .unwrap_or_else(|_| "http://keycloak:8080".to_string());
        let keycloak_realm = env::var("KEYCLOAK_REALM")
            .unwrap_or_else(|_| "lianel".to_string());
        let keycloak_admin_user = env::var("KEYCLOAK_ADMIN_USER")
            .expect("KEYCLOAK_ADMIN_USER must be set");
        let keycloak_admin_password = env::var("KEYCLOAK_ADMIN_PASSWORD")
            .expect("KEYCLOAK_ADMIN_PASSWORD must be set");

        Self {
            port,
            database_url,
            keycloak_url,
            keycloak_realm,
            keycloak_admin_user,
            keycloak_admin_password,
        }
    }
}

