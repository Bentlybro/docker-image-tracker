use anyhow::{bail, Result};
use bytesize::ByteSize;
use colored::Colorize;
use std::collections::HashMap;

use crate::format::format_size;
use crate::models::ImageSnapshot;
use crate::track::load_history;

const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Show bar chart for a single image
pub async fn show_chart(image: &str, last: Option<usize>) -> Result<()> {
    let history = load_history()?;

    if history.is_empty() {
        bail!("No history found. Run 'dit track' first.");
    }

    // Filter history for the specified image
    let mut image_history: Vec<_> = history
        .into_iter()
        .filter(|s| s.image == image)
        .collect();

    if image_history.is_empty() {
        bail!("No history found for image '{}'", image);
    }

    // Sort by timestamp (oldest first)
    image_history.sort_by_key(|s| s.timestamp);

    // Limit if requested
    let limit = last.unwrap_or(20);
    if image_history.len() > limit {
        let start = image_history.len() - limit;
        image_history = image_history[start..].to_vec();
    }

    // Handle single snapshot
    if image_history.len() == 1 {
        let snapshot = &image_history[0];
        let commit_short = snapshot.commit_sha.chars().take(7).collect::<String>();
        println!("\n{}", format!("{} — Size History", image).bold().underline());
        println!(
            "\n  {} │ {} {}",
            commit_short.bright_cyan(),
            "█".repeat(40),
            format_size(snapshot.total_size).bold()
        );
        println!("\n{}", "Only one snapshot available. Track more commits to see trends!".dimmed());
        return Ok(());
    }

    // Find min and max for scaling
    let sizes: Vec<u64> = image_history.iter().map(|s| s.total_size).collect();
    let min_size = *sizes.iter().min().unwrap();
    let max_size = *sizes.iter().max().unwrap();

    println!("\n{}", format!("{} — Size History", image).bold().underline());
    println!();

    // Draw bar chart
    for (i, snapshot) in image_history.iter().enumerate() {
        let commit_short = snapshot.commit_sha.chars().take(7).collect::<String>();
        
        // Calculate bar width (40 chars max)
        let bar_width = if max_size == min_size {
            40
        } else {
            let normalized = (snapshot.total_size - min_size) as f64 / (max_size - min_size) as f64;
            ((normalized * 40.0).round() as usize).max(1)
        };

        // Calculate delta from previous
        let (delta_str, bar_color) = if i > 0 {
            let prev_size = image_history[i - 1].total_size;
            let delta = snapshot.total_size as i64 - prev_size as i64;
            
            if delta > 0 {
                let delta_display = format!(" (+{})", ByteSize(delta as u64).to_string_as(true));
                (delta_display.red().to_string(), "█".red())
            } else if delta < 0 {
                let delta_display = format!(" (-{})", ByteSize((-delta) as u64).to_string_as(true));
                (delta_display.green().to_string(), "█".green())
            } else {
                ("".to_string(), "█".normal())
            }
        } else {
            ("".to_string(), "█".normal())
        };

        let bar = bar_color.to_string().repeat(bar_width);
        let size_str = format_size(snapshot.total_size).bold();

        println!(
            "  {} │ {} {}{}",
            commit_short.bright_cyan(),
            bar,
            size_str,
            delta_str
        );
    }

    println!();
    Ok(())
}

