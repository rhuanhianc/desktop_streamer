let ws = null;
let pc = null;
let localStream = null;
let statsInterval = null;
let vrSession = null;

// DOM Elements
const statusDot = document.getElementById('statusDot');
const statusText = document.getElementById('statusText');
const remoteVideo = document.getElementById('remoteVideo');
const vrVideo = document.getElementById('vrVideo');
const placeholder = document.getElementById('placeholder');
const connectBtn = document.getElementById('connectBtn');
const disconnectBtn = document.getElementById('disconnectBtn');
const vrBtn = document.getElementById('vrBtn');
const sourceSelect = document.getElementById('sourceSelect');
const vrMode = document.getElementById('vrMode');

// New UI elements
const statsToggle = document.getElementById('statsToggle');
const statsSidebar = document.getElementById('statsSidebar');
const closeSidebar = document.getElementById('closeSidebar');
const container = document.getElementById('container');
const fullscreenBtn = document.getElementById('fullscreenBtn');
const pipBtn = document.getElementById('pipBtn');
const clearLogsBtn = document.getElementById('clearLogsBtn');
const videoContainer = document.getElementById('videoContainer');

// Quick stats elements
const quickFps = document.getElementById('quickFps');
const quickBitrate = document.getElementById('quickBitrate');
const quickRes = document.getElementById('quickRes');

// Audio elements
const enableAudio = document.getElementById('enableAudio');
const enableMicrophone = document.getElementById('enableMicrophone');
const audioSourceSelect = document.getElementById('audioSourceSelect');
const audioBitrate = document.getElementById('audioBitrate');
const audioSourceGroup = document.getElementById('audioSourceGroup');

// Statistics tracking
let stats = {
    fps: 0,
    bitrate: 0,
    frameCount: 0,
    lastTime: 0,
    lastBytes: 0,
    packetsSent: 0,
    packetsLost: 0,
    rtt: 0,
    jitter: 0
};

// Audio state
let audioDevices = [];
let currentAudioConfig = {
    enabled: false,
    microphoneEnabled: false,
    source: null,
    bitrate: 128000
};

let microphoneStream = null;

// Initialize event listeners
document.addEventListener('DOMContentLoaded', function() {
    initializeEventListeners();
});

function initializeEventListeners() {
    console.log('Inicializando event listeners...');
    console.log('Elementos encontrados:', {
        enableAudio: !!enableAudio,
        audioSourceSelect: !!audioSourceSelect,
        audioBitrate: !!audioBitrate,
        connectBtn: !!connectBtn,
        disconnectBtn: !!disconnectBtn
    });
    
    // Stats sidebar toggle
    if (statsToggle) statsToggle.addEventListener('click', toggleStatsSidebar);
    if (closeSidebar) closeSidebar.addEventListener('click', closeStatsSidebar);
    
    // Fullscreen controls
    if (fullscreenBtn) fullscreenBtn.addEventListener('click', toggleFullscreen);
    if (pipBtn) pipBtn.addEventListener('click', togglePictureInPicture);
    
    // Clear logs
    if (clearLogsBtn) clearLogsBtn.addEventListener('click', clearLogs);
    
    // Audio controls
    if (enableAudio) enableAudio.addEventListener('change', toggleAudio);
    if (enableMicrophone) enableMicrophone.addEventListener('change', toggleMicrophone);
    if (audioSourceSelect) audioSourceSelect.addEventListener('change', updateAudioSource);
    if (audioBitrate) audioBitrate.addEventListener('change', updateAudioBitrate);
    
    // Keyboard shortcuts
    document.addEventListener('keydown', handleKeyboardShortcuts);
    
    // Auto-hide controls in video area
    let hideControlsTimeout;
    videoContainer.addEventListener('mousemove', () => {
        clearTimeout(hideControlsTimeout);
        hideControlsTimeout = setTimeout(() => {
            // Auto-hide logic can be added here
        }, 3000);
    });
    
    // Touch support for mobile
    videoContainer.addEventListener('touchstart', handleTouch);
    
    // Audio controls
    enableAudio.addEventListener('change', toggleAudio);
    audioSourceSelect.addEventListener('change', updateAudioSource);
    audioBitrate.addEventListener('change', updateAudioBitrate);
    
    // Initialize audio configuration from HTML defaults
    initializeAudioConfig();
    
    log('Interface inicializada', 'success');
}

function initializeAudioConfig() {
    // Set initial audio config based on HTML defaults
    currentAudioConfig.enabled = enableAudio.checked;
    currentAudioConfig.source = audioSourceSelect.value || null;
    currentAudioConfig.bitrate = parseInt(audioBitrate.value);
    currentAudioConfig.microphoneEnabled = enableMicrophone.checked;
    
    // Show/hide audio source group based on audio enable state
    if (currentAudioConfig.enabled) {
        audioSourceGroup.style.display = 'flex';
    } else {
        audioSourceGroup.style.display = 'none';
    }
    
    log(`Configuração inicial de áudio: ${JSON.stringify(currentAudioConfig)}`, 'info');
}

function handleKeyboardShortcuts(event) {
    if (event.target.tagName === 'INPUT' || event.target.tagName === 'SELECT') return;
    
    switch(event.key) {
        case 'f':
        case 'F':
            event.preventDefault();
            toggleFullscreen();
            break;
        case 'p':
        case 'P':
            event.preventDefault();
            togglePictureInPicture();
            break;
        case 's':
        case 'S':
            event.preventDefault();
            toggleStatsSidebar();
            break;
        case 'c':
        case 'C':
            if (event.ctrlKey) return; // Don't override Ctrl+C
            event.preventDefault();
            if (connectBtn.disabled) {
                disconnect();
            } else {
                connect();
            }
            break;
        case 'Escape':
            if (document.fullscreenElement) {
                exitFullscreen();
            }
            if (statsSidebar.classList.contains('open')) {
                closeStatsSidebar();
            }
            break;
    }
}

