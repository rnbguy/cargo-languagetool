//! This main module for cargo grammar checking.
//! Use wisely.

mod doc;
mod utils;

use doc::{Docs, FixedDoc, FixedDocs};

const ENVIRONMENT_VARIABLE_NAME: &str = "GRAMMARLY_API_KEY";

use clap::Parser;
use color_eyre::{eyre::ContextCompat, Result};

#[derive(Parser)]
#[command(name = "cargo-grammarly")]
#[command(version, about)]
pub struct App {
    #[clap(short, long, value_name = "API_KEY")]
    api_key: Option<String>,
}

impl App {
    pub fn run(&self) -> Result<()> {
        let api_key = dbg!(std::env::var(ENVIRONMENT_VARIABLE_NAME)
            .ok()
            .or_else(|| self.api_key.clone())
            .context("API key is not provided")?);

        let source_directory = format!("{}/src", std::env::var("PWD")?);
        check_grammar(&api_key, &fetch_docs(&source_directory)?)?;

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

fn doc_checked<'a>(api_key: &str, doc: &'a mut FixedDoc) -> &'a mut FixedDoc {
    doc.check_response = grammarbot_io::Request::from(&doc.text)
        .api_key(api_key)
        .send()
        .ok();
    doc
}

fn docs_checked<'a>(api_key: &str, docs: &'a mut FixedDocs) -> &'a mut FixedDocs {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            let _ = doc_checked(api_key, doc);
        }
    }
    docs
}

fn print_response(file: &str, doc: &FixedDoc) -> Result<()> {
    let mut t = term::stdout().context("Failed to get stdout")?;

    if let Some(grammarbot_io::Response::Success { matches, .. }) = &doc.check_response {
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

fn check_grammar(api_key: &str, docs: &[Docs]) -> Result<()> {
    // dbg!(api_key);
    // dbg!(docs);
    let mut docs_for_grammarly: Vec<FixedDocs> = docs
        .iter()
        .map(|d| FixedDocs::try_from(d.clone()))
        .collect::<Result<_>>()?;
    // dbg!(&docs_for_grammarly);
    for doc in docs_for_grammarly
        .iter_mut()
        .map(|d| docs_checked(api_key, d))
    {
        print_docs(doc)?;
    }

    Ok(())
}
