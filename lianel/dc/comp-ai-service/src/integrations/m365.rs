//! D3: Microsoft 365 (Graph) email integration – list recent mail metadata and create evidence.

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GraphTokenResponse {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphMessage {
    from: Option<GraphRecipient>,
    subject: Option<String>,
    #[serde(rename = "receivedDateTime")]
    received_date_time: Option<String>,
    #[serde(rename = "webLink")]
    web_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphRecipient {
    #[serde(rename = "emailAddress")]
    email_address: Option<GraphEmailAddress>,
}

#[derive(Debug, Deserialize)]
struct GraphEmailAddress {
    address: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphMessagesResponse {
    value: Option<Vec<GraphMessage>>,
}

/// One email's metadata for evidence (D3).
pub struct M365EmailEvidence {
    pub source: String,
    pub link_url: Option<String>,
}

fn format_datetime(s: &str) -> String {
    // Graph returns ISO 8601 e.g. 2026-01-15T10:30:00Z; show date + time for auditors
    if s.len() >= 19 {
        format!("{} {}", &s[..10], &s[11..19].trim_end_matches('Z'))
    } else {
        s.to_string()
    }
}

/// Fetch access token using client_credentials (app-only).
async fn fetch_token(tenant_id: &str, client_id: &str, client_secret: &str) -> Result<String> {
    let tenant = tenant_id.trim().trim_end_matches('/');
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        tenant
    );
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-M365-Integration/1.0")
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
        .context("M365 token request")?;

    let status = resp.status();
    let body = resp.text().await.context("M365 token response body")?;
    if !status.is_success() {
        anyhow::bail!("M365 token failed {}: {}", status, body);
    }

    let data: GraphTokenResponse =
        serde_json::from_str(&body).context("parse M365 token response")?;
    data.access_token
        .filter(|t| !t.is_empty())
        .context("M365 token response missing access_token")
}

/// List recent messages (metadata only) from the given mailbox. Max 100.
pub async fn fetch_m365_email_list(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    mailbox_user_id: &str,
    limit: u32,
) -> Result<Vec<M365EmailEvidence>> {
    let token = fetch_token(tenant_id, client_id, client_secret).await?;

    let user_id = urlencoding::encode(mailbox_user_id.trim());
    let top = limit.min(100).max(1);
    let url = format!(
        "https://graph.microsoft.com/v1.0/users/{}/messages?$select=from,subject,receivedDateTime,webLink&$top={}&$orderby=receivedDateTime%20desc",
        user_id, top
    );

    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-M365-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .context("M365 messages request")?;

    let status = resp.status();
    let body = resp.text().await.context("M365 messages response body")?;
    if !status.is_success() {
        anyhow::bail!("M365 messages failed {}: {}", status, body);
    }

    let data: GraphMessagesResponse =
        serde_json::from_str(&body).context("parse M365 messages response")?;

    let list = data.value.unwrap_or_default();
    let out: Vec<M365EmailEvidence> = list
        .into_iter()
        .map(|m| {
            let from = m
                .from
                .as_ref()
                .and_then(|r| r.email_address.as_ref())
                .map(|e| e.address.as_deref().unwrap_or("").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let subj = m.subject.as_deref().unwrap_or("(no subject)").to_string();
            let date = m
                .received_date_time
                .as_ref()
                .map(|s| format_datetime(s))
                .unwrap_or_else(|| "".to_string());
            let source = format!("From: {}; Subject: {}; Date: {}", from, subj, date);
            M365EmailEvidence {
                source,
                link_url: m.web_link,
            }
        })
        .collect();

    Ok(out)
}