function handleTouch(event) {
    // Double tap to fullscreen on mobile
    const now = Date.now();
    if (this.lastTap && (now - this.lastTap < 300)) {
        toggleFullscreen();
    }
    this.lastTap = now;
}

function toggleStatsSidebar() {
    const isOpen = statsSidebar.classList.contains('open');
    if (isOpen) {
        closeStatsSidebar();
    } else {
        openStatsSidebar();
    }
}

function openStatsSidebar() {
    statsSidebar.classList.add('open');
    container.classList.add('stats-open');
    log('Painel de estatísticas aberto', 'info');
}

function closeStatsSidebar() {
    statsSidebar.classList.remove('open');
    container.classList.remove('stats-open');
    log('Painel de estatísticas fechado', 'info');
}

function toggleFullscreen() {
    if (!document.fullscreenElement) {
        enterFullscreen();
    } else {
        exitFullscreen();
    }
}

function enterFullscreen() {
    if (videoContainer.requestFullscreen) {
        videoContainer.requestFullscreen();
    } else if (videoContainer.webkitRequestFullscreen) {
        videoContainer.webkitRequestFullscreen();
    } else if (videoContainer.msRequestFullscreen) {
        videoContainer.msRequestFullscreen();
    }
    log('Modo tela cheia ativado', 'info');
}

function exitFullscreen() {
    if (document.exitFullscreen) {
        document.exitFullscreen();
    } else if (document.webkitExitFullscreen) {
        document.webkitExitFullscreen();
    } else if (document.msExitFullscreen) {
        document.msExitFullscreen();
    }
    log('Modo tela cheia desativado', 'info');
}

function togglePictureInPicture() {
    if (!document.pictureInPictureElement) {
        if (remoteVideo.requestPictureInPicture) {
            remoteVideo.requestPictureInPicture()
                .then(() => {
                    log('Picture-in-Picture ativado', 'info');
                })
                .catch(error => {
                    log('Erro ao ativar Picture-in-Picture: ' + error.message, 'error');
                });
        } else {
            log('Picture-in-Picture não suportado neste navegador', 'warning');
        }
    } else {
        document.exitPictureInPicture()
            .then(() => {
                log('Picture-in-Picture desativado', 'info');
            });
    }
}

function clearLogs() {
    const logContainer = document.getElementById('logContainer');
    logContainer.innerHTML = '';
    log('Logs limpos', 'info');
}

function updateQuickStats() {
    if (quickFps) quickFps.textContent = `${stats.fps}fps`;
    if (quickBitrate) quickBitrate.textContent = `${(stats.bitrate / 1000000).toFixed(1)}Mbps`;
    if (quickRes && remoteVideo.videoWidth) {
        quickRes.textContent = `${remoteVideo.videoWidth}x${remoteVideo.videoHeight}`;
    }
}

function updateStatus(status, message) {
    statusDot.className = `status-dot ${status}`;
    statusText.textContent = message;
}

function log(message, type = 'info') {
    const logContainer = document.getElementById('logContainer');
    if (!logContainer) return;
    
    const entry = document.createElement('div');
    entry.className = `log-entry`;
    
    const time = new Date().toLocaleTimeString('pt-BR', { 
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    });
    
    entry.innerHTML = `
        <span class="log-time">${time}</span>
        <span class="log-message log-${type}">${message}</span>
    `;
    
    logContainer.appendChild(entry);
    logContainer.scrollTop = logContainer.scrollHeight;
    
    // Limit log entries to prevent memory issues (more efficient now)
    while (logContainer.children.length > 50) {
        logContainer.removeChild(logContainer.firstChild);
    }
    
    // Also log to console for debugging
    console.log(`[${time}] ${type.toUpperCase()}: ${message}`);
}

function updateSourceList(monitors) {
    log(`Recebido lista de ${monitors.length} fontes disponíveis`, 'info');
    
    // Clear existing options
    sourceSelect.innerHTML = '';
    
    // Create groups
    const screenGroup = document.createElement('optgroup');
    screenGroup.label = 'Telas';
    
    const cameraGroup = document.createElement('optgroup');
    cameraGroup.label = 'Câmeras';
    
    // Add monitors and cameras
    monitors.forEach(monitor => {
        const option = document.createElement('option');
        option.value = monitor.id;
        
        if (monitor.source_type === 'camera') {
            option.textContent = `📹 ${monitor.name}`;
            cameraGroup.appendChild(option);
        } else {
            const icon = monitor.primary ? '🖥️ ' : '📺 ';
            option.textContent = `${icon}${monitor.name} (${monitor.resolution})`;
            if (monitor.primary) {
                option.textContent += ' - Principal';
            }
            screenGroup.appendChild(option);
        }
    });
    
    // Add groups to select
    if (screenGroup.children.length > 0) {
        sourceSelect.appendChild(screenGroup);
    }
    
    if (cameraGroup.children.length > 0) {
        sourceSelect.appendChild(cameraGroup);
    }
    
    // Fallback if no sources
    if (sourceSelect.options.length === 0) {
        sourceSelect.innerHTML = `
            <optgroup label="Telas">
                <option value="fallback">🖥️ Tela Principal</option>
            </optgroup>
        `;
    }
    
    log(`Lista de fontes atualizada: ${sourceSelect.options.length} opções disponíveis`, 'success');
}

