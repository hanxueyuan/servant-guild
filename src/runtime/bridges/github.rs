//! GitHub Bridge - Genome Access for Self-Evolution
//!
//! This module provides the GitHub integration capabilities that enable
//! ServantGuild to access and modify its own source code, implement
//! self-evolution, and manage the codebase autonomously.

use crate::runtime::state::HostState;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// GitHub Personal Access Token storage
#[derive(Debug, Clone)]
pub struct GitHubCredentials {
    /// Personal Access Token
    pub pat: String,
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Base URL (for GitHub Enterprise)
    pub base_url: Option<String>,
}

impl GitHubCredentials {
    /// Create new credentials
    pub fn new(pat: String, owner: String, repo: String) -> Self {
        Self {
            pat,
            owner,
            repo,
            base_url: None,
        }
    }

    /// Set custom base URL (for GitHub Enterprise)
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Get the full repository name
    pub fn full_repo_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// Get the API base URL
    pub fn api_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| "https://api.github.com".to_string())
    }
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    /// Repository full name (owner/repo)
    pub full_name: String,
    /// Clone URL
    pub clone_url: String,
    /// Default branch
    pub default_branch: String,
    /// Repository description
    pub description: Option<String>,
    /// Language
    pub language: Option<String>,
    /// Stars count
    pub stars: u32,
    /// Forks count
    pub forks: u32,
}

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    /// PR number
    pub number: u64,
    /// PR title
    pub title: String,
    /// PR description
    pub body: Option<String>,
    /// Source branch
    pub head_ref: String,
    /// Target branch
    pub base_ref: String,
    /// State (open, closed, merged)
    pub state: String,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Author
    pub author: String,
    /// Review status
    pub review_status: Option<String>,
}

/// Release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    /// Tag name
    pub tag_name: String,
    /// Release name
    pub name: String,
    /// Release body/description
    pub body: Option<String>,
    /// Draft status
    pub draft: bool,
    /// Pre-release status
    pub prerelease: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Published at
    pub published_at: Option<DateTime<Utc>>,
    /// Author
    pub author: String,
    /// Asset URLs
    pub assets: Vec<GitHubAsset>,
}

