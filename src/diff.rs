use anyhow::{bail, Context, Result};
use std::collections::HashMap;

use crate::format::print_diff_table;
use crate::models::{ImageSnapshot, LayerChange, SizeDiff};
use crate::track::load_history;

pub async fn diff_images(
    image: &str,
    commit_a: Option<String>,
    commit_b: Option<String>,
    base_branch: Option<String>,
) -> Result<()> {
    let history = load_history()?;

    if history.is_empty() {
        bail!("No history found. Run 'dit track' first.");
    }

    // Filter history for the specified image
    let image_history: Vec<_> = history
        .iter()
        .filter(|s| s.image == image)
        .collect();

    if image_history.is_empty() {
        bail!("No history found for image '{}'", image);
    }

    // Determine which snapshots to compare
    let (before, after): (&ImageSnapshot, &ImageSnapshot) = if let (Some(a), Some(b)) = (commit_a, commit_b) {
        // Compare two specific commits
        let snap_a = find_snapshot_by_commit(&image_history, &a)?;
        let snap_b = find_snapshot_by_commit(&image_history, &b)?;
        (snap_a, snap_b)
    } else if let Some(base) = base_branch {
        // Compare against base branch
        let base_snap = find_latest_snapshot_by_branch(&image_history, &base)?;
        let current_snap = *image_history.last().unwrap();
        (base_snap, current_snap)
    } else {
        // Compare last two snapshots
        if image_history.len() < 2 {
            bail!("Not enough history to compare. Need at least 2 snapshots.");
        }
        let before = image_history[image_history.len() - 2];
        let after = image_history[image_history.len() - 1];
        (before, after)
    };

    // Compute diff
    let diff = compute_diff((*before).clone(), (*after).clone());

    // Display diff
    print_diff_table(&diff);

    Ok(())
}

fn find_snapshot_by_commit<'a>(
    history: &[&'a ImageSnapshot],
    commit: &str,
) -> Result<&'a ImageSnapshot> {
    history
        .iter()
        .find(|s| s.commit_sha.starts_with(commit))
        .copied()
        .context(format!("No snapshot found for commit '{}'", commit))
}

fn find_latest_snapshot_by_branch<'a>(
    history: &[&'a ImageSnapshot],
    branch: &str,
) -> Result<&'a ImageSnapshot> {
    history
        .iter()
        .rev()
        .find(|s| s.branch == branch)
        .copied()
        .context(format!("No snapshot found for branch '{}'", branch))
}

fn compute_diff(before: ImageSnapshot, after: ImageSnapshot) -> SizeDiff {
    let total_delta = after.total_size as i64 - before.total_size as i64;

    // Build maps of layers by digest for quick lookup
    let before_layers: HashMap<_, _> = before
        .layers
        .iter()
        .map(|l| (l.digest.clone(), l.clone()))
        .collect();

    let after_layers: HashMap<_, _> = after
        .layers
        .iter()
        .map(|l| (l.digest.clone(), l.clone()))
        .collect();

    let mut layer_changes = Vec::new();

    // Find modified/unchanged/removed layers
    for layer in &before.layers {
        if let Some(after_layer) = after_layers.get(&layer.digest) {
            if layer.size == after_layer.size {
                layer_changes.push(LayerChange::Unchanged(layer.clone()));
            } else {
                layer_changes.push(LayerChange::Modified {
                    before: layer.clone(),
                    after: after_layer.clone(),
                });
            }
        } else {
            layer_changes.push(LayerChange::Removed(layer.clone()));
        }
    }

    // Find added layers
    for layer in &after.layers {
        if !before_layers.contains_key(&layer.digest) {
            layer_changes.push(LayerChange::Added(layer.clone()));
        }
    }

    SizeDiff {
        before,
        after,
        total_delta,
        layer_changes,
    }
}
