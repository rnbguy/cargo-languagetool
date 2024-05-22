use std::path::PathBuf;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::{
    languagetool::Categories,
    utils::{check_grammar, fetch_docs},
};

#[derive(Parser)]
pub struct Config {
    #[clap(
        short,
        long,
        env = "LANGUAGETOOL_HOSTNAME",
        default_value = "https://api.languagetoolplus.com"
    )]
    pub hostname: String,
    #[clap(short, long, env = "LANGUAGETOOL_PORT")]
    pub port: Option<String>,

    #[clap(short, long, env = "LANGUAGETOOL_USERNAME")]
    pub username: Option<String>,
    #[clap(short, long, env = "LANGUAGETOOL_API_KEY")]
    pub api_key: Option<String>,

    #[clap(long)]
    pub disable_categories: Vec<Categories>,
    #[clap(long)]
    pub enable_categories: Vec<Categories>,

    #[clap(long)]
    pub disable_rules: Vec<String>,
    #[clap(long)]
    pub enable_rules: Vec<String>,

    #[clap(long)]
    pub enable_only: bool,

    #[clap(long, default_value = "en-US")]
    pub language: String,

    #[clap(long)]
    pub picky: bool,
}

#[derive(Args)]
#[command(version, about)]
pub struct LanguageTool {
    #[clap(default_value = ".")]
    paths: Vec<PathBuf>,

    #[clap(flatten)]
    config: Config,
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

        let server = languagetool_rust::ServerClient::new(
            &cmd.config.hostname,
            cmd.config.port.as_deref().unwrap_or(""),
        )
        .with_max_suggestions(5);

        let docs_result: Result<Vec<_>> =
            cmd.paths
                .iter()
                .map(fetch_docs)
                .try_fold(Vec::new(), |mut acc, docs_result| {
                    acc.extend(docs_result?);
                    Ok(acc)
                });

        let docs = docs_result?;

        check_grammar(&server, &cmd.config, &docs)?;

        println!("Checked {} files.", docs.len());

        Ok(())
    }
}
