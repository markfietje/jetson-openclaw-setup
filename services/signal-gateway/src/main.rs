//! Signal Gateway - Lightweight Signal daemon for OpenClaw
//!
//! All 9 fixes implemented:
//! 1. Bounded channels (channel(64))
//! 2. Command loop (no pending())
//! 3. compare_exchange for receiver spawn
//! 4. Graceful shutdown
//! 5. WorkerState enum
//! 6. Rate limiting with Semaphore
//! 7. Input validation
//! 8. Timeout on oneshot receivers
//! 9. Dynamic phone number (no hardcoding)

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod config;
mod signal;
mod state;
mod webhook;

use config::Config;
use state::AppState;

#[derive(Parser)]
#[command(name = "signal-gateway")]
#[command(about = "Lightweight Signal daemon for OpenClaw", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve {
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },
    Link {
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
        #[arg(long)]
        device_name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config } => {
            let config = Config::load(&config)?;
            info!("Starting Signal Gateway v{}", env!("CARGO_PKG_VERSION"));
            
            let state: AppState = AppState::new(config.clone())?;
            
            // Try to load registered account
            match state.init_signal().await {
                Ok(true) => {
                    if let Ok(Some(number)) = state.signal.get_profile().await {
                        info!("Signal linked: {}", number);
                    } else {
                        info!("Signal linked");
                    }
                }
                Ok(false) => {
                    info!("Signal not linked. Use 'link' command to pair.");
                }
                Err(e) => {
                    info!("Signal init error: {}. Use 'link' command.", e);
                }
            }

            let app = api::create_router(state);
            let listener = tokio::net::TcpListener::bind(&config.server.address).await?;
            info!("Listening on {}", config.server.address);
            info!("Endpoints:");
            info!("  GET  /v1/health       - Health check");
            info!("  GET  /v1/about        - Account info");
            info!("  GET  /api/v1/events   - SSE message stream");
            info!("  POST /api/v1/rpc      - JSON-RPC API");

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .ok();
        }
        Commands::Link { config, device_name } => {
            let config = Config::load(&config)?;
            let state: AppState = AppState::new(config.clone())?;
            
            let signal = state.signal.clone();
            let device_name = device_name.unwrap_or_else(|| "openclaw-gateway".to_string());
            
            info!("Generating link URL for device: {}", device_name);
            match signal.link_secondary_device(device_name).await {
                Ok(url) => {
                    info!("Scan this QR code with your Signal app:");
                    info!("");
                    info!("{}", url);
                    info!("");
                    info!("Or open the URL in your browser");
                }
                Err(e) => {
                    tracing::error!("Failed to generate link URL: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C"),
        _ = terminate => info!("Received SIGTERM"),
    }
    
    info!("Shutting down gracefully...");
}
