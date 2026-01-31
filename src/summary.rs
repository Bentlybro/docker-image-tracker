use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

use crate::chart::calculate_trend_with_sparkline;
use crate::format::format_size;
use crate::models::ImageSnapshot;
use crate::track::load_history;

pub async fn show_summary() -> Result<()> {
    let history = load_history()?;

    if history.is_empty() {
        println!("No tracked images found. Use 'dit track' or 'dit track-all' to start tracking.");
        return Ok(());
    }

    // Group snapshots by image
    let mut by_image: HashMap<String, Vec<ImageSnapshot>> = HashMap::new();

    for snapshot in history {
        let key = format!("{}:{}", 
            snapshot.image, 
            snapshot.tag.as_deref().unwrap_or("latest")
        );
        by_image.entry(key).or_insert_with(Vec::new).push(snapshot);
    }

    // Sort each image's snapshots by timestamp
    for snapshots in by_image.values_mut() {
        snapshots.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    println!("\n{}", "Docker Image Tracker Summary".bold().underline());
    println!("Total tracked images: {}\n", by_image.len());

    let mut builder = Builder::default();
    builder.push_record([
        "Image",
        "Latest Size",
        "Trend",
        "Snapshots",
        "Last Tracked",
    ]);

    let mut total_size = 0u64;

    // Convert to sorted vector for consistent output
    let mut images: Vec<_> = by_image.iter().collect();
    images.sort_by(|a, b| a.0.cmp(b.0));

    for (image_name, snapshots) in images {
        if snapshots.is_empty() {
            continue;
        }

        let latest = snapshots.last().unwrap();
        total_size += latest.total_size;

        // Calculate trend with sparkline (last 10 snapshots)
        let trend = calculate_trend_with_sparkline(snapshots, 10);

        let last_tracked = latest.timestamp.format("%Y-%m-%d %H:%M").to_string();

        builder.push_record([
            image_name,
            &format_size(latest.total_size),
            &trend,
            &snapshots.len().to_string(),
            &last_tracked,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()));

    println!("{}\n", table);

    println!(
        "{}",
        format!("Total combined size: {}", format_size(total_size)).bold()
    );

    Ok(())
}
