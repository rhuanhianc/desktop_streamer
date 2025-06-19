use anyhow::{Result, anyhow};
use bytes::Bytes;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use webrtc::{
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{TrackLocalWriter, track_local_static_rtp::TrackLocalStaticRTP},
};

use crate::config::AppState;
use crate::config::PipelineConfig;

static NUM_CPUS: Lazy<String> = Lazy::new(|| num_cpus::get().to_string());

#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub has_nvenc: bool,
    pub has_nvh264enc_basic: bool, // nvh264enc without nvvidconv
    pub has_vaapi: bool,
    pub has_v4l2h264enc: bool,
    pub has_intel_gpu: bool,
}

pub fn detect_hardware_capabilities() -> HardwareInfo {
    info!("Detecting hardware acceleration capabilities...");

    // Check for individual NVIDIA elements
    let has_nvh264enc = gst::ElementFactory::find("nvh264enc").is_some();
    let has_nvvidconv = gst::ElementFactory::find("nvvidconv").is_some();
    let has_nvenc = has_nvh264enc && has_nvvidconv;
    
    // Check for VAAPI elements
    let has_vaapi = gst::ElementFactory::find("vaapivp8enc").is_some()
        || gst::ElementFactory::find("vaapih264enc").is_some();
    let has_v4l2h264enc = gst::ElementFactory::find("v4l2h264enc").is_some();
    let has_intel_gpu = std::path::Path::new("/dev/dri/renderD128").exists();

    info!("Hardware detection results:");
    info!("  NVIDIA NVENC elements:");
    info!("    nvh264enc: {}", has_nvh264enc);
    info!("    nvvidconv: {}", has_nvvidconv);
    info!("    Complete NVENC support: {}", has_nvenc);
    
    // Additional diagnostic information
    if has_nvh264enc && !has_nvvidconv {
        warn!("NVIDIA GPU detected but nvvidconv is missing. This usually means gstreamer-plugins-bad with NVIDIA support is not installed.");
        info!("Try installing: gstreamer1.0-plugins-bad or gst-plugins-bad (depending on your distribution)");
    } else if !has_nvh264enc {
        info!("No NVIDIA hardware encoding support detected. This is normal if you don't have an NVIDIA GPU or the drivers aren't installed.");
    }
    
    info!("  VAAPI: {}", has_vaapi);
    info!("  V4L2 H264: {}", has_v4l2h264enc);
    info!("  Intel GPU: {}", has_intel_gpu);

    HardwareInfo {
        has_nvenc,
        has_nvh264enc_basic: has_nvh264enc,
        has_vaapi,
        has_v4l2h264enc,
        has_intel_gpu,
    }
}

pub struct PipelineFactory {
    // Agora sem dispositivo virtual local - será usado o compartilhado no AppState
}

impl PipelineFactory {
    pub fn new() -> Self {
        Self {
            // Sem campos locais
        }
    }

    pub async fn create_pipeline(
        &mut self,
        config: &PipelineConfig,
        hw_info: &HardwareInfo,
        pipewire_node_id: Option<u32>,
        session_type: &str,
        has_portal: bool,
        app_state: &AppState,
    ) -> Result<(gst::Pipeline, Vec<Arc<TrackLocalStaticRTP>>)> {
        // Create WebRTC tracks
        let (video_track, video_sender) =
            Self::create_webrtc_track("video", "desktop-video", "video/VP8")?;
        let mut tracks = vec![video_track];

        // Create audio track if enabled
        let audio_sender = if config.enable_audio {
            let (audio_track, audio_sender) =
                Self::create_webrtc_track("audio", "desktop-audio", "audio/opus")?;
            tracks.push(audio_track);
            Some(audio_sender)
        } else {
            None
        };

        // Try different pipeline configurations with fallback
        let pipeline_configurations = self.get_pipeline_configurations(
            config,
            hw_info,
            pipewire_node_id,
            session_type,
            has_portal,
            app_state,
        );

        for (desc, pipeline_str) in pipeline_configurations {
            info!("Attempting to create pipeline: {}", desc);
            info!("Pipeline string: {}", pipeline_str);

            match self.try_create_pipeline(
                &pipeline_str,
                video_sender.clone(),
                audio_sender.clone(),
            ) {
                Ok(pipeline) => {
                    info!("Successfully created pipeline: {}", desc);
                    return Ok((pipeline, tracks));
                }
                Err(e) => {
                    warn!(
                        "Failed to create pipeline '{}': {}. Trying fallback...",
                        desc, e
                    );
                    continue;
                }
            }
        }

        Err(anyhow!("All pipeline configurations failed"))
    }

