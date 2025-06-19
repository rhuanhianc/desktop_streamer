
use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::StreamExt, SinkExt};
use gstreamer::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::{ice_candidate::{RTCIceCandidate, RTCIceCandidateInit}, ice_server::RTCIceServer},
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
    },
    track::track_local::TrackLocal,
    rtp_transceiver::rtp_receiver::RTCRtpReceiver,
};
use tokio::sync::mpsc;
use ashpd::desktop::screencast::{Screencast, SourceType, CursorMode};
use ashpd::enumflags2::BitFlags;

use crate::{
    config::{AppState, PeerState, SignalMessage, PipelineConfig, AudioDeviceInfo},
    media::PipelineFactory,
    server::{detect_monitors, detect_audio_devices},
};

pub async fn handle_connection(socket: WebSocket, peer_id: Uuid, state: Arc<AppState>) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));

    // Send initial monitor list
    let monitors = detect_monitors(&state.session_type, state.has_portal).await;
    let monitors_msg = SignalMessage::Monitors { monitors };
    if let Ok(msg_json) = serde_json::to_string(&monitors_msg) {
        let mut sender = ws_sender.lock().await;
        let _ = sender.send(Message::Text(msg_json.into())).await;
    }
    
    // Send audio devices list
    let audio_devices = detect_audio_devices().await;
    let audio_devices_info: Vec<AudioDeviceInfo> = audio_devices.into_iter().map(|device| {
        AudioDeviceInfo {
            id: device.name.clone(),
            name: device.description,
            device_type: match device.device_type.as_str() {
                "source" => "microphone".to_string(),
                "monitor" => "system".to_string(),
                _ => "application".to_string(),
            },
            description: device.name,
        }
    }).collect();
    
    let audio_msg = SignalMessage::AudioDevices { devices: audio_devices_info };
    if let Ok(msg_json) = serde_json::to_string(&audio_msg) {
        let mut sender = ws_sender.lock().await;
        let _ = sender.send(Message::Text(msg_json.into())).await;
    }

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            let signal: SignalMessage = match serde_json::from_str(&text) {
                Ok(s) => s,
                Err(e) => {
                    error!(%peer_id, "Failed to decode signal: {}", e);
                    let error_msg = SignalMessage::Error { 
                        message: format!("Erro ao decodificar mensagem: {}", e) 
                    };
                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        let mut sender = ws_sender.lock().await;
                        let _ = sender.send(Message::Text(json.into())).await;
                    }
                    continue;
                }
            };

            match signal {
                SignalMessage::Offer { sdp, config } => {
                    info!(%peer_id, "Received offer with config: {:?}", config);
                    info!(%peer_id, "Audio config: enable_audio={}, audio_source={:?}, audio_bitrate={}", 
                          config.enable_audio, config.audio_source, config.audio_bitrate);

                    // Clean up any existing peer state
                    if let Some(existing_peer) = state.peers.lock().await.remove(&peer_id) {
                        let _ = existing_peer.pipeline.set_state(gstreamer::State::Null);
                        let _ = existing_peer.peer_connection.close().await;
                    }

                    // Create ICE candidate channel
                    let (ice_tx, mut ice_rx) = mpsc::unbounded_channel::<String>();
                    let ws_sender_clone = ws_sender.clone();

                    // Spawn task to handle ICE candidates with websocket sender
                    let peer_id_clone = peer_id;
                    tokio::spawn(async move {
                        while let Some(candidate_json) = ice_rx.recv().await {
                            let msg = SignalMessage::IceCandidate { candidate: candidate_json };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let mut sender = ws_sender_clone.lock().await;
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    warn!(%peer_id_clone, "Failed to send ICE candidate, client disconnected.");
                                    break;
                                }
                            }
                        }
                    });

                    // Parse the client's offer first
                    let offer = match RTCSessionDescription::offer(sdp) {
                        Ok(offer) => offer,
                        Err(e) => {
                            error!(%peer_id, "Failed to parse client offer: {}", e);
                            continue;
                        }
                    };

                    match create_peer_connection_and_pipeline(
                        peer_id,
                        &state,
                        config,
                        ice_tx,
                        offer,
                    ).await {
                        Ok(answer_sdp) => {
                            // Send answer back to client
                            let answer_msg = SignalMessage::Answer { sdp: answer_sdp };
                            if let Ok(json) = serde_json::to_string(&answer_msg) {
                                let mut sender = ws_sender.lock().await;
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    error!(%peer_id, "Failed to send answer to client");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!(%peer_id, "Failed to create peer connection: {}", e);
                            let err_msg = SignalMessage::Error { 
                                message: format!("Erro ao criar conexão: {}", e) 
                            };
                            if let Ok(json) = serde_json::to_string(&err_msg) {
                                let mut sender = ws_sender.lock().await;
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                }
                SignalMessage::IceCandidate { candidate } => {
                    if let Some(peer_state) = state.peers.lock().await.get(&peer_id) {
                        debug!(%peer_id, "Received ICE candidate from client");
                        match serde_json::from_str::<RTCIceCandidateInit>(&candidate) {
                            Ok(candidate_init) => {
                                if let Err(e) = peer_state.peer_connection.add_ice_candidate(candidate_init).await {
                                    error!(%peer_id, "Failed to add ICE candidate: {}", e);
                                } else {
                                    debug!(%peer_id, "ICE candidate added successfully");
                                }
                            }
                            Err(e) => {
                                error!(%peer_id, "Failed to decode ICE candidate: {}", e);
                            }
                        }
                    } else {
                        warn!(%peer_id, "Received ICE candidate for non-existent peer.");
                    }
                }
                _ => {
                    debug!(%peer_id, "Received unhandled message type");
                }
            }
        }
    }

    // Cleanup when WebSocket closes
    info!("WebSocket connection closed for Peer ID: {}", peer_id);
    if let Some(peer_state) = state.peers.lock().await.remove(&peer_id) {
        let _ = peer_state.pipeline.set_state(gstreamer::State::Null);
        let _ = peer_state.peer_connection.close().await;
        info!("Cleaned up resources for Peer ID: {}", peer_id);
    }
}

