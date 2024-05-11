use std::path::PathBuf;

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

    #[clap(long, default_value = ".")]
    path: PathBuf,
}

impl App {
    pub async fn run(&self) -> Result<()> {
        let server = languagetool_rust::ServerClient::new(&self.addr, &self.port);

        check_grammar(&server, &fetch_docs(&self.path)?).await?;

        Ok(())
    }
}
