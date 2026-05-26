use tongwen::{app, shutdown_signal};

#[tokio::main]
async fn main() {
    let host = std::env::var("TONGWEN_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TONGWEN_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(1180);

    let app = app();

    let addr_str = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr_str).await.unwrap();
    println!("Listening on http://{}", addr_str);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
