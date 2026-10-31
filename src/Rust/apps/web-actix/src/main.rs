//! 起動: `PORT`（既定 8080）で待ち受ける。外から見えないよう 127.0.0.1 にだけ bind する。

use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = match std::env::var("PORT") {
        Ok(p) => p.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("PORT が不正: {e}"),
            )
        })?,
        Err(_) => 8080,
    };
    println!("http://127.0.0.1:{port} で待ち受ける");
    HttpServer::new(|| App::new().configure(web_actix::configure))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
