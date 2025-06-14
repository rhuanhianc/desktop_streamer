use axum::body::Body;
use axum::http::StatusCode;
use axum::{Router, routing::get};
use axum::response::{AppendHeaders, IntoResponse, Response};
use image::buffer;
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use screenshots::Screen;
use tokio::io::BufStream;
use tokio_util::io::ReaderStream;
use std::fs::{self, File};
use std::net::SocketAddr;
use std::{io::Read, time::Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Servidor escutando em http://{}", addr);

    let start = Instant::now();

    let screens = Screen::all()?;
    println!("Monitores encontrados: {}", screens.len());

    for (i, screen) in screens.iter().enumerate() {
        println!(
            "Capturando monitor {} (ID: {}, Resolução: {}x{})",
            i, screen.display_info.id, screen.display_info.width, screen.display_info.height
        );

        let image = screen.capture()?;
        println!("Tamenho do buffer RAW: {}KB", image.len() / 1024);
        let mut compressed_buffer = Vec::new();
        let encoder = PngEncoder::new(&mut compressed_buffer);

        encoder.write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )?;

        let file_name = format!("screenshot-{}.png", i);
        fs::write(&file_name, &compressed_buffer)?;
        println!("Imagem salva como '{}' e com tamanho:", file_name);
    }
    //rota inicial
    let app = Router::new().route("/", get(get_file_image));

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    let duration = start.elapsed();
    println!(
        "\nTodas as capturas concluídas com sucesso em {:?}",
        duration
    );

    Ok(())
}

async fn get_file_image() -> Result<Response, (StatusCode, String)>{
   let file_path = "screenshot-0.png";

   let file = match tokio::fs::File::open(&file_path).await {
       Ok(file) => file,
       Err(_) => return Err((StatusCode::NOT_FOUND, "Imagem não encontrada".to_string())),
   };

   let stream = ReaderStream::new(file);
   let body = Body::from_stream(stream);

   Ok((body).into_response())
}
