use std::{thread, time::{self, Duration, Instant}};
use xcap::Monitor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Teste de captura de tela");

    let monitors = Monitor::all()?;
    println!("Numero de monirores: {}", monitors.len());

    // let primary_monitor = monitors
    //     .into_iter()
    //     .find(|m| m.is_primary().is_ok_and(|primary| primary))
    //     .ok_or("Não foi encontrado nenhum monitor")?;

    for (i, monitor) in monitors.iter().enumerate() {
        let monitor_name = monitor.name()?;
        println!("Moninor sendo capturado {}", monitor_name);

        let start_time = Instant::now();
        let image = monitor.capture_image()?;

        let durarion = start_time.elapsed();
        println!("Captura de tela em: {:?}", durarion);

        let file_name = format!("screenshot-monit-{}.png", i);

        image.save(file_name)?;
        println!("Imagem salva");
        thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}
