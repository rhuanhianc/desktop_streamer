use anyhow::Result;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, env};
use tokio::sync::Mutex;
use tokio::signal;
use tracing::{info, warn, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

mod config;
mod media;
mod server;
mod webrtc_handler;
mod virtual_audio;
mod audio_detection;

use config::AppState;
use media::detect_hardware_capabilities;
use server::{run_server, check_portal_availability};
use virtual_audio::VirtualAudioDevice;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing for logging
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    // 2. Initialize GStreamer
    gstreamer::init()?;
    info!("GStreamer initialized successfully");

    // 3. Detect system capabilities
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "x11".to_string());
    let has_portal = check_portal_availability().await;
    
    info!("Session type: {}, Portal available: {}", session_type, has_portal);

    // 4. Detect hardware capabilities at startup
    let hw_info = detect_hardware_capabilities();
    info!("Detected Hardware Capabilities: {:?}", hw_info);

    // 4. Clean up any orphan virtual audio devices first
    if let Err(e) = VirtualAudioDevice::remove_all_orphan_devices() {
        warn!("Failed to clean up orphan virtual audio devices: {}", e);
    }

    // 5. Initialize virtual audio device
    let mut virtual_audio = VirtualAudioDevice::new();
    if let Err(e) = virtual_audio.create_virtual_devices() {
        warn!("Failed to create virtual audio devices: {}", e);
    } else {
        info!("Virtual audio devices created successfully");
    }

    // 6. Create the shared application state
    let app_state = Arc::new(AppState {
        peers: Mutex::new(HashMap::new()),
        hw_info: Arc::new(hw_info),
        session_type,
        has_portal,
        virtual_audio: Mutex::new(Some(virtual_audio)),
    });

    // Clone app_state for signal handler
    let app_state_cleanup = app_state.clone();

    // 6. Set up signal handlers for graceful shutdown
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");
            
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down gracefully...");
                    cleanup_virtual_devices(&app_state_cleanup).await;
                    std::process::exit(0);
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl+C), shutting down gracefully...");
                    cleanup_virtual_devices(&app_state_cleanup).await;
                    std::process::exit(0);
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received Ctrl+C, shutting down gracefully...");
                    cleanup_virtual_devices(&app_state_cleanup).await;
                    std::process::exit(0);
                }
                Err(err) => {
                    warn!("Failed to listen for shutdown signal: {}", err);
                }
            }
        }
    });

    // 7. Start the Axum server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("🚀 High-Performance Desktop Streamer Enhanced v2.0 starting on http://0.0.0.0:3000");
    
    run_server(app_state, addr).await?;

    Ok(())
}

async fn cleanup_virtual_devices(app_state: &Arc<AppState>) {
    info!("Cleaning up virtual audio devices...");
    if let Ok(mut virtual_audio_guard) = app_state.virtual_audio.try_lock() {
        if let Some(virtual_device) = virtual_audio_guard.take() {
            if let Err(e) = virtual_device.remove_virtual_devices() {
                warn!("Failed to cleanup virtual audio devices: {}", e);
            } else {
                info!("Virtual audio devices cleaned up successfully");
            }
        }
    } else {
        warn!("Could not acquire lock to cleanup virtual audio devices");
    }
}
