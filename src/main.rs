mod analyze;
mod analyze_all;
mod compose;
mod diff;
mod docker;
mod format;
mod history;
mod models;
mod summary;
mod track;
mod track_all;

use anyhow::Result;
use clap::{Parser, Subcommand};

use analyze::{analyze_image, OutputFormat};
use analyze_all::analyze_all_images;
use compose::{compose_analyze, compose_history, compose_track};
use diff::diff_images;
use history::show_history;
use summary::show_summary;
use track::track_image;
use track_all::track_all_images;

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

    /// Analyze all local Docker images at once
    AnalyzeAll {
        /// Filter images by name (substring match)
        #[arg(long)]
        filter: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Track an image snapshot with git context
    Track {
        /// Docker image to track (e.g., myapp:latest)
        image: String,
    },

    /// Track all local Docker images at once
    TrackAll {
        /// Filter images by name (substring match)
        #[arg(long)]
        filter: Option<String>,
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

    /// Docker Compose support
    #[command(subcommand)]
    Compose(ComposeCommands),

    /// Show summary dashboard of all tracked images
    Summary,
}

#[derive(Subcommand)]
enum ComposeCommands {
    /// Analyze all compose-built images
    Analyze {
        /// Path to docker-compose file
        #[arg(long)]
        file: Option<String>,
    },

    /// Track all compose-built images
    Track {
        /// Path to docker-compose file
        #[arg(long)]
        file: Option<String>,
    },

    /// Show history for all compose images
    History {
        /// Path to docker-compose file
        #[arg(long)]
        file: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { image, format } => {
            analyze_image(&image, format).await?;
        }
        Commands::AnalyzeAll { filter, format } => {
            analyze_all_images(filter.as_deref(), format).await?;
        }
        Commands::Track { image } => {
            track_image(&image).await?;
        }
        Commands::TrackAll { filter } => {
            track_all_images(filter.as_deref()).await?;
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
        Commands::Compose(compose_cmd) => match compose_cmd {
            ComposeCommands::Analyze { file } => {
                compose_analyze(file.as_deref()).await?;
            }
            ComposeCommands::Track { file } => {
                compose_track(file.as_deref()).await?;
            }
            ComposeCommands::History { file } => {
                compose_history(file.as_deref()).await?;
            }
        },
        Commands::Summary => {
            show_summary().await?;
        }
    }

    Ok(())
}
