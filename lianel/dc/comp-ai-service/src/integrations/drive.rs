//! C3: Google Drive integration – list files in a folder and create evidence (link per file).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct CredentialsJson {
    client_email: Option<String>,
    private_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFile {
    id: Option<String>,
    name: Option<String>,
    #[serde(rename = "webViewLink")]
    web_view_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFilesResponse {
    files: Option<Vec<DriveFile>>,
}

/// One file from Drive for evidence (C3).
pub struct DriveFileEvidence {
    pub source: String,
    pub link_url: Option<String>,
}

fn jwt_claim(client_email: &str, now_secs: u64) -> jsonwebtoken::EncodingKey {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("drive-rs".to_string());
    let claims = serde_json::json!({
        "iss": client_email,
        "scope": "https://www.googleapis.com/auth/drive.readonly",
        "aud": "https://oauth2.googleapis.com/token",
        "iat": now_secs,
        "exp": now_secs + 3600,
    });
    // EncodingKey expects the private key in PEM; credentials JSON has it as a string
    unimplemented!()
}

/// Fetch access token using service account JWT (C3).
async fn fetch_drive_token(credentials_path: &str) -> Result<String> {
    let contents = std::fs::read_to_string(credentials_path).context("read Drive credentials file")?;
    let creds: CredentialsJson = serde_json::from_str(&contents).context("parse Drive credentials JSON")?;
    let client_email = creds
        .client_email
        .filter(|s| !s.is_empty())
        .context("credentials missing client_email")?;
    let private_key = creds
        .private_key
        .filter(|s| !s.is_empty())
        .context("credentials missing private_key")?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time")?.as_secs();
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let claims = serde_json::json!({
        "iss": client_email,
        "scope": "https://www.googleapis.com/auth/drive.readonly",
        "aud": "https://oauth2.googleapis.com/token",
        "iat": now,
        "exp": now + 3600,
    });

    let token = jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("invalid private_key PEM")?,
    )
    .context("JWT encode")?;

    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-Drive-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &token),
    ];

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .context("Drive token request")?;

    let status = resp.status();
    let body = resp.text().await.context("Drive token response body")?;
    if !status.is_success() {
        anyhow::bail!("Drive token failed {}: {}", status, body);
    }

    let data: TokenResponse = serde_json::from_str(&body).context("parse Drive token response")?;
    data.access_token
        .filter(|t| !t.is_empty())
        .context("Drive token response missing access_token")
}

/// List files in a Google Drive folder (metadata only). Max 100. C3.
pub async fn fetch_drive_file_list(
    credentials_path: &str,
    folder_id: &str,
    limit: u32,
) -> Result<Vec<DriveFileEvidence>> {
    let token = fetch_drive_token(credentials_path).await?;

    let folder_id = folder_id.trim();
    if folder_id.is_empty() {
        anyhow::bail!("folder_id is empty");
    }

    let page_size = limit.min(100).max(1);
    let q = format!("'{}' in parents", folder_id);
    let url = format!(
        "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,webViewLink)&pageSize={}",
        urlencoding::encode(&q),
        page_size
    );

    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-Drive-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .context("Drive files request")?;

    let status = resp.status();
    let body = resp.text().await.context("Drive files response body")?;
    if !status.is_success() {
        anyhow::bail!("Drive files failed {}: {}", status, body);
    }

    let data: DriveFilesResponse = serde_json::from_str(&body).context("parse Drive files response")?;

    let list = data.files.unwrap_or_default();
    let out: Vec<DriveFileEvidence> = list
        .into_iter()
        .map(|f| {
            let name = f.name.unwrap_or_else(|| "(unnamed)".to_string());
            DriveFileEvidence {
                source: name.clone(),
                link_url: f.web_view_link,
            }
        })
        .collect();

    Ok(out)
}
