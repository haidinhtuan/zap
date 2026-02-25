use zap::config;

fn main() {
    // Initialize logging (hold the guard for the lifetime of the process).
    let _log_guard = zap::logging::init_logging(&config::log_dir())
        .expect("failed to initialize logging");

    tracing::info!("zap v0.1.0 starting");
    println!("zap v0.1.0");
}