async function connect() {
    try {
        updateStatus('connecting', 'Conectando...');
        log('Iniciando conexão WebRTC...', 'info');
        
        connectBtn.disabled = true;
        
        // Request microphone access if enabled
        if (currentAudioConfig.microphoneEnabled && !microphoneStream) {
            log('Solicitando acesso ao microfone...', 'info');
            await requestMicrophoneAccess();
        }
        
        const sourceType = sourceSelect.value;
        log(`Fonte selecionada: ${sourceType}`, 'info');
        log('Tentando criar WebSocket connection...', 'info');
        // Create WebSocket connection
        const wsUrl = `ws://${window.location.hostname}:3000/ws`;
        ws = new WebSocket(wsUrl);
        log(`WebSocket object created for ${wsUrl}`, 'info');
        
        ws.onopen = () => {
            log('WebSocket onopen event disparado. Conectado com sucesso.', 'success');
        };
        
        ws.onmessage = handleSignalingMessage;
        
        ws.onclose = () => {
            log('WebSocket desconectado', 'warning');
            updateStatus('', 'Desconectado');
            cleanup();
        };
        
        ws.onerror = (error) => {
            log('WebSocket onerror event disparado. Erro: ' + JSON.stringify(error), 'error');
            updateStatus('', 'Erro de conexão');
            cleanup();
        };
        
    } catch (error) {
        log('Erro GERAL na função connect(): ' + error.message, 'error');
        updateStatus('', 'Erro');
        connectBtn.disabled = false;
    }
}

async function setupWebRTC(sourceType) {
    log(`Iniciando setupWebRTC para sourceType: ${sourceType}`, 'info');
    try {
        log('Configurando conexão WebRTC...', 'info');
        
        // Create RTCPeerConnection with optimized config
        log('Criando RTCPeerConnection...', 'info');
        pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' },
                { urls: 'stun:stun1.l.google.com:19302' }
            ],
            iceCandidatePoolSize: 10
        });
        
        let candidateQueue = [];
        let remoteDescriptionSet = false;
        
        // Handle incoming stream
        pc.ontrack = (event) => {
            log(`Track recebido: ${event.track.kind} (${event.track.label})`, 'success');
            
            // Only set srcObject if it's not already set to preserve existing tracks
          //  if (!remoteVideo.srcObject) {
                remoteVideo.srcObject = event.streams[0];
                vrVideo.srcObject = event.streams[0]; // Also set for VR mode
            //    log('Stream atribuída aos elementos de vídeo', 'debug');
           // }
            
            if (event.track.kind === 'video') {
                log('Stream de vídeo recebido do servidor!', 'success');
                
                // Wait for video to load
                remoteVideo.onloadedmetadata = () => {
                    log(`Vídeo carregado: ${remoteVideo.videoWidth}x${remoteVideo.videoHeight}`, 'success');
                    placeholder.style.display = 'none';
                    remoteVideo.style.display = 'block';
                    
                    // Enable VR button when stream is available
                    vrBtn.disabled = false;
                    
                    // Start stats monitoring
                    startStatsMonitoring();
                };
                
                remoteVideo.onerror = (e) => {
                    log('Erro no vídeo: ' + e.message, 'error');
                };
            } else if (event.track.kind === 'audio') {
                log('Stream de áudio recebido do servidor!', 'success');
                
                // Debug: Log detailed audio track information
                log(`Audio track details: label="${event.track.label}", id="${event.track.id}", enabled=${event.track.enabled}, muted=${event.track.muted}, readyState=${event.track.readyState}`, 'debug');
                
                // Try to enable the track if it's disabled
                if (!event.track.enabled) {
                    log('Tentando habilitar track de áudio...', 'debug');
                    event.track.enabled = true;
                }
                
                // Debug: Check stream audio tracks
                const stream = event.streams[0];
                const audioTracks = stream.getAudioTracks();
                log(`Total audio tracks in stream: ${audioTracks.length}`, 'debug');
                audioTracks.forEach((track, index) => {
                    log(`Audio track ${index}: enabled=${track.enabled}, muted=${track.muted}, readyState=${track.readyState}`, 'debug');
                    // Try to enable tracks
                    if (!track.enabled) {
                        track.enabled = true;
                        log(`Habilitado audio track ${index}`, 'debug');
                    }
                });
                
                // Configure audio playback
                remoteVideo.muted = false;
                remoteVideo.volume = 1.0; // Set to maximum volume
                
                // Try to enable autoplay for audio (required for mobile browsers)
                remoteVideo.setAttribute('autoplay', '');
                remoteVideo.setAttribute('playsinline', '');
                
                // Debug: Log video element state
                log(`Video element state: muted=${remoteVideo.muted}, volume=${remoteVideo.volume}, paused=${remoteVideo.paused}`, 'debug');
                
                // Force play if needed (handle autoplay restrictions)
                setTimeout(() => {
                    log(`Video state after timeout: paused=${remoteVideo.paused}, muted=${remoteVideo.muted}, volume=${remoteVideo.volume}`, 'debug');
                    
                    if (remoteVideo.paused) {
                        remoteVideo.play().then(() => {
                            log('Reprodução de áudio iniciada automaticamente', 'success');
                            // Check if we can actually hear audio
                            setTimeout(() => {
                                checkAudioPlayback();
                            }, 2000);
                        }).catch(error => {
                            log('Autoplay bloqueado - clique no vídeo para reproduzir áudio', 'warning');
                            log(`Erro de autoplay: ${error.message}`, 'debug');
                        });
                    } else {
                        log('Vídeo já está reproduzindo áudio', 'success');
                        checkAudioPlayback();
                    }
                    
                    // Check audio tracks in the stream
                    const audioTracks = event.streams[0].getAudioTracks();
                    log(`Tracks de áudio na stream: ${audioTracks.length}`, 'info');
                    audioTracks.forEach((track, index) => {
                        log(`Audio track ${index}: enabled=${track.enabled}, readyState=${track.readyState}`, 'debug');
                    });
                }, 100);
                
                log(`Áudio configurado: volume=${remoteVideo.volume}, muted=${remoteVideo.muted}`, 'success');
            }
        };
        
        // Handle ICE candidates
        pc.onicecandidate = (event) => {
            if (event.candidate) {
                const candidateMsg = {
                    type: 'ice-candidate',
                    candidate: JSON.stringify(event.candidate)
                };
                
                if (remoteDescriptionSet && ws.readyState === WebSocket.OPEN) {
                    ws.send(JSON.stringify(candidateMsg));
                } else {
                    candidateQueue.push(candidateMsg);
                }
            }
        };
        
        // Handle connection state changes
        pc.onconnectionstatechange = () => {
            log(`Estado da conexão: ${pc.connectionState}`, 'info');
            
            switch (pc.connectionState) {
                case 'connected':
                    updateStatus('connected', 'Conectado');
                    disconnectBtn.disabled = false;
                    log('🎉 Streaming ativo!', 'success');
                    break;
                case 'connecting':
                    updateStatus('connecting', 'Conectando...');
                    break;
                case 'disconnected':
                case 'failed':
                    updateStatus('', 'Desconectado');
                    log('❌ Conexão perdida', 'error');
                    cleanup();
                    break;
                case 'closed':
                    updateStatus('', 'Desconectado');
                    log('🔌 Conexão fechada', 'warning');
                    break;
            }
        };
        
        // Add microphone track if enabled
        if (currentAudioConfig.microphoneEnabled && microphoneStream) {
            microphoneStream.getAudioTracks().forEach(track => {
                pc.addTrack(track, microphoneStream);
                log('Track de microfone adicionado à conexão WebRTC', 'info');
            });
        }
        
        // Create offer to receive video and audio from server
        const offer = await pc.createOffer({
            offerToReceiveVideo: true,
            offerToReceiveAudio: true
        });
        log('RTCPeerConnection criado.', 'info');
        await pc.setLocalDescription(offer);
        log('Local description configurado.', 'info');
        
        log('Oferta WebRTC criada', 'info');
        
        // Send offer with source type and configuration
        const config = {
            source_type: sourceType,
            audio_source: currentAudioConfig.enabled ? currentAudioConfig.source : null,
            enable_audio: currentAudioConfig.enabled,
            enable_microphone_input: currentAudioConfig.microphoneEnabled,
            audio_bitrate: currentAudioConfig.bitrate,
            audio_sample_rate: 48000,
            resolution: [1920, 1080],
            framerate: 30,
            use_hardware_encoding: true
        };
        
        log(`Configuração enviada: ${JSON.stringify(config)}`, 'debug');
        log(`Enviando oferta para o servidor com sourceType: ${sourceType}`, 'info');
        ws.send(JSON.stringify({
            type: 'offer',
            sdp: pc.localDescription.sdp,
            config: config
        }));
        log('Oferta enviada.', 'info');
        
        log(`Configurado para: ${sourceType}`, 'success');
        
        // Function to send queued candidates
        window.sendQueuedCandidates = () => {
            if (remoteDescriptionSet && ws.readyState === WebSocket.OPEN) {
                candidateQueue.forEach(candidate => {
                    ws.send(JSON.stringify(candidate));
                });
                candidateQueue = [];
            }
        };
        
        // Store reference for cleanup
        window.candidateQueue = candidateQueue;
        window.setRemoteDescriptionSet = (value) => {
            remoteDescriptionSet = value;
            if (value) {
                window.sendQueuedCandidates();
            }
        };
        
    } catch (error) {
        log('Erro GERAL na função setupWebRTC(): ' + error.message, 'error');
        throw error;
    }
}

