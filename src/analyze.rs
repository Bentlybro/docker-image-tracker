use anyhow::Result;
use clap::ValueEnum;

use crate::docker::DockerClient;
use crate::format::print_snapshot_table;
use crate::models::ImageSnapshot;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

pub async fn analyze_image(image: &str, format: OutputFormat) -> Result<ImageSnapshot> {
    let docker = DockerClient::new()?;
    let snapshot = docker.inspect_image(image).await?;

    match format {
        OutputFormat::Table => {
            print_snapshot_table(&snapshot);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
        }
    }

    Ok(snapshot)
}
