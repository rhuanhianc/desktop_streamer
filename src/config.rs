use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use gstreamer as gst;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;
use serde::{Deserialize, Serialize};

use crate::media::HardwareInfo;
use crate::virtual_audio::VirtualAudioDevice;

// Unique identifier for each peer connection
pub type PeerId = Uuid;

// Global state shared across all connections and tasks
pub struct AppState {
    pub peers: Mutex<HashMap<PeerId, Arc<PeerState>>>,
    pub hw_info: Arc<HardwareInfo>,
    pub session_type: String,
    pub has_portal: bool,
    pub virtual_audio: Mutex<Option<VirtualAudioDevice>>,
}

// State specific to a single connected peer
pub struct PeerState {
    pub peer_connection: Arc<RTCPeerConnection>,
    pub pipeline: gst::Pipeline,
    // Channel to send signaling messages back to the client's WebSocket
    pub ws_sender: Mutex<Option<SplitSink<WebSocket, Message>>>,
}

// Configuration for creating a new media pipeline
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineConfig {
    pub source_type: String, // e.g., "x11-0", "wayland-portal", "camera-0", "test"
    pub audio_source: Option<String>, // e.g., "microphone", "system", "application"
    pub enable_audio: bool,
    pub enable_microphone_input: bool, // Para receber áudio do cliente
    pub audio_bitrate: u32, // 96000, 128000, 256000
    pub audio_sample_rate: u32, // 48000, 44100
    pub resolution: (u32, u32),
    pub framerate: u32,
    pub use_hardware_encoding: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            source_type: "x11-0".to_string(),
            audio_source: None,
            enable_audio: false,
            enable_microphone_input: false,
            audio_bitrate: 128000,
            audio_sample_rate: 48000,
            resolution: (1920, 1080),
            framerate: 30,
            use_hardware_encoding: true,
        }
    }
}

// WebSocket signaling messages, serialized to/from JSON
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SignalMessage {
    Offer { sdp: String, config: PipelineConfig },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    Error { message: String },
    Monitors { monitors: Vec<MonitorInfo> },
    AudioDevices { devices: Vec<AudioDeviceInfo> },
    PortalRequest { request_id: String },
    PortalResponse { request_id: String, node_id: Option<u32> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub resolution: String,
    #[serde(rename = "type")]
    pub source_type: String, // "screen" or "camera"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String, // "microphone", "system", "application"
    pub description: String,
}
