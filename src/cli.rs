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
        env = "LANGUAGETOOL_HOSTNAME",
        default_value = "https://api.languagetoolplus.com"
    )]
    hostname: String,

    #[clap(short, long, env = "LANGUAGETOOL_PORT", default_value = "")]
    port: String,

    #[clap(default_value = ".")]
    paths: Vec<PathBuf>,
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
            languagetool_rust::ServerClient::new(&cmd.hostname, &cmd.port).with_max_suggestions(5);

        let docs_result: Result<Vec<_>> =
            cmd.paths
                .iter()
                .map(fetch_docs)
                .try_fold(Vec::new(), |mut acc, docs_result| {
                    acc.extend(docs_result?);
                    Ok(acc)
                });

        let docs = docs_result?;

        check_grammar(&server, &docs)?;

        println!("Checked {} files.", docs.len());

        Ok(())
    }
}
