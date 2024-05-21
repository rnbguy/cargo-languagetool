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
pub enum Cargo {
    #[clap(name = "languagetool")]
    LanguageTool(LanguageTool),
}

impl Cargo {
    pub fn run(&self) -> Result<()> {
        let Self::LanguageTool(cmd) = self;

        let server =
            languagetool_rust::ServerClient::new(&cmd.addr, &cmd.port).with_max_suggestions(5);

        check_grammar(&server, &fetch_docs(&cmd.path)?)?;

        Ok(())
    }
}
