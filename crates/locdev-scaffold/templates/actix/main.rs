use actix_web::{App, HttpServer, get};

#[get("/")]
async fn index() -> &'static str {
    "It works: Actix Web, served by LocDev."
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // LocDev gives the site its port in PORT.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    println!("listening on http://127.0.0.1:{port}");
    HttpServer::new(|| App::new().service(index))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
