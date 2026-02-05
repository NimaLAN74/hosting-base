//! SharePoint integration – list files in a site/document library and create evidence (link per file).
//! Uses Microsoft Graph (same app as M365; requires Sites.Read.All or Files.Read.All).

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GraphTokenResponse {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphDriveItem {
    id: Option<String>,
    name: Option<String>,
    #[serde(rename = "webUrl")]
    web_url: Option<String>,
    #[serde(rename = "lastModifiedDateTime")]
    last_modified_date_time: Option<String>,
    file: Option<serde_json::Value>,
    folder: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GraphDriveItemResponse {
    value: Option<Vec<GraphDriveItem>>,
}

#[derive(Debug, Deserialize)]
struct GraphDrive {
    id: Option<String>,
}

/// One file/item from SharePoint for evidence.
pub struct SharepointFileEvidence {
    pub source: String,
    pub link_url: Option<String>,
}

async fn fetch_token(tenant_id: &str, client_id: &str, client_secret: &str) -> Result<String> {
    let tenant = tenant_id.trim().trim_end_matches('/');
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        tenant
    );
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-SharePoint-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("scope", "https://graph.microsoft.com/.default"),
    ];

    let resp = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .context("SharePoint token request")?;

    let status = resp.status();
    let body = resp.text().await.context("SharePoint token response body")?;
    if !status.is_success() {
        anyhow::bail!("SharePoint token failed {}: {}", status, body);
    }

    let data: GraphTokenResponse =
        serde_json::from_str(&body).context("parse SharePoint token response")?;
    data.access_token
        .filter(|t| !t.is_empty())
        .context("SharePoint token response missing access_token")
}

/// Resolve default drive ID for the site (document library).
async fn fetch_default_drive_id(token: &str, site_id: &str) -> Result<String> {
    let url = format!(
        "https://graph.microsoft.com/v1.0/sites/{}/drive",
        urlencoding::encode(site_id)
    );
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-SharePoint-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .context("SharePoint drive request")?;

    let status = resp.status();
    let body = resp.text().await.context("SharePoint drive response body")?;
    if !status.is_success() {
        anyhow::bail!("SharePoint drive failed {}: {}", status, body);
    }

    let data: GraphDrive = serde_json::from_str(&body).context("parse drive response")?;
    data.id
        .filter(|s| !s.is_empty())
        .context("Drive response missing id")
}

/// List files in a SharePoint site drive (root or folder path). Max 100 items.
/// Uses same M365 app credentials; app needs Sites.Read.All or Files.Read.All.
pub async fn fetch_sharepoint_file_list(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    site_id: &str,
    drive_id: Option<&str>,
    folder_path: Option<&str>,
    limit: u32,
) -> Result<Vec<SharepointFileEvidence>> {
    let token = fetch_token(tenant_id, client_id, client_secret).await?;

    let site_id = site_id.trim();
    if site_id.is_empty() {
        anyhow::bail!("site_id is empty");
    }

    let drive_id = match drive_id.filter(|s| !s.is_empty()) {
        Some(d) => d.to_string(),
        None => fetch_default_drive_id(&token, site_id).await?,
    };

    let top = limit.min(100).max(1);
    let url = if let Some(path) = folder_path.filter(|s| !s.is_empty()) {
        let path_encoded = urlencoding::encode(path);
        format!(
            "https://graph.microsoft.com/v1.0/sites/{}/drives/{}/root:/{}:/children?$top={}&$orderby=name",
            urlencoding::encode(site_id),
            urlencoding::encode(&drive_id),
            path_encoded,
            top
        )
    } else {
        format!(
            "https://graph.microsoft.com/v1.0/sites/{}/drives/{}/root/children?$top={}&$orderby=name",
            urlencoding::encode(site_id),
            urlencoding::encode(&drive_id),
            top
        )
    };

    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-SharePoint-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .context("SharePoint children request")?;

    let status = resp.status();
    let body = resp.text().await.context("SharePoint children response body")?;
    if !status.is_success() {
        anyhow::bail!("SharePoint children failed {}: {}", status, body);
    }

    let data: GraphDriveItemResponse =
        serde_json::from_str(&body).context("parse SharePoint children response")?;

    let list = data.value.unwrap_or_default();
    let out: Vec<SharepointFileEvidence> = list
        .into_iter()
        .map(|f| {
            let name = f.name.unwrap_or_else(|| "(unnamed)".to_string());
            SharepointFileEvidence {
                source: name.clone(),
                link_url: f.web_url,
            }
        })
        .collect();

    Ok(out)
}