async function handleSignalingMessage(event) {
    log(`handleSignalingMessage: RAW event.data recebido: ${event.data}`, 'info');
    try {
        const signal = JSON.parse(event.data);
        log(`handleSignalingMessage: Sinal decodificado: ${JSON.stringify(signal)}`, 'info');
        
        switch (signal.type) {
            case 'monitors':
                updateSourceList(signal.monitors);
                log('Após updateSourceList no case "monitors".', 'info');
                
                // Explicitly call setupWebRTC logic here instead of relying on fall-through
                if (!pc) { // Check if pc is null, as in the original default case logic
                    log('Condição para setup WebRTC atendida (monitors recebido, pc nulo).', 'info');
                    const sourceType = sourceSelect.value || signal.monitors[0]?.id;
                    log(`sourceType para setup: ${sourceType}`, 'info');
                    if (sourceType) {
                        log(`Tentando chamar setupWebRTC com sourceType: ${sourceType}. Estado de ws: ${ws ? ws.readyState : 'null'}, pc: ${pc ? 'existe' : 'null'}`, 'info');
                        // Launch as a separate async task to not block the switch case
                        (async () => {
                            try {
                                await setupWebRTC(sourceType);
                                log('setupWebRTC chamado com sucesso a partir do case "monitors".', 'success');
                            } catch (e) {
                                log(`ERRO ao chamar setupWebRTC a partir do case "monitors": ${e.message}`, 'error');
                            }
                        })();
                    } else {
                        log('Nenhuma sourceType disponível para setup WebRTC no case "monitors".', 'warning');
                    }
                } else {
                     log(`Setup WebRTC não iniciado no case "monitors" porque pc já existe.`, 'info');
                }
                break; // Add break here now that logic is self-contained
                
            case 'audio-devices':
                populateAudioDevices(signal.devices);
                break;
                
            case 'answer':
                if (pc) {
                    log('Resposta recebida do servidor', 'info');
                    await pc.setRemoteDescription(new RTCSessionDescription({
                        type: 'answer',
                        sdp: signal.sdp
                    }));
                    window.setRemoteDescriptionSet(true);
                    log('Remote description configurada', 'success');
                }
                break;
                
            case 'ice-candidate':
                if (pc) {
                    const candidate = JSON.parse(signal.candidate);
                    await pc.addIceCandidate(new RTCIceCandidate(candidate));
                    log('ICE candidate adicionado', 'info');
                }
                break;
                
            case 'error':
                log('Erro do servidor: ' + signal.message, 'error');
                updateStatus('', 'Erro');
                break;
                
            default:
                log(`Sinal não tratado ou já tratado no case 'monitors': ${signal.type}`, 'warning');
                break;
        }
    } catch (error) {
        log('Erro ao processar mensagem: ' + error.message, 'error');
    }
}

