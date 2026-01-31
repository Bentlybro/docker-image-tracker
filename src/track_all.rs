use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::docker::DockerClient;
use crate::format::format_size;
use crate::models::ImageSnapshot;

const HISTORY_DIR: &str = ".dit";
const HISTORY_FILE: &str = "history.json";

pub async fn track_all_images(filter: Option<&str>) -> Result<()> {
    let docker = DockerClient::new()?;
    let images = docker.list_all_images(filter).await?;

    if images.is_empty() {
        println!("No images found");
        return Ok(());
    }

    // Get git context once for all images
    let git_context = get_git_context()?;

    println!("Tracking {} images at commit {}...\n", 
        images.len(),
        git_context.commit_sha.chars().take(7).collect::<String>()
    );

    let mut total_size = 0u64;
    let mut success_count = 0;

    for image in &images {
        print!("  {} ... ", image);
        
        match docker.inspect_image(image).await {
            Ok(mut snapshot) => {
                // Apply git context
                snapshot.commit_sha = git_context.commit_sha.clone();
                snapshot.branch = git_context.branch.clone();
                snapshot.commit_message = git_context.commit_message.clone();
                snapshot.author = git_context.author.clone();
                snapshot.timestamp = Utc::now();

                // Save snapshot
                if let Err(e) = save_snapshot(&snapshot) {
                    println!("❌ Failed to save: {}", e);
                } else {
                    total_size += snapshot.total_size;
                    success_count += 1;
                    println!("✅ {} tracked", format_size(snapshot.total_size));
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
    }

    println!(
        "\n✅ Tracked {} images, total size: {}",
        success_count,
        format_size(total_size)
    );

    Ok(())
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
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to execute git command. Is git installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git command failed: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn save_snapshot(snapshot: &ImageSnapshot) -> Result<()> {
    // Create .dit directory if it doesn't exist
    let dit_dir = PathBuf::from(HISTORY_DIR);
    if !dit_dir.exists() {
        fs::create_dir(&dit_dir).context("Failed to create .dit directory")?;
    }

    // Load existing history
    let history_path = dit_dir.join(HISTORY_FILE);
    let mut snapshots: Vec<ImageSnapshot> = if history_path.exists() {
        let content = fs::read_to_string(&history_path)
            .context("Failed to read history.json")?;
        serde_json::from_str(&content).context("Failed to parse history.json")?
    } else {
        Vec::new()
    };

    // Append new snapshot
    snapshots.push(snapshot.clone());

    // Save back to file
    let json = serde_json::to_string_pretty(&snapshots)?;
    fs::write(&history_path, json).context("Failed to write history.json")?;

    Ok(())
}
