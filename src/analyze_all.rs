use anyhow::Result;
use colored::Colorize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

use crate::analyze::OutputFormat;
use crate::docker::DockerClient;
use crate::format::format_size;

pub async fn analyze_all_images(filter: Option<&str>, format: OutputFormat) -> Result<()> {
    let docker = DockerClient::new()?;
    let images = docker.list_all_images(filter).await?;

    if images.is_empty() {
        println!("No images found");
        return Ok(());
    }

    println!("Analyzing {} images...\n", images.len());

    let mut snapshots = Vec::new();
    for image in &images {
        match docker.inspect_image(image).await {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(e) => eprintln!("⚠️  Failed to analyze {}: {}", image, e),
        }
    }

    // Sort by size (biggest first)
    snapshots.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&snapshots)?);
        }
        OutputFormat::Table => {
            print_analyze_all_table(&snapshots);
        }
    }

    Ok(())
}

fn print_analyze_all_table(snapshots: &[crate::models::ImageSnapshot]) {
    let total_size: u64 = snapshots.iter().map(|s| s.total_size).sum();

    println!("{}", "All Docker Images".bold().underline());

    let mut builder = Builder::default();
    builder.push_record(["Image", "Tag", "Size", "Layers", "OS/Arch"]);

    for snapshot in snapshots {
        let tag = snapshot.tag.as_deref().unwrap_or("latest");
        let os_arch = format!("{}/{}", snapshot.os, snapshot.arch);

        builder.push_record([
            &snapshot.image,
            tag,
            &format_size(snapshot.total_size),
            &snapshot.layer_count.to_string(),
            &os_arch,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()));

    println!("{}\n", table);

    println!(
        "{}",
        format!(
            "Total: {} images, {} combined",
            snapshots.len(),
            format_size(total_size)
        )
        .bold()
    );
}
