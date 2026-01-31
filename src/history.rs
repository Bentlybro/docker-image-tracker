use anyhow::{bail, Result};

use crate::format::print_history_table;
use crate::track::load_history;

pub async fn show_history(image: &str, last: Option<usize>) -> Result<()> {
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
    if let Some(n) = last {
        let start = image_history.len().saturating_sub(n);
        image_history = image_history[start..].to_vec();
    }

    // Display history
    print_history_table(&image_history);

    Ok(())
}
