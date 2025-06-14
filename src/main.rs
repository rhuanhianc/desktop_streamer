use axum::body::Body;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::{Router, routing::get};
use image::buffer;
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use screenshots::Screen;
use std::fs::{self, File};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io::Read, time::Instant};
use tokio::io::BufStream;
use tokio_util::io::ReaderStream;

type SharedState = Arc<Mutex<Vec<u8>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Servidor escutando em http://{}", addr);

    let state: SharedState = Arc::new(Mutex::new(Vec::new()));
    tokio::spawn(capture_loop(Arc::clone(&state)));

    //rota inicial
    let app = Router::new().route("/", get(serve_latest_frame).with_state(state));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn capture_loop(state: SharedState) {
    let screen = Screen::all().unwrap().into_iter().next().unwrap();

    loop {
        let image = screen.capture().unwrap();

        let mut buffer = Vec::new();
        let encoder = PngEncoder::new(&mut buffer);
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgba8.into(),
            )
            .unwrap();

        {
            let mut latest_frame = state.lock().unwrap();
            *latest_frame = buffer;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
async fn serve_latest_frame(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let frame_data = state.lock().unwrap().clone();

    if frame_data.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Aguardando o primeiro frame...",
        ));
    }

    Ok(([(header::CONTENT_TYPE, "image/png")], frame_data))
}
async fn get_file_image() -> Result<Response, (StatusCode, String)> {
    let file_path = "screenshot-0.png";

    let file = match tokio::fs::File::open(&file_path).await {
        Ok(file) => file,
        Err(_) => return Err((StatusCode::NOT_FOUND, "Imagem não encontrada".to_string())),
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((body).into_response())
}
