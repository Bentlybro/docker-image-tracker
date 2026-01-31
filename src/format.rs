use bytesize::ByteSize;
use colored::Colorize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

use crate::models::{ImageSnapshot, LayerChange, SizeDiff};

pub fn format_size(bytes: u64) -> String {
    ByteSize(bytes).to_string_as(true)
}

pub fn format_size_delta(delta: i64) -> String {
    if delta == 0 {
        "unchanged".to_string()
    } else if delta > 0 {
        format!("+{}", ByteSize(delta as u64).to_string_as(true)).red().to_string()
    } else {
        format!("-{}", ByteSize((-delta) as u64).to_string_as(true))
            .green()
            .to_string()
    }
}

pub fn print_snapshot_table(snapshot: &ImageSnapshot) {
    println!("\n{}", "Image Analysis".bold().underline());
    println!("Image: {}", snapshot.image.bright_cyan());
    if let Some(ref tag) = snapshot.tag {
        println!("Tag: {}", tag);
    }
    println!("Total Size: {}", format_size(snapshot.total_size).bold());
    println!("Layers: {}", snapshot.layer_count);
    println!("OS/Arch: {}/{}", snapshot.os, snapshot.arch);

    if !snapshot.layers.is_empty() {
        println!("\n{}", "Layer Breakdown".bold().underline());

        let mut builder = Builder::default();
        builder.push_record(["#", "Size", "Created", "Command"]);

        for (i, layer) in snapshot.layers.iter().enumerate() {
            builder.push_record([
                &format!("{}", i + 1),
                &format_size(layer.size),
                &layer.created.format("%Y-%m-%d").to_string(),
                &layer.command,
            ]);
        }

        let mut table = builder.build();
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()));

        println!("{}", table);
    }
}

pub fn print_diff_table(diff: &SizeDiff) {
    let total_delta = diff.total_delta;
    let total_percent = if diff.before.total_size > 0 {
        (total_delta as f64 / diff.before.total_size as f64) * 100.0
    } else {
        0.0
    };

    println!("\n{}", "Image Size Diff".bold().underline());
    println!("Image: {}", diff.after.image.bright_cyan());
    println!(
        "Before ({}): {}",
        diff.before.commit_sha.chars().take(7).collect::<String>(),
        format_size(diff.before.total_size)
    );
    println!(
        "After ({}): {}",
        diff.after.commit_sha.chars().take(7).collect::<String>(),
        format_size(diff.after.total_size)
    );

    let delta_str = format_size_delta(total_delta);
    let trend = if total_delta > 0 {
        "📈"
    } else if total_delta < 0 {
        "📉"
    } else {
        "✅"
    };

    println!(
        "Change: {} ({:+.1}%) {}",
        delta_str.bold(),
        total_percent,
        trend
    );

    println!("\n{}", "Layer Changes".bold().underline());

    let mut builder = Builder::default();
    builder.push_record(["Status", "Size", "Delta", "Command"]);

    for change in &diff.layer_changes {
        let status = match change {
            LayerChange::Added(_) => "Added".green().to_string(),
            LayerChange::Removed(_) => "Removed".red().to_string(),
            LayerChange::Modified { .. } => "Modified".yellow().to_string(),
            LayerChange::Unchanged(_) => "Unchanged".dimmed().to_string(),
        };

        let layer = change.layer();
        let size_delta = change.size_delta();

        builder.push_record([
            &status,
            &format_size(layer.size),
            &format_size_delta(size_delta),
            &layer.command,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()));

    println!("{}", table);
}

pub fn print_history_table(snapshots: &[ImageSnapshot]) {
    if snapshots.is_empty() {
        println!("No history found");
        return;
    }

    println!("\n{}", "Image Size History".bold().underline());
    println!("Image: {}", snapshots[0].image.bright_cyan());

    let mut builder = Builder::default();
    builder.push_record(["Commit", "Branch", "Date", "Size", "Delta", "Trend"]);

    let mut prev_size: Option<u64> = None;

    for snapshot in snapshots {
        let commit_short = snapshot.commit_sha.chars().take(7).collect::<String>();
        let date = snapshot.timestamp.format("%Y-%m-%d %H:%M").to_string();
        let size = format_size(snapshot.total_size);

        let (delta_str, trend) = if let Some(prev) = prev_size {
            let delta = snapshot.total_size as i64 - prev as i64;
            let trend = if delta > 0 {
                "📈"
            } else if delta < 0 {
                "📉"
            } else {
                "✅"
            };
            (format_size_delta(delta), trend)
        } else {
            ("—".to_string(), "—")
        };

        builder.push_record([
            &commit_short,
            &snapshot.branch,
            &date,
            &size,
            &delta_str,
            trend,
        ]);

        prev_size = Some(snapshot.total_size);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()));

    println!("{}", table);
}
