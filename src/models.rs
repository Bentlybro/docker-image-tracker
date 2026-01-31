use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSnapshot {
    // Identity
    pub image: String,
    pub tag: Option<String>,
    pub digest: Option<String>,

    // Git context
    pub commit_sha: String,
    pub branch: String,
    pub commit_message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,

    // Metrics
    pub total_size: u64,
    pub layer_count: usize,
    pub layers: Vec<LayerInfo>,

    // Metadata
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub digest: String,
    pub size: u64,
    pub command: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug)]
pub struct SizeDiff {
    pub before: ImageSnapshot,
    pub after: ImageSnapshot,
    pub total_delta: i64,
    pub layer_changes: Vec<LayerChange>,
}

#[derive(Debug, Clone)]
pub enum LayerChange {
    Added(LayerInfo),
    Removed(LayerInfo),
    Modified {
        before: LayerInfo,
        after: LayerInfo,
    },
    Unchanged(LayerInfo),
}

impl LayerChange {
    pub fn size_delta(&self) -> i64 {
        match self {
            LayerChange::Added(layer) => layer.size as i64,
            LayerChange::Removed(layer) => -(layer.size as i64),
            LayerChange::Modified { before, after } => after.size as i64 - before.size as i64,
            LayerChange::Unchanged(_) => 0,
        }
    }

    pub fn layer(&self) -> &LayerInfo {
        match self {
            LayerChange::Added(layer) => layer,
            LayerChange::Removed(layer) => layer,
            LayerChange::Modified { after, .. } => after,
            LayerChange::Unchanged(layer) => layer,
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            LayerChange::Added(_) => "added",
            LayerChange::Removed(_) => "removed",
            LayerChange::Modified { .. } => "modified",
            LayerChange::Unchanged(_) => "unchanged",
        }
    }
}
