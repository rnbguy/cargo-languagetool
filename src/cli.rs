use clap::Parser;
use color_eyre::Result;

use crate::utils::{check_grammar, fetch_docs};

#[derive(Parser)]
#[command(version, about)]
pub struct App {
    #[clap(
        short,
        long,
        env = "LT_ADDR",
        default_value = "https://api.languagetoolplus.com"
    )]
    addr: String,

    #[clap(short, long, env = "LT_PORT", default_value = "")]
    port: String,
}

impl App {
    pub async fn run(&self) -> Result<()> {
        let source_directory = format!("{}/src", std::env::var("PWD")?);

        let server = languagetool_rust::ServerClient::new(&self.addr, &self.port);

        check_grammar(&server, &fetch_docs(&source_directory)?).await?;

        Ok(())
    }
}
