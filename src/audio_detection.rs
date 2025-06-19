use tracing::info;

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub device_type: String, // "source", "monitor", "sink", "virtual"
}

#[derive(Debug, Clone)]
pub struct AudioInfo {
    pub has_pulseaudio: bool,
    pub has_pipewire: bool,
    pub has_alsa: bool,
    pub default_source: Option<String>,
    pub microphone_devices: Vec<AudioDevice>,
    pub monitor_devices: Vec<AudioDevice>,
}

/// Detecta as capacidades de áudio do sistema
pub fn detect_audio_capabilities() -> AudioInfo {
    info!("Detecting audio capabilities...");
    
    let has_pulseaudio = std::process::Command::new("pactl")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
        
    let has_pipewire = std::process::Command::new("pw-cli")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
        
    let has_alsa = std::path::Path::new("/proc/asound/cards").exists();
    
    let mut microphone_devices = Vec::new();
    let mut monitor_devices = Vec::new();
    let mut default_source = None;
    
    // Adicionar opção de dispositivo virtual primeiro
    monitor_devices.push(AudioDevice {
        name: "desktop_streamer_virtual".to_string(),
        description: "🔊 Desktop Streamer Virtual (Auto-capture system audio)".to_string(),
        device_type: "virtual".to_string(),
    });
    
    if has_pulseaudio {
        detect_pulseaudio_devices(&mut microphone_devices, &mut monitor_devices, &mut default_source);
    }
    
    info!("Audio detection results:");
    info!("  PulseAudio: {}", has_pulseaudio);
    info!("  PipeWire: {}", has_pipewire);
    info!("  ALSA: {}", has_alsa);
    info!("  Microphones found: {}", microphone_devices.len());
    info!("  Monitor devices found: {}", monitor_devices.len());
    
    AudioInfo {
        has_pulseaudio,
        has_pipewire,
        has_alsa,
        default_source,
        microphone_devices,
        monitor_devices,
    }
}

fn detect_pulseaudio_devices(
    microphone_devices: &mut Vec<AudioDevice>,
    monitor_devices: &mut Vec<AudioDevice>,
    default_source: &mut Option<String>
) {
    if let Ok(output) = std::process::Command::new("pactl")
        .args(&["list", "short", "sources"])
        .output()
    {
        let sources_str = String::from_utf8_lossy(&output.stdout);
        for line in sources_str.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                let description = parts.get(4).unwrap_or(&"Unknown").to_string();
                
                // Pular dispositivos do Desktop Streamer para evitar conflitos
                if name.contains("desktop_streamer") {
                    continue;
                }
                
                if name.contains(".monitor") {
                    monitor_devices.push(AudioDevice {
                        name: name.clone(),
                        description: format!("🎵 Monitor: {}", description),
                        device_type: "monitor".to_string(),
                    });
                } else {
                    microphone_devices.push(AudioDevice {
                        name: name.clone(),
                        description: format!("🎤 {}", description),
                        device_type: "source".to_string(),
                    });
                    
                    if default_source.is_none() {
                        *default_source = Some(name);
                    }
                }
            }
        }
    }
}

/// Verifica se um dispositivo específico está disponível
pub fn is_device_available(device_name: &str) -> bool {
    if device_name == "desktop_streamer_virtual" {
        return true; // Sempre disponível, será criado quando necessário
    }
    
    let output = std::process::Command::new("pactl")
        .args(&["list", "short", "sources"])
        .output();
        
    match output {
        Ok(output) if output.status.success() => {
            let sources_str = String::from_utf8_lossy(&output.stdout);
            sources_str.lines().any(|line| line.contains(device_name))
        }
        _ => false
    }
}

/// Obtém informações detalhadas sobre um dispositivo
pub fn get_device_info(device_name: &str) -> Option<AudioDevice> {
    if device_name == "desktop_streamer_virtual" {
        return Some(AudioDevice {
            name: "desktop_streamer_virtual".to_string(),
            description: "🔊 Desktop Streamer Virtual (Auto-capture system audio)".to_string(),
            device_type: "virtual".to_string(),
        });
    }
    
    let audio_info = detect_audio_capabilities();
    
    // Procurar nos dispositivos de monitor
    for device in &audio_info.monitor_devices {
        if device.name == device_name {
            return Some(device.clone());
        }
    }
    
    // Procurar nos dispositivos de microfone
    for device in &audio_info.microphone_devices {
        if device.name == device_name {
            return Some(device.clone());
        }
    }
    
    None
}
