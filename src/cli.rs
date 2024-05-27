use std::path::PathBuf;

use clap::{Args, Parser};
use color_eyre::eyre::ContextCompat;
use color_eyre::Result;

use crate::cache::sled::SledCacheStore;
use crate::cache::Cacheable;
use crate::languagetool::categories::Categories;
use crate::utils::{check_and_annotate, fetch_docs};

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser)]
pub struct Config {
    #[clap(
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

    #[clap(long, help = "Disable cache query.")]
    pub no_cache: bool,

    #[clap(long, help = "Show all doc comments (even cached).")]
    pub show_all: bool,
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
    /// Run the command.
    ///
    /// # Errors
    /// If an error occurs.
    pub fn run(&self) -> Result<()> {
        let Self::LanguageTool(cmd) = self;

        let server = languagetool_rust::ServerClient::new(
            &cmd.config.hostname,
            cmd.config.port.as_deref().unwrap_or(""),
        )
        .with_max_suggestions(5);

        let docs = cmd
            .paths
            .iter()
            .map(fetch_docs)
            .try_fold::<_, _, Result<_>>(Vec::new(), |mut acc, docs_result| {
                acc.extend(docs_result?);
                Ok(acc)
            })?;

        let n_files = docs.len();

        let project_dir = directories::ProjectDirs::from("rnbguy", "github", "cargo-languagetool")
            .context("failed to get cache directory")?;
        let cache = SledCacheStore::new(project_dir.cache_dir())?;

        check_and_annotate(&server, &cmd.config, docs, &cache)?;

        println!("Checked {n_files} files.");

        Ok(())
    }
}
