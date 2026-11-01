use loco_rs::{
    app::Hooks,
    boot::{ServeParams, StartMode, create_app, start},
    environment::{Environment, resolve_from_env},
};
use web_loco::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    let environment = Environment::from(resolve_from_env());
    let config = App::load_config(&environment).await?;
    let serve = ServeParams {
        port: config.server.port,
        binding: config.server.binding.clone(),
    };
    let boot = create_app::<App>(StartMode::ServerOnly, &environment, config).await?;
    start::<App>(boot, serve, false).await
}
