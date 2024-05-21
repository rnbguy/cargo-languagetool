use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    cargo_languagetool::cli::Cargo::parse().run()
}