function startStatsMonitoring() {
    if (statsInterval) {
        clearInterval(statsInterval);
    }
    
    statsInterval = setInterval(async () => {
        if (pc && pc.connectionState === 'connected') {
            try {
                const stats = await pc.getStats();
                updateStatsDisplay(stats);
            } catch (error) {
                console.error('Error getting stats:', error);
            }
        }
    }, 1000);
}

function updateStatsDisplay(stats) {
    let inboundRtp = null;
    let remoteInbound = null;
    let outboundRtp = null;
    let transport = null;
    
    stats.forEach(report => {
        if (report.type === 'inbound-rtp' && report.mediaType === 'video') {
            inboundRtp = report;
        } else if (report.type === 'remote-inbound-rtp' && report.mediaType === 'video') {
            remoteInbound = report;
        } else if (report.type === 'outbound-rtp' && report.mediaType === 'video') {
            outboundRtp = report;
        } else if (report.type === 'transport') {
            transport = report;
        }
    });
    
    if (inboundRtp) {
        // Calculate FPS
        const now = Date.now();
        const framesDelta = inboundRtp.framesDecoded - (window.lastFrames || 0);
        const timeDelta = now - (window.lastTime || now);
        const fps = timeDelta > 0 ? Math.round((framesDelta * 1000) / timeDelta) : 0;
        
        // Calculate bitrate (in Kbps)
        const bytesDelta = inboundRtp.bytesReceived - (window.lastBytes || 0);
        const bitrate = timeDelta > 0 ? Math.round((bytesDelta * 8 * 1000) / (timeDelta * 1024)) : 0;
        
        // Get actual resolution
        const width = inboundRtp.frameWidth || 0;
        const height = inboundRtp.frameHeight || 0;
        
        // Calculate packet loss percentage
        const packetsReceived = inboundRtp.packetsReceived || 0;
        const packetsLost = inboundRtp.packetsLost || 0;
        const totalPackets = packetsReceived + packetsLost;
        const lossPercentage = totalPackets > 0 ? ((packetsLost / totalPackets) * 100).toFixed(2) : 0;
        
        // Update detailed stats in sidebar
        const fpsElement = document.getElementById('fps');
        const bitrateElement = document.getElementById('bitrate');
        const resolutionElement = document.getElementById('resolution');
        const bytesElement = document.getElementById('bytesSent');
        const packetsElement = document.getElementById('packetsLost');
        
        if (fpsElement) {
            fpsElement.textContent = fps;
            fpsElement.className = fps >= 25 ? 'stat-value' : fps >= 15 ? 'stat-value warning' : 'stat-value error';
        }
        
        if (bitrateElement) {
            bitrateElement.textContent = `${bitrate} Kbps`;
            bitrateElement.className = bitrate >= 2000 ? 'stat-value' : bitrate >= 1000 ? 'stat-value warning' : 'stat-value error';
        }
        
        if (resolutionElement) {
            resolutionElement.textContent = `${width}x${height}`;
            resolutionElement.className = (width >= 1920 && height >= 1080) ? 'stat-value' : 
                                         (width >= 1280 && height >= 720) ? 'stat-value warning' : 'stat-value error';
        }
        
        if (bytesElement) {
            bytesElement.textContent = `${Math.round((inboundRtp.bytesReceived || 0) / 1024 / 1024)} MB`;
        }
        
        if (packetsElement) {
            packetsElement.textContent = `${packetsLost} (${lossPercentage}%)`;
            packetsElement.className = packetsLost === 0 ? 'stat-value' : packetsLost < 10 ? 'stat-value warning' : 'stat-value error';
        }
        
        // Update quick stats in bottom bar
        if (quickFps) quickFps.textContent = `${fps}fps`;
        if (quickBitrate) quickBitrate.textContent = `${(bitrate / 1000).toFixed(1)}Mbps`;
        if (quickRes && width && height) {
            quickRes.textContent = `${width}x${height}`;
        }
        
        // Update global stats object
        window.stats = {
            fps: fps,
            bitrate: bitrate,
            resolution: `${width}x${height}`,
            packetsLost: packetsLost,
            lossPercentage: lossPercentage
        };
        
        // Log quality issues (throttled to avoid spam)
        if (fps < 15 && fps > 0 && !window.lowFpsWarned) {
            log(`⚠️ FPS baixo detectado: ${fps}`, 'warning');
            window.lowFpsWarned = true;
            setTimeout(() => window.lowFpsWarned = false, 10000);
        }
        if (bitrate < 500 && bitrate > 0 && !window.lowBitrateWarned) {
            log(`⚠️ Bitrate baixo detectado: ${bitrate} Kbps`, 'warning');
            window.lowBitrateWarned = true;
            setTimeout(() => window.lowBitrateWarned = false, 10000);
        }
        if (lossPercentage > 5 && !window.packetLossWarned) {
            log(`⚠️ Perda de pacotes detectada: ${lossPercentage}%`, 'warning');
            window.packetLossWarned = true;
            setTimeout(() => window.packetLossWarned = false, 10000);
        }
        if ((width < 1280 || height < 720) && width > 0 && height > 0 && !window.lowResWarned) {
            log(`⚠️ Resolução baixa detectada: ${width}x${height}`, 'warning');
            window.lowResWarned = true;
            setTimeout(() => window.lowResWarned = false, 10000);
        }
        
        // Store for next calculation
        window.lastFrames = inboundRtp.framesDecoded;
        window.lastBytes = inboundRtp.bytesReceived;
        window.lastTime = now;
        
        // Log codec info (only once)
        if (inboundRtp.decoderImplementation && !window.codecLogged) {
            const codecInfo = `${inboundRtp.mimeType || 'VP8'} (${inboundRtp.decoderImplementation})`;
            log(`🎥 Codec: ${codecInfo}`, 'info');
            window.codecLogged = true;
        }
    }
    
    if (remoteInbound) {
        const rtt = remoteInbound.roundTripTime;
        const latencyElement = document.getElementById('latency');
        if (latencyElement) {
            const latencyMs = rtt ? Math.round(rtt * 1000) : 0;
            latencyElement.textContent = `${latencyMs}ms`;
            latencyElement.style.color = latencyMs <= 50 ? '#10b981' : latencyMs <= 100 ? '#f59e0b' : '#ef4444';
            
            if (latencyMs > 200 && !window.highLatencyWarned) {
                log(`⚠️ Latência alta: ${latencyMs}ms`, 'warning');
                window.highLatencyWarned = true;
                setTimeout(() => window.highLatencyWarned = false, 5000);
            }
        }
    }
}

