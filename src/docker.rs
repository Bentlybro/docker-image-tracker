use anyhow::{Context, Result};
use bollard::Docker;
use chrono::Utc;

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

        // Extract layer information from root_fs
        let mut layers = Vec::new();
        if let Some(root_fs) = inspect.root_fs {
            if let Some(layer_digests) = root_fs.layers {
                for digest in layer_digests.iter() {
                    // Note: Without history API, we can't get exact layer sizes and commands
                    // We'll use the digest and placeholder data for now
                    layers.push(LayerInfo {
                        digest: digest.clone(),
                        size: 0, // Will be computed differently
                        command: "<layer>".to_string(),
                        created: Utc::now(),
                    });
                }
            }
        }

        // If we have layer data, estimate size per layer
        // (total size divided by number of layers)
        let layer_count = layers.len();
        if layer_count > 0 {
            let size_per_layer = total_size / layer_count as u64;
            for layer in &mut layers {
                layer.size = size_per_layer;
            }
        }

        // For now, we'll fill in git context with placeholders
        // The track command will override these
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
}

fn truncate_command(cmd: &str, max_len: usize) -> String {
    if cmd.len() <= max_len {
        cmd.to_string()
    } else {
        format!("{}...", &cmd[..max_len - 3])
    }
}
