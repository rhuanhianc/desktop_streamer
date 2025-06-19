use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::{net::SocketAddr, sync::Arc, process::Command};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use uuid::Uuid;

use crate::{config::{AppState, MonitorInfo}, webrtc_handler, audio_detection::{detect_audio_capabilities, AudioDevice}};

pub async fn run_server(app_state: Arc<AppState>, addr: SocketAddr) -> Result<()> {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(websocket_handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(app_state)
        .layer(cors);

    axum_server::bind(addr).serve(app.into_make_service()).await?;
    Ok(())
}

async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("static/index.html").await {
        Ok(content) => axum::response::Html(content),
        Err(_) => {
            // Fallback to embedded content if file doesn't exist
            let default_content = r#"
<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Desktop Streamer Enhanced</title>
</head>
<body>
    <h1>Desktop Streamer Enhanced v2.0</h1>
    <p>O arquivo index.html não foi encontrado no diretório static/</p>
    <p>Por favor, certifique-se de que o arquivo static/index.html existe.</p>
</body>
</html>
            "#;
            axum::response::Html(default_content.to_string())
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let peer_id = Uuid::new_v4();
    info!("New WebSocket connection attempt with Peer ID: {}", peer_id);
    
    ws.on_upgrade(move |socket| {
        info!("WebSocket upgrade successful for Peer ID: {}", peer_id);
        webrtc_handler::handle_connection(socket, peer_id, state)
    })
}

// Detect monitors using multiple methods
pub async fn detect_monitors(session_type: &str, has_portal: bool) -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    
    if session_type == "wayland" && has_portal {
        monitors.push(MonitorInfo {
            id: "wayland-portal".to_string(),
            name: "Tela (via Portal)".to_string(),
            primary: true,
            resolution: "Dinâmica".to_string(),
            source_type: "screen".to_string(),
        });
    } else if session_type == "x11" {
        if let Ok(output) = Command::new("xrandr").arg("--query").output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut monitor_idx = 0;
            
            for line in output_str.lines() {
                if line.contains(" connected") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name = parts[0].to_string();
                        let is_primary = line.contains("primary");
                        let resolution = parts.iter()
                            .find(|p| p.contains("x") && p.chars().any(|c| c.is_numeric()))
                            .unwrap_or(&"1920x1080")
                            .split('+').next()
                            .unwrap_or("1920x1080")
                            .to_string();
                        
                        monitors.push(MonitorInfo {
                            id: format!("x11-{}", monitor_idx),
                            name: format!("{} (X11)", name),
                            primary: is_primary,
                            resolution,
                            source_type: "screen".to_string(),
                        });
                        monitor_idx += 1;
                    }
                }
            }
        }
    }
    
    // Detect cameras
    for i in 0..5 {
        let device = format!("/dev/video{}", i);
        if std::path::Path::new(&device).exists() {
            let name = if let Ok(output) = Command::new("v4l2-ctl")
                .args(&["--device", &device, "--info"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.lines()
                    .find(|l| l.contains("Card type"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| format!("Câmera {}", i))
            } else {
                format!("Câmera {}", i)
            };
            
            monitors.push(MonitorInfo {
                id: format!("camera-{}", i),
                name,
                primary: i == 0,
                resolution: "1280x720".to_string(),
                source_type: "camera".to_string(),
            });
        }
    }
    
    // Add fallback if no monitors detected
    if monitors.is_empty() {
        monitors.push(MonitorInfo {
            id: "fallback".to_string(),
            name: "Tela Principal".to_string(),
            primary: true,
            resolution: "1920x1080".to_string(),
            source_type: "screen".to_string(),
        });
    }
    
    monitors
}

pub async fn check_portal_availability() -> bool {
    // Check if xdg-desktop-portal is running
    if let Ok(output) = Command::new("busctl")
        .args(&["--user", "list"])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        return output_str.contains("org.freedesktop.portal.Desktop");
    }
    false
}

// Detect audio devices
pub async fn detect_audio_devices() -> Vec<AudioDevice> {
    let audio_info = detect_audio_capabilities();
    let mut devices = Vec::new();
    
    // Add microphone devices
    for mic in audio_info.microphone_devices {
        devices.push(mic);
    }
    
    // Add monitor devices (system audio)
    for monitor in audio_info.monitor_devices {
        devices.push(monitor);
    }
    
    devices
}