// Audio Control Functions
function toggleAudio() {
    currentAudioConfig.enabled = enableAudio.checked;
    
    if (currentAudioConfig.enabled) {
        audioSourceGroup.style.display = 'flex';
        log('Áudio habilitado', 'success');
    } else {
        audioSourceGroup.style.display = 'none';
        log('Áudio desabilitado', 'info');
    }
    
    // Log current audio config for debugging
    log(`Config atual de áudio: ${JSON.stringify(currentAudioConfig)}`, 'debug');
    
    updateConnectionConfig();
}

function updateAudioSource() {
    const selectedValue = audioSourceSelect.value;
    currentAudioConfig.source = selectedValue || null;
    
    // Show/hide virtual audio info
    const virtualAudioInfo = document.getElementById('virtualAudioInfo');
    if (virtualAudioInfo) {
        if (selectedValue === 'desktop_streamer_virtual') {
            virtualAudioInfo.style.display = 'block';
        } else {
            virtualAudioInfo.style.display = 'none';
        }
    }
    
    if (selectedValue) {
        log(`Fonte de áudio alterada para: ${getAudioSourceDescription(selectedValue)}`, 'info');
    }
    
    // Log current audio config for debugging
    log(`Config atual de áudio: ${JSON.stringify(currentAudioConfig)}`, 'debug');
    
    updateConnectionConfig();
}

function updateAudioBitrate() {
    currentAudioConfig.bitrate = parseInt(audioBitrate.value);
    log(`Bitrate de áudio alterado para: ${currentAudioConfig.bitrate / 1000} kbps`, 'info');
    updateConnectionConfig();
}

function getAudioSourceDescription(sourceId) {
    if (sourceId === 'desktop_streamer_virtual') {
        return 'Virtual Audio Driver (Auto-capture)';
    }
    const device = audioDevices.find(d => d.name === sourceId);
    return device ? device.description || device.name : sourceId;
}

function populateAudioDevices(devices) {
    audioDevices = devices;
    log(`Recebidos ${devices.length} dispositivos de áudio`, 'info');
    
    // Clear existing options except the first one and virtual option
    const firstOption = audioSourceSelect.firstElementChild;
    audioSourceSelect.innerHTML = '';
    audioSourceSelect.appendChild(firstOption);
    
    // Add virtual device option first
    const virtualGroup = document.createElement('optgroup');
    virtualGroup.label = '🚀 Recomendado';
    
    const virtualOption = document.createElement('option');
    virtualOption.value = 'desktop_streamer_virtual';
    virtualOption.textContent = '🔊 Virtual Audio Driver (Auto-capture)';
    virtualGroup.appendChild(virtualOption);
    audioSourceSelect.appendChild(virtualGroup);
    
    // Create optgroups for real devices
    const micGroup = document.createElement('optgroup');
    micGroup.label = 'Microfone';
    
    const systemGroup = document.createElement('optgroup');
    systemGroup.label = 'Sistema';
    
    let defaultSystemDevice = null;
    
    // Add devices to appropriate groups
    devices.forEach(device => {
        // Skip virtual devices that are already added
        if (device.name === 'desktop_streamer_virtual') return;
        
        const option = document.createElement('option');
        option.value = device.name;
        option.textContent = device.description || device.name;
        
        log(`Adicionando dispositivo: ${device.name} (${device.device_type})`, 'info');
        
        switch (device.device_type) {
            case 'source':
                micGroup.appendChild(option);
                break;
            case 'monitor':
                systemGroup.appendChild(option);
                if (!defaultSystemDevice) {
                    defaultSystemDevice = device.name;
                }
                break;
            case 'virtual':
                // Virtual devices are handled separately
                break;
        }
    });
    
    // Only add groups that have options
    if (micGroup.children.length > 0) {
        audioSourceSelect.appendChild(micGroup);
        log(`Adicionado grupo Microfone com ${micGroup.children.length} dispositivos`, 'info');
    }
    if (systemGroup.children.length > 0) {
        audioSourceSelect.appendChild(systemGroup);
        log(`Adicionado grupo Sistema com ${systemGroup.children.length} dispositivos`, 'info');
    }
    
    // Auto-enable audio with virtual device as default
    enableAudio.checked = true;
    audioSourceSelect.value = 'desktop_streamer_virtual';
    currentAudioConfig.enabled = true;
    currentAudioConfig.source = 'desktop_streamer_virtual';
    updateAudioSource();
    audioSourceGroup.style.display = 'flex';
    log(`Áudio virtual habilitado por padrão`, 'success');
    
    log(`${devices.length} dispositivos de áudio detectados`, 'success');
}

