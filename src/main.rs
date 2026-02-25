use clap::Parser;
use std::time::Duration;
use tokio::sync::mpsc;
use zap::app::App;
use zap::config;
use zap::event::EventHandler;

/// Zap - A terminal messenger client using Matrix bridges.
#[derive(Parser, Debug)]
#[command(name = "zap", version, about)]
struct Cli {
    /// Path to a custom config file
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    // Set up logging.
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    let _log_guard = zap::logging::init_logging(&config::log_dir())
        .expect("failed to initialize logging");

    tracing::info!("zap v0.1.0 starting");

    // Ensure config files exist on disk.
    config::ensure_config_files()?;

    // Set up app state and event handler.
    let mut app = App::new();
    let mut events = EventHandler::new(
        Duration::from_millis(250),  // tick rate
        Duration::from_millis(16),   // ~60 fps render rate
    );

    // Placeholder Matrix channel.
    let (_matrix_tx, mut matrix_rx) = mpsc::unbounded_channel::<String>();

    // Initialize terminal.
    let mut terminal = ratatui::init();

    // Run the main event loop.
    let result = zap::run_app(&mut terminal, &mut app, &mut events, &mut matrix_rx).await;

    // Restore terminal.
    ratatui::restore();

    result
}