/// Release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    /// Asset name
    pub name: String,
    /// Download URL
    pub download_url: String,
    /// Size in bytes
    pub size: u64,
    /// Content type
    pub content_type: String,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommit {
    /// SHA hash
    pub sha: String,
    /// Commit message
    pub message: String,
    /// Author
    pub author: String,
    /// Committer
    pub committer: String,
    /// Commit date
    pub date: DateTime<Utc>,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubFile {
    /// File path
    pub path: String,
    /// File SHA
    pub sha: String,
    /// Content (base64 encoded)
    pub content: Option<String>,
    /// File size
    pub size: u64,
    /// File type (file, dir, symlink)
    pub file_type: String,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubBranch {
    /// Branch name
    pub name: String,
    /// Commit SHA
    pub commit_sha: String,
    /// Commit message
    pub commit_message: String,
}

/// GitHub Bridge trait
#[async_trait]
pub trait GitHubBridge: Send + Sync {
    /// Clone the repository to local path
    async fn clone_repo(&self, path: PathBuf) -> Result<()>;

    /// Pull latest changes
    async fn pull(&self, path: PathBuf) -> Result<()>;

    /// Commit changes
    async fn commit(&self, path: PathBuf, message: String, author: Option<String>) -> Result<()>;

    /// Push changes to remote
    async fn push(&self, path: PathBuf, branch: Option<String>) -> Result<()>;

    /// Create a branch
    async fn create_branch(&self, path: PathBuf, branch_name: String) -> Result<()>;

    /// Get repository information
    async fn get_repo_info(&self) -> Result<GitHubRepo>;

    /// List branches
    async fn list_branches(&self) -> Result<Vec<GitHubBranch>>;

    /// Get file content
    async fn get_file(&self, path: String, branch: Option<String>) -> Result<GitHubFile>;

    /// Update file content
    async fn update_file(
        &self,
        path: String,
        content: String,
        message: String,
        branch: Option<String>,
    ) -> Result<()>;

    /// Create a pull request
    async fn create_pr(
        &self,
        title: String,
        body: String,
        head: String,
        base: String,
    ) -> Result<GitHubPullRequest>;

    /// List pull requests
    async fn list_prs(&self, state: Option<String>) -> Result<Vec<GitHubPullRequest>>;

    /// Add comment to PR
    async fn comment_pr(&self, pr_number: u64, comment: String) -> Result<()>;

    /// Merge a pull request
    async fn merge_pr(&self, pr_number: u64, method: Option<String>) -> Result<()>;

    /// Create a release
    async fn create_release(
        &self,
        tag_name: String,
        name: String,
        body: String,
        draft: bool,
        prerelease: bool,
    ) -> Result<GitHubRelease>;

    /// Upload asset to release
    async fn upload_release_asset(
        &self,
        release_id: u64,
        asset_name: String,
        asset_path: PathBuf,
    ) -> Result<GitHubAsset>;

    /// List commits
    async fn list_commits(
        &self,
        branch: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<GitHubCommit>>;
}

/// Implementation of GitHub Bridge
pub struct GitHubBridgeImpl {
    credentials: GitHubCredentials,
    local_repo_path: Option<PathBuf>,
}

impl GitHubBridgeImpl {
    /// Create new GitHub bridge
    pub fn new(credentials: GitHubCredentials) -> Self {
        Self {
            credentials,
            local_repo_path: None,
        }
    }

    /// Set local repository path
    pub fn with_local_path(mut self, path: PathBuf) -> Self {
        self.local_repo_path = Some(path);
        self
    }

    /// Get the GitHub API client URL for repo operations
    fn repo_api_url(&self) -> String {
        format!(
            "{}/repos/{}",
            self.credentials.api_url(),
            self.credentials.full_repo_name()
        )
    }

    /// Get authentication headers
    fn auth_headers(&self) -> Vec<(&str, &str)> {
        vec![("Authorization", &format!("token {}", self.credentials.pat))]
    }
}

#[async_trait]
impl GitHubBridge for GitHubBridgeImpl {
    async fn clone_repo(&self, path: PathBuf) -> Result<()> {
        // Use git2 crate or system git command
        let clone_url = self.credentials.clone_url.clone();
        let full_path = path.join(self.credentials.repo);

        // Create parent directory if it doesn't exist
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create parent directory")?;
        }

        // Use git command for cloning
        let output = std::process::Command::new("git")
            .args(["clone", &clone_url, full_path.to_str().unwrap()])
            .output()
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            anyhow::bail!(
                "Git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    async fn pull(&self, path: PathBuf) -> Result<()> {
        let repo_path = path.join(self.credentials.repo);

        let output = std::process::Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "pull"])
            .output()
            .context("Failed to execute git pull")?;

        if !output.status.success() {
            anyhow::bail!(
                "Git pull failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    async fn commit(&self, path: PathBuf, message: String, author: Option<String>) -> Result<()> {
        let repo_path = path.join(self.credentials.repo);

        // Configure author if provided
        if let Some(author_str) = author {
            let parts: Vec<&str> = author_str.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let _ = std::process::Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap(),
                        "config",
                        "user.name",
                        parts[0],
                    ])
                    .output();
                let _ = std::process::Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap(),
                        "config",
                        "user.email",
                        parts[1],
                    ])
                    .output();
            }
        }

        // Stage all changes
        let add_output = std::process::Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "add", "."])
            .output()
            .context("Failed to git add")?;

        if !add_output.status.success() {
            anyhow::bail!(
                "Git add failed: {}",
                String::from_utf8_lossy(&add_output.stderr)
            );
        }

        // Commit
        let commit_output = std::process::Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "commit", "-m", &message])
            .output()
            .context("Failed to git commit")?;

        if !commit_output.status.success() {
            anyhow::bail!(
                "Git commit failed: {}",
                String::from_utf8_lossy(&commit_output.stderr)
            );
        }

        Ok(())
    }

    async fn push(&self, path: PathBuf, branch: Option<String>) -> Result<()> {
        let repo_path = path.join(self.credentials.repo);
        let branch_ref = branch.unwrap_or_else(|| "HEAD".to_string());

        let output = std::process::Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "push",
                "origin",
                &branch_ref,
            ])
            .output()
            .context("Failed to execute git push")?;

        if !output.status.success() {
            anyhow::bail!(
                "Git push failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    async fn create_branch(&self, path: PathBuf, branch_name: String) -> Result<()> {
        let repo_path = path.join(self.credentials.repo);

        let output = std::process::Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "checkout",
                "-b",
                &branch_name,
            ])
            .output()
            .context("Failed to create branch")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    async fn get_repo_info(&self) -> Result<GitHubRepo> {
        // This would use the GitHub API
        // For now, return a mock implementation
        Ok(GitHubRepo {
            full_name: self.credentials.full_repo_name(),
            clone_url: self.credentials.clone_url.clone(),
            default_branch: "main".to_string(),
            description: None,
            language: Some("Rust".to_string()),
            stars: 0,
            forks: 0,
        })
    }

    async fn list_branches(&self) -> Result<Vec<GitHubBranch>> {
        let repo_path = if let Some(ref path) = self.local_repo_path {
            path.join(&self.credentials.repo)
        } else {
            anyhow::bail!("Local repository path not set");
        };

        let output = std::process::Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "branch", "-r"])
            .output()
            .context("Failed to list branches")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to list branches: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let branches_str = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();

        for line in branches_str.lines() {
            let branch_name = line.trim().replace("origin/", "");
            if branch_name.contains("HEAD") {
                continue;
            }

            // Get commit info for branch
            let log_output = std::process::Command::new("git")
                .args([
                    "-C",
                    repo_path.to_str().unwrap(),
                    "log",
                    &format!("origin/{}", branch_name),
                    "-1",
                    "--format=%H %s",
                ])
                .output();

            if let Ok(log_out) = log_output {
                if log_out.status.success() {
                    let log_str = String::from_utf8_lossy(&log_out.stdout);
                    let parts: Vec<&str> = log_str.splitn(2, ' ').collect();
                    if parts.len() >= 2 {
                        branches.push(GitHubBranch {
                            name: branch_name.clone(),
                            commit_sha: parts[0].to_string(),
                            commit_message: parts[1].to_string(),
                        });
                    }
                }
            }
        }

        Ok(branches)
    }

    async fn get_file(&self, path: String, branch: Option<String>) -> Result<GitHubFile> {
        let repo_path = if let Some(ref local_path) = self.local_repo_path {
            local_path.join(&self.credentials.repo)
        } else {
            anyhow::bail!("Local repository path not set");
        };

        let branch_ref = branch.unwrap_or_else(|| "HEAD".to_string());

        let output = std::process::Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "show",
                &format!("{}:{}", branch_ref, path),
            ])
            .output()
            .context("Failed to get file")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to get file: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        let sha = format!("{:x}", md5::compute(&content.as_bytes()));

        Ok(GitHubFile {
            path: path.clone(),
            sha,
            content: Some(content),
            size: output.stdout.len() as u64,
            file_type: "file".to_string(),
        })
    }

    async fn update_file(
        &self,
        path: String,
        content: String,
        message: String,
        branch: Option<String>,
    ) -> Result<()> {
        let repo_path = if let Some(ref local_path) = self.local_repo_path {
            local_path.join(&self.credentials.repo)
        } else {
            anyhow::bail!("Local repository path not set");
        };

        // Switch to branch if specified
        if let Some(branch_name) = branch {
            let _ = std::process::Command::new("git")
                .args(["-C", repo_path.to_str().unwrap(), "checkout", &branch_name])
                .output();
        }

        // Ensure parent directory exists
        let file_path = repo_path.join(&path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write content
        std::fs::write(&file_path, content)?;

        // Commit changes
        self.commit(repo_path.parent().unwrap().to_path_buf(), message, None)
            .await?;

        Ok(())
    }

    async fn create_pr(
        &self,
        title: String,
        body: String,
        head: String,
        base: String,
    ) -> Result<GitHubPullRequest> {
        // This would use GitHub API to create PR
        // For now, return a mock PR
        Ok(GitHubPullRequest {
            number: 1,
            title,
            body: Some(body),
            head_ref: head,
            base_ref: base,
            state: "open".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author: "servant-guild".to_string(),
            review_status: None,
        })
    }

    async fn list_prs(&self, state: Option<String>) -> Result<Vec<GitHubPullRequest>> {
        // This would use GitHub API to list PRs
        // For now, return empty list
        Ok(Vec::new())
    }

    async fn comment_pr(&self, _pr_number: u64, _comment: String) -> Result<()> {
        // This would use GitHub API to comment on PR
        Ok(())
    }

    async fn merge_pr(&self, _pr_number: u64, _method: Option<String>) -> Result<()> {
        // This would use GitHub API to merge PR
        Ok(())
    }

    async fn create_release(
        &self,
        tag_name: String,
        name: String,
        body: String,
        draft: bool,
        prerelease: bool,
    ) -> Result<GitHubRelease> {
        // This would use GitHub API to create release
        Ok(GitHubRelease {
            tag_name,
            name,
            body: Some(body),
            draft,
            prerelease,
            created_at: Utc::now(),
            published_at: Some(Utc::now()),
            author: "servant-guild".to_string(),
            assets: Vec::new(),
        })
    }

    async fn upload_release_asset(
        &self,
        _release_id: u64,
        _asset_name: String,
        _asset_path: PathBuf,
    ) -> Result<GitHubAsset> {
        // This would use GitHub API to upload asset
        Ok(GitHubAsset {
            name: "asset.wasm".to_string(),
            download_url: "https://example.com/asset.wasm".to_string(),
            size: 0,
            content_type: "application/wasm".to_string(),
        })
    }

    async fn list_commits(
        &self,
        _branch: Option<String>,
        _limit: Option<u32>,
    ) -> Result<Vec<GitHubCommit>> {
        // This would use GitHub API to list commits
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_credentials() {
        let creds = GitHubCredentials::new(
            "test-pat".to_string(),
            "owner".to_string(),
            "repo".to_string(),
        );
        assert_eq!(creds.full_repo_name(), "owner/repo");
        assert_eq!(creds.api_url(), "https://api.github.com");
    }

    #[test]
    fn test_github_credentials_with_base_url() {
        let creds = GitHubCredentials::new(
            "test-pat".to_string(),
            "owner".to_string(),
            "repo".to_string(),
        )
        .with_base_url("https://github.enterprise.com".to_string());
        assert_eq!(creds.api_url(), "https://github.enterprise.com");
    }
}
