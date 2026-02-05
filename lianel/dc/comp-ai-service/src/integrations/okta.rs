//! G3: Okta IdP integration for evidence collection (org summary, users snapshot, groups snapshot).

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OktaOrgResponse {
    company_name: Option<String>,
    #[serde(rename = "_links")]
    links: Option<OktaLinks>,
}

#[derive(Debug, Deserialize)]
struct OktaLinks {
    #[serde(rename = "self")]
    self_: Option<OktaHref>,
}

#[derive(Debug, Deserialize)]
struct OktaHref {
    href: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OktaUser {
    id: Option<String>,
    profile: Option<OktaUserProfile>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OktaUserProfile {
    login: Option<String>,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OktaGroup {
    id: Option<String>,
    profile: Option<OktaGroupProfile>,
}

#[derive(Debug, Deserialize)]
struct OktaGroupProfile {
    name: Option<String>,
    description: Option<String>,
}

/// Evidence collected from Okta (type, source, description, link_url).
pub struct OktaEvidence {
    pub evidence_type: String,
    pub source: String,
    pub description: String,
    pub link_url: Option<String>,
}

fn okta_base(domain: &str) -> String {
    let domain = domain.trim().trim_end_matches('/');
    format!(
        "https://{}",
        if domain.starts_with("http") {
            domain.to_string()
        } else {
            format!("{}.okta.com", domain.replace(".okta.com", ""))
        }
    )
}

/// Fetch Okta org summary (company name, link). Evidence type: okta_org_summary.
pub async fn fetch_okta_org_summary(domain: &str, token: &str) -> Result<OktaEvidence> {
    let base = okta_base(domain);
    let url = format!("{}/api/v1/org", base);
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-Okta-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let res: OktaOrgResponse = client
        .get(&url)
        .header("Authorization", format!("SSWS {}", token.trim()))
        .header("Accept", "application/json")
        .send()
        .await
        .context("fetch Okta org")?
        .error_for_status()
        .context("Okta org API error")?
        .json()
        .await
        .context("parse Okta org response")?;

    let company = res.company_name.as_deref().unwrap_or("Unknown").to_string();
    let link_url = res
        .links
        .and_then(|l| l.self_)
        .and_then(|s| s.href)
        .or_else(|| Some(format!("{}/admin", base)));

    Ok(OktaEvidence {
        evidence_type: "okta_org_summary".to_string(),
        source: format!("okta:{}", domain),
        description: format!(
            "Okta org: {}. IdP integration for identity and access.",
            company
        ),
        link_url,
    })
}

/// Fetch users snapshot (count and sample). Evidence type: okta_users_snapshot.
pub async fn fetch_okta_users_snapshot(domain: &str, token: &str) -> Result<OktaEvidence> {
    let base = okta_base(domain);
    let url = format!("{}/api/v1/users?limit=200", base);
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-Okta-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let users: Vec<OktaUser> = client
        .get(&url)
        .header("Authorization", format!("SSWS {}", token.trim()))
        .header("Accept", "application/json")
        .send()
        .await
        .context("fetch Okta users")?
        .error_for_status()
        .context("Okta users API error")?
        .json()
        .await
        .context("parse Okta users response")?;

    let count = users.len();
    let active: usize = users
        .iter()
        .filter(|u| u.status.as_deref() == Some("ACTIVE"))
        .count();
    let sample: Vec<String> = users
        .iter()
        .take(5)
        .filter_map(|u| u.profile.as_ref().and_then(|p| p.login.clone()))
        .collect();

    Ok(OktaEvidence {
        evidence_type: "okta_users_snapshot".to_string(),
        source: format!("okta:{}", domain),
        description: format!(
            "Okta users snapshot: {} users ({} active). Sample logins: {}.",
            count,
            active,
            if sample.is_empty() {
                "—".to_string()
            } else {
                sample.join(", ")
            }
        ),
        link_url: Some(format!("{}/admin/users", base)),
    })
}

/// Fetch groups snapshot (count and sample). Evidence type: okta_groups_snapshot.
pub async fn fetch_okta_groups_snapshot(domain: &str, token: &str) -> Result<OktaEvidence> {
    let base = okta_base(domain);
    let url = format!("{}/api/v1/groups?limit=200", base);
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-Okta-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let groups: Vec<OktaGroup> = client
        .get(&url)
        .header("Authorization", format!("SSWS {}", token.trim()))
        .header("Accept", "application/json")
        .send()
        .await
        .context("fetch Okta groups")?
        .error_for_status()
        .context("Okta groups API error")?
        .json()
        .await
        .context("parse Okta groups response")?;

    let count = groups.len();
    let sample: Vec<String> = groups
        .iter()
        .take(5)
        .filter_map(|g| g.profile.as_ref().and_then(|p| p.name.clone()))
        .collect();

    Ok(OktaEvidence {
        evidence_type: "okta_groups_snapshot".to_string(),
        source: format!("okta:{}", domain),
        description: format!(
            "Okta groups snapshot: {} groups. Sample: {}.",
            count,
            if sample.is_empty() {
                "—".to_string()
            } else {
                sample.join(", ")
            }
        ),
        link_url: Some(format!("{}/admin/groups", base)),
    })
}
