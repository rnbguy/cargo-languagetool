use clap::Parser;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    cargo_grammarly::App::parse().run().await
}
