use clap::Parser;
use mcp_filesystem_rs::config::{self, Cli};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("mcp_filesystem_rs={}", cli.log_level)));
    tracing_subscriber::fmt().with_env_filter(env_filter).with_writer(std::io::stderr).init();

    let app_config = match config::load_config(cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    let roots = app_config.sandbox.list_allowed_directories();
    info!("roots: {:?}", roots);
    info!(
        "limits: maxReadBytes={}, maxSearchResults={}, maxDirectoryEntries={}",
        app_config.limits.max_read_bytes,
        app_config.limits.max_search_results,
        app_config.limits.max_directory_entries
    );

    let app = Arc::new(app_config);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(mcp_filesystem_rs::server::run_stdio(app))?;

    Ok(())
}
