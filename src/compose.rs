use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::docker::DockerClient;
use crate::format::format_size;
use crate::history::show_history;
use crate::track_all::track_all_images;

#[derive(Debug, Deserialize, Serialize)]
struct ComposeFile {
    #[serde(default)]
    services: HashMap<String, Service>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Service {
    #[serde(default)]
    build: Option<BuildConfig>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum BuildConfig {
    Simple(String),
    Complex {
        #[serde(default)]
        context: Option<String>,
        #[serde(default)]
        dockerfile: Option<String>,
    },
}

pub async fn compose_analyze(file: Option<&str>) -> Result<()> {
    let compose_path = find_compose_file(file)?;
    let project_name = get_project_name(&compose_path)?;
    let services = parse_compose_file_internal(&compose_path)?;

    if services.is_empty() {
        println!("No services with build directives found in {}", compose_path.display());
        return Ok(());
    }

    println!("Found {} services with build directives in {}:\n", 
        services.len(), 
        compose_path.display()
    );

    // Build filter pattern for compose images
    // Docker Compose typically names images as: <project>_<service> or <project>-<service>
    let mut images = Vec::new();
    for service in &services {
        let image_name = format!("{}_{}", project_name, service);
        let alt_name = format!("{}-{}", project_name, service);
        images.push(image_name);
        images.push(alt_name);
    }

    // Try to find matching images
    let docker = DockerClient::new()?;
    let all_images = docker.list_all_images(None).await?;

    let mut found_images = Vec::new();
    for image in &all_images {
        for service in &services {
            let patterns = vec![
                format!("{}_{}", project_name, service),
                format!("{}-{}", project_name, service),
                format!("{}/{}", project_name, service),
            ];

            for pattern in patterns {
                if image.to_lowercase().contains(&pattern.to_lowercase()) {
                    found_images.push(image.clone());
                    break;
                }
            }
        }
    }

    if found_images.is_empty() {
        println!("⚠️  No built images found for services: {}", services.join(", "));
        println!("Run 'docker-compose build' first or check that images are tagged correctly.");
        return Ok(());
    }

    println!("Analyzing {} compose images...\n", found_images.len());

    for image in &found_images {
        match docker.inspect_image(image).await {
            Ok(snapshot) => {
                println!("  {} — {} ({} layers)", 
                    image, 
                    format_size(snapshot.total_size),
                    snapshot.layer_count
                );
            }
            Err(e) => {
                eprintln!("  ⚠️  {} — Failed: {}", image, e);
            }
        }
    }

    Ok(())
}

pub async fn compose_track(file: Option<&str>) -> Result<()> {
    let compose_path = find_compose_file(file)?;
    let project_name = get_project_name(&compose_path)?;
    let services = parse_compose_file_internal(&compose_path)?;

    if services.is_empty() {
        println!("No services with build directives found in {}", compose_path.display());
        return Ok(());
    }

    // Find compose images
    let docker = DockerClient::new()?;
    let all_images = docker.list_all_images(None).await?;

    let mut found_images = Vec::new();
    for image in &all_images {
        for service in &services {
            let patterns = vec![
                format!("{}_{}", project_name, service),
                format!("{}-{}", project_name, service),
                format!("{}/{}", project_name, service),
            ];

            for pattern in patterns {
                if image.to_lowercase().contains(&pattern.to_lowercase()) {
                    found_images.push(image.clone());
                    break;
                }
            }
        }
    }

    if found_images.is_empty() {
        println!("⚠️  No built images found for compose services");
        return Ok(());
    }

    println!("Tracking {} compose images...\n", found_images.len());
    
    // Track all found images
    track_all_images(None).await?;

    Ok(())
}

pub async fn compose_history(file: Option<&str>) -> Result<()> {
    let compose_path = find_compose_file(file)?;
    let project_name = get_project_name(&compose_path)?;
    let services = parse_compose_file_internal(&compose_path)?;

    if services.is_empty() {
        println!("No services with build directives found");
        return Ok(());
    }

    // Show history for each service
    for service in &services {
        let patterns = vec![
            format!("{}_{}", project_name, service),
            format!("{}-{}", project_name, service),
        ];

        for pattern in patterns {
            // Try to show history for this pattern
            if let Ok(_) = show_history(&pattern, None).await {
                break;
            }
        }
    }

    Ok(())
}

fn find_compose_file(file: Option<&str>) -> Result<PathBuf> {
    if let Some(f) = file {
        let path = PathBuf::from(f);
        if !path.exists() {
            anyhow::bail!("Compose file not found: {}", f);
        }
        return Ok(path);
    }

    // Try common compose file names
    let candidates = vec![
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("No docker-compose file found in current directory. Use --file to specify a path.")
}

pub fn parse_compose_file(path: Option<&str>) -> Result<Vec<String>> {
    let compose_path = find_compose_file(path)?;
    parse_compose_file_internal(&compose_path)
}

fn parse_compose_file_internal(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read compose file: {}", path.display()))?;

    let compose: ComposeFile = serde_yaml::from_str(&content)
        .context("Failed to parse docker-compose file")?;

    let mut services_with_build = Vec::new();

    for (name, service) in compose.services {
        if service.build.is_some() {
            services_with_build.push(name);
        }
    }

    Ok(services_with_build)
}

fn get_project_name(compose_path: &Path) -> Result<String> {
    // Get project name from parent directory
    let parent = compose_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let project_name = parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    Ok(project_name)
}
