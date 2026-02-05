//! G4: AWS IAM integration for evidence collection (account summary).

use anyhow::{Context, Result};
use aws_sdk_iam::config::Region;
use aws_sdk_iam::types::SummaryKeyType;
use aws_sdk_iam::Client;
use std::collections::HashMap;

/// Evidence collected from AWS IAM (type, source, description, link_url).
pub struct AwsEvidence {
    pub evidence_type: String,
    pub source: String,
    pub description: String,
    pub link_url: Option<String>,
}

/// Build IAM client. Uses default credential chain (env, profile, instance role). Region from config or env.
pub async fn build_iam_client(region: Option<&str>) -> Result<Client> {
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(r) = region.filter(|s| !s.is_empty()) {
        loader = loader.region(Region::new(r.to_string()));
    }
    let config = loader.load().await;
    Ok(Client::new(&config))
}

fn get_summary_val(m: &HashMap<SummaryKeyType, i32>, k: SummaryKeyType) -> String {
    m.get(&k)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".to_string())
}

/// Fetch AWS IAM account summary (users, groups, roles, MFA). Evidence type: aws_iam_summary.
pub async fn fetch_aws_iam_summary(region: Option<&str>) -> Result<AwsEvidence> {
    let client = build_iam_client(region)
        .await
        .context("build IAM client")?;

    let summary = client
        .get_account_summary()
        .send()
        .await
        .context("GetAccountSummary API error")?
        .summary_map
        .unwrap_or_default();

    let users = get_summary_val(&summary, SummaryKeyType::Users);
    let groups = get_summary_val(&summary, SummaryKeyType::Groups);
    let roles = get_summary_val(&summary, SummaryKeyType::Roles);
    let account_mfa = get_summary_val(&summary, SummaryKeyType::AccountMfaEnabled);
    let mfa_in_use = get_summary_val(&summary, SummaryKeyType::MfaDevicesInUse);

    let region_str = region.unwrap_or("default");
    let description = format!(
        "AWS IAM summary: Users={}, Groups={}, Roles={}, AccountMFAEnabled={}, MFADevicesInUse={}. Region: {}.",
        users, groups, roles, account_mfa, mfa_in_use, region_str
    );

    Ok(AwsEvidence {
        evidence_type: "aws_iam_summary".to_string(),
        source: format!("aws:iam:{}", region_str),
        description,
        link_url: None,
    })
}
