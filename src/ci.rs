use anyhow::{bail, Context, Result};
use bytesize::ByteSize;
use chrono::Utc;
use std::collections::HashMap;

use crate::docker::DockerClient;
use crate::github::{GitHubClient, GitHubContext};
use crate::models::{ImageSnapshot, LayerChange, SizeDiff};
use crate::track::{load_history, save_snapshot};

#[derive(Debug)]
pub struct CiConfig {
    pub images: Vec<String>,
    pub budget_bytes: Option<u64>,
    pub budget_increase_percent: Option<f64>,
    pub github_comment: bool,
    pub base_branch: Option<String>,
    pub fail_on_increase: bool,
    pub format: CiOutputFormat,
}

#[derive(Debug, Clone)]
pub enum CiOutputFormat {
    Table,
    Json,
    Markdown,
}

pub async fn run_ci(config: CiConfig) -> Result<()> {
    // Track current images
    let docker = DockerClient::new()?;
    let mut current_snapshots = Vec::new();
    
    println!("📊 Analyzing {} image(s)...", config.images.len());
    
    for image in &config.images {
        let mut snapshot = docker.inspect_image(image).await?;
        
        // Get git context
        if let Ok(git_ctx) = get_git_context() {
            snapshot.commit_sha = git_ctx.commit_sha;
            snapshot.branch = git_ctx.branch;
            snapshot.commit_message = git_ctx.commit_message;
            snapshot.author = git_ctx.author;
        }
        snapshot.timestamp = Utc::now();
        
        current_snapshots.push(snapshot);
    }
    
    // Load history and find baseline snapshots
    let history = load_history()?;
    let mut comparisons = Vec::new();
    let mut first_run = false;
    
    for current in &current_snapshots {
        let baseline = find_baseline_snapshot(&history, &current.image, config.base_branch.as_deref());
        
        if let Some(base) = baseline {
            let diff = compute_diff(base.clone(), current.clone());
            comparisons.push((current.clone(), Some(diff)));
        } else {
            // First run for this image
            comparisons.push((current.clone(), None));
            first_run = true;
        }
        
        // Save the current snapshot to history
        save_snapshot(current)?;
    }
    
    // Generate report
    let report = generate_report(&comparisons, &config)?;
    
    // Output based on format
    match config.format {
        CiOutputFormat::Table => {
            println!("\n{}", report);
        }
        CiOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&comparisons)?);
        }
        CiOutputFormat::Markdown => {
            println!("{}", report);
        }
    }
    
    // Post to GitHub if requested
    if config.github_comment {
        post_github_comment(&report).await?;
    }
    
    // Check budgets and determine exit code
    let should_fail = check_budgets(&comparisons, &config)?;
    
    if should_fail {
        std::process::exit(1);
    }
    
    if first_run {
        println!("\n💡 First run detected. Baseline established for future comparisons.");
    }
    
    Ok(())
}

fn find_baseline_snapshot<'a>(
    history: &'a [ImageSnapshot],
    image: &str,
    base_branch: Option<&str>,
) -> Option<&'a ImageSnapshot> {
    let image_history: Vec<_> = history
        .iter()
        .filter(|s| s.image == image)
        .collect();
    
    if image_history.is_empty() {
        return None;
    }
    
    // If base branch specified, find latest snapshot from that branch
    if let Some(branch) = base_branch {
        return image_history
            .iter()
            .rev()
            .find(|s| s.branch == branch)
            .copied();
    }
    
    // Otherwise, return the most recent snapshot
    image_history.last().copied()
}

fn compute_diff(before: ImageSnapshot, after: ImageSnapshot) -> SizeDiff {
    let total_delta = after.total_size as i64 - before.total_size as i64;

    let before_layers: HashMap<_, _> = before
        .layers
        .iter()
        .map(|l| (l.digest.clone(), l.clone()))
        .collect();

    let after_layers: HashMap<_, _> = after
        .layers
        .iter()
        .map(|l| (l.digest.clone(), l.clone()))
        .collect();

    let mut layer_changes = Vec::new();

    for layer in &before.layers {
        if let Some(after_layer) = after_layers.get(&layer.digest) {
            if layer.size == after_layer.size {
                layer_changes.push(LayerChange::Unchanged(layer.clone()));
            } else {
                layer_changes.push(LayerChange::Modified {
                    before: layer.clone(),
                    after: after_layer.clone(),
                });
            }
        } else {
            layer_changes.push(LayerChange::Removed(layer.clone()));
        }
    }

    for layer in &after.layers {
        if !before_layers.contains_key(&layer.digest) {
            layer_changes.push(LayerChange::Added(layer.clone()));
        }
    }

    SizeDiff {
        before,
        after,
        total_delta,
        layer_changes,
    }
}

