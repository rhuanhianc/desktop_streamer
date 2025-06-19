use anyhow::Result;
use tracing::{info, warn};
use std::process::Command;

/// Gerenciador de dispositivos de áudio virtuais
#[derive(Debug)]
pub struct VirtualAudioDevice {
    pub sink_name: String,
    pub sink_description: String,
    pub source_name: String,
    pub source_description: String,
    pub sink_module_id: Option<u32>,
    pub source_module_id: Option<u32>,
}

impl VirtualAudioDevice {
    pub fn new() -> Self {
        Self {
            sink_name: "desktop_streamer_output".to_string(),
            sink_description: "Desktop Streamer Output (Send to Mobile)".to_string(),
            source_name: "desktop_streamer_input".to_string(),
            source_description: "Desktop Streamer Input (Mobile Microphone)".to_string(),
            sink_module_id: None,
            source_module_id: None,
        }
    }

    /// Cria os dispositivos virtuais no sistema de áudio
    pub fn create_virtual_devices(&mut self) -> Result<()> {
        info!("Creating virtual audio devices...");
        
        // Criar sink virtual (dispositivo de saída que enviará áudio para o mobile)
        self.create_virtual_sink()?;
        
        // Criar source virtual (dispositivo de entrada que receberá áudio do microfone do mobile)
        self.create_virtual_source()?;
        
        Ok(())
    }

    fn create_virtual_sink(&mut self) -> Result<()> {
        let sink_result = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", self.sink_name),
                &format!("sink_properties=device.description='{}'", self.sink_description),
                "rate=48000",
                "channels=2"
            ])
            .output();

        match sink_result {
            Ok(output) if output.status.success() => {
                let module_id_output = String::from_utf8_lossy(&output.stdout);
                let module_id_str = module_id_output.trim();
                if let Ok(module_id) = module_id_str.parse::<u32>() {
                    self.sink_module_id = Some(module_id);
                    info!("Created virtual sink '{}' with module ID: {}", self.sink_name, module_id);
                } else {
                    warn!("Could not parse sink module ID: {}", module_id_str);
                }
            }
            Ok(output) => {
                warn!("Failed to create virtual sink: {}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                warn!("Error creating virtual sink: {}", e);
            }
        }

        Ok(())
    }

    fn create_virtual_source(&mut self) -> Result<()> {
        // Criar um sink para o loopback que funcionará como source
        let source_result = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={}_loopback", self.source_name),
                &format!("sink_properties=device.description='{} Loopback'", self.source_description),
                "rate=48000",
                "channels=2"
            ])
            .output();

        match source_result {
            Ok(output) if output.status.success() => {
                let module_id_output = String::from_utf8_lossy(&output.stdout);
                let module_id_str = module_id_output.trim();
                if let Ok(module_id) = module_id_str.parse::<u32>() {
                    self.source_module_id = Some(module_id);
                    info!("Created virtual source '{}' with module ID: {}", self.source_name, module_id);
                } else {
                    warn!("Could not parse source module ID: {}", module_id_str);
                }
            }
            Ok(output) => {
                warn!("Failed to create virtual source: {}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                warn!("Error creating virtual source: {}", e);
            }
        }

        Ok(())
    }

    /// Remove os dispositivos virtuais do sistema
    pub fn remove_virtual_devices(&self) -> Result<()> {
        info!("Removing virtual audio devices...");

        if let Some(sink_id) = self.sink_module_id {
            self.remove_module(sink_id, "sink")?;
        }

        if let Some(source_id) = self.source_module_id {
            self.remove_module(source_id, "source")?;
        }

        Ok(())
    }

    fn remove_module(&self, module_id: u32, device_type: &str) -> Result<()> {
        let result = Command::new("pactl")
            .args(["unload-module", &module_id.to_string()])
            .output();
        
        match result {
            Ok(output) if output.status.success() => {
                info!("Removed virtual {} module {}", device_type, module_id);
            }
            Ok(output) => {
                warn!("Failed to remove {} module {}: {}", 
                     device_type, module_id, String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                warn!("Error removing {} module {}: {}", device_type, module_id, e);
            }
        }
        
        Ok(())
    }

    /// Remove todos os dispositivos virtuais desktop_streamer órfãos do sistema
    pub fn remove_all_orphan_devices() -> Result<()> {
        info!("Removing all orphan desktop_streamer audio devices...");
        
        // Obter lista de módulos carregados
        let output = Command::new("pactl")
            .args(["list", "modules", "short"])
            .output()?;
            
        let modules_output = String::from_utf8_lossy(&output.stdout);
        let mut removed_count = 0;
        
        for line in modules_output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let module_id = parts[0];
                let module_type = parts[1];
                let module_args = parts[2..].join(" ");
                
                // Verificar se é um módulo null-sink com nome desktop_streamer
                if module_type == "module-null-sink" && 
                   (module_args.contains("desktop_streamer_output") || 
                    module_args.contains("desktop_streamer_input")) {
                    
                    if let Ok(id) = module_id.parse::<u32>() {
                        let result = Command::new("pactl")
                            .args(["unload-module", &id.to_string()])
                            .output();
                            
                        match result {
                            Ok(output) if output.status.success() => {
                                info!("Removed orphan module {} ({})", id, module_args);
                                removed_count += 1;
                            }
                            Ok(output) => {
                                warn!("Failed to remove module {}: {}", 
                                     id, String::from_utf8_lossy(&output.stderr));
                            }
                            Err(e) => {
                                warn!("Error removing module {}: {}", id, e);
                            }
                        }
                    }
                }
            }
        }
        
        info!("Removed {} orphan desktop_streamer audio devices", removed_count);
        Ok(())
    }

    /// Retorna o nome do monitor do sink virtual (usado para captura de áudio)
    pub fn get_monitor_source_name(&self) -> String {
        format!("{}.monitor", self.sink_name)
    }

    /// Retorna o nome do source virtual (usado para entrada de microfone)
    pub fn get_source_name(&self) -> String {
        format!("{}_loopback.monitor", self.source_name)
    }

    /// Retorna o nome do sink virtual (para onde o sistema deve enviar áudio)
    pub fn get_sink_name(&self) -> String {
        self.sink_name.clone()
    }

    /// Retorna o nome do sink de entrada (onde o áudio do microfone será reproduzido)
    pub fn get_input_sink_name(&self) -> String {
        format!("{}_loopback", self.source_name)
    }
}

impl Drop for VirtualAudioDevice {
    fn drop(&mut self) {
        if let Err(e) = self.remove_virtual_devices() {
            warn!("Failed to cleanup virtual audio devices: {}", e);
        }
    }
}
