use std::path::PathBuf;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::utils::{check_grammar, fetch_docs};

#[derive(Args)]
#[command(version, about)]
pub struct LanguageTool {
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

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
pub enum CargoCli {
    Languagetool(LanguageTool),
}

impl CargoCli {
    pub async fn run(&self) -> Result<()> {
        let Self::Languagetool(cmd) = self;

        let server = languagetool_rust::ServerClient::new(&cmd.addr, &cmd.port);

        check_grammar(&server, &fetch_docs(&cmd.path)?).await?;

        Ok(())
    }
}