fn generate_report(
    comparisons: &[(ImageSnapshot, Option<SizeDiff>)],
    config: &CiConfig,
) -> Result<String> {
    let mut report = String::new();
    
    // Get git context for header
    let git_ctx = get_git_context().ok();
    
    // Header
    report.push_str("## 🐋 Docker Image Size Report\n\n");
    
    if let Some(ctx) = &git_ctx {
        let commit_short = ctx.commit_sha.chars().take(7).collect::<String>();
        let branch = &ctx.branch;
        let date = Utc::now().format("%Y-%m-%d").to_string();
        report.push_str(&format!(
            "**Commit:** `{}` | **Branch:** `{}` | **Date:** {}\n\n",
            commit_short, branch, date
        ));
    }
    
    // Summary table
    report.push_str("### Summary\n\n");
    report.push_str("| Image | Previous | Current | Change |\n");
    report.push_str("|-------|----------|---------|--------|\n");
    
    let mut total_previous = 0u64;
    let mut total_current = 0u64;
    
    for (current, diff_opt) in comparisons {
        let image_name = format!("{}:{}", current.image, current.tag.as_deref().unwrap_or("latest"));
        let current_size = ByteSize(current.total_size).to_string_as(true);
        
        if let Some(diff) = diff_opt {
            let previous_size = ByteSize(diff.before.total_size).to_string_as(true);
            let delta = diff.total_delta;
            let percent = if diff.before.total_size > 0 {
                (delta as f64 / diff.before.total_size as f64) * 100.0
            } else {
                0.0
            };
            
            let change_str = if delta == 0 {
                "— ✅".to_string()
            } else if delta > 0 {
                format!("+{} (+{:.1}%) 📈", ByteSize(delta as u64).to_string_as(true), percent)
            } else {
                format!("-{} ({:.1}%) 📉", ByteSize((-delta) as u64).to_string_as(true), percent)
            };
            
            report.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                image_name, previous_size, current_size, change_str
            ));
            
            total_previous += diff.before.total_size;
            total_current += current.total_size;
        } else {
            // First run, no previous data
            report.push_str(&format!(
                "| {} | — | {} | *First run* 🆕 |\n",
                image_name, current_size
            ));
            total_current += current.total_size;
        }
    }
    
    // Total row
    if total_previous > 0 {
        let total_delta = total_current as i64 - total_previous as i64;
        let total_percent = (total_delta as f64 / total_previous as f64) * 100.0;
        let total_change = if total_delta == 0 {
            "— ✅".to_string()
        } else if total_delta > 0 {
            format!("+{} (+{:.1}%)", ByteSize(total_delta as u64).to_string_as(true), total_percent)
        } else {
            format!("-{} ({:.1}%)", ByteSize((-total_delta) as u64).to_string_as(true), total_percent)
        };
        
        report.push_str(&format!(
            "| **Total** | **{}** | **{}** | **{}** |\n\n",
            ByteSize(total_previous).to_string_as(true),
            ByteSize(total_current).to_string_as(true),
            total_change
        ));
    }
    
    // Layer details for images that changed
    for (current, diff_opt) in comparisons {
        if let Some(diff) = diff_opt {
            if diff.total_delta != 0 {
                let image_name = format!("{}:{}", current.image, current.tag.as_deref().unwrap_or("latest"));
                report.push_str(&format!("\n<details>\n<summary>Layer Details: {}</summary>\n\n", image_name));
                report.push_str("| Status | Size | Delta | Command |\n");
                report.push_str("|--------|------|-------|----------|\n");
                
                for change in &diff.layer_changes {
                    let (status, size, delta, cmd) = match change {
                        LayerChange::Added(layer) => (
                            "Added ➕",
                            ByteSize(layer.size).to_string_as(true),
                            format!("+{}", ByteSize(layer.size).to_string_as(true)),
                            truncate(&layer.command, 50),
                        ),
                        LayerChange::Removed(layer) => (
                            "Removed ➖",
                            ByteSize(layer.size).to_string_as(true),
                            format!("-{}", ByteSize(layer.size).to_string_as(true)),
                            truncate(&layer.command, 50),
                        ),
                        LayerChange::Modified { before, after } => {
                            let delta = after.size as i64 - before.size as i64;
                            let delta_str = if delta > 0 {
                                format!("+{}", ByteSize(delta as u64).to_string_as(true))
                            } else {
                                format!("-{}", ByteSize((-delta) as u64).to_string_as(true))
                            };
                            (
                                "Modified 🔄",
                                ByteSize(after.size).to_string_as(true),
                                delta_str,
                                truncate(&after.command, 50),
                            )
                        },
                        LayerChange::Unchanged(layer) => (
                            "Unchanged ✅",
                            ByteSize(layer.size).to_string_as(true),
                            "—".to_string(),
                            truncate(&layer.command, 50),
                        ),
                    };
                    
                    report.push_str(&format!("| {} | {} | {} | `{}` |\n", status, size, delta, cmd));
                }
                
                report.push_str("\n</details>\n");
            }
        }
    }
    
    // Budget status
    report.push_str("\n### Budget Status\n\n");
    
    if let Some(budget) = config.budget_bytes {
        let status = if total_current <= budget {
            "✅"
        } else {
            "❌"
        };
        report.push_str(&format!(
            "{} Total size: {} (budget: {})\n\n",
            status,
            ByteSize(total_current).to_string_as(true),
            ByteSize(budget).to_string_as(true)
        ));
    }
    
    if let Some(threshold) = config.budget_increase_percent {
        for (current, diff_opt) in comparisons {
            if let Some(diff) = diff_opt {
                if diff.before.total_size > 0 {
                    let percent = (diff.total_delta as f64 / diff.before.total_size as f64) * 100.0;
                    if percent.abs() > threshold {
                        let image_name = format!("{}:{}", current.image, current.tag.as_deref().unwrap_or("latest"));
                        let status = if percent > 0.0 { "⚠️" } else { "✅" };
                        report.push_str(&format!(
                            "{} {} changed by {:.1}% (threshold: {}%)\n\n",
                            status, image_name, percent, threshold
                        ));
                    }
                }
            }
        }
    }
    
    report.push_str("---\n");
    report.push_str("*Tracked by [dit](https://github.com/Bentlybro/docker-image-tracker) 🐋*\n");
    
    Ok(report)
}