/// Show sparklines for all tracked images
pub async fn show_chart_all(last: Option<usize>) -> Result<()> {
    let history = load_history()?;

    if history.is_empty() {
        bail!("No history found. Run 'dit track' first.");
    }

    // Group snapshots by image
    let mut by_image: HashMap<String, Vec<ImageSnapshot>> = HashMap::new();

    for snapshot in history {
        let key = snapshot.image.clone();
        by_image.entry(key).or_insert_with(Vec::new).push(snapshot);
    }

    // Sort each image's snapshots by timestamp
    for snapshots in by_image.values_mut() {
        snapshots.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    let limit = last.unwrap_or(10);
    
    println!("\n{}", format!("Image Trends (last {} snapshots)", limit).bold().underline());
    println!();

    // Convert to sorted vector for consistent output
    let mut images: Vec<_> = by_image.iter().collect();
    images.sort_by(|a, b| a.0.cmp(b.0));

    // Find longest image name for alignment
    let max_name_len = images.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

    for (image_name, snapshots) in images {
        if snapshots.is_empty() {
            continue;
        }

        // Take last N snapshots
        let recent_count = snapshots.len().min(limit);
        let recent = &snapshots[snapshots.len() - recent_count..];

        let sparkline = generate_sparkline(recent);
        
        let latest = recent.last().unwrap();
        let first = recent.first().unwrap();
        
        // Calculate overall change
        let (change_str, change_color) = if recent.len() > 1 {
            let total_delta = latest.total_size as i64 - first.total_size as i64;
            let percent = if first.total_size > 0 {
                (total_delta as f64 / first.total_size as f64) * 100.0
            } else {
                0.0
            };

            if total_delta.abs() < 1024 * 10 {
                // Less than 10KB change
                ("(stable)".dimmed().to_string(), "stable")
            } else if total_delta > 0 {
                (format!("(+{:.1}%)", percent).red().to_string(), "increase")
            } else {
                (format!("({:.1}%)", percent).green().to_string(), "decrease")
            }
        } else {
            ("".to_string(), "stable")
        };

        // Add colored sparkline
        let colored_sparkline = match change_color {
            "increase" => sparkline.red().to_string(),
            "decrease" => sparkline.green().to_string(),
            _ => sparkline.dimmed().to_string(),
        };

        println!(
            "  {:<width$}  {}  {} {}",
            image_name.bright_cyan(),
            colored_sparkline,
            format_size(latest.total_size).bold(),
            change_str,
            width = max_name_len
        );
    }

    println!();
    Ok(())
}

/// Generate sparkline from snapshots
pub fn generate_sparkline(snapshots: &[ImageSnapshot]) -> String {
    if snapshots.is_empty() {
        return "".to_string();
    }

    if snapshots.len() == 1 {
        return SPARKLINE_CHARS[4].to_string();
    }

    let sizes: Vec<u64> = snapshots.iter().map(|s| s.total_size).collect();
    let min_size = *sizes.iter().min().unwrap();
    let max_size = *sizes.iter().max().unwrap();

    sizes
        .iter()
        .map(|&size| {
            if max_size == min_size {
                SPARKLINE_CHARS[4] // Middle character if all same
            } else {
                let normalized = (size - min_size) as f64 / (max_size - min_size) as f64;
                let index = (normalized * (SPARKLINE_CHARS.len() - 1) as f64).round() as usize;
                SPARKLINE_CHARS[index.min(SPARKLINE_CHARS.len() - 1)]
            }
        })
        .collect()
}

/// Calculate trend description for summary
pub fn calculate_trend_with_sparkline(snapshots: &[ImageSnapshot], sparkline_count: usize) -> String {
    if snapshots.is_empty() {
        return "—".to_string();
    }

    // Take last N snapshots for sparkline
    let recent_count = snapshots.len().min(sparkline_count);
    let recent = &snapshots[snapshots.len() - recent_count..];

    let sparkline = generate_sparkline(recent);

    if snapshots.len() < 2 {
        return sparkline;
    }

    // Calculate overall trend
    let first = recent.first().unwrap();
    let latest = recent.last().unwrap();
    let delta = latest.total_size as i64 - first.total_size as i64;

    let trend_indicator = if delta.abs() < 1024 * 10 {
        // Less than 10KB change - stable
        sparkline.dimmed().to_string()
    } else if delta > 0 {
        sparkline.red().to_string()
    } else {
        sparkline.green().to_string()
    };

    trend_indicator
}
