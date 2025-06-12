use image::buffer;
use screenshots::Screen;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    let screens = Screen::all()?;
    println!("Monitores encontrados: {}", screens.len());

    for (i, screen) in screens.iter().enumerate() {
        println!(
            "Capturando monitor {} (ID: {}, Resolução: {}x{})",
            i, screen.display_info.id, screen.display_info.width, screen.display_info.height
        );
        
        let image = screen.capture()?;

        let file_name = format!("screenshot-{}.png", i);
        image.save(&file_name)?;
        
        println!("Imagem salva como '{}'", file_name);
    }

    let duration = start.elapsed();
    println!("\nTodas as capturas concluídas com sucesso em {:?}", duration);

    Ok(())
}