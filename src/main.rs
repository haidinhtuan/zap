use clap::Parser;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;
use zap::app::App;
use zap::config;
use zap::event::EventHandler;
use zap::input::KeymapManager;
use zap::matrix::sync::MatrixEvent;
use zap::store::LocalStore;

/// Zap - A terminal messenger client using Matrix bridges.
#[derive(Parser, Debug)]
#[command(name = "zap", version, about)]
struct Cli {
    /// Path to a custom config directory
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run in offline mode (no Matrix connection)
    #[arg(long)]
    offline: bool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let paths = config::resolve_paths(cli.config.as_deref().map(Path::new));

    // Set up logging.
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    let _log_guard = zap::logging::init_logging(&paths.log_dir)
        .expect("failed to initialize logging");

    tracing::info!("zap v0.1.0 starting");

    // Ensure config files exist and load them.
    config::ensure_config_files_in(&paths)?;

    let config_dir = &paths.config_dir;
    let app_config = config::parse_config(
        &std::fs::read_to_string(config_dir.join("config.toml"))?,
    )?;
    let theme_config = config::parse_theme(
        &std::fs::read_to_string(config_dir.join("theme.toml"))?,
    )?;
    let keymap_config = config::parse_keymap(
        &std::fs::read_to_string(config_dir.join("keymap.toml"))?,
    )?;

    tracing::info!(
        "Config loaded: homeserver={}, theme={}",
        app_config.matrix.homeserver,
        app_config.ui.theme
    );

    // Build keybinding manager from config.
    let mut keymap = KeymapManager::from_config(&keymap_config);

    // Set up Matrix connection or offline placeholder.
    let (matrix_client, mut matrix_rx) = if !cli.offline && !app_config.matrix.username.is_empty() {
        tracing::info!("Connecting to Matrix at {}", app_config.matrix.homeserver);

        match zap::matrix::client::create_client(
            &app_config.matrix.homeserver,
            &paths.data_dir,
        )
        .await
        {
            Ok(client) => {
                // Login (restores session or prompts for password).
                if let Err(e) = zap::matrix::login::login(&client, &app_config.matrix.username, &paths.data_dir).await {
                    tracing::warn!("Matrix login failed: {}. Running in offline mode.", e);
                    let (_tx, rx) = mpsc::unbounded_channel::<MatrixEvent>();
                    (None, rx)
                } else {
                    // Start background sync.
                    let rx = zap::matrix::sync::start_sync(client.clone());
                    (Some(client), rx)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create Matrix client: {}. Running in offline mode.", e);
                let (_tx, rx) = mpsc::unbounded_channel::<MatrixEvent>();
                (None, rx)
            }
        }
    } else {
        if cli.offline {
            tracing::info!("Running in offline mode (--offline flag)");
        } else {
            tracing::info!("No Matrix username configured. Running in offline mode.");
        }
        let (_tx, rx) = mpsc::unbounded_channel::<MatrixEvent>();
        (None, rx)
    };

    // Open local storage for drafts/preferences.
    let db_path = paths.data_dir.join("zap.db");
    let store = match LocalStore::open(&db_path) {
        Ok(store) => {
            tracing::info!("Local storage opened at {:?}", db_path);
            Some(store)
        }
        Err(e) => {
            tracing::warn!("Failed to open local storage: {}. Drafts will not be saved.", e);
            None
        }
    };

    // Set up app state and event handler.
    let mut app = App::new();

    // Apply config to app state.
    app.theme = Some(theme_config);
    app.room_list_width = app_config.ui.room_list_width;
    app.show_help_bar = app_config.ui.show_help_bar;
    app.send_read_receipts = app_config.behavior.send_read_receipts;
    app.timestamp_format = app_config.ui.timestamp_format.clone();
    app.vim_mode = app_config.behavior.vim_mode;

    if let Some(store) = store.as_ref() {
        if let Ok(Some(vigo_enabled)) = store.load_preference("vigo_enabled") {
            app.vigo_enabled = vigo_enabled == "true";
        }
    }

    // Store own user ID for is_own detection.
    if let Some(ref client) = matrix_client {
        app.own_user_id = Some(client.user_id().map(|id| id.to_string()).unwrap_or_default());
    }

    let mut events = EventHandler::new(
        Duration::from_millis(250), // tick rate
        Duration::from_millis(16),  // ~60 fps render rate
    );

    // Initialize terminal with mouse support and kitty keyboard protocol.
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    )
    .ok();
    let mut terminal = ratatui::init();

    // Run the main event loop.
    let result = zap::run_app(
        &mut terminal,
        &mut app,
        &mut events,
        &mut keymap,
        &mut matrix_rx,
        matrix_client.as_ref(),
        store.as_ref(),
    )
    .await;

    // Restore terminal.
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::PopKeyboardEnhancementFlags
    )
    .ok();
    ratatui::restore();

    tracing::info!("zap exiting");
    result
}
