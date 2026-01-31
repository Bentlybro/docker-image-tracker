mod analyze;
mod diff;
mod docker;
mod format;
mod history;
mod models;
mod track;

use anyhow::Result;
use clap::{Parser, Subcommand};

use analyze::{analyze_image, OutputFormat};
use diff::diff_images;
use history::show_history;
use track::track_image;

#[derive(Parser)]
#[command(name = "dit")]
#[command(author = "Bentlybro <github@bentlybro.com>")]
#[command(version = "0.1.0")]
#[command(about = "Docker Image Tracker - Track Docker image sizes over time", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a Docker image and show layer breakdown
    Analyze {
        /// Docker image to analyze (e.g., myapp:latest)
        image: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Track an image snapshot with git context
    Track {
        /// Docker image to track (e.g., myapp:latest)
        image: String,
    },

    /// Compare two image snapshots
    Diff {
        /// Docker image to compare (e.g., myapp:latest)
        image: String,

        /// First commit SHA (optional)
        commit_a: Option<String>,

        /// Second commit SHA (optional)
        commit_b: Option<String>,

        /// Compare against latest snapshot from this branch
        #[arg(long)]
        base: Option<String>,
    },

    /// Show image size history
    History {
        /// Docker image to show history for (e.g., myapp:latest)
        image: String,

        /// Limit to last N snapshots
        #[arg(long)]
        last: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { image, format } => {
            analyze_image(&image, format).await?;
        }
        Commands::Track { image } => {
            track_image(&image).await?;
        }
        Commands::Diff {
            image,
            commit_a,
            commit_b,
            base,
        } => {
            diff_images(&image, commit_a, commit_b, base).await?;
        }
        Commands::History { image, last } => {
            show_history(&image, last).await?;
        }
    }

    Ok(())
}
