use tongwen::{app, config, shutdown_signal};

#[tokio::main]
async fn main() {
    let cfg = config::get();
    let addr_str = format!("{}:{}", cfg.host, cfg.port);

    let app = app();
    let listener = tokio::net::TcpListener::bind(&addr_str).await.unwrap();
    println!("Listening on http://{}", addr_str);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
