use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

const GITHUB_API_BASE: &str = "https://api.github.com";
const DIT_MARKER: &str = "<!-- dit-report -->";

#[derive(Debug)]
#[allow(dead_code)]
pub struct GitHubContext {
    pub token: String,
    pub repo: String,
    pub pr_number: Option<u64>,
    pub sha: String,
    pub ref_name: String,
}

impl GitHubContext {
    pub fn from_env() -> Result<Self> {
        let token = env::var("GITHUB_TOKEN")
            .context("GITHUB_TOKEN environment variable not set")?;
        
        let repo = env::var("GITHUB_REPOSITORY")
            .context("GITHUB_REPOSITORY environment variable not set")?;
        
        let sha = env::var("GITHUB_SHA")
            .unwrap_or_else(|_| "unknown".to_string());
        
        let ref_name = env::var("GITHUB_REF")
            .unwrap_or_else(|_| "unknown".to_string());
        
        // Try to extract PR number from GITHUB_EVENT_PATH
        let pr_number = Self::extract_pr_number()?;
        
        Ok(Self {
            token,
            repo,
            pr_number,
            sha,
            ref_name,
        })
    }
    
    fn extract_pr_number() -> Result<Option<u64>> {
        let event_path = match env::var("GITHUB_EVENT_PATH") {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        
        let content = std::fs::read_to_string(&event_path)
            .context("Failed to read GITHUB_EVENT_PATH file")?;
        
        let event: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse GitHub event JSON")?;
        
        // Try to get PR number from event
        let pr_number = event
            .get("pull_request")
            .and_then(|pr| pr.get("number"))
            .and_then(|n| n.as_u64());
        
        Ok(pr_number)
    }
    
    pub fn is_pr(&self) -> bool {
        self.pr_number.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Comment {
    id: u64,
    body: String,
}

#[derive(Debug, Serialize)]
struct CreateComment {
    body: String,
}

pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(token: String, repo: String) -> Self {
        let client = reqwest::Client::new();
        Self { client, token, repo }
    }
    
    pub async fn post_or_update_pr_comment(&self, pr_number: u64, body: String) -> Result<()> {
        // Add marker to the comment body
        let marked_body = format!("{}\n{}", DIT_MARKER, body);
        
        // Check if we already have a comment
        let existing_comment = self.find_existing_comment(pr_number).await?;
        
        if let Some(comment_id) = existing_comment {
            // Update existing comment
            self.update_comment(comment_id, marked_body).await?;
            println!("✅ Updated existing PR comment");
        } else {
            // Create new comment
            self.create_comment(pr_number, marked_body).await?;
            println!("✅ Posted new PR comment");
        }
        
        Ok(())
    }
    
    async fn find_existing_comment(&self, pr_number: u64) -> Result<Option<u64>> {
        let url = format!(
            "{}/repos/{}/issues/{}/comments",
            GITHUB_API_BASE, self.repo, pr_number
        );
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "dit-docker-image-tracker")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch PR comments")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list comments: {} - {}", status, text);
        }
        
        let comments: Vec<Comment> = response.json().await?;
        
        // Find comment with our marker
        for comment in comments {
            if comment.body.contains(DIT_MARKER) {
                return Ok(Some(comment.id));
            }
        }
        
        Ok(None)
    }
    
    async fn create_comment(&self, pr_number: u64, body: String) -> Result<()> {
        let url = format!(
            "{}/repos/{}/issues/{}/comments",
            GITHUB_API_BASE, self.repo, pr_number
        );
        
        let payload = CreateComment { body };
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "dit-docker-image-tracker")
            .header("Accept", "application/vnd.github.v3+json")
            .json(&payload)
            .send()
            .await
            .context("Failed to create PR comment")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create comment: {} - {}", status, text);
        }
        
        Ok(())
    }
    
    async fn update_comment(&self, comment_id: u64, body: String) -> Result<()> {
        let url = format!(
            "{}/repos/{}/issues/comments/{}",
            GITHUB_API_BASE, self.repo, comment_id
        );
        
        let payload = CreateComment { body };
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "dit-docker-image-tracker")
            .header("Accept", "application/vnd.github.v3+json")
            .json(&payload)
            .send()
            .await
            .context("Failed to update PR comment")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update comment: {} - {}", status, text);
        }
        
        Ok(())
    }
}
