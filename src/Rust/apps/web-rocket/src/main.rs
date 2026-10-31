//! 起動: `cargo run`。待ち受けの port は環境変数 `ROCKET_PORT`（既定 8000）で変える。

#[rocket::launch]
fn rocket() -> _ {
    web_rocket::app()
}