async fn create_peer_connection_and_pipeline(
    peer_id: Uuid,
    state: &Arc<AppState>,
    config: PipelineConfig,
    ice_sender: mpsc::UnboundedSender<String>,
    offer: RTCSessionDescription,
) -> Result<String> {
    // Get PipeWire node ID if needed for Wayland portal
    let pipewire_node_id = if config.source_type == "wayland-portal" 
        && state.session_type == "wayland" 
        && state.has_portal {
        match get_wayland_pipewire_node_id().await {
            Ok(node_id) => {
                info!(%peer_id, "Successfully obtained PipeWire node ID: {}", node_id);
                Some(node_id)
            }
            Err(e) => {
                error!(%peer_id, "Failed to get PipeWire node ID: {}", e);
                return Err(anyhow!("Failed to get PipeWire node ID: {}", e));
            }
        }
    } else {
        None
    };

    // Create WebRTC peer connection
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;
    
    info!(%peer_id, "Registered default codecs with WebRTC MediaEngine");
    let api = APIBuilder::new().with_media_engine(m).build();

    let rtc_config = RTCConfiguration {
        ice_servers: vec![
            RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            }
        ],
        ..Default::default()
    };

    let pc = Arc::new(api.new_peer_connection(rtc_config).await?);

    // Create media pipeline and tracks
    let mut pipeline_factory = PipelineFactory::new();
    
    let (pipeline, tracks) = pipeline_factory.create_pipeline(
        &config, 
        &state.hw_info, 
        pipewire_node_id,
        &state.session_type,
        state.has_portal,
        &state
    ).await?;

    // Add tracks to peer connection
    for track in &tracks {
        pc.add_track(track.clone() as Arc<dyn TrackLocal + Send + Sync>).await?;
    }
    
    // Add incoming audio track handler if microphone input is enabled
    if config.enable_microphone_input {
        let app_state_clone = state.clone();
        pc.on_track(Box::new(move |track, receiver, _transceiver| {
            let track_kind = track.kind();
            let track_id = track.id();
            let app_state = app_state_clone.clone();
            
            info!("Received track from client: {} ({})", track_id, track_kind);
            
            if track_kind.to_string() == "audio" {
                // Handle incoming audio from client microphone
                tokio::spawn(async move {
                    info!("Setting up audio input pipeline for client microphone");
                    
                    if let Err(e) = setup_microphone_playback_pipeline(receiver, &app_state).await {
                        error!("Failed to setup microphone playback pipeline: {}", e);
                    }
                });
            }
            
            Box::pin(async {})
        }));
    }
    
    // Set up ICE candidate handler
    let ice_sender_clone = ice_sender.clone();
    pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
        let ice_sender = ice_sender_clone.clone();
        Box::pin(async move {
            if let Some(candidate) = c {
                if let Ok(init) = candidate.to_json() {
                    if let Ok(candidate_json) = serde_json::to_string(&init) {
                        let _ = ice_sender.send(candidate_json);
                    }
                }
            }
        })
    }));

    // Set the remote description from the client's offer first
    pc.set_remote_description(offer).await?;

    // Create answer (this needs to be done after setting remote description)
    let answer = pc.create_answer(None).await?;
    let answer_sdp = answer.sdp.clone();
    pc.set_local_description(answer).await?;

    // Store peer state
    let peer_state = Arc::new(PeerState {
        peer_connection: pc,
        pipeline,
        ws_sender: tokio::sync::Mutex::new(None), // Will be set later if needed
    });
    state.peers.lock().await.insert(peer_id, peer_state);

    Ok(answer_sdp)
}

