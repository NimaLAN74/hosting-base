//! Phase 4: GitHub integration for evidence collection (e.g. last commit, branch protection).

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RepoResponse {
    default_branch: Option<String>,
    html_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommitResponse {
    sha: Option<String>,
    commit: Option<CommitDetail>,
}

#[derive(Debug, Deserialize)]
struct CommitDetail {
    message: Option<String>,
    author: Option<Author>,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BranchProtectionResponse {
    required_pull_request_reviews: Option<serde_json::Value>,
    enforce_admins: Option<serde_json::Value>,
    restrictions: Option<serde_json::Value>,
}

/// Evidence collected from GitHub (type, source, description, link_url).
pub struct GitHubEvidence {
    pub evidence_type: String,
    pub source: String,
    pub description: String,
    pub link_url: Option<String>,
}

/// Fetch last commit on default branch and return evidence (type, source, description, link_url).
pub async fn fetch_last_commit_evidence(
    token: &str,
    owner: &str,
    repo: &str,
) -> Result<GitHubEvidence> {
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-GitHub-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let repo_url = format!(
        "https://api.github.com/repos/{}/{}",
        owner.trim(),
        repo.trim()
    );
    let repo_res: RepoResponse = client
        .get(&repo_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("fetch repo")?
        .error_for_status()
        .context("repo API error")?
        .json()
        .await
        .context("parse repo response")?;

    let default_branch = repo_res
        .default_branch
        .as_deref()
        .unwrap_or("main")
        .to_string();

    let commits_url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        owner, repo, default_branch
    );
    let commit_res: CommitResponse = client
        .get(&commits_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("fetch commit")?
        .error_for_status()
        .context("commit API error")?
        .json()
        .await
        .context("parse commit response")?;

    let sha = commit_res.sha.as_deref().unwrap_or("unknown").to_string();
    let message = commit_res
        .commit
        .as_ref()
        .and_then(|c| c.message.as_deref())
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    let author = commit_res
        .commit
        .as_ref()
        .and_then(|c| c.author.as_ref())
        .and_then(|a| a.name.as_deref())
        .unwrap_or("")
        .to_string();
    let link_url = repo_res
        .html_url
        .map(|u| format!("{}/commit/{}", u.trim_end_matches('/'), sha));

    Ok(GitHubEvidence {
        evidence_type: "github_last_commit".to_string(),
        source: format!("github:{}/{}", owner, repo),
        description: format!(
            "Default branch: {}. Last commit: {} by {}. {}",
            default_branch, sha, author, message
        ),
        link_url,
    })
}

/// Fetch branch protection for default branch and return evidence.
pub async fn fetch_branch_protection_evidence(
    token: &str,
    owner: &str,
    repo: &str,
) -> Result<GitHubEvidence> {
    let client = reqwest::Client::builder()
        .user_agent("Comp-AI-GitHub-Integration/1.0")
        .build()
        .context("build reqwest client")?;

    let repo_url = format!(
        "https://api.github.com/repos/{}/{}",
        owner.trim(),
        repo.trim()
    );
    let repo_res: RepoResponse = client
        .get(&repo_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("fetch repo")?
        .error_for_status()
        .context("repo API error")?
        .json()
        .await
        .context("parse repo response")?;

    let default_branch = repo_res
        .default_branch
        .as_deref()
        .unwrap_or("main")
        .to_string();

    let protection_url = format!(
        "https://api.github.com/repos/{}/{}/branches/{}/protection",
        owner, repo, default_branch
    );
    let protection_res = client
        .get(&protection_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("fetch branch protection")?;

    let (description, link_url) = if protection_res.status().is_success() {
        let _: BranchProtectionResponse = protection_res
            .json()
            .await
            .context("parse branch protection")?;
        (
            format!("Branch protection enabled on default branch: {}", default_branch),
            repo_res.html_url,
        )
    } else {
        (
            format!(
                "Branch protection not configured or not accessible for branch: {}",
                default_branch
            ),
            repo_res.html_url,
        )
    };

    Ok(GitHubEvidence {
        evidence_type: "github_branch_protection".to_string(),
        source: format!("github:{}/{}", owner, repo),
        description,
        link_url,
    })
}
