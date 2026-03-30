pub mod events;
pub mod routes;
pub mod websocket;

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use tokio::sync::broadcast;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

use crate::tui::server::DaemonState;
use events::BmuxEvent;
use routes::{
    AppState, get_adversarial_history, get_adversarial_status, get_agents, get_audit, get_context,
    get_metrics, get_pane_output, get_sessions, get_tasks, get_ws, post_adversarial_start,
    post_adversarial_stop, post_task,
};

/// Start the Axum HTTP + WebSocket server.
///
/// Binds to `0.0.0.0:{port}`. If the port is already in use (e.g., another
/// daemon is running), logs a warning and returns without error.
pub async fn start_web_server(
    daemon: Arc<DaemonState>,
    port: u16,
    events_tx: broadcast::Sender<Arc<BmuxEvent>>,
) -> Result<()> {
    let state = AppState {
        daemon,
        events_tx,
    };

    // CORS: allow localhost origins (Next.js dev server + production build)
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse::<HeaderValue>()?,
            "http://localhost:7432".parse::<HeaderValue>()?,
            format!("http://localhost:{port}").parse::<HeaderValue>()?,
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    // Static file directory for Next.js build output (Story 1.4)
    // Looks for `web-dist` relative to the current working directory,
    // then falls back to the same directory as the binary.
    let static_dir = {
        let cwd_path = std::path::PathBuf::from("web-dist");
        if cwd_path.exists() {
            cwd_path
        } else if let Ok(exe) = std::env::current_exe() {
            exe.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("web-dist")
        } else {
            std::path::PathBuf::from("web-dist")
        }
    };

    let router = Router::new()
        // REST API
        .route("/api/sessions", get(get_sessions))
        .route("/api/agents", get(get_agents))
        .route("/api/context", get(get_context))
        .route("/api/tasks", get(get_tasks).post(post_task))
        .route("/api/metrics", get(get_metrics))
        .route("/api/pane-output", get(get_pane_output))
        .route("/api/audit", get(get_audit))
        // Adversarial harness (Stories 2.1–2.5)
        .route(
            "/api/adversarial/start",
            axum::routing::post(post_adversarial_start),
        )
        .route(
            "/api/adversarial/stop",
            axum::routing::post(post_adversarial_stop),
        )
        .route("/api/adversarial/status", get(get_adversarial_status))
        .route("/api/adversarial/history", get(get_adversarial_history))
        // WebSocket
        .route("/ws", get(get_ws))
        // Static files (Next.js build) — served from /
        .fallback_service(ServeDir::new(&static_dir).append_index_html_on_directories(true))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Web server could not bind to {addr}: {e}");
            return Ok(());
        }
    };

    tracing::info!("Web server listening on http://{addr}");
    axum::serve(listener, router).await?;
    Ok(())
}