function getAudioEmoji(deviceType) {
    switch (deviceType) {
        case 'microphone': return '🎤';
        case 'system': return '🔊';
        case 'application': return '🎵';
        default: return '🎧';
    }
}

function updateConnectionConfig() {
    // This will be called when audio settings change
    // If connected, we might need to renegotiate
    if (pc && pc.connectionState === 'connected') {
        log('Configuração de áudio alterada. Reconecte para aplicar as mudanças.', 'warning');
    }
}

function toggleMicrophone() {
    currentAudioConfig.microphoneEnabled = enableMicrophone.checked;
    
    if (currentAudioConfig.microphoneEnabled) {
        log('Microfone habilitado - capturando áudio do dispositivo', 'success');
        requestMicrophoneAccess();
    } else {
        log('Microfone desabilitado', 'info');
        if (microphoneStream) {
            microphoneStream.getTracks().forEach(track => track.stop());
            microphoneStream = null;
        }
    }
    
    updateConnectionConfig();
}

async function requestMicrophoneAccess() {
    try {
        microphoneStream = await navigator.mediaDevices.getUserMedia({ 
            audio: {
                echoCancellation: true,
                noiseSuppression: true,
                autoGainControl: true,
                sampleRate: 48000
            } 
        });
        log('Acesso ao microfone concedido', 'success');
    } catch (error) {
        log(`Erro ao acessar microfone: ${error.message}`, 'error');
        currentAudioConfig.microphoneEnabled = false;
        enableMicrophone.checked = false;
    }
}

async function toggleVR() {
    if (!navigator.xr) {
        log('WebXR não suportado neste navegador', 'error');
        return;
    }
    
    try {
        if (!vrSession) {
            // Enter VR mode
            log('Iniciando modo VR...', 'info');
            
            const isSupported = await navigator.xr.isSessionSupported('immersive-vr');
            if (!isSupported) {
                log('Modo VR imersivo não suportado', 'warning');
                // Fallback to fullscreen mode
                enterFullscreenVR();
                return;
            }
            
            vrSession = await navigator.xr.requestSession('immersive-vr');
            vrMode.classList.add('active');
            vrBtn.textContent = 'Sair do VR';
            log('Modo VR ativado', 'success');
            
            vrSession.addEventListener('end', () => {
                exitVR();
            });
            
        } else {
            // Exit VR mode
            await vrSession.end();
        }
    } catch (error) {
        log('Erro ao alternar modo VR: ' + error.message, 'error');
        // Fallback to fullscreen mode
        enterFullscreenVR();
    }
}

function enterFullscreenVR() {
    vrMode.classList.add('active');
    vrBtn.innerHTML = `
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z"/>
        </svg>
        Sair do VR
    `;
    log('Modo VR em tela cheia ativado', 'success');
}

function exitVR() {
    vrSession = null;
    vrMode.classList.remove('active');
    vrBtn.innerHTML = `
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20.26 7.8a4.82 4.82 0 0 0-3.93-2.44 3.91 3.91 0 0 0-2.54 1.09 10.58 10.58 0 0 0-1.52 1.71 11 11 0 0 0-1.52-1.71 3.91 3.91 0 0 0-2.54-1.09A4.82 4.82 0 0 0 4.26 7.8 4.27 4.27 0 0 0 3 11.33a4.27 4.27 0 0 0 1.26 3.53c.57.57 1.24 1.05 2 1.43l.11.05c.07.03.14.06.21.09l.09.04c.07.03.14.06.21.09l.11.05c.76.38 1.43.86 2 1.43a4.27 4.27 0 0 0 3.53 1.26 4.27 4.27 0 0 0 3.53-1.26c.57-.57 1.24-1.05 2-1.43l.11-.05c.07-.03.14-.06-.21-.09l.09-.04c.07-.03.14-.06.21-.09l.11-.05c.76-.38 1.43-.86 2-1.43a4.27 4.27 0 0 0 1.26-3.53 4.27 4.27 0 0 0-1.26-3.53z"/>
        </svg>
        Modo VR
    `;
    log('Modo VR desativado', 'info');
}

function disconnect() {
    log('Desconectando...', 'info');
    cleanup();
}

