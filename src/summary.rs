use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

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
        "Trend (Last 3)",
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

        // Calculate trend from last 3 snapshots
        let trend = calculate_trend(snapshots);

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

fn calculate_trend(snapshots: &[ImageSnapshot]) -> String {
    if snapshots.len() < 2 {
        return "—".to_string();
    }

    // Get last 3 snapshots (or less if not available)
    let count = snapshots.len().min(3);
    let recent = &snapshots[snapshots.len() - count..];

    let mut deltas = Vec::new();
    for i in 1..recent.len() {
        let delta = recent[i].total_size as i64 - recent[i - 1].total_size as i64;
        deltas.push(delta);
    }

    // Format deltas
    if deltas.is_empty() {
        return "—".to_string();
    }

    let mut trend_parts = Vec::new();
    for delta in deltas {
        if delta == 0 {
            trend_parts.push("→".dimmed().to_string());
        } else if delta > 0 {
            trend_parts.push(format!("+{}", format_size(delta as u64)).red().to_string());
        } else {
            trend_parts.push(format!("-{}", format_size((-delta) as u64)).green().to_string());
        }
    }

    trend_parts.join(" → ")
}
