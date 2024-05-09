use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    let app = cargo_grammarly::App::parse();
    app.run()
}
