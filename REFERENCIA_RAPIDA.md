# 📋 Referência Rápida - Desktop Streamer

Guia de referência rápida para desenvolvimento e troubleshooting do Desktop Streamer.

## 🚀 Comandos Úteis

### Desenvolvimento
```bash
# Build e run em desenvolvimento
cargo run

# Build otimizado
cargo build --release

# Executar com logs detalhados
RUST_LOG=desktop_streamer=debug cargo run

# Executar com logs do GStreamer
GST_DEBUG=3 cargo run

# Formatação de código
cargo fmt

# Linting
cargo clippy

# Testes
cargo test
```

### Debug GStreamer
```bash
# Ver plugins disponíveis
gst-inspect-1.0 | grep plugin-name

# Testar pipeline específico
gst-launch-1.0 ximagesrc ! videoconvert ! autovideosink

# Debug pipeline em runtime
export GST_DEBUG=3
export GST_DEBUG_FILE=/tmp/gst.log
```

### Sistema
```bash
# Verificar tipo de sessão
echo $XDG_SESSION_TYPE

# Verificar portal desktop
busctl --user list | grep portal

# Verificar câmeras
v4l2-ctl --list-devices

# Verificar monitores X11
xrandr --query

# Verificar áudio
pactl list sources short
```

## 🔧 Configurações Importantes

### Variáveis de Ambiente
```bash
export RUST_LOG=desktop_streamer=info
export PORT=3000
export HOST=0.0.0.0
export STUN_SERVER=stun:stun.l.google.com:19302
```

### Parâmetros VP8 Encoder
```rust
// Baixa latência
"vp8enc deadline=1 cpu-used=8 threads=2 target-bitrate=2000000"

// Alta qualidade  
"vp8enc deadline=4 cpu-used=1 threads=8 target-bitrate=15000000"

// Balanceado
"vp8enc deadline=16 cpu-used=4 threads=4 target-bitrate=8000000"
```

### Configurações WebRTC
```rust
RTCConfiguration {
    ice_servers: vec![
        RTCIceServer { 
            urls: vec!["stun:stun.l.google.com:19302".to_string()], 
            ..Default::default() 
        }
    ],
    ice_transport_policy: RTCIceTransportPolicy::All,
    bundle_policy: RTCBundlePolicy::Balanced,
    ..Default::default()
}
```

## 📊 Métricas de Performance

### Bitrates Recomendados
| Resolução | Framerate | Bitrate Mín | Bitrate Máx |
|-----------|-----------|-------------|-------------|
| 720p      | 30 FPS    | 2 Mbps      | 5 Mbps      |
| 1080p     | 30 FPS    | 5 Mbps      | 10 Mbps     |
| 1080p     | 60 FPS    | 8 Mbps      | 15 Mbps     |
| 1440p     | 60 FPS    | 15 Mbps     | 25 Mbps     |
| 4K        | 30 FPS    | 25 Mbps     | 50 Mbps     |

### CPU Usage vs Qualidade
| cpu-used | Velocidade | Qualidade | Uso Recomendado |
|----------|------------|-----------|-----------------|
| 1-2      | Lento      | Excelente | Gravação        |
| 3-4      | Médio      | Boa       | Streaming HD    |
| 5-6      | Rápido     | Média     | Streaming casual|
| 7-8      | Muito rápido| Baixa    | Baixa latência  |

## 🐛 Troubleshooting

### Erro: "Pipeline failed to start"
```bash
# Verificar plugins
gst-inspect-1.0 ximagesrc
gst-inspect-1.0 pipewiresrc
gst-inspect-1.0 v4l2src

# Instalar plugins faltantes
sudo apt install gstreamer1.0-plugins-{base,good,bad,ugly}
```

### Erro: "WebRTC connection failed"
```bash
# Verificar conectividade
telnet stun.l.google.com 19302

# Verificar firewall
sudo ufw status
sudo ufw allow 3000

# Debug JavaScript
# Abrir DevTools → Console → Network
```

### Erro: "Portal não disponível"
```bash
# Verificar serviços
systemctl --user status xdg-desktop-portal
systemctl --user status xdg-desktop-portal-gtk

# Reiniciar serviços
systemctl --user restart xdg-desktop-portal
systemctl --user restart xdg-desktop-portal-gtk

# Para GNOME
sudo apt install xdg-desktop-portal-gnome
```

### Erro: "Permission denied" (câmera)
```bash
# Adicionar usuário ao grupo video
sudo usermod -a -G video $USER

# Logout e login novamente
# Ou usar newgrp video

# Verificar permissões
ls -l /dev/video*
```

### Latência Alta
```rust
// Reduzir buffers
max-buffers=1

// Usar deadline baixo
deadline=1

// Desabilitar sync
sync=false

// Queue menor
max-size-buffers=1
```

