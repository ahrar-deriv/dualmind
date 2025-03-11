//! API server implementation

use axum::{
    Router,
    routing::{get, options, post},
    Json,
    response::IntoResponse,
};
use reqwest::Client;
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::signal;

use crate::api::handlers::{
    chat_completions, clear_session, get_model, options_handler
};
use crate::config::Config;
use crate::middleware;
use crate::models::Message;

pub struct ChatSession {
    pub messages: Vec<Message>,
    pub last_active: Instant,
}

pub struct AppState {
    pub client: Client,
    pub sessions: Arc<Mutex<HashMap<String, ChatSession>>>,
    pub last_cleanup: Arc<Mutex<Instant>>,
    pub config: Config,
}

const SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 30); // 30 minutes

/// Start the API server
pub async fn start(
    client: Client,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState {
        client,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        last_cleanup: Arc::new(Mutex::new(Instant::now())),
        config,
    });

    // Build our application with routes
    let app = Router::new()
        .route(
            "/",
            get(|| async { "DualMind API Server - OpenAI Compatible" }),
        )
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/chat/completions", options(options_handler))
        .route("/v1/models", get(list_models))
        .route("/v1/models/:model", get(get_model))
        .route("/v1/sessions/:session_id/clear", post(clear_session))
        .layer(axum::middleware::from_fn(middleware::log_request))
        .with_state(state);

    // Run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 API Server running on {addr}");
    println!("Example curl:");
    println!("curl http://localhost:3000/v1/chat/completions \\");
    println!("  -H \"Content-Type: application/json\" \\");
    println!("  -H \"Authorization: Bearer YOUR_API_KEY\" \\");
    println!(
        "  -d '{{\"model\": \"dualmind\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello world\"}}], \"stream\": false}}'"
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Helper function to clean up old sessions
pub fn cleanup_old_sessions(sessions: &mut HashMap<String, ChatSession>) {
    let now = Instant::now();
    let expired_sessions: Vec<String> = sessions
        .iter()
        .filter(|(_, session)| now.duration_since(session.last_active) > SESSION_TIMEOUT)
        .map(|(id, _)| id.clone())
        .collect();

    for id in expired_sessions {
        sessions.remove(&id);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Shutting down gracefully...");
}

/// List available models
pub async fn list_models() -> impl IntoResponse {
    let models = vec![
        json!({
            "id": "dualmind",
            "object": "model",
            "created": 1677610602,
            "owned_by": "organization-owner"
        })
    ];
    
    Json(json!({
        "object": "list",
        "data": models
    }))
} 