async fn post_github_comment(report: &str) -> Result<()> {
    let ctx = GitHubContext::from_env()
        .context("Failed to load GitHub context. Not running in GitHub Actions?")?;
    
    if !ctx.is_pr() {
        println!("⚠️ Not a pull request, skipping comment posting");
        return Ok(());
    }
    
    let pr_number = ctx.pr_number.unwrap();
    let client = GitHubClient::new(ctx.token, ctx.repo);
    
    client.post_or_update_pr_comment(pr_number, report.to_string()).await?;
    
    Ok(())
}

fn check_budgets(
    comparisons: &[(ImageSnapshot, Option<SizeDiff>)],
    config: &CiConfig,
) -> Result<bool> {
    let mut failed = false;
    
    // Calculate totals
    let total_current: u64 = comparisons.iter().map(|(s, _)| s.total_size).sum();
    
    // Check total budget
    if let Some(budget) = config.budget_bytes {
        if total_current > budget {
            eprintln!(
                "❌ Budget exceeded: {} > {} (budget)",
                ByteSize(total_current).to_string_as(true),
                ByteSize(budget).to_string_as(true)
            );
            failed = true;
        }
    }
    
    // Check increase threshold
    if let Some(threshold) = config.budget_increase_percent {
        for (current, diff_opt) in comparisons {
            if let Some(diff) = diff_opt {
                if diff.before.total_size > 0 {
                    let percent = (diff.total_delta as f64 / diff.before.total_size as f64) * 100.0;
                    if percent > threshold {
                        let image_name = format!("{}:{}", current.image, current.tag.as_deref().unwrap_or("latest"));
                        eprintln!(
                            "❌ Image {} grew by {:.1}% (threshold: {}%)",
                            image_name, percent, threshold
                        );
                        failed = true;
                    }
                }
            }
        }
    }
    
    // Check fail-on-increase
    if config.fail_on_increase {
        for (current, diff_opt) in comparisons {
            if let Some(diff) = diff_opt {
                if diff.total_delta > 0 {
                    let image_name = format!("{}:{}", current.image, current.tag.as_deref().unwrap_or("latest"));
                    eprintln!(
                        "❌ Image {} increased in size (fail-on-increase enabled)",
                        image_name
                    );
                    failed = true;
                }
            }
        }
    }
    
    Ok(failed)
}

#[derive(Debug)]
struct GitContext {
    commit_sha: String,
    branch: String,
    commit_message: String,
    author: String,
}

fn get_git_context() -> Result<GitContext> {
    let commit_sha = run_git(&["rev-parse", "HEAD"])?;
    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let commit_message = run_git(&["log", "-1", "--pretty=%s"])?;
    let author = run_git(&["log", "-1", "--pretty=%an <%ae>"])?;

    Ok(GitContext {
        commit_sha,
        branch,
        commit_message,
        author,
    })
}

fn run_git(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .context("Failed to execute git command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Git command failed: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    
    let (num_str, multiplier) = if s.ends_with("GB") {
        (&s[..s.len() - 2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len() - 2], 1024u64 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len() - 2], 1024u64)
    } else if s.ends_with('G') {
        (&s[..s.len() - 1], 1024u64 * 1024 * 1024)
    } else if s.ends_with('M') {
        (&s[..s.len() - 1], 1024u64 * 1024)
    } else if s.ends_with('K') {
        (&s[..s.len() - 1], 1024u64)
    } else {
        (s.as_str(), 1u64)
    };
    
    let num: f64 = num_str.trim().parse()
        .context("Failed to parse size number")?;
    
    Ok((num * multiplier as f64) as u64)
}
