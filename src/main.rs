use std::net::SocketAddr;

use axum::routing::get;
use axum::Router;

mod routes;

const ADDR: &str = "192.168.0.10:3000";

#[tokio::main]
async fn main() {
    let api_route = routes::api_router().await;
    let router = Router::new()
        .merge(api_route)
        .route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind(ADDR).await;

    let listener = match listener {
        Ok(listener) => {
            println!("Listening on: {}", ADDR);
            listener
        }
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            return;
        }
    };

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap_or_else(|e| eprintln!("Server failed: {}", e));
}

async fn shutdown_signal() {
    println!("Press Ctrl+C to shutdown the server.");
    tokio::signal::ctrl_c().await.unwrap();
    println!("Shutting down the server.");
}
