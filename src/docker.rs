use anyhow::{Context, Result};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use chrono::{DateTime, Utc};

use crate::models::{ImageSnapshot, LayerInfo};

pub struct DockerClient {
    client: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker daemon. Is Docker running?")?;
        Ok(Self { client })
    }

    pub async fn inspect_image(&self, image: &str) -> Result<ImageSnapshot> {
        let inspect = self
            .client
            .inspect_image(image)
            .await
            .context(format!("Failed to inspect image '{}'", image))?;

        // Extract basic metadata
        let total_size = inspect.size.unwrap_or(0) as u64;
        let os = inspect.os.unwrap_or_else(|| "linux".to_string());
        let arch = inspect.architecture.unwrap_or_else(|| "amd64".to_string());
        let digest = inspect.repo_digests.and_then(|d| d.first().cloned());

        // Parse tag from image name
        let (image_name, tag) = if image.contains(':') {
            let parts: Vec<&str> = image.split(':').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (image.to_string(), Some("latest".to_string()))
        };

        // Use image history API to get actual per-layer sizes and commands
        let history = self
            .client
            .image_history(image)
            .await
            .context(format!("Failed to get history for image '{}'", image))?;

        let mut layers = Vec::new();
        for entry in history.iter().rev() {
            // Skip empty layers (size 0) that are just metadata
            let size = entry.size as u64;

            // Extract the command — clean up the Docker format
            let command = if entry.created_by.is_empty() {
                "<unknown>".to_string()
            } else {
                entry.created_by.clone()
            };

            // Clean up command: remove "/bin/sh -c #(nop) " prefix and trim
            let command = clean_command(&command);

            // Parse the created timestamp
            let created = if entry.created > 0 {
                DateTime::from_timestamp(entry.created, 0)
                    .unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            // Use the ID as digest, or generate a placeholder
            let layer_digest = if entry.id.is_empty() {
                "<missing>".to_string()
            } else {
                entry.id.clone()
            };

            layers.push(LayerInfo {
                digest: layer_digest,
                size,
                command,
                created,
            });
        }

        let layer_count = layers.len();

        Ok(ImageSnapshot {
            image: image_name,
            tag,
            digest,
            commit_sha: String::new(),
            branch: String::new(),
            commit_message: String::new(),
            author: String::new(),
            timestamp: Utc::now(),
            total_size,
            layer_count,
            layers,
            os,
            arch,
        })
    }

    pub async fn list_all_images(&self, filter: Option<&str>) -> Result<Vec<String>> {
        let options = ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        };

        let images = self
            .client
            .list_images(Some(options))
            .await
            .context("Failed to list Docker images")?;

        let mut result = Vec::new();

        for image in images {
            for tag in &image.repo_tags {
                // Skip <none>:<none> images
                if tag == "<none>:<none>" {
                    continue;
                }

                // Apply filter if provided
                if let Some(f) = filter {
                    if !tag.to_lowercase().contains(&f.to_lowercase()) {
                        continue;
                    }
                }

                result.push(tag.clone());
            }
        }

        // Sort alphabetically
        result.sort();
        Ok(result)
    }
}

/// Clean up Docker command strings for display
fn clean_command(cmd: &str) -> String {
    let mut cleaned = cmd.to_string();

    // Remove /bin/sh -c #(nop) prefix (metadata commands like ENV, LABEL, etc.)
    cleaned = cleaned
        .replace("/bin/sh -c #(nop)  ", "")
        .replace("/bin/sh -c #(nop) ", "");

    // Remove /bin/sh -c prefix (RUN commands from old builder)
    cleaned = cleaned.replace("/bin/sh -c ", "RUN ");

    // Fix double RUN from BuildKit (BuildKit already includes RUN prefix)
    cleaned = cleaned.replace("RUN RUN ", "RUN ");

    // Remove BuildKit suffix
    cleaned = cleaned.replace(" # buildkit", "");

    let cleaned = cleaned.trim().to_string();

    if cleaned.is_empty() {
        "<layer>".to_string()
    } else {
        // Truncate very long commands
        if cleaned.len() > 120 {
            format!("{}...", &cleaned[..117])
        } else {
            cleaned
        }
    }
}
