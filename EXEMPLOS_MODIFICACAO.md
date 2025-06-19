# 🔧 Exemplos Práticos de Modificação

Este documento contém exemplos step-by-step de como fazer modificações comuns no Desktop Streamer.

## 📋 Índice

1. [Adicionando Suporte a Áudio](#1-adicionando-suporte-a-áudio)
2. [Criando Controles de Qualidade](#2-criando-controles-de-qualidade)
3. [Implementando Gravação de Vídeo](#3-implementando-gravação-de-vídeo)
4. [Adicionando Autenticação](#4-adicionando-autenticação)
5. [Melhorando a Interface](#5-melhorando-a-interface)
6. [Otimizações de Performance](#6-otimizações-de-performance)

---

## 1. Adicionando Suporte a Áudio

### 1.1 Modificar as Estruturas

Primeiro, adicione suporte a áudio nas mensagens:

```rust
// Em SignalMessage, adicione uma nova variante
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SignalMessage {
    // ... mensagens existentes ...
    
    #[serde(rename = "audio-settings")]
    AudioSettings { 
        enabled: bool, 
        source: String, // "system", "microphone", "none"
        bitrate: u32,
    },
}
```

### 1.2 Modificar o Pipeline GStreamer

```rust
fn create_audio_video_pipeline(
    tx_video: mpsc::UnboundedSender<Bytes>,
    tx_audio: mpsc::UnboundedSender<Bytes>,
    source_type: Option<String>,
    audio_enabled: bool,
) -> Result<gst::Pipeline> {
    
    let pipeline_str = if audio_enabled {
        // Pipeline com áudio e vídeo
        format!(
            "ximagesrc display-name=:0 ! \
             videoconvert ! videoscale ! video/x-raw,format=I420 ! \
             vp8enc deadline=16 cpu-used=4 ! rtpvp8pay ! \
             appsink name=video_sink sync=false \
             \
             pulseaudiorc ! audioconvert ! audioresample ! \
             audio/x-raw,rate=48000,channels=2 ! \
             opusenc bitrate=128000 ! rtpopuspay ! \
             appsink name=audio_sink sync=false"
        )
    } else {
        // Pipeline só com vídeo (código existente)
        format!(
            "ximagesrc display-name=:0 ! \
             videoconvert ! videoscale ! video/x-raw,format=I420 ! \
             vp8enc deadline=16 cpu-used=4 ! rtpvp8pay ! \
             appsink name=video_sink sync=false"
        )
    };
    
    let pipeline = gst::parse::launch(&pipeline_str)?
        .downcast::<gst::Pipeline>()
        .unwrap();
    
    // Configurar video sink
    let video_sink = pipeline.by_name("video_sink").unwrap()
        .downcast::<AppSink>().unwrap();
    
    video_sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                let _ = tx_video.send(Bytes::copy_from_slice(&map));
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    
    // Configurar audio sink se áudio estiver habilitado
    if audio_enabled {
        let audio_sink = pipeline.by_name("audio_sink").unwrap()
            .downcast::<AppSink>().unwrap();
        
        audio_sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                    let _ = tx_audio.send(Bytes::copy_from_slice(&map));
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
    }
    
    pipeline.set_state(gst::State::Playing)?;
    Ok(pipeline)
}
```

### 1.3 Modificar a Conexão WebRTC

```rust
async fn create_peer_connection_with_audio(
    tx_to_ws: mpsc::UnboundedSender<SignalMessage>,
    tx_video: mpsc::UnboundedSender<Bytes>,
    tx_audio: Option<mpsc::UnboundedSender<Bytes>>,
    // ... outros parâmetros
) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticRTP>, Option<Arc<TrackLocalStaticRTP>>, gst::Pipeline)> {
    
    // ... código existente para criar peer connection ...
    
    // Video track (existente)
    let video_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability { 
            mime_type: MIME_TYPE_VP8.to_owned(), 
            ..Default::default() 
        },
        "video".to_owned(),
        "desktop-stream".to_owned(),
    ));
    pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>).await?;
    
    // Audio track (novo)
    let audio_track = if tx_audio.is_some() {
        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { 
                mime_type: "audio/opus".to_owned(),
                ..Default::default() 
            },
            "audio".to_owned(),
            "desktop-audio".to_owned(),
        ));
        pc.add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>).await?;
        Some(track)
    } else {
        None
    };
    
    // ... criar pipeline ...
    
    Ok((pc, video_track, audio_track, pipeline))
}
```

### 1.4 Atualizar o Handler WebSocket

```rust
async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    // ... código existente ...
    
    let mut audio_enabled = false;
    let mut audio_bitrate = 128000u32;
    
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            let signal: SignalMessage = match serde_json::from_str(&text) {
                Ok(s) => s,
                Err(e) => continue,
            };
            
            match signal {
                SignalMessage::AudioSettings { enabled, source, bitrate } => {
                    audio_enabled = enabled;
                    audio_bitrate = bitrate;
                    info!("Audio settings updated: enabled={}, source={}, bitrate={}", 
                          enabled, source, bitrate);
                }
                SignalMessage::Offer { sdp, source_type } => {
                    // Usar as configurações de áudio ao criar pipeline
                    // ... resto do código de offer ...
                }
                // ... outros casos ...
            }
        }
    }
}
```

---

## 2. Criando Controles de Qualidade

### 2.1 Estrutura para Configurações

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
struct StreamConfig {
    video_bitrate: u32,      // bps
    audio_bitrate: u32,      // bps  
    framerate: u32,          // fps
    resolution: Resolution,
    cpu_usage: CpuUsage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Resolution {
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum CpuUsage {
    Low,     // cpu-used=8
    Medium,  // cpu-used=4  
    High,    // cpu-used=1
}

impl CpuUsage {
    fn to_gstreamer_value(&self) -> u32 {
        match self {
            CpuUsage::Low => 8,
            CpuUsage::Medium => 4,
            CpuUsage::High => 1,
        }
    }
}
```

### 2.2 Presets de Qualidade

```rust
impl StreamConfig {
    fn low_quality() -> Self {
        Self {
            video_bitrate: 1_000_000,  // 1 Mbps
            audio_bitrate: 64_000,     // 64 kbps
            framerate: 15,
            resolution: Resolution { width: 1280, height: 720 },
            cpu_usage: CpuUsage::Low,
        }
    }
    
    fn medium_quality() -> Self {
        Self {
            video_bitrate: 5_000_000,  // 5 Mbps
            audio_bitrate: 128_000,    // 128 kbps
            framerate: 30,
            resolution: Resolution { width: 1920, height: 1080 },
            cpu_usage: CpuUsage::Medium,
        }
    }
    
    fn high_quality() -> Self {
        Self {
            video_bitrate: 15_000_000, // 15 Mbps
            audio_bitrate: 256_000,    // 256 kbps
            framerate: 60,
            resolution: Resolution { width: 2560, height: 1440 },
            cpu_usage: CpuUsage::High,
        }
    }
    
    fn ultra_quality() -> Self {
        Self {
            video_bitrate: 50_000_000, // 50 Mbps
            audio_bitrate: 320_000,    // 320 kbps
            framerate: 60,
            resolution: Resolution { width: 3840, height: 2160 },
            cpu_usage: CpuUsage::High,
        }
    }
}
```

### 2.3 Pipeline Dinâmico

```rust
fn create_dynamic_pipeline(
    config: &StreamConfig,
    source_type: &str,
    tx_video: mpsc::UnboundedSender<Bytes>,
    tx_audio: Option<mpsc::UnboundedSender<Bytes>>,
) -> Result<gst::Pipeline> {
    
    let video_pipeline = format!(
        "ximagesrc display-name=:0 ! \
         video/x-raw,framerate={}/1 ! \
         videoscale ! \
         video/x-raw,width={},height={},format=I420 ! \
         videoconvert ! \
         queue max-size-buffers=2 leaky=downstream ! \
         vp8enc deadline=16 cpu-used={} threads=4 \
         target-bitrate={} end-usage=1 \
         keyframe-max-dist={} ! \
         rtpvp8pay pt=96 mtu=1200 ! \
         appsink name=video_sink sync=false drop=true max-buffers=2",
        config.framerate,
        config.resolution.width,
        config.resolution.height,
        config.cpu_usage.to_gstreamer_value(),
        config.video_bitrate,
        config.framerate * 2  // Keyframe a cada 2 segundos
    );
    
    let full_pipeline = if tx_audio.is_some() {
        format!(
            "{} \
             pulseaudiorc ! audioconvert ! audioresample ! \
             audio/x-raw,rate=48000,channels=2 ! \
             opusenc bitrate={} ! rtpopuspay ! \
             appsink name=audio_sink sync=false drop=true max-buffers=2",
            video_pipeline,
            config.audio_bitrate
        )
    } else {
        video_pipeline
    };
    
    // ... resto da implementação igual aos exemplos anteriores ...
    
    Ok(pipeline)
}
```

### 2.4 Interface para Controles

Adicione ao HTML (`static/index.html`):

```html
<!-- Controles de Qualidade -->
<div class="quality-controls">
    <h3>Qualidade do Stream</h3>
    
    <div class="preset-buttons">
        <button onclick="setQualityPreset('low')">Baixa</button>
        <button onclick="setQualityPreset('medium')">Média</button>
        <button onclick="setQualityPreset('high')">Alta</button>
        <button onclick="setQualityPreset('ultra')">Ultra</button>
    </div>
    
    <div class="custom-controls">
        <label>
            Bitrate Vídeo (Mbps):
            <input type="range" id="videoBitrate" min="1" max="50" value="5" 
                   oninput="updateConfig()">
            <span id="videoBitrateValue">5</span>
        </label>
        
        <label>
            Framerate (FPS):
            <input type="range" id="framerate" min="15" max="60" value="30" 
                   oninput="updateConfig()">
            <span id="framerateValue">30</span>
        </label>
        
        <label>
            Resolução:
            <select id="resolution" onchange="updateConfig()">
                <option value="1280x720">HD (720p)</option>
                <option value="1920x1080" selected>Full HD (1080p)</option>
                <option value="2560x1440">2K (1440p)</option>
                <option value="3840x2160">4K (2160p)</option>
            </select>
        </label>
        
        <label>
            CPU Usage:
            <select id="cpuUsage" onchange="updateConfig()">
                <option value="low">Baixo (mais rápido)</option>
                <option value="medium" selected>Médio</option>
                <option value="high">Alto (melhor qualidade)</option>
            </select>
        </label>
    </div>
</div>
```

JavaScript correspondente:

```javascript
function setQualityPreset(preset) {
    const presets = {
        low: {
            videoBitrate: 1,
            audioBitrate: 64,
            framerate: 15,
            resolution: '1280x720',
            cpuUsage: 'low'
        },
        medium: {
            videoBitrate: 5,
            audioBitrate: 128,
            framerate: 30,
            resolution: '1920x1080',
            cpuUsage: 'medium'
        },
        high: {
            videoBitrate: 15,
            audioBitrate: 256,
            framerate: 60,
            resolution: '2560x1440',
            cpuUsage: 'high'
        },
        ultra: {
            videoBitrate: 50,
            audioBitrate: 320,
            framerate: 60,
            resolution: '3840x2160',
            cpuUsage: 'high'
        }
    };
    
    const config = presets[preset];
    if (config) {
        document.getElementById('videoBitrate').value = config.videoBitrate;
        document.getElementById('framerate').value = config.framerate;
        document.getElementById('resolution').value = config.resolution;
        document.getElementById('cpuUsage').value = config.cpuUsage;
        updateConfig();
    }
}

function updateConfig() {
    const videoBitrate = parseInt(document.getElementById('videoBitrate').value);
    const framerate = parseInt(document.getElementById('framerate').value);
    const resolution = document.getElementById('resolution').value;
    const cpuUsage = document.getElementById('cpuUsage').value;
    
    // Atualizar displays
    document.getElementById('videoBitrateValue').textContent = videoBitrate;
    document.getElementById('framerateValue').textContent = framerate;
    
    // Enviar configuração para servidor
    const [width, height] = resolution.split('x').map(Number);
    
    const config = {
        type: 'quality-config',
        video_bitrate: videoBitrate * 1000000, // Converter para bps
        audio_bitrate: 128000, // Fixo por enquanto
        framerate: framerate,
        resolution: { width, height },
        cpu_usage: cpuUsage
    };
    
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(config));
    }
}
```

---

## 3. Implementando Gravação de Vídeo

### 3.1 Estrutura para Gravação

```rust
#[derive(Debug, Clone)]
struct RecordingConfig {
    enabled: bool,
    format: RecordingFormat,
    filename: String,
    max_duration: Option<u32>, // segundos
    quality: RecordingQuality,
}

#[derive(Debug, Clone)]
enum RecordingFormat {
    MP4,
    WebM,
    MKV,
}

#[derive(Debug, Clone)]
enum RecordingQuality {
    Lossless,
    High,
    Medium,
    Low,
}
```

### 3.2 Pipeline com Gravação

```rust
fn create_recording_pipeline(
    config: &StreamConfig,
    recording_config: &RecordingConfig,
    source_type: &str,
    tx_video: mpsc::UnboundedSender<Bytes>,
) -> Result<gst::Pipeline> {
    
    let base_capture = "ximagesrc display-name=:0 ! \
                       video/x-raw,framerate=30/1 ! \
                       videoconvert ! videoscale ! \
                       video/x-raw,format=I420 ! \
                       tee name=t";
    
    let stream_branch = "t. ! queue ! \
                        vp8enc deadline=16 cpu-used=4 target-bitrate=5000000 ! \
                        rtpvp8pay pt=96 ! \
                        appsink name=stream_sink sync=false";
    
    let recording_branch = if recording_config.enabled {
        match recording_config.format {
            RecordingFormat::MP4 => format!(
                "t. ! queue ! \
                 x264enc bitrate=10000 speed-preset=medium ! \
                 mp4mux ! \
                 filesink location={} sync=false",
                recording_config.filename
            ),
            RecordingFormat::WebM => format!(
                "t. ! queue ! \
                 vp8enc deadline=4 cpu-used=2 target-bitrate=15000000 ! \
                 webmmux ! \
                 filesink location={} sync=false",
                recording_config.filename
            ),
            RecordingFormat::MKV => format!(
                "t. ! queue ! \
                 x264enc bitrate=20000 speed-preset=slow ! \
                 matroskamux ! \
                 filesink location={} sync=false",
                recording_config.filename
            ),
        }
    } else {
        String::new()
    };
    
    let full_pipeline = format!("{} {} {}", 
                               base_capture, 
                               stream_branch, 
                               recording_branch);
    
    let pipeline = gst::parse::launch(&full_pipeline)?
        .downcast::<gst::Pipeline>()
        .unwrap();
    
    // Configurar stream sink
    let stream_sink = pipeline.by_name("stream_sink").unwrap()
        .downcast::<AppSink>().unwrap();
    
    stream_sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                let _ = tx_video.send(Bytes::copy_from_slice(&map));
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    
    pipeline.set_state(gst::State::Playing)?;
    Ok(pipeline)
}
```

### 3.3 Controles de Gravação

Adicione ao Rust:

```rust
// Em SignalMessage
#[serde(rename = "recording-control")]
RecordingControl { 
    action: String,  // "start", "stop", "pause"
    format: Option<String>,
    filename: Option<String>,
},

#[serde(rename = "recording-status")]
RecordingStatus {
    recording: bool,
    duration: u32,  // segundos
    filename: Option<String>,
    file_size: u64, // bytes
},
```

Interface HTML:

```html
<div class="recording-controls">
    <h3>Gravação</h3>
    
    <div class="recording-settings">
        <label>
            Formato:
            <select id="recordingFormat">
                <option value="mp4">MP4 (H.264)</option>
                <option value="webm">WebM (VP8)</option>
                <option value="mkv">MKV (H.264)</option>
            </select>
        </label>
        
        <label>
            Nome do arquivo:
            <input type="text" id="recordingFilename" 
                   placeholder="recording_YYYY-MM-DD_HH-MM-SS">
        </label>
    </div>
    
    <div class="recording-buttons">
        <button id="startRecording" onclick="startRecording()">
            🔴 Iniciar Gravação
        </button>
        <button id="stopRecording" onclick="stopRecording()" disabled>
            ⏹️ Parar Gravação
        </button>
    </div>
    
    <div class="recording-status">
        <span id="recordingTime">00:00:00</span>
        <span id="recordingSize">0 MB</span>
    </div>
</div>
```

JavaScript:

```javascript
let recordingStartTime = null;
let recordingTimer = null;

function startRecording() {
    const format = document.getElementById('recordingFormat').value;
    const filename = document.getElementById('recordingFilename').value || 
                    `recording_${new Date().toISOString().replace(/[:.]/g, '-')}.${format}`;
    
    const message = {
        type: 'recording-control',
        action: 'start',
        format: format,
        filename: filename
    };
    
    ws.send(JSON.stringify(message));
    
    recordingStartTime = Date.now();
    recordingTimer = setInterval(updateRecordingTime, 1000);
    
    document.getElementById('startRecording').disabled = true;
    document.getElementById('stopRecording').disabled = false;
}

function stopRecording() {
    const message = {
        type: 'recording-control',
        action: 'stop'
    };
    
    ws.send(JSON.stringify(message));
    
    if (recordingTimer) {
        clearInterval(recordingTimer);
        recordingTimer = null;
    }
    
    document.getElementById('startRecording').disabled = false;
    document.getElementById('stopRecording').disabled = true;
}

function updateRecordingTime() {
    if (recordingStartTime) {
        const elapsed = Math.floor((Date.now() - recordingStartTime) / 1000);
        const hours = Math.floor(elapsed / 3600);
        const minutes = Math.floor((elapsed % 3600) / 60);
        const seconds = elapsed % 60;
        
        document.getElementById('recordingTime').textContent = 
            `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
    }
}
```

---

Este documento fornece exemplos práticos detalhados para as modificações mais comuns. Cada exemplo inclui:

1. **Código Rust completo** com explicações
2. **Interface HTML/JavaScript** correspondente  
3. **Configurações específicas** para cada feature
4. **Tratamento de erros** adequado

Para implementar essas modificações:

1. **Comece pequeno**: Implemente uma feature por vez
2. **Teste incrementalmente**: Compile e teste após cada mudança
3. **Use logs**: Adicione `info!()` e `debug!()` para debugar
4. **Backup**: Sempre faça backup antes de grandes mudanças

Estes exemplos servem como base para outras modificações mais complexas!