async fn get_wayland_pipewire_node_id() -> Result<u32> {
    use ashpd::desktop::PersistMode;
    
    let screencast = Screencast::new().await
        .map_err(|e| anyhow!("Failed to create Screencast: {}", e))?;

    info!("Requesting screen cast session from portal (ashpd)...");
    let session = screencast.create_session().await
        .map_err(|e| anyhow!("Failed to create screen cast session object: {}", e))?;
    
    // First select sources - this is mandatory before calling start
    info!("Selecting screen sources...");
    let sources = BitFlags::from(SourceType::Monitor);
    screencast
        .select_sources(
            &session,
            CursorMode::Hidden,
            sources,
            false, // multiple
            None,  // restore_token
            PersistMode::DoNot,  // persist_mode
        )
        .await
        .map_err(|e| anyhow!("Failed to select sources: {}", e))?;
    
    info!("Starting screen cast session (ashpd)... User interaction may be required.");
    let response = screencast.start(&session, None).await
        .map_err(|e| anyhow!("Failed to call Start method on Screencast: {}", e))?
        .response()
        .map_err(|e| anyhow!("Portal StartScreenCast request failed or was cancelled: {}", e))?;

    info!("Portal interaction completed. Fetching streams from response...");
    let streams_info = response.streams();

    info!("Portal session streams: {:?}", streams_info);

    if let Some(stream_detail) = streams_info.get(0) { 
        let node_id = stream_detail.pipe_wire_node_id();
        info!("Selected PipeWire node ID from portal: {}", node_id);
        Ok(node_id)
    } else {
        Err(anyhow!("No streams found in portal session response"))
    }
}

/// Configura pipeline para reproduzir áudio do microfone do cliente
async fn setup_microphone_playback_pipeline(
    _receiver: Arc<RTCRtpReceiver>,
    app_state: &Arc<AppState>,
) -> Result<()> {
    
    info!("Setting up microphone playback pipeline");
    
    // Obter sink de entrada do dispositivo virtual
    let sink_name = if let Ok(virtual_audio_guard) = app_state.virtual_audio.try_lock() {
        if let Some(virtual_device) = virtual_audio_guard.as_ref() {
            virtual_device.get_input_sink_name()
        } else {
            "@DEFAULT_SINK@".to_string()
        }
    } else {
        "@DEFAULT_SINK@".to_string()
    };
    
    info!("Will play client microphone audio to sink: {}", sink_name);
    
    // Criar pipeline GStreamer para reproduzir o áudio recebido
    let pipeline_str = format!(
        "udpsrc port=0 ! application/x-rtp,media=audio,payload=111,encoding-name=OPUS ! \
         rtpopusdepay ! opusdec ! audioconvert ! audioresample ! \
         audio/x-raw,rate=48000,channels=2,format=S16LE ! \
         volume volume=1.0 ! pulsesink device={} sync=false",
        sink_name
    );
    
    info!("Microphone playback pipeline: {}", pipeline_str);
    
    // TODO: Integrar com o receiver do WebRTC
    // Por enquanto, apenas logamos que o pipeline foi configurado
    info!("Microphone playback pipeline configured (integration pending)");
    
    Ok(())
}
