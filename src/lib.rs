//! DualMind - A hybrid AI chat application

pub mod api;
pub mod cli;
pub mod config;
pub mod core;
pub mod middleware;
pub mod models;
pub mod streaming;
pub mod utils;

use reqwest::Client;
use std::env;

/// Run the application with the given configuration
#[tokio::main]
pub async fn run() -> Result<(), String> {
    // Load configuration
    let config = config::load_from_args().map_err(|e| format!("Configuration error: {}", e))?;
    
    // Create HTTP client
    let client = Client::new();
    
    // Choose mode based on command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--api" => {
                // Run API server
                api::server::start(client, config).await.map_err(|e| e.to_string())?;
            }
            "--test-client" => {
                // Run test client
                api::client::test().await.map_err(|e| e.to_string())?;
            }
            _ => {
                // Run terminal interface
                cli::terminal::start(client, config).await.map_err(|e| e.to_string())?;
            }
        }
    } else {
        // Run terminal interface by default
        cli::terminal::start(client, config).await.map_err(|e| e.to_string())?;
    }

    Ok(())
} 