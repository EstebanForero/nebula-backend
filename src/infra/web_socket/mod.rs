use axum::http;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{
    self,
    handshake::server::{ErrorResponse, Request, Response},
};
use tracing::info;

async fn start_web_socket_api(addr: &str) {
    let listener = TcpListener::bind(addr)
        .await
        .expect("Error binding WS TCP listener");

    info!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        // Validate the headers in the stream

        let ws_stream = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, response| {
            let auth = req
                .headers()
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Bearer "));

            if let Some(token) = auth {
                Ok(response)
            } else {
                // response = Response::builder()
                //     .status(401)
                //     .body("Invalid or missing bearer token")
                //     .unwrap();
                todo!()
            }
        });
    }
}
