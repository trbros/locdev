use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    // LocDev gives the site its port in PORT.
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("127.0.0.1:{port}");

    let app = Router::new().route("/", get(|| async { "It works: Axum, served by LocDev." }));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