function cleanup() {
    if (statsInterval) {
        clearInterval(statsInterval);
        statsInterval = null;
    }
    
    if (pc) {
        pc.close();
        pc = null;
    }
    
    if (ws) {
        ws.close();
        ws = null;
    }
    
    if (vrSession) {
        vrSession.end();
    }
    
    remoteVideo.srcObject = null;
    vrVideo.srcObject = null;
    remoteVideo.style.display = 'none';
    placeholder.style.display = 'block';
    
    connectBtn.disabled = false;
    disconnectBtn.disabled = true;
    vrBtn.disabled = true;
    
    updateStatus('', 'Desconectado');
    
    // Reset detailed stats
    const statsElements = {
        'fps': '0',
        'bitrate': '0 Mbps',
        'resolution': '-',
        'latency': '-',
        'bytesSent': '0 MB',
        'packetsLost': '0'
    };
    
    Object.entries(statsElements).forEach(([id, value]) => {
        const element = document.getElementById(id);
        if (element) {
            element.textContent = value;
            element.className = 'stat-value';
        }
    });
    
    // Reset quick stats
    if (quickFps) quickFps.textContent = '0fps';
    if (quickBitrate) quickBitrate.textContent = '0Mbps';
    if (quickRes) quickRes.textContent = '-';
    
    // Reset warning flags
    window.lowFpsWarned = false;
    window.lowBitrateWarned = false;
    window.packetLossWarned = false;
    
    log('Conexão encerrada e recursos limpos', 'info');
    document.getElementById('bytesSent').textContent = '0 MB';
    document.getElementById('packetsLost').textContent = '0';
}

// Handle VR mode exit with Escape key
document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && vrMode.classList.contains('active')) {
        exitVR();
    }
});

// Initialize audio devices discovery
async function initializeAudioDevices() {
    try {
        log('Descobrindo dispositivos de áudio...', 'info');
        const wsUrl = `ws://${window.location.hostname}:3000/ws`;
        const discoveryWs = new WebSocket(wsUrl);
        
        discoveryWs.onopen = () => {
            log('Conexão estabelecida para descoberta de dispositivos', 'info');
        };
        
        discoveryWs.onmessage = (event) => {
            try {
                const signal = JSON.parse(event.data);
                if (signal.type === 'audio-devices') {
                    populateAudioDevices(signal.devices);
                    discoveryWs.close();
                }
            } catch (error) {
                log(`Erro ao processar mensagem de descoberta: ${error.message}`, 'error');
            }
        };
        
        discoveryWs.onerror = (error) => {
            log(`Erro na conexão de descoberta: ${error}`, 'error');
        };
        
        discoveryWs.onclose = () => {
            log('Conexão de descoberta fechada', 'info');
        };
        
        // Close discovery connection after 5 seconds if no devices received
        setTimeout(() => {
            if (discoveryWs.readyState === WebSocket.OPEN) {
                discoveryWs.close();
                log('Timeout na descoberta de dispositivos', 'warning');
            }
        }, 5000);
        
    } catch (error) {
        log(`Erro ao inicializar descoberta de dispositivos: ${error.message}`, 'error');
    }
}

// Audio debugging function
function checkAudioPlayback() {
    log('Verificando estado de reprodução de áudio...', 'debug');
    
    // Check video element
    log(`Video element: muted=${remoteVideo.muted}, volume=${remoteVideo.volume}, paused=${remoteVideo.paused}`, 'debug');
    
    // Check if stream has audio tracks
    if (remoteVideo.srcObject) {
        const audioTracks = remoteVideo.srcObject.getAudioTracks();
        log(`Audio tracks: ${audioTracks.length}`, 'debug');
        
        audioTracks.forEach((track, index) => {
            log(`Track ${index}: enabled=${track.enabled}, muted=${track.muted}, readyState=${track.readyState}`, 'debug');
            
            // Try to unmute and enable if needed
            if (!track.enabled) {
                track.enabled = true;
                log(`Forcibly enabled track ${index}`, 'debug');
            }
        });
        
        // If we have audio tracks, try to create audio context
        if (audioTracks.length > 0) {
            // Try to create an audio context to check if audio is flowing
            try {
                if (!window.audioContext) {
                    window.audioContext = new (window.AudioContext || window.webkitAudioContext)();
                }
                
                const source = window.audioContext.createMediaStreamSource(remoteVideo.srcObject);
                const analyser = window.audioContext.createAnalyser();
                source.connect(analyser);
                
                analyser.fftSize = 256;
                const bufferLength = analyser.frequencyBinCount;
                const dataArray = new Uint8Array(bufferLength);
                
                // Check for audio data
                let checkCount = 0;
                const maxChecks = 10;
                
                function checkAudioData() {
                    analyser.getByteFrequencyData(dataArray);
                    const sum = dataArray.reduce((a, b) => a + b, 0);
                    const average = sum / bufferLength;
                    
                    log(`Audio data check ${checkCount + 1}/${maxChecks}: average level = ${average.toFixed(2)}`, 'debug');
                    
                    if (average > 0) {
                        log('✅ Dados de áudio detectados! O áudio está fluindo.', 'success');
                        return;
                    }
                    
                    checkCount++;
                    if (checkCount < maxChecks) {
                        setTimeout(checkAudioData, 500);
                    } else {
                        log('⚠️ Nenhum dado de áudio detectado após 5 segundos.', 'warning');
                        log('💡 Tente gerar áudio no sistema para testar a captura.', 'info');
                    }
                }
                
                setTimeout(checkAudioData, 500);
                
            } catch (error) {
                log(`Erro ao criar AudioContext: ${error.message}`, 'error');
            }
        } else {
            log('❌ Nenhuma track de áudio encontrada na stream.', 'error');
        }
    } else {
        log('❌ Nenhuma stream encontrada no video element.', 'error');
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    log('Desktop Streamer v2.0 inicializado', 'success');
    log('Suporte WebXR: ' + (navigator.xr ? 'Disponível' : 'Não disponível'), 'info');
    
    // Discover audio devices on page load
    setTimeout(initializeAudioDevices, 1000);
});