    fn get_pipeline_configurations(
        &self,
        config: &PipelineConfig,
        hw_info: &HardwareInfo,
        pipewire_node_id: Option<u32>,
        session_type: &str,
        has_portal: bool,
        app_state: &AppState,
    ) -> Vec<(String, String)> {
        let mut configurations = Vec::new();

        match config.source_type.as_str() {
            "wayland-portal" if session_type == "wayland" && has_portal => {
                if let Some(node_id) = pipewire_node_id {
                    // Try hardware acceleration first, then software
                    if config.use_hardware_encoding && hw_info.has_nvenc {
                        configurations.push((
                            "Wayland Portal + NVIDIA NVENC (Full)".to_string(),
                            self.build_combined_pipeline_str(
                                &self.build_wayland_nvenc_pipeline_str(node_id),
                                &self.build_audio_pipeline_str(config, app_state),
                            ),
                        ));
                    }
                    // Fallback to basic NVIDIA if nvh264enc is available but nvvidconv is not
                    if config.use_hardware_encoding && hw_info.has_nvh264enc_basic {
                        configurations.push((
                            "Wayland Portal + NVIDIA NVENC (Basic)".to_string(),
                            self.build_combined_pipeline_str(
                                &self.build_wayland_nvenc_basic_pipeline_str(node_id),
                                &self.build_audio_pipeline_str(config, app_state),
                            ),
                        ));
                        // Ultra-minimal fallback for limited nvh264enc implementations
                        configurations.push((
                            "Wayland Portal + NVIDIA NVENC (Minimal)".to_string(),
                            self.build_combined_pipeline_str(
                                &self.build_wayland_nvenc_minimal_pipeline_str(node_id),
                                &self.build_audio_pipeline_str(config, app_state),
                            ),
                        ));
                    }
                    if config.use_hardware_encoding && hw_info.has_vaapi {
                        configurations.push((
                            "Wayland Portal + VAAPI".to_string(),
                            self.build_combined_pipeline_str(
                                &self.build_wayland_vaapi_pipeline_str(node_id),
                                &self.build_audio_pipeline_str(config, app_state),
                            ),
                        ));
                    }
                    configurations.push((
                        "Wayland Portal + Software VP8".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_wayland_software_pipeline_str(node_id),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
            }
            s if s.starts_with("camera-") => {
                let device_num = s.strip_prefix("camera-").unwrap_or("0");
                if config.use_hardware_encoding && hw_info.has_nvenc {
                    configurations.push((
                        "Camera + NVIDIA NVENC (Full)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_camera_nvenc_pipeline_str(device_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                // Fallback to basic NVIDIA if nvh264enc is available but nvvidconv is not
                if config.use_hardware_encoding && hw_info.has_nvh264enc_basic {
                    configurations.push((
                        "Camera + NVIDIA NVENC (Basic)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_camera_nvenc_basic_pipeline_str(device_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                    // Ultra-minimal fallback for limited nvh264enc implementations
                    configurations.push((
                        "Camera + NVIDIA NVENC (Minimal)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_camera_nvenc_minimal_pipeline_str(device_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                if config.use_hardware_encoding && hw_info.has_vaapi {
                    configurations.push((
                        "Camera + VAAPI".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_camera_vaapi_pipeline_str(device_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                configurations.push((
                    "Camera + Software VP8".to_string(),
                    self.build_combined_pipeline_str(
                        &self.build_camera_software_pipeline_str(device_num),
                        &self.build_audio_pipeline_str(config, app_state),
                    ),
                ));
            }
            s if s.starts_with("x11-") => {
                let screen_num = s
                    .strip_prefix("x11-")
                    .and_then(|n| n.parse::<i32>().ok())
                    .unwrap_or(0);

                if config.use_hardware_encoding && hw_info.has_nvenc {
                    configurations.push((
                        "X11 + NVIDIA NVENC (Full)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_x11_nvenc_pipeline_str(screen_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                // Fallback to basic NVIDIA if nvh264enc is available but nvvidconv is not
                if config.use_hardware_encoding && hw_info.has_nvh264enc_basic {
                    configurations.push((
                        "X11 + NVIDIA NVENC (Basic)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_x11_nvenc_basic_pipeline_str(screen_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                    // Ultra-minimal fallback for limited nvh264enc implementations
                    configurations.push((
                        "X11 + NVIDIA NVENC (Minimal)".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_x11_nvenc_minimal_pipeline_str(screen_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                if config.use_hardware_encoding && hw_info.has_vaapi {
                    configurations.push((
                        "X11 + VAAPI".to_string(),
                        self.build_combined_pipeline_str(
                            &self.build_x11_vaapi_pipeline_str(screen_num),
                            &self.build_audio_pipeline_str(config, app_state),
                        ),
                    ));
                }
                configurations.push((
                    "X11 + Software VP8".to_string(),
                    self.build_combined_pipeline_str(
                        &self.build_x11_software_pipeline_str(screen_num),
                        &self.build_audio_pipeline_str(config, app_state),
                    ),
                ));
            }
            _ => {
                configurations.push((
                    "Test Pattern".to_string(),
                    self.build_combined_pipeline_str(
                        &self.build_test_pipeline_str(),
                        &self.build_audio_pipeline_str(config, app_state),
                    ),
                ));
            }
        }

        configurations
    }

    fn try_create_pipeline(
        &self,
        pipeline_str: &str,
        video_sender: mpsc::UnboundedSender<Bytes>,
        audio_sender: Option<mpsc::UnboundedSender<Bytes>>,
    ) -> Result<gst::Pipeline> {
        let pipeline = gst::parse::launch(pipeline_str)
            .map_err(|e| {
                // Enhanced error reporting for missing elements
                let error_msg = format!("{}", e);
                if error_msg.contains("no element") {
                    let missing_element = error_msg
                        .split("no element \"")
                        .nth(1)
                        .and_then(|s| s.split('"').next())
                        .unwrap_or("unknown");
                    anyhow!("Failed to parse pipeline: Missing GStreamer element '{}'. This element may not be installed or available on your system.", missing_element)
                } else {
                    anyhow!("Failed to parse pipeline: {}", e)
                }
            })?
            .downcast::<gst::Pipeline>()
            .expect("Must be a pipeline");

        // Configure video appsink
        self.setup_appsink("videosink", &pipeline, video_sender)?;

        // Configure audio appsink if present
        if let Some(audio_sender) = audio_sender {
            if let Some(_audio_sink) = pipeline.by_name("audiosink") {
                info!("Setting up audio appsink...");
                self.setup_appsink("audiosink", &pipeline, audio_sender)?;
                info!("Audio appsink configured successfully");
            } else {
                warn!("Audio enabled but no audiosink found in pipeline");
            }
        } else {
            info!("Audio sender not provided - audio disabled");
        }

        // Try to set pipeline to playing state
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| anyhow!("Failed to set pipeline to playing: {}", e))?;

        // Verify pipeline state change with timeout
        let (res, current_state, _pending_state) =
            pipeline.state(Some(gst::ClockTime::from_seconds(5)));
        match res {
            Ok(gst::StateChangeSuccess::Success) => {
                if current_state == gst::State::Playing {
                    info!("Pipeline successfully reached PLAYING state");
                    Ok(pipeline)
                } else {
                    pipeline.set_state(gst::State::Null)?;
                    Err(anyhow!(
                        "Pipeline state change succeeded but not in PLAYING state: {:?}",
                        current_state
                    ))
                }
            }
            Ok(other) => {
                pipeline.set_state(gst::State::Null)?;
                Err(anyhow!(
                    "Pipeline state change had non-success result: {:?}",
                    other
                ))
            }
            Err(e) => {
                pipeline.set_state(gst::State::Null)?;
                Err(anyhow!("Failed to change pipeline state: {:?}", e))
            }
        }
    }

    // NVIDIA NVENC pipeline builders
    fn build_x11_nvenc_pipeline_str(&self, screen_num: i32) -> String {
        format!(
            "ximagesrc display-name=:0 screen-num={} show-pointer=true use-damage=false ! \
             video/x-raw,framerate=30/1 ! \
             videoconvert ! nvvidconv ! 'video/x-raw(memory:NVMM)' ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            screen_num
        )
    }

    fn build_wayland_nvenc_pipeline_str(&self, node_id: u32) -> String {
        format!(
            "pipewiresrc path={} do-timestamp=true ! \
             videoconvert ! nvvidconv ! 'video/x-raw(memory:NVMM)' ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            node_id
        )
    }

    fn build_camera_nvenc_pipeline_str(&self, device_num: &str) -> String {
        format!(
            "v4l2src device=/dev/video{} ! \
             video/x-raw,width=1280,height=720,framerate=30/1 ! \
             videoconvert ! nvvidconv ! 'video/x-raw(memory:NVMM)' ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=4000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            device_num
        )
    }

    // Basic NVIDIA pipeline builders (without nvvidconv for compatibility)
    fn build_x11_nvenc_basic_pipeline_str(&self, screen_num: i32) -> String {
        format!(
            "ximagesrc display-name=:0 screen-num={} show-pointer=true use-damage=false ! \
             video/x-raw,framerate=30/1 ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            screen_num
        )
    }

    fn build_wayland_nvenc_basic_pipeline_str(&self, node_id: u32) -> String {
        format!(
            "pipewiresrc path={} do-timestamp=true ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            node_id
        )
    }

    fn build_camera_nvenc_basic_pipeline_str(&self, device_num: &str) -> String {
        format!(
            "v4l2src device=/dev/video{} ! \
             video/x-raw,width=1280,height=720,framerate=30/1 ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc preset=low-latency-hq rc-mode=cbr gop-size=60 bitrate=4000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            device_num
        )
    }

    // Ultra-basic NVIDIA pipeline builders (minimal properties for compatibility)
    fn build_x11_nvenc_minimal_pipeline_str(&self, screen_num: i32) -> String {
        format!(
            "ximagesrc display-name=:0 screen-num={} show-pointer=true use-damage=false ! \
             video/x-raw,framerate=30/1 ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            screen_num
        )
    }

    fn build_wayland_nvenc_minimal_pipeline_str(&self, node_id: u32) -> String {
        format!(
            "pipewiresrc path={} do-timestamp=true ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc bitrate=8000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            node_id
        )
    }

    fn build_camera_nvenc_minimal_pipeline_str(&self, device_num: &str) -> String {
        format!(
            "v4l2src device=/dev/video{} ! \
             video/x-raw,width=1280,height=720,framerate=30/1 ! \
             videoconvert ! video/x-raw,format=NV12 ! \
             nvh264enc bitrate=4000 ! \
             h264parse ! rtph264pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            device_num
        )
    }

    // VAAPI pipeline builders
    fn build_x11_vaapi_pipeline_str(&self, screen_num: i32) -> String {
        format!(
            "ximagesrc display-name=:0 screen-num={} show-pointer=true use-damage=false ! \
             video/x-raw,framerate=30/1 ! \
             videoconvert ! vaapipostproc ! 'video/x-raw(memory:VASurface)' ! \
             vaapivp8enc rate-control=cbr quality-level=5 bitrate=8000 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            screen_num
        )
    }

    fn build_wayland_vaapi_pipeline_str(&self, node_id: u32) -> String {
        format!(
            "pipewiresrc path={} do-timestamp=true ! \
             videoconvert ! vaapipostproc ! 'video/x-raw(memory:VASurface)' ! \
             vaapivp8enc rate-control=cbr quality-level=5 bitrate=8000 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            node_id
        )
    }

    fn build_camera_vaapi_pipeline_str(&self, device_num: &str) -> String {
        format!(
            "v4l2src device=/dev/video{} ! \
             video/x-raw,width=1280,height=720,framerate=30/1 ! \
             videoconvert ! vaapipostproc ! 'video/x-raw(memory:VASurface)' ! \
             vaapivp8enc rate-control=cbr quality-level=5 bitrate=4000 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            device_num
        )
    }

    // Software VP8 pipeline builders
    fn build_x11_software_pipeline_str(&self, screen_num: i32) -> String {
        format!(
            "ximagesrc display-name=:0 screen-num={} show-pointer=true use-damage=false ! \
             video/x-raw,framerate=30/1 ! \
             videoconvert ! videoscale ! video/x-raw,format=I420 ! \
             queue max-size-buffers=2 leaky=downstream ! \
             vp8enc deadline=1 cpu-used=8 threads={} error-resilient=1 keyframe-max-dist=60 \
             target-bitrate=8000000 end-usage=1 min-quantizer=4 max-quantizer=56 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            screen_num, *NUM_CPUS
        )
    }

    fn build_wayland_software_pipeline_str(&self, node_id: u32) -> String {
        format!(
            "pipewiresrc path={} do-timestamp=true ! \
             videoconvert ! videoscale ! video/x-raw,format=I420 ! \
             queue max-size-buffers=2 leaky=downstream ! \
             vp8enc deadline=1 cpu-used=8 threads={} error-resilient=1 keyframe-max-dist=60 \
             target-bitrate=8000000 end-usage=1 min-quantizer=4 max-quantizer=56 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            node_id, *NUM_CPUS
        )
    }

    fn build_camera_software_pipeline_str(&self, device_num: &str) -> String {
        format!(
            "v4l2src device=/dev/video{} ! \
             video/x-raw,width=1280,height=720,framerate=30/1 ! \
             videoconvert ! videoscale ! video/x-raw,format=I420 ! \
             queue max-size-buffers=2 leaky=downstream ! \
             vp8enc deadline=1 cpu-used=8 threads={} error-resilient=1 keyframe-max-dist=60 \
             target-bitrate=4000000 end-usage=1 min-quantizer=4 max-quantizer=56 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            device_num, *NUM_CPUS
        )
    }

    fn build_test_pipeline_str(&self) -> String {
        format!(
            "videotestsrc pattern=smpte ! \
             video/x-raw,width=1920,height=1080,framerate=30/1,format=I420 ! \
             videoconvert ! queue max-size-buffers=2 leaky=downstream ! \
             vp8enc deadline=1 cpu-used=8 threads={} error-resilient=1 keyframe-max-dist=60 \
             target-bitrate=8000000 end-usage=1 min-quantizer=4 max-quantizer=56 ! \
             rtpvp8pay pt=96 mtu=1200 ! \
             appsink name=videosink sync=false drop=true max-buffers=2",
            *NUM_CPUS
        )
    }

    // Audio pipeline builders
    fn build_audio_pipeline_str(&self, config: &PipelineConfig, app_state: &AppState) -> String {
        info!(
            "Building audio pipeline - enable_audio: {}, audio_source: {:?}",
            config.enable_audio, config.audio_source
        );

        if !config.enable_audio {
            info!("Audio disabled, returning empty pipeline");
            return String::new();
        }

        let pipeline_str = match config.audio_source.as_deref() {
            Some(device_id) if device_id == "desktop_streamer_virtual" => {
                // Use virtual device for system audio capture
                if let Ok(virtual_audio_guard) = app_state.virtual_audio.try_lock() {
                    if let Some(virtual_device) = virtual_audio_guard.as_ref() {
                        let pipeline = format!(
                            "pulsesrc device={} ! audioconvert ! audioresample ! \
                             audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                             volume volume=2.0 ! \
                             opusenc bitrate={} ! rtpopuspay pt=111 ! \
                             appsink name=audiosink sync=false drop=true max-buffers=2",
                            virtual_device.get_monitor_source_name(),
                            config.audio_bitrate
                        );
                        info!("Created virtual device audio pipeline");
                        pipeline
                    } else {
                        warn!(
                            "Virtual device requested but not available, falling back to default"
                        );
                        format!(
                            "pulsesrc device=@DEFAULT_MONITOR@ ! audioconvert ! audioresample ! \
                             audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                             volume volume=2.0 ! \
                             opusenc bitrate={} ! rtpopuspay pt=111 ! \
                             appsink name=audiosink sync=false drop=true max-buffers=2",
                            config.audio_bitrate
                        )
                    }
                } else {
                    warn!("Cannot access virtual device (locked), falling back to default");
                    format!(
                        "pulsesrc device=@DEFAULT_MONITOR@ ! audioconvert ! audioresample ! \
                         audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                         volume volume=2.0 ! \
                         opusenc bitrate={} ! rtpopuspay pt=111 ! \
                         appsink name=audiosink sync=false drop=true max-buffers=2",
                        config.audio_bitrate
                    )
                }
            }
            Some(device_id) if device_id.contains(".monitor") => {
                // System audio (monitor device)
                let pipeline = format!(
                    "pulsesrc device={} ! audioconvert ! audioresample ! \
                     audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                     volume volume=2.0 ! \
                     opusenc bitrate={} ! rtpopuspay pt=111 ! \
                     appsink name=audiosink sync=false drop=true max-buffers=2",
                    device_id, config.audio_bitrate
                );
                info!("Created monitor audio pipeline for device: {}", device_id);
                pipeline
            }
            Some(device_id) => {
                // Microphone or other input device
                let pipeline = format!(
                    "pulsesrc device={} ! audioconvert ! audioresample ! \
                     audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                     volume volume=1.5 ! \
                     opusenc bitrate={} ! rtpopuspay pt=111 ! \
                     appsink name=audiosink sync=false drop=true max-buffers=2",
                    device_id, config.audio_bitrate
                );
                info!("Created input audio pipeline for device: {}", device_id);
                pipeline
            }
            None => {
                // Fallback to default system audio
                let pipeline = format!(
                    "pulsesrc device=@DEFAULT_MONITOR@ ! audioconvert ! audioresample ! \
                     audio/x-raw,rate=48000,channels=2,format=S16LE ! \
                     volume volume=2.0 ! \
                     opusenc bitrate={} ! rtpopuspay pt=111 ! \
                     appsink name=audiosink sync=false drop=true max-buffers=2",
                    config.audio_bitrate
                );
                info!("Created default monitor audio pipeline");
                pipeline
            }
        };

        info!("Final audio pipeline: {}", pipeline_str);
        pipeline_str
    }

    fn build_combined_pipeline_str(&self, video_pipeline: &str, audio_pipeline: &str) -> String {
        info!(
            "Combining pipelines - video: {} chars, audio: {} chars",
            video_pipeline.len(),
            audio_pipeline.len()
        );

        if audio_pipeline.is_empty() {
            info!("Audio pipeline empty, using video only");
            video_pipeline.to_string()
        } else {
            let combined = format!("{} {}", video_pipeline, audio_pipeline);
            info!("Combined pipeline length: {} chars", combined.len());
            combined
        }
    }

    fn create_webrtc_track(
        id: &str,
        stream_id: &str,
        mime_type: &str,
    ) -> Result<(Arc<TrackLocalStaticRTP>, mpsc::UnboundedSender<Bytes>)> {
        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: mime_type.to_owned(),
                ..Default::default()
            },
            id.to_owned(),
            stream_id.to_owned(),
        ));

        let (sender, mut receiver) = mpsc::unbounded_channel::<Bytes>();
        let track_writer = track.clone();
        let track_id = id.to_owned(); // Convert to owned String

        tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if track_writer.write(&bytes).await.is_err() {
                    warn!("WebRTC track writer closed for track: {}", track_id);
                    break;
                }
            }
        });

        Ok((track, sender))
    }

    fn setup_appsink(
        &self,
        name: &str,
        pipeline: &gst::Pipeline,
        sender: mpsc::UnboundedSender<Bytes>,
    ) -> Result<()> {
        let appsink = pipeline
            .by_name(name)
            .ok_or_else(|| anyhow!("Failed to get appsink named '{}'", name))?
            .downcast::<AppSink>()
            .map_err(|_| anyhow!("Failed to downcast to AppSink"))?;

        info!("Setting up appsink callbacks for: {}", name);
        if let Some(caps) = appsink.caps() {
            info!("Appsink '{}' caps: {}", name, caps);
        }

        let sample_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sample_count_clone = sample_count.clone();
        let name_clone = name.to_string();

        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let count =
                        sample_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    // Log every 100th sample for audio to avoid spam, every 30th for video
                    // let log_interval = if name_clone == "audiosink" { 100 } else { 30 };
                    // if count % log_interval == 0 {
                    //     info!(
                    //         "{} sample #{} received: {} bytes",
                    //         name_clone,
                    //         count,
                    //         map.len()
                    //     );
                    // }

                    if let Err(e) = sender.send(Bytes::copy_from_slice(map.as_slice())) {
                        warn!("Failed to send {} sample #{}: {}", name_clone, count, e);
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        Ok(())
    }
}
