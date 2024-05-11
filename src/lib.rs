//! This main module for cargo grammar checking.
//! Use wisel.

mod doc;
mod utils;

use doc::{Docs, FixedDoc, FixedDocs};

use clap::Parser;
use color_eyre::{eyre::ContextCompat, Result};

#[derive(Parser)]
#[command(version, about)]
pub struct App {
    #[clap(
        short,
        long,
        env = "LT_ADDR",
        default_value = "https://api.languagetoolplus.com/v2"
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

/// Reads the .rs files in the directory recursively.
fn fetch_docs(dir: &str) -> Result<Vec<Docs>> {
    use proc_macro2::TokenStream;

    // dbg!(dir);

    let is_rs = |e: &walkdir::DirEntry| -> bool {
        e.file_type().is_file()
            && e.path()
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("rs"))
    };
    let parse_docs = |path: &String| -> Result<Docs> {
        use std::fs;
        let content = fs::read_to_string(path)?;
        let stream: TokenStream = syn::parse_str(&content)?;
        // dbg!(&stream);
        Ok(Docs::from((path, stream)))
    };

    let files = walkdir::WalkDir::new(dir)
        .max_depth(999)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(is_rs)
        .filter_map(|e| Some(e.path().to_str()?.to_owned()))
        .collect::<Vec<String>>();
    // dbg!(&files);

    files
        .iter()
        .map(parse_docs)
        .filter(|d| d.as_ref().map(|x| x.0.is_empty()).ok() == Some(false))
        .collect::<Result<_>>()
}

async fn doc_checked<'a>(server: &languagetool_rust::ServerClient, doc: &'a mut FixedDoc) {
    let check_request = languagetool_rust::CheckRequest::default()
        .with_text(doc.text.clone())
        .with_language("en-US".to_owned());

    doc.check_response = server.check(&check_request).await.ok();
}

async fn docs_checked<'a>(server: &languagetool_rust::ServerClient, docs: &'a mut FixedDocs) {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            doc_checked(server, doc).await;
        }
    }
}

fn print_response(file: &str, doc: &FixedDoc) -> Result<()> {
    let mut t = term::stdout().context("Failed to get stdout")?;

    if let Some(languagetool_rust::CheckResponse { matches, .. }) = &doc.check_response {
        for m in matches {
            // dbg!(&m);

            let line_width = utils::decimal_places(doc.span.start.line) + 2;

            t.attr(term::Attr::Bold)?;
            t.fg(term::color::RED)?;
            write!(t, "error")?;
            t.fg(term::color::WHITE)?;
            writeln!(t, ": {}", m.short_message)?;
            t.fg(term::color::BLUE)?;
            write!(t, "{:>width$}", "-->", width = line_width)?;
            let _ = t.reset();
            writeln!(t, " {file}:{line}", file = file, line = doc.span.start.line)?;
            t.fg(term::color::BLUE)?;
            t.attr(term::Attr::Bold)?;
            writeln!(t, "{:^width$}| ", " ", width = line_width)?;
            write!(
                t,
                "{line:^width$}| ",
                line = doc.span.start.line,
                width = line_width
            )?;
            let _ = t.reset();
            writeln!(t, "{}", m.sentence)?;
            t.fg(term::color::BLUE)?;
            t.attr(term::Attr::Bold)?;
            write!(t, "{:^width$}| ", " ", width = line_width)?;
            t.fg(term::color::RED)?;
            writeln!(t, "- {}", m.message)?;
            t.fg(term::color::BLUE)?;
            writeln!(t, "{:^width$}| \n", " ", width = line_width)?;
            let _ = t.reset();
            t.flush()?;
        }
    }

    Ok(())
}

/// Pretty-printer.
fn print_docs(docs: &mut FixedDocs) -> Result<()> {
    for (file, docs) in &mut docs.fixed {
        for doc in docs {
            print_response(file, doc)?;
        }
    }
    Ok(())
}

async fn check_grammar(server: &languagetool_rust::ServerClient, docs: &[Docs]) -> Result<()> {
    // dbg!(api_key);
    // dbg!(docs);
    let mut docs_for_grammarly: Vec<FixedDocs> = docs
        .iter()
        .map(|d| FixedDocs::try_from(d.clone()))
        .collect::<Result<_>>()?;
    // dbg!(&docs_for_grammarly);
    for doc in &mut docs_for_grammarly {
        docs_checked(server, doc).await;
        print_docs(doc)?;
    }

    Ok(())
}
