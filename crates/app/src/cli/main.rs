//! Troubadour CLI Application

use clap::Parser;

#[derive(Parser)]
#[command(name = "troubadour")]
#[command(about = "A modern virtual audio mixer", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    tracing::info!("🎼 Troubadour starting...");

    Ok(())
}
