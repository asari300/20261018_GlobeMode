//! 起動: `PORT`（既定 8080）で待ち受ける。外から見えないよう 127.0.0.1 にだけ bind する。

use web_axum::app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = match std::env::var("PORT") {
        Ok(p) => p.parse().map_err(|e| format!("PORT が不正: {e}"))?,
        Err(_) => 8080,
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("http://127.0.0.1:{port} で待ち受ける");
    axum::serve(listener, app()).await?;
    Ok(())
}