### CPU Alto
```rust
// Menos threads
threads=2

// CPU usage alto
cpu-used=8

// Resolução menor
video/x-raw,width=1280,height=720

// Framerate menor
framerate=15/1
```

## 📁 Estrutura de Arquivos

```
desktop_streamer/
├── src/
│   └── main.rs              # Código principal (608 linhas)
├── static/
│   ├── index.html           # Interface web (1135 linhas)
│   ├── performance-monitor.js
│   └── vr-environment.js
├── Cargo.toml              # Dependências
├── README.md               # Documentação principal
├── GUIA_ESTUDOS.md         # Este guia
└── EXEMPLOS_MODIFICACAO.md # Exemplos práticos
```

### Principais Funções
| Função | Linha | Responsabilidade |
|--------|-------|------------------|
| `main()` | 52 | Inicialização e servidor |
| `handle_websocket()` | 208 | Gerenciar conexões WS |
| `create_peer_connection_and_pipeline()` | 340 | Setup WebRTC + GStreamer |
| `create_optimized_pipeline()` | 425 | Criar pipeline GStreamer |
| `detect_monitors()` | 100 | Detectar fontes disponíveis |
| `get_wayland_pipewire_node_id()` | 578 | Portal Wayland |

## 🔗 APIs e Protocolos

### WebSocket Messages
```javascript
// Cliente → Servidor
{
  "type": "offer",
  "sdp": "v=0\r\no=- ...",
  "sourceType": "wayland-portal"
}

{
  "type": "ice-candidate",
  "candidate": "{\"candidate\":\"...\",\"sdpMid\":\"0\"}"
}

// Servidor → Cliente  
{
  "type": "monitors",
  "monitors": [...]
}

{
  "type": "answer", 
  "sdp": "v=0\r\no=- ..."
}
```

### GStreamer Elements
| Element | Função | Parâmetros Principais |
|---------|--------|-----------------------|
| `ximagesrc` | Captura X11 | `display-name`, `screen-num` |
| `pipewiresrc` | Captura Wayland | `path` (node ID) |
| `v4l2src` | Captura câmera | `device` |
| `vp8enc` | Encoder VP8 | `deadline`, `cpu-used`, `target-bitrate` |
| `rtpvp8pay` | RTP packaging | `pt`, `mtu` |
| `appsink` | Saída para app | `sync`, `drop`, `max-buffers` |

### WebRTC States
```
new → connecting → connected → disconnected → closed
     ↓           ↓
  gathering → complete
```

## 📚 Dependências Principais

### Rust Crates
```toml
axum = "0.8.4"           # Web framework
webrtc = "0.13.0"        # WebRTC implementation  
gstreamer = "0.23.6"     # Media pipeline
tokio = "1.45.1"         # Async runtime
serde = "1.0.219"        # Serialization
anyhow = "1.0"           # Error handling
tracing = "0.1"          # Logging
ashpd = "0.8"            # Desktop portal
```

### Sistema
```bash
# Ubuntu/Debian
libgstreamer1.0-dev
gstreamer1.0-plugins-*
xdg-desktop-portal
v4l-utils

# Runtime
pulseaudio ou pipewire
X11 ou Wayland
```

## 🔍 Logs Importantes

### Sucesso
```
INFO desktop_streamer: GStreamer initialized successfully
INFO desktop_streamer: Session type: wayland, Portal available: true  
INFO desktop_streamer: 🚀 Desktop Streamer v2.0 starting on http://0.0.0.0:3000
INFO desktop_streamer: Answer created and sent successfully
```

### Erros Comuns
```
ERROR desktop_streamer: Failed to create pipeline: ...
ERROR desktop_streamer: Failed to get PipeWire node ID via portal: ...
ERROR desktop_streamer: Failed to add ICE candidate: ...
WARN desktop_streamer: Using fallback test pattern pipeline
```

## 🎯 Quick Fixes

### Reiniciar tudo
```bash
# Parar aplicação
Ctrl+C

# Reiniciar portais (Wayland)
systemctl --user restart xdg-desktop-portal*

# Reiniciar PipeWire (se necessário)
systemctl --user restart pipewire

# Restart aplicação
cargo run
```

### Reset completo do ambiente
```bash
# Limpar build
cargo clean

# Reinstalar dependências
sudo apt install --reinstall gstreamer1.0-*

# Rebuild
cargo build --release
```

### Test pipeline manual
```bash
# Testar captura X11
gst-launch-1.0 ximagesrc ! videoconvert ! autovideosink

# Testar encoder VP8
gst-launch-1.0 videotestsrc ! vp8enc ! rtpvp8pay ! udpsink host=127.0.0.1 port=5000

# Testar câmera
gst-launch-1.0 v4l2src device=/dev/video0 ! videoconvert ! autovideosink
```

---

Esta referência deve cobrir 90% das situações do dia-a-dia. Para casos específicos, consulte o GUIA_ESTUDOS.md completo.